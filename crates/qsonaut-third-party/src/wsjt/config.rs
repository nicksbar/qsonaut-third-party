use std::time::Duration;

use qsonaut_modems::ModemId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fst4Submode {
    S15,
    S30,
    S60,
    S120,
    S300,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Q65Submode {
    A15,
    A30,
    A60,
    B60,
    C60,
    D60,
    E60,
    D120,
    E120,
    A300,
}

impl Q65Submode {
    pub const fn seconds(self) -> u64 {
        match self {
            Self::A15 => 15,
            Self::A30 => 30,
            Self::A60 | Self::B60 | Self::C60 | Self::D60 | Self::E60 => 60,
            Self::D120 | Self::E120 => 120,
            Self::A300 => 300,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::A15 => "q65-a15",
            Self::A30 => "q65-a30",
            Self::A60 => "q65-a60",
            Self::B60 => "q65-b60",
            Self::C60 => "q65-c60",
            Self::D60 => "q65-d60",
            Self::E60 => "q65-e60",
            Self::D120 => "q65-d120",
            Self::E120 => "q65-e120",
            Self::A300 => "q65-a300",
        }
    }
}

impl Fst4Submode {
    pub const fn seconds(self) -> u64 {
        match self {
            Self::S15 => 15,
            Self::S30 => 30,
            Self::S60 => 60,
            Self::S120 => 120,
            Self::S300 => 300,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::S15 => "fst4-15",
            Self::S30 => "fst4-30",
            Self::S60 => "fst4-60",
            Self::S120 => "fst4-120",
            Self::S300 => "fst4-300",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsjtMode {
    Ft8,
    Ft4,
    Fst4(Fst4Submode),
    Wspr,
    Jt9,
    Jt65,
    Q65(Q65Submode),
    Msk144,
}

impl WsjtMode {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ft8 => "ft8",
            Self::Ft4 => "ft4",
            Self::Fst4(submode) => submode.name(),
            Self::Wspr => "wspr",
            Self::Jt9 => "jt9",
            Self::Jt65 => "jt65",
            Self::Q65(submode) => submode.name(),
            Self::Msk144 => "msk144",
        }
    }

    pub const fn modem_id(self) -> ModemId {
        ModemId(self.name())
    }

    pub const fn slot(self) -> Duration {
        match self {
            Self::Ft8 => Duration::from_secs(15),
            Self::Ft4 => Duration::from_millis(7_500),
            Self::Fst4(submode) => Duration::from_secs(submode.seconds()),
            Self::Wspr => Duration::from_secs(120),
            Self::Jt9 | Self::Jt65 => Duration::from_secs(60),
            Self::Q65(submode) => Duration::from_secs(submode.seconds()),
            Self::Msk144 => Duration::from_secs(15),
        }
    }
}

/// Shared configuration for the protocol adapters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WsjtDecodeConfig {
    pub frequency_min_hz: f32,
    pub frequency_max_hz: f32,
    pub sync_min: f32,
    /// Symmetric time search window around the caller's slot anchor.
    /// Protocols with an asymmetric reference window clamp or extend this
    /// value in their adapter while retaining one consumer-facing setting.
    pub time_tolerance_sec: f32,
    /// Minimum coarse-search score for protocols that expose a normalized
    /// sync score. `sync_min` remains the FT/FST4 threshold.
    pub score_threshold: f32,
    pub max_candidates: usize,
    pub deep_decode: bool,
    pub frequency_hint_hz: Option<f32>,
}

impl Default for WsjtDecodeConfig {
    fn default() -> Self {
        Self {
            frequency_min_hz: 100.0,
            frequency_max_hz: 3_000.0,
            sync_min: 0.6,
            time_tolerance_sec: 2.0,
            score_threshold: 0.1,
            max_candidates: 120,
            deep_decode: false,
            frequency_hint_hz: None,
        }
    }
}
