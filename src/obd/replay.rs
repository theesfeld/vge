//! Replay a capture as a synthetic ELM transport.

use crate::obd::capture::{self, Frame};
use crate::obd::error::{Error, Result};
use crate::obd::transport::Transport;
use std::collections::VecDeque;
use std::path::Path;
use std::time::Duration;

/// Feeds stored RX payloads when the host sends matching TX.
pub struct ReplayTransport {
    label: String,
    frames: VecDeque<Frame>,
    read_buf: Vec<u8>,
    open: bool,
}

impl ReplayTransport {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let frames = capture::load_frames(path)?;
        Ok(Self {
            label: format!("replay:{}", path.display()),
            frames: frames.into(),
            read_buf: Vec::new(),
            open: false,
        })
    }
}

fn norm_hex(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_hexdigit())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

impl Transport for ReplayTransport {
    fn name(&self) -> &str {
        &self.label
    }

    fn open(&mut self) -> Result<()> {
        self.open = true;
        Ok(())
    }

    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        if !self.open {
            return Err(Error::NotOpen);
        }
        let cmd_raw = String::from_utf8_lossy(data);
        let cmd = cmd_raw.trim().trim_end_matches('\r').to_ascii_uppercase();

        // AT/ST: do not consume capture frames
        if cmd.starts_with("AT") || cmd.starts_with("ST") {
            self.read_buf.extend_from_slice(b"OK\r\n>");
            return Ok(());
        }

        let want = norm_hex(&cmd);
        // Find matching TX, then its following RX
        let mut i = 0usize;
        while i < self.frames.len() {
            if self.frames[i].dir != "tx" {
                i += 1;
                continue;
            }
            let tx = norm_hex(&self.frames[i].data);
            if tx == want || want.starts_with(&tx) || tx.starts_with(&want) {
                // remove this TX
                self.frames.remove(i);
                // next RX after it (skip non-rx)
                while i < self.frames.len() {
                    if self.frames[i].dir == "rx" {
                        let rx = self.frames.remove(i).unwrap();
                        let body = format!("{}\r\n>", rx.data);
                        self.read_buf.extend_from_slice(body.as_bytes());
                        return Ok(());
                    }
                    i += 1;
                }
                self.read_buf.extend_from_slice(b"NO DATA\r\n>");
                return Ok(());
            }
            i += 1;
        }
        self.read_buf.extend_from_slice(b"NO DATA\r\n>");
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.read_buf.is_empty() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "replay idle",
            )));
        }
        let n = self.read_buf.len().min(buf.len());
        buf[..n].copy_from_slice(&self.read_buf[..n]);
        self.read_buf.drain(..n);
        Ok(n)
    }

    fn set_timeout(&mut self, _timeout: Duration) -> Result<()> {
        Ok(())
    }

    fn close(&mut self) {
        self.open = false;
    }
}
