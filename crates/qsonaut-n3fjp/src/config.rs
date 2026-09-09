use crate::invalid;
use serde::{Deserialize, Serialize};
use std::io;

/// Persist this in consumer settings. Disabled configurations perform no DNS or I/O.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub connect_timeout_ms: u64,
    pub io_timeout_ms: u64,
    pub max_frame_bytes: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".into(),
            port: 1100,
            connect_timeout_ms: 3000,
            io_timeout_ms: 1000,
            max_frame_bytes: 1024 * 1024,
        }
    }
}
impl Config {
    pub fn station_network() -> Self {
        Self {
            port: 1000,
            ..Self::default()
        }
    }
    pub fn validate(&self) -> io::Result<()> {
        if self.host.trim().is_empty() || self.host.chars().any(char::is_control) || self.port == 0
        {
            return Err(invalid("host and nonzero port required"));
        }
        if self.connect_timeout_ms == 0 || self.io_timeout_ms == 0 {
            return Err(invalid("timeouts must be positive"));
        }
        if !(64..=16 * 1024 * 1024).contains(&self.max_frame_bytes) {
            return Err(invalid("frame limit must be between 64 bytes and 16 MiB"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::value::{Error, MapDeserializer};
    #[test]
    fn persisted_settings_default_to_disabled_and_reject_typos() {
        let empty = std::iter::empty::<(&str, bool)>();
        let default = Config::deserialize(MapDeserializer::<_, Error>::new(empty)).unwrap();
        assert_eq!(default, Config::default());
        let enabled = Config::deserialize(MapDeserializer::<_, Error>::new(
            [("enabled", true)].into_iter(),
        ))
        .unwrap();
        assert!(enabled.enabled);
        assert!(Config::deserialize(MapDeserializer::<_, Error>::new(
            [("enabeld", true)].into_iter()
        ))
        .is_err());
    }
}
