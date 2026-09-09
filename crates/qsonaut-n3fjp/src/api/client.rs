use super::{Command, CommandKind, Decoder, Packet, ProgramInfo, Version};
use crate::{invalid, transport::Transport, Config};
use std::{
    io, thread,
    time::{Duration, Instant},
};

/// Synchronous worker-thread client. There is no hidden worker, retry, or QSO replay.
/// Connection alone performs no queries, subscriptions, or station mutations.
pub struct Client {
    transport: Transport,
    decoder: Decoder,
    program: Option<ProgramInfo>,
    api_version: Option<Version>,
    next_send: Instant,
    max_frame_bytes: usize,
}
impl Client {
    pub fn connect(config: &Config) -> io::Result<Option<Self>> {
        Ok(Transport::connect(config)?.map(|transport| Self {
            transport,
            decoder: Decoder::new(config.max_frame_bytes),
            program: None,
            api_version: None,
            next_send: Instant::now(),
            max_frame_bytes: config.max_frame_bytes,
        }))
    }
    pub fn is_connected(&self) -> bool {
        self.transport.is_open()
    }
    pub fn program(&self) -> Option<&ProgramInfo> {
        self.program.as_ref()
    }
    pub fn api_version(&self) -> Option<Version> {
        self.api_version
    }
    /// Call PROGRAM or APIVER explicitly, then poll; known API versions gate commands.
    /// Success means bytes written, not a remotely saved contact.
    pub fn send(&mut self, command: &Command) -> io::Result<()> {
        if command.kind().spec().requires_transmit_authorization {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "transmit/CAT command requires current consumer authorization",
            ));
        }
        self.send_checked(command, || true)
    }
    /// Consumer must recheck its global arm state in this callback. Authorization is
    /// evaluated after pacing and is never retained by the client. Call stop_transmit
    /// on global disarm; an incoming SEND event is data, never automatically executed.
    pub fn send_with_transmit_authorization(
        &mut self,
        command: &Command,
        authorized: impl FnOnce() -> bool,
    ) -> io::Result<()> {
        self.send_checked(command, authorized)
    }
    fn send_checked(
        &mut self,
        command: &Command,
        authorized: impl FnOnce() -> bool,
    ) -> io::Result<()> {
        let bytes = self.validate_command(command)?;
        if let Some(delay) = self.next_send.checked_duration_since(Instant::now()) {
            thread::sleep(delay);
        }
        if !authorized() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "consumer disarmed transmit",
            ));
        }
        self.transport.send(&bytes)?;
        if command.kind() == CommandKind::Action {
            self.next_send = Instant::now() + Duration::from_millis(5);
        }
        Ok(())
    }
    pub(super) fn validate_command(&self, command: &Command) -> io::Result<Vec<u8>> {
        if self.api_version.is_some_and(|v| !command.supported_by(v)) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "command requires a newer N3FJP API",
            ));
        }
        let bytes = command.encode()?;
        if bytes.len() > self.max_frame_bytes {
            return Err(invalid("outgoing API frame exceeds limit"));
        }
        Ok(bytes)
    }

    /// Explicit emergency release of N3FJP's CW queue, CW key line, and rig TX.
    /// Attempts every release command and reports failure; remote release cannot be
    /// guaranteed over a broken connection. The consumer owns hardware fail-safes.
    pub fn stop_transmit(&mut self) -> io::Result<()> {
        let mut error = None;
        for kind in [
            CommandKind::CwStop,
            CommandKind::CwComPortKeyUp,
            CommandKind::RigRx,
        ] {
            if let Err(e) = self.send(&Command::new(kind)) {
                error.get_or_insert(e);
            }
        }
        error.map_or(Ok(()), Err)
    }
    /// Empty output means timeout or an incomplete frame. EOF/protocol failure closes
    /// this client. Every complete response/event is returned in wire order.
    pub fn poll(&mut self) -> io::Result<Vec<Packet>> {
        let mut buffer = [0; 8192];
        let Some(n) = self.transport.read(&mut buffer)? else {
            return Ok(Vec::new());
        };
        let packets = match self.decoder.feed(&buffer[..n]) {
            Ok(packets) => packets,
            Err(e) => {
                self.transport.close();
                return Err(e);
            }
        };
        for packet in &packets {
            // Preserve malformed/unknown metadata for caller inspection; it must not
            // discard unrelated events received in the same TCP read.
            if let Ok(Some(info)) = packet.program_info() {
                self.api_version = Some(info.api_version);
                self.program = Some(info);
            } else if packet.id() == "APIVERRESPONSE" {
                if let Some(version) = packet.value("APIVER").and_then(|s| s.parse().ok()) {
                    self.api_version = Some(version);
                }
            }
        }
        Ok(packets)
    }
    pub fn disconnect(&mut self) -> io::Result<()> {
        let result = self.transport.send(b"\r\n");
        self.transport.close();
        result
    }
}
