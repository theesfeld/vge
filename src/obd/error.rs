//! OBD / ELM / UDS errors.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Serial(String),
    Adapter(String),
    Timeout,
    Protocol(String),
    Decode(String),
    NotOpen,
    ForbiddenWrite,
    NoData,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Serial(s) => write!(f, "serial: {s}"),
            Error::Adapter(s) => write!(f, "adapter: {s}"),
            Error::Timeout => write!(f, "timeout"),
            Error::Protocol(s) => write!(f, "protocol: {s}"),
            Error::Decode(s) => write!(f, "decode: {s}"),
            Error::NotOpen => write!(f, "transport not open"),
            Error::ForbiddenWrite => write!(f, "write blocked (set MFD_OBD_ALLOW_WRITE=1)"),
            Error::NoData => write!(f, "NO DATA"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
