//! Byte-level transports: serial path and Bluetooth RFCOMM.

use crate::obd::error::{Error, Result};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

/// Bidirectional adapter link (ELM/STN text protocol on top).
pub trait Transport: Send {
    fn name(&self) -> &str;
    fn open(&mut self) -> Result<()>;
    fn write_all(&mut self, data: &[u8]) -> Result<()>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn set_timeout(&mut self, timeout: Duration) -> Result<()>;
    fn close(&mut self);

    fn write_str(&mut self, s: &str) -> Result<()> {
        self.write_all(s.as_bytes())
    }

    /// Read until ELM `>` prompt or timeout.
    fn read_until_prompt(&mut self, timeout: Duration) -> Result<String> {
        let start = Instant::now();
        let mut out = Vec::with_capacity(256);
        let mut buf = [0u8; 64];
        while start.elapsed() < timeout {
            match self.read(&mut buf) {
                Ok(0) => std::thread::sleep(Duration::from_millis(5)),
                Ok(n) => {
                    out.extend_from_slice(&buf[..n]);
                    if out.contains(&b'>') {
                        break;
                    }
                }
                Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if !out.is_empty() && out.contains(&b'>') {
                        break;
                    }
                    if out.is_empty() && start.elapsed() > timeout {
                        return Err(Error::Timeout);
                    }
                }
                Err(Error::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::Interrupted =>
                {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(e) => return Err(e),
            }
        }
        if out.is_empty() {
            return Err(Error::Timeout);
        }
        Ok(String::from_utf8_lossy(&out).into_owned())
    }
}

// ─── Serial (USB / existing rfcomm device node) ──────────────────────────────

pub struct SerialTransport {
    path: String,
    baud: u32,
    timeout: Duration,
    port: Option<Box<dyn serialport::SerialPort>>,
}

impl SerialTransport {
    pub fn new(path: impl Into<String>, baud: u32, timeout: Duration) -> Self {
        Self {
            path: path.into(),
            baud,
            timeout,
            port: None,
        }
    }
}

impl Transport for SerialTransport {
    fn name(&self) -> &str {
        &self.path
    }

    fn open(&mut self) -> Result<()> {
        let port = serialport::new(&self.path, self.baud)
            .timeout(self.timeout)
            .open()
            .map_err(|e| Error::Serial(e.to_string()))?;
        self.port = Some(port);
        Ok(())
    }

    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        let p = self.port.as_mut().ok_or(Error::NotOpen)?;
        p.write_all(data).map_err(Error::Io)?;
        p.flush().map_err(Error::Io)?;
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let p = self.port.as_mut().ok_or(Error::NotOpen)?;
        p.read(buf).map_err(Error::Io)
    }

    fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
        self.timeout = timeout;
        if let Some(p) = self.port.as_mut() {
            p.set_timeout(timeout)
                .map_err(|e| Error::Serial(e.to_string()))?;
        }
        Ok(())
    }

    fn close(&mut self) {
        self.port = None;
    }
}

// ─── Bluetooth classic SPP via Linux RFCOMM socket ───────────────────────────

#[cfg(target_os = "linux")]
mod bt {
    use super::*;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    const AF_BLUETOOTH: libc::c_int = 31;
    const BTPROTO_RFCOMM: libc::c_int = 3;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct BdAddr {
        b: [u8; 6],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SockaddrRc {
        rc_family: libc::sa_family_t,
        rc_bdaddr: BdAddr,
        rc_channel: u8,
    }

    pub fn normalize_mac(mac: &str) -> Option<String> {
        let parts: Vec<&str> = mac.split([':', '-']).collect();
        if parts.len() != 6 {
            return None;
        }
        let mut out = Vec::with_capacity(6);
        for p in parts {
            if p.len() != 2 {
                return None;
            }
            let _ = u8::from_str_radix(p, 16).ok()?;
            out.push(p.to_ascii_uppercase());
        }
        Some(out.join(":"))
    }

    fn parse_bdaddr(mac: &str) -> Result<BdAddr> {
        let norm = normalize_mac(mac)
            .ok_or_else(|| Error::Adapter(format!("invalid Bluetooth MAC: {mac}")))?;
        let mut parts = [0u8; 6];
        for (i, p) in norm.split(':').enumerate() {
            parts[i] = u8::from_str_radix(p, 16)
                .map_err(|e| Error::Adapter(format!("bad MAC byte: {e}")))?;
        }
        // BlueZ stores BDADDR little-endian (reversed).
        parts.reverse();
        Ok(BdAddr { b: parts })
    }

    pub struct BtSppTransport {
        label: String,
        mac: String,
        channel: u8,
        timeout: Duration,
        fd: Option<OwnedFd>,
    }

    impl BtSppTransport {
        pub fn new(mac: impl Into<String>, channel: u8, timeout: Duration) -> Result<Self> {
            let raw = mac.into();
            let mac = normalize_mac(&raw)
                .ok_or_else(|| Error::Adapter(format!("invalid Bluetooth MAC: {raw}")))?;
            Ok(Self {
                label: format!("bt://{mac}"),
                mac,
                channel,
                timeout,
                fd: None,
            })
        }
    }

    impl Transport for BtSppTransport {
        fn name(&self) -> &str {
            &self.label
        }

        fn open(&mut self) -> Result<()> {
            let fd = unsafe { libc::socket(AF_BLUETOOTH, libc::SOCK_STREAM, BTPROTO_RFCOMM) };
            if fd < 0 {
                return Err(Error::Io(std::io::Error::last_os_error()));
            }
            let owned = unsafe { OwnedFd::from_raw_fd(fd) };
            let addr = SockaddrRc {
                rc_family: AF_BLUETOOTH as libc::sa_family_t,
                rc_bdaddr: parse_bdaddr(&self.mac)?,
                rc_channel: self.channel,
            };
            let rc = unsafe {
                libc::connect(
                    owned.as_raw_fd(),
                    &addr as *const _ as *const libc::sockaddr,
                    std::mem::size_of::<SockaddrRc>() as libc::socklen_t,
                )
            };
            if rc != 0 {
                return Err(Error::Adapter(format!(
                    "RFCOMM connect {}:{}: {}",
                    self.mac,
                    self.channel,
                    std::io::Error::last_os_error()
                )));
            }
            // Socket timeouts
            let tv = libc::timeval {
                tv_sec: self.timeout.as_secs() as libc::time_t,
                tv_usec: (self.timeout.subsec_micros()) as libc::suseconds_t,
            };
            unsafe {
                libc::setsockopt(
                    owned.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_RCVTIMEO,
                    &tv as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::timeval>() as libc::socklen_t,
                );
                libc::setsockopt(
                    owned.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_SNDTIMEO,
                    &tv as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::timeval>() as libc::socklen_t,
                );
            }
            self.fd = Some(owned);
            Ok(())
        }

        fn write_all(&mut self, data: &[u8]) -> Result<()> {
            let fd = self.fd.as_ref().ok_or(Error::NotOpen)?.as_raw_fd();
            let mut off = 0;
            while off < data.len() {
                let n = unsafe {
                    libc::write(
                        fd,
                        data[off..].as_ptr() as *const libc::c_void,
                        data.len() - off,
                    )
                };
                if n < 0 {
                    return Err(Error::Io(std::io::Error::last_os_error()));
                }
                off += n as usize;
            }
            Ok(())
        }

        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            let fd = self.fd.as_ref().ok_or(Error::NotOpen)?.as_raw_fd();
            let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n < 0 {
                return Err(Error::Io(std::io::Error::last_os_error()));
            }
            Ok(n as usize)
        }

        fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
            self.timeout = timeout;
            if let Some(fd) = self.fd.as_ref() {
                let tv = libc::timeval {
                    tv_sec: timeout.as_secs() as libc::time_t,
                    tv_usec: timeout.subsec_micros() as libc::suseconds_t,
                };
                unsafe {
                    libc::setsockopt(
                        fd.as_raw_fd(),
                        libc::SOL_SOCKET,
                        libc::SO_RCVTIMEO,
                        &tv as *const _ as *const libc::c_void,
                        std::mem::size_of::<libc::timeval>() as libc::socklen_t,
                    );
                }
            }
            Ok(())
        }

        fn close(&mut self) {
            self.fd = None;
        }
    }
}

#[cfg(target_os = "linux")]
pub use bt::{normalize_mac, BtSppTransport};

#[cfg(not(target_os = "linux"))]
pub fn normalize_mac(mac: &str) -> Option<String> {
    Some(mac.to_string())
}
