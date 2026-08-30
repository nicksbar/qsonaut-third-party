//! Adapters around third-party modem libraries.
//!
//! The public output is the first-party `qsonaut-modems` contract. No GUI,
//! audio-device, radio, Android, QSO, or TX policy code belongs here.

pub mod cw;
mod errors;
pub mod sstv;
pub mod wsjt;

pub use errors::AdapterError;
