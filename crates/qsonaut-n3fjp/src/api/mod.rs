//! Published N3FJP API 2.2: TCP 1100, CMD records, CRLF writes.
mod catalog;
mod client;
mod command;
mod packet;
pub use catalog::{CommandKind, CommandSpec, Version};
pub use client::Client;
pub use command::{Action, Command, Parameter};
pub use packet::{Decoder, Packet, ProgramInfo};

mod entry;
pub use entry::Entry;

#[cfg(test)]
mod tests;
