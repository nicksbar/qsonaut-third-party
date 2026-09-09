//! Opt-in UDP broadcasts for external logging applications.
//! UDP delivery is best effort; persist a QSO before broadcasting it.
use crate::invalid;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub enabled: bool,
    pub destinations: Vec<SocketAddr>,
    pub bind: SocketAddr,
    pub max_payload_bytes: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            destinations: Vec::new(),
            bind: SocketAddr::from(([0, 0, 0, 0], 0)),
            max_payload_bytes: 65_507,
        }
    }
}
impl Config {
    pub fn validate(&self) -> io::Result<()> {
        if self.enabled && self.destinations.is_empty() {
            return Err(invalid(
                "at least one UDP destination is required when enabled",
            ));
        }
        if !(256..=65_507).contains(&self.max_payload_bytes) {
            return Err(invalid(
                "UDP payload limit must be between 256 and 65507 bytes",
            ));
        }
        Ok(())
    }
    pub fn destination(host: &str, port: u16) -> io::Result<SocketAddr> {
        if port == 0 || host.trim().is_empty() {
            return Err(invalid("UDP destination host and port are required"));
        }
        (host.trim(), port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| invalid("UDP destination did not resolve"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Qso {
    pub call: String,
    pub my_call: String,
    pub band: String,
    pub mode: String,
    pub timestamp: String,
    pub qso_date: String,
    pub time_on: String,
    pub rst_sent: String,
    pub rst_received: String,
    pub grid: String,
    pub exchange: String,
    pub comment: String,
    pub station_name: String,
    pub id: String,
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
impl Qso {
    fn field(name: &str, value: &str) -> String {
        format!("<{name}>{}</{name}>", escape(value))
    }
    pub fn n1mm_contactinfo(&self) -> String {
        let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?><contactinfo>");
        for (name, value) in [
            ("app", "QSONaut"),
            ("timestamp", &self.timestamp),
            ("mycall", &self.my_call),
            ("band", &self.band),
            ("mode", &self.mode),
            ("call", &self.call),
            ("gridsquare", &self.grid),
            ("exchangel", &self.exchange),
            ("comment", &self.comment),
            ("snt", &self.rst_sent),
            ("rcv", &self.rst_received),
            ("StationName", &self.station_name),
            ("ID", &self.id),
        ] {
            out.push_str(&Self::field(name, value));
        }
        out.push_str("</contactinfo>");
        out
    }
    pub fn adif(&self) -> String {
        let mut out = String::new();
        for (name, value) in [
            ("CALL", &self.call),
            ("MY_CALLSIGN", &self.my_call),
            ("BAND", &self.band),
            ("MODE", &self.mode),
            ("QSO_DATE", &self.qso_date),
            ("TIME_ON", &self.time_on),
            ("RST_SENT", &self.rst_sent),
            ("RST_RCVD", &self.rst_received),
            ("GRIDSQUARE", &self.grid),
            ("COMMENT", &self.comment),
        ] {
            if !value.is_empty() {
                out.push_str(&format!("<{name}:{}>{value}", value.len()));
            }
        }
        out.push_str("<EOR>");
        out
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    N1mmContactInfo,
    Adif,
}
pub struct Broadcaster {
    socket: UdpSocket,
    config: Config,
}
impl Broadcaster {
    pub fn bind(config: Config) -> io::Result<Self> {
        config.validate()?;
        Ok(Self {
            socket: UdpSocket::bind(config.bind)?,
            config,
        })
    }
    pub fn broadcast(&self, qso: &Qso, format: Format) -> io::Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }
        let payload = match format {
            Format::N1mmContactInfo => qso.n1mm_contactinfo(),
            Format::Adif => qso.adif(),
        };
        if payload.len() > self.config.max_payload_bytes {
            return Err(invalid("UDP payload exceeds configured limit"));
        }
        let mut sent = 0;
        for destination in &self.config.destinations {
            self.socket.send_to(payload.as_bytes(), destination)?;
            sent += 1;
        }
        Ok(sent)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn qso() -> Qso {
        Qso {
            call: "W1AW".into(),
            my_call: "K7TEST".into(),
            band: "20".into(),
            mode: "CW".into(),
            timestamp: "2026-09-09 12:34:56".into(),
            qso_date: "20260909".into(),
            time_on: "123456".into(),
            rst_sent: "599".into(),
            rst_received: "579".into(),
            grid: "FN31".into(),
            exchange: "1A WMA".into(),
            comment: "A & B".into(),
            station_name: "TEST-PC".into(),
            id: "id-1".into(),
        }
    }
    #[test]
    fn packets_escape_and_preserve_qso_fields() {
        let q = qso();
        assert!(q.n1mm_contactinfo().contains("<call>W1AW</call>"));
        assert!(q.n1mm_contactinfo().contains("A &amp; B"));
        assert!(q.adif().contains("<CALL:4>W1AW"));
    }
    #[test]
    fn invalid_config_rejected() {
        let mut c = Config {
            enabled: true,
            ..Config::default()
        };
        assert!(c.validate().is_err());
        c.enabled = false;
        c.max_payload_bytes = 1;
        assert!(c.validate().is_err());
        assert!(Config::destination("", 1000).is_err());
    }
}
