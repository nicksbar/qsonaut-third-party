use crate::Config;
use std::{
    io::{self, Read, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    time::{Duration, Instant},
};

pub(crate) struct Transport {
    stream: TcpStream,
    open: bool,
}
impl Transport {
    pub(crate) fn connect(config: &Config) -> io::Result<Option<Self>> {
        if !config.enabled {
            return Ok(None);
        }
        config.validate()?;
        // OS DNS resolution is synchronous and is outside the TCP timeout budget.
        let addresses = (config.host.as_str(), config.port).to_socket_addrs()?;
        let budget = Duration::from_millis(config.connect_timeout_ms);
        let start = Instant::now();
        let mut error = io::Error::new(io::ErrorKind::AddrNotAvailable, "no resolved addresses");
        for address in addresses {
            let Some(remaining) = budget.checked_sub(start.elapsed()).filter(|d| !d.is_zero())
            else {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "connection deadline",
                ));
            };
            match TcpStream::connect_timeout(&address, remaining) {
                Ok(stream) => {
                    let timeout = Some(Duration::from_millis(config.io_timeout_ms));
                    stream.set_read_timeout(timeout)?;
                    stream.set_write_timeout(timeout)?;
                    stream.set_nodelay(true)?;
                    return Ok(Some(Self { stream, open: true }));
                }
                Err(e) => error = e,
            }
        }
        Err(error)
    }
    fn require_open(&self) -> io::Result<()> {
        if self.open {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "client is disconnected",
            ))
        }
    }
    pub(crate) fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.require_open()?;
        if let Err(e) = self.stream.write_all(bytes) {
            self.close();
            return Err(e);
        }
        Ok(())
    }
    pub(crate) fn read(&mut self, bytes: &mut [u8]) -> io::Result<Option<usize>> {
        self.require_open()?;
        loop {
            match self.stream.read(bytes) {
                Ok(0) => {
                    self.close();
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "N3FJP peer closed connection",
                    ));
                }
                Ok(n) => return Ok(Some(n)),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) =>
                {
                    return Ok(None)
                }
                Err(e) => {
                    self.close();
                    return Err(e);
                }
            }
        }
    }
    pub(crate) fn close(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
        self.open = false;
    }
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }
}
