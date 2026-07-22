//! **Native OBD-II / UDS stack** for MFD (no dependency on defunct obdtui).
//!
//! Layers:
//! - [`transport`] — serial + Linux Bluetooth RFCOMM SPP
//! - [`elm`] — ELM327/STN AT commands
//! - [`j1979`] — Mode 01 PID decode
//! - [`isotp`] — multi-frame reassembly
//! - [`uds`] — session control, tester present, 0x22 DID, gated 0x27
//! - [`capture`] / [`replay`] — log + play back frames
//! - [`feed`] — background poll → vehicle snapshot
//!
//! # Env
//! | Variable | Meaning |
//! |----------|---------|
//! | `MFD_OBD_BT=AA:BB:…` | Bluetooth classic MAC (SPP) |
//! | `MFD_OBD_BT_CHANNEL=1` | RFCOMM channel (default 1) |
//! | `MFD_OBD_PORT=/dev/ttyUSB0` | Serial / rfcomm device |
//! | `MFD_OBD_BAUD=115200` | Serial baud |
//! | `MFD_OBD_REPLAY=path` | Capture dir or frames.ndjson |
//! | `MFD_OBD_ALLOW_WRITE=1` | Allow SecurityAccess / writes |

#![cfg(feature = "obd")]

pub mod capture;
pub mod elm;
pub mod error;
pub mod feed;
pub mod isotp;
pub mod j1979;
pub mod replay;
pub mod session;
pub mod transport;
pub mod uds;

pub use error::{Error, Result};
pub use feed::ObdFeed;
pub use j1979::{decode_mode01, LiveValue, PRIORITY_PIDS};
pub use session::{ConnectOpts, Session};
