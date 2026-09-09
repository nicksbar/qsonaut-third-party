#![doc = include_str!("../README.md")]
//! N3FJP API 2.2 and the separate, captured station-network protocol.
//! Consumers own contact persistence, retry decisions, UI, and radio authorization.
pub mod api;
mod config;
pub mod network;
mod transport;
pub mod udp;
mod wire;
pub use config::Config;
use std::io;
pub(crate) fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
