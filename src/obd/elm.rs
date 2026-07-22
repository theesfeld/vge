//! ELM327 / STN **AT** command layer over a [`Transport`](super::transport::Transport).

use crate::obd::error::{Error, Result};
use crate::obd::transport::Transport;
use std::time::Duration;

/// Live ELM/STN session (text protocol).
pub struct Elm {
    transport: Box<dyn Transport>,
    timeout: Duration,
    identity: String,
    protocol: String,
}

impl Elm {
    pub fn new(transport: Box<dyn Transport>, timeout: Duration) -> Self {
        Self {
            transport,
            timeout,
            identity: String::new(),
            protocol: String::new(),
        }
    }

    pub fn name(&self) -> &str {
        self.transport.name()
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    /// Open link and run standard ELM init (reset, echo off, spaces off, auto protocol).
    pub fn open_and_init(&mut self) -> Result<()> {
        self.transport.open()?;
        // Warm drain
        let _ = self.transport.read_until_prompt(Duration::from_millis(200));
        self.cmd("ATZ")?;
        std::thread::sleep(Duration::from_millis(500));
        let id = self.cmd("ATI")?;
        self.identity = clean_response(&id);
        self.cmd("ATE0")?;
        self.cmd("ATL0")?;
        self.cmd("ATS0")?;
        self.cmd("ATH0")?;
        // Longer headers off; adaptive timing
        let _ = self.cmd("ATAT1");
        let _ = self.cmd("ATSP0"); // auto
        let proto = self.cmd("ATDP")?;
        self.protocol = clean_response(&proto);
        Ok(())
    }

    /// Send AT or OBD command (without CR); returns raw text before `>`.
    pub fn cmd(&mut self, command: &str) -> Result<String> {
        let mut line = command.trim().to_string();
        if !line.ends_with('\r') {
            line.push('\r');
        }
        self.transport.write_str(&line)?;
        let raw = self.transport.read_until_prompt(self.timeout)?;
        if raw.to_ascii_uppercase().contains("UNABLE TO CONNECT") {
            return Err(Error::Protocol("UNABLE TO CONNECT".into()));
        }
        if raw.to_ascii_uppercase().contains("BUS INIT")
            && raw.to_ascii_uppercase().contains("ERROR")
        {
            return Err(Error::Protocol("BUS INIT ERROR".into()));
        }
        Ok(raw)
    }

    /// Request Mode/PID hex payload (e.g. `010C`) and return response hex bytes.
    pub fn request_hex(&mut self, req: &str) -> Result<Vec<u8>> {
        let raw = self.cmd(req)?;
        if raw.to_ascii_uppercase().contains("NO DATA") {
            return Err(Error::NoData);
        }
        parse_elm_hex_payload(&raw)
    }

    /// Set 11-bit CAN header for ECU targeting (e.g. `7E0` for ECM).
    pub fn set_header(&mut self, header_hex: &str) -> Result<()> {
        let _ = self.cmd(&format!("ATSH{header_hex}"))?;
        Ok(())
    }

    /// STN: try dual-bus / MS-CAN helpers when available (ignore failures).
    pub fn try_stn_hints(&mut self) {
        let _ = self.cmd("STP 33"); // ISO 15765-4 CAN 11/500 often already set
        let _ = self.cmd("STCSWM 1"); // allow STN extended if present
    }
}

/// Strip prompts, spaces, line noise → printable summary.
pub fn clean_response(raw: &str) -> String {
    raw.chars()
        .filter(|c| *c != '>' && *c != '\r')
        .collect::<String>()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && *l != "OK" && !l.starts_with("SEARCHING"))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// Collect hex pairs from ELM response lines into bytes.
pub fn parse_elm_hex_payload(raw: &str) -> Result<Vec<u8>> {
    let mut hex = String::new();
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() || t == ">" || t.eq_ignore_ascii_case("OK") {
            continue;
        }
        if t.eq_ignore_ascii_case("SEARCHING...") || t.eq_ignore_ascii_case("SEARCHING") {
            continue;
        }
        if t.eq_ignore_ascii_case("NO DATA") {
            return Err(Error::NoData);
        }
        // Skip lines that are pure AT echoes
        if t.starts_with("AT") || t.starts_with("ST") {
            continue;
        }
        for ch in t.chars() {
            if ch.is_ascii_hexdigit() {
                hex.push(ch);
            }
        }
    }
    if hex.len() < 2 || hex.len() % 2 != 0 {
        return Err(Error::Decode(format!("bad hex payload: {raw:?}")));
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let b = hex.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        let s = std::str::from_utf8(&b[i..i + 2]).unwrap();
        out.push(u8::from_str_radix(s, 16).map_err(|e| Error::Decode(format!("hex byte: {e}")))?);
        i += 2;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mode01_rpm() {
        let raw = "410C0A8E\r\n>";
        let b = parse_elm_hex_payload(raw).unwrap();
        assert_eq!(b, vec![0x41, 0x0C, 0x0A, 0x8E]);
    }
}
