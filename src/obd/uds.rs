//! UDS (ISO 14229) over ELM/ISO-TP — read-focused; writes gated.
//!
//! Services:
//! - `0x10` DiagnosticSessionControl
//! - `0x3E` TesterPresent (keep-alive)
//! - `0x22` ReadDataByIdentifier
//! - `0x27` SecurityAccess (skeleton; requires write gate)

use crate::obd::elm::Elm;
use crate::obd::error::{Error, Result};
use crate::obd::isotp;

/// Default ECM request header on 11-bit OBD (functional often 7DF; physical 7E0).
pub const DEFAULT_ECM_HEADER: &str = "7E0";
pub const DEFAULT_FUNCTIONAL_HEADER: &str = "7DF";

fn writes_allowed() -> bool {
    matches!(
        std::env::var("MFD_OBD_ALLOW_WRITE").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

/// Format bytes as continuous uppercase hex for ELM.
pub fn hex_bytes(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect()
}

/// Send raw UDS request bytes (ISO-TP single-frame via ELM) to current header.
pub fn request(elm: &mut Elm, payload: &[u8]) -> Result<Vec<u8>> {
    let cmd = hex_bytes(payload);
    let raw = elm.cmd(&cmd)?;
    if raw.to_ascii_uppercase().contains("NO DATA") {
        return Err(Error::NoData);
    }
    let bytes = crate::obd::elm::parse_elm_hex_payload(&raw)?;
    // ELM may return with or without PCI
    isotp::reassemble(&bytes).or(Ok(bytes))
}

/// `0x10` DiagnosticSessionControl — session type e.g. `0x01` default, `0x03` extended.
pub fn diagnostic_session_control(elm: &mut Elm, session: u8) -> Result<Vec<u8>> {
    // Session control can be considered a state change; allow by default for extended read.
    request(elm, &[0x10, session])
}

/// `0x3E 0x00` TesterPresent (keep-alive), suppressPosRsp optional via 0x80 bit.
pub fn tester_present(elm: &mut Elm, suppress_pos: bool) -> Result<()> {
    let sub = if suppress_pos { 0x80 } else { 0x00 };
    match request(elm, &[0x3E, sub]) {
        Ok(_) | Err(Error::NoData) => Ok(()),
        Err(e) => Err(e),
    }
}

/// `0x22` ReadDataByIdentifier — 16-bit DID.
pub fn read_data_by_identifier(elm: &mut Elm, did: u16) -> Result<Vec<u8>> {
    let hi = (did >> 8) as u8;
    let lo = (did & 0xFF) as u8;
    let resp = request(elm, &[0x22, hi, lo])?;
    // Positive: 0x62 DID_H DID_L data…
    if resp.first() == Some(&0x7F) {
        return Err(Error::Protocol(format!("UDS NRC: {resp:02X?}")));
    }
    if resp.len() >= 3 && resp[0] == 0x62 {
        return Ok(resp[3..].to_vec());
    }
    // Some adapters return payload without SID echo strip issues
    Ok(resp)
}

/// `0x27` SecurityAccess — **gated**. seed request (odd subfn) / key (even).
pub fn security_access(elm: &mut Elm, sub_function: u8, key: Option<&[u8]>) -> Result<Vec<u8>> {
    if !writes_allowed() {
        return Err(Error::ForbiddenWrite);
    }
    let mut p = vec![0x27, sub_function];
    if let Some(k) = key {
        p.extend_from_slice(k);
    }
    request(elm, &p)
}

/// Convenience: set header, extended session, keep-alive, read DID.
pub fn read_did_on_module(elm: &mut Elm, header_hex: &str, did: u16) -> Result<Vec<u8>> {
    elm.set_header(header_hex)?;
    let _ = diagnostic_session_control(elm, 0x03);
    let _ = tester_present(elm, true);
    read_data_by_identifier(elm, did)
}

/// Common DIDs worth probing on many ECUs (may NRC on some).
pub const PROBE_DIDS: &[(u16, &str)] = &[
    (0xF190, "VIN"),
    (0xF191, "vehicle_manufacturer_ecu_hw"),
    (0xF18C, "ecu_serial"),
    (0xF187, "spare_part_number"),
    (0xF189, "hw_version"),
    (0xF189, "sw_version_alt"),
    (0xF1A0, "approval"),
];
