//! ISO-TP (ISO 15765-2) helpers for multi-frame payloads from ELM text.
//!
//! ELM often returns PCI + data mixed as continuous hex. This module reassembles
//! first-frame + consecutive-frame style streams when present.

use crate::obd::error::{Error, Result};

/// Reassemble ISO-TP payload from a sequence of CAN data bytes (no CAN ID).
///
/// Accepts:
/// - Single frame: `0x0N <N data bytes>`
/// - First + consecutive: `0x1N LL …` then `0x2x …`
/// - Already-stripped UDS payload (no PCI) when `raw[0]` looks like a UDS SID
pub fn reassemble(raw: &[u8]) -> Result<Vec<u8>> {
    if raw.is_empty() {
        return Err(Error::Decode("empty ISO-TP".into()));
    }
    let pci = raw[0] >> 4;
    match pci {
        0x0 => {
            // Single frame
            let len = (raw[0] & 0x0F) as usize;
            if raw.len() < 1 + len {
                return Err(Error::Decode("short single frame".into()));
            }
            Ok(raw[1..1 + len].to_vec())
        }
        0x1 => {
            // First frame: length in low nibble + next byte
            if raw.len() < 2 {
                return Err(Error::Decode("short first frame".into()));
            }
            let len = (((raw[0] & 0x0F) as usize) << 8) | (raw[1] as usize);
            let mut out = Vec::with_capacity(len);
            out.extend_from_slice(&raw[2..]);
            // Remaining should be CF bytes interleaved; ELM may already strip IDs
            // and concatenate. Take until len.
            if out.len() >= len {
                out.truncate(len);
                return Ok(out);
            }
            // Re-parse CF (0x2x) from remainder; ELM may also concatenate payload.
            let mut data = Vec::with_capacity(len);
            data.extend_from_slice(&raw[2..raw.len().min(2 + 6)]);
            let mut idx = 2 + data.len();
            while data.len() < len && idx < raw.len() {
                let b = raw[idx];
                if b >> 4 == 0x2 {
                    idx += 1;
                    let take = (len - data.len()).min(7).min(raw.len() - idx);
                    data.extend_from_slice(&raw[idx..idx + take]);
                    idx += take;
                } else {
                    data.extend_from_slice(&raw[idx..]);
                    break;
                }
            }
            data.truncate(len);
            if data.len() < len {
                return Err(Error::Decode(format!(
                    "ISO-TP incomplete {}/{}",
                    data.len(),
                    len
                )));
            }
            Ok(data)
        }
        _ => {
            // No PCI — assume UDS payload already
            Ok(raw.to_vec())
        }
    }
}

/// Parse multi-line ELM hex that may include CAN IDs (when ATH1) or pure data.
pub fn bytes_from_elm_lines(lines: &str) -> Result<Vec<u8>> {
    use crate::obd::elm::parse_elm_hex_payload;
    parse_elm_hex_payload(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_frame_uds() {
        // SF len 3: 62 F1 90
        let p = reassemble(&[0x03, 0x62, 0xF1, 0x90]).unwrap();
        assert_eq!(p, vec![0x62, 0xF1, 0x90]);
    }

    #[test]
    fn passthrough_mode01() {
        let p = reassemble(&[0x41, 0x0C, 0x0A, 0x8E]).unwrap();
        assert_eq!(p[0], 0x41);
    }
}
