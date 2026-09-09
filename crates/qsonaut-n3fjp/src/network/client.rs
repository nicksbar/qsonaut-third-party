use super::{encode, Decoder, Message};
use crate::{invalid, transport::Transport, Config};
use std::{
    io,
    time::{Duration, Instant},
};

/// Explicit station-network session. No automatic log synchronization or replay.
pub struct Client {
    transport: Transport,
    decoder: Decoder,
    max_frame_bytes: usize,
    last_check: Instant,
}
impl Client {
    pub fn connect(config: &Config) -> io::Result<Option<Self>> {
        Ok(Transport::connect(config)?.map(|transport| Self {
            transport,
            decoder: Decoder::new(config.max_frame_bytes),
            max_frame_bytes: config.max_frame_bytes,
            last_check: Instant::now(),
        }))
    }
    pub fn is_connected(&self) -> bool {
        self.transport.is_open()
    }
    /// Announce this station, open its network session, and request the roster.
    /// Server greetings/CLEAR/LIST are delivered through poll, not treated as log edits.
    pub fn open_session(&mut self, station: &str, band: &str, mode: &str) -> io::Result<()> {
        let announce = Message::BandMode {
            station: station.into(),
            band: band.into(),
            mode: mode.into(),
        };
        // Validate the announcement before sending any part of the sequence.
        encode(&announce)?;
        self.send(&announce)?;
        self.send(&Message::Open)?;
        self.send(&Message::Who(Vec::new()))
    }
    pub fn send(&mut self, message: &Message) -> io::Result<()> {
        let bytes = encode(message)?;
        if bytes.len() > self.max_frame_bytes {
            return Err(invalid("outgoing network frame exceeds limit"));
        }
        self.transport.send(&bytes)
    }
    /// Caller schedules this tick. Returns true when a CHECK was sent.
    pub fn heartbeat(&mut self, interval: Duration) -> io::Result<bool> {
        if interval.is_zero() {
            return Err(invalid("heartbeat interval must be positive"));
        }
        if self.last_check.elapsed() < interval {
            return Ok(false);
        }
        self.send(&Message::Check)?;
        self.last_check = Instant::now();
        Ok(true)
    }
    pub fn poll(&mut self) -> io::Result<Vec<Message>> {
        let mut buffer = [0; 8192];
        let Some(n) = self.transport.read(&mut buffer)? else {
            return Ok(Vec::new());
        };
        match self.decoder.feed(&buffer[..n]) {
            Ok(messages) => Ok(messages),
            Err(e) => {
                self.transport.close();
                Err(e)
            }
        }
    }
    pub fn disconnect(&mut self) {
        self.transport.close();
    }
}
