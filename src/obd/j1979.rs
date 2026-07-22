//! SAE J1979 Mode 01 PID decode (standard powertrain).

use crate::obd::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct LiveValue {
    pub name: &'static str,
    pub value: f64,
    pub unit: &'static str,
    pub mode: u8,
    pub pid: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct PidDef {
    pub pid: u8,
    pub name: &'static str,
    pub unit: &'static str,
    pub bytes: u8,
}

/// Priority poll order for live dashboards.
pub const PRIORITY_PIDS: &[u8] = &[
    0x0C, // RPM
    0x0D, // Speed
    0x11, // TPS
    0x04, // Load
    0x05, // Coolant
    0x0F, // IAT
    0x2F, // Fuel level
    0x42, // Control module voltage
    0x5C, // Oil temp
    0x10, // MAF
    0x46, // Ambient
];

pub fn pid_def(pid: u8) -> Option<PidDef> {
    Some(match pid {
        0x04 => PidDef {
            pid,
            name: "engine_load",
            unit: "%",
            bytes: 1,
        },
        0x05 => PidDef {
            pid,
            name: "coolant_temp",
            unit: "C",
            bytes: 1,
        },
        0x0C => PidDef {
            pid,
            name: "engine_rpm",
            unit: "rpm",
            bytes: 2,
        },
        0x0D => PidDef {
            pid,
            name: "vehicle_speed",
            unit: "km/h",
            bytes: 1,
        },
        0x0F => PidDef {
            pid,
            name: "intake_temp",
            unit: "C",
            bytes: 1,
        },
        0x10 => PidDef {
            pid,
            name: "maf",
            unit: "g/s",
            bytes: 2,
        },
        0x11 => PidDef {
            pid,
            name: "throttle",
            unit: "%",
            bytes: 1,
        },
        0x2F => PidDef {
            pid,
            name: "fuel_level",
            unit: "%",
            bytes: 1,
        },
        0x42 => PidDef {
            pid,
            name: "control_module_voltage",
            unit: "V",
            bytes: 2,
        },
        0x46 => PidDef {
            pid,
            name: "ambient_temp",
            unit: "C",
            bytes: 1,
        },
        0x5C => PidDef {
            pid,
            name: "oil_temp",
            unit: "C",
            bytes: 1,
        },
        _ => return None,
    })
}

/// Decode Mode 01 response bytes (`41 <pid> <data…>`).
pub fn decode_mode01(payload: &[u8]) -> Result<LiveValue> {
    if payload.len() < 3 {
        return Err(Error::Decode(format!("short Mode01: {payload:02X?}")));
    }
    if payload[0] != 0x41 {
        return Err(Error::Decode(format!(
            "not Mode01 positive: {payload:02X?}"
        )));
    }
    let pid = payload[1];
    let data = &payload[2..];
    let def = pid_def(pid).ok_or_else(|| Error::Decode(format!("unknown PID {pid:02X}")))?;
    if data.len() < def.bytes as usize {
        return Err(Error::Decode(format!(
            "PID {pid:02X} needs {} bytes, got {}",
            def.bytes,
            data.len()
        )));
    }
    let value = match pid {
        0x04 | 0x11 | 0x2F => data[0] as f64 * 100.0 / 255.0,
        0x05 | 0x0F | 0x46 | 0x5C => data[0] as f64 - 40.0,
        0x0C => ((data[0] as u16) << 8 | data[1] as u16) as f64 / 4.0,
        0x0D => data[0] as f64,
        0x10 => ((data[0] as u16) << 8 | data[1] as u16) as f64 / 100.0,
        0x42 => ((data[0] as u16) << 8 | data[1] as u16) as f64 / 1000.0,
        _ => return Err(Error::Decode(format!("no formula for PID {pid:02X}"))),
    };
    Ok(LiveValue {
        name: def.name,
        value,
        unit: def.unit,
        mode: 1,
        pid,
    })
}

pub fn mode01_command(pid: u8) -> String {
    format!("01{pid:02X}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpm_from_capture() {
        // 410C0A8E → (0x0A8E)/4 = 675.5
        let v = decode_mode01(&[0x41, 0x0C, 0x0A, 0x8E]).unwrap();
        assert_eq!(v.name, "engine_rpm");
        assert!((v.value - 675.5).abs() < 0.01);
    }

    #[test]
    fn speed_zero() {
        let v = decode_mode01(&[0x41, 0x0D, 0x00]).unwrap();
        assert_eq!(v.value, 0.0);
    }
}
