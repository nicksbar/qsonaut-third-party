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

    /// Exact `mfsk-core` registry name for this adapter mode.
    pub const fn upstream_name(self) -> &'static str {
        match self {
            Self::Ft8 => "FT8",
            Self::Ft4 => "FT4",
            Self::Fst4(Fst4Submode::S15) => "FST4-15",
            Self::Fst4(Fst4Submode::S30) => "FST4-30",
            Self::Fst4(Fst4Submode::S60) => "FST4-60A",
            Self::Fst4(Fst4Submode::S120) => "FST4-120",
            Self::Fst4(Fst4Submode::S300) => "FST4-300",
            Self::Wspr => "WSPR",
            Self::Jt9 => "JT9",
            Self::Jt65 => "JT65",
            Self::Q65(Q65Submode::A15) => "Q65-15A",
            Self::Q65(Q65Submode::A30) => "Q65-30A",
            Self::Q65(Q65Submode::A60) => "Q65-60A",
            Self::Q65(Q65Submode::B60) => "Q65-60B",
            Self::Q65(Q65Submode::C60) => "Q65-60C",
            Self::Q65(Q65Submode::D60) => "Q65-60D",
            Self::Q65(Q65Submode::E60) => "Q65-60E",
            Self::Q65(Q65Submode::D120) => "Q65-120D",
            Self::Q65(Q65Submode::E120) => "Q65-120E",
            Self::Q65(Q65Submode::A300) => "Q65-300A",
            Self::Msk144 => "MSK144",
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
    /// Stop FT8, FT4, or FST4 candidate work after this wall-clock budget.
    /// Other protocol families retain their upstream search behavior because
    /// their 0.11 APIs do not accept the same budget predicate.
    pub decode_budget_ms: Option<u64>,
    /// Apply `mfsk-core`'s local Costas-pilot equalizer to frame-decodable
    /// modes. This is useful for audio from a tilted analogue filter and is
    /// intentionally opt-in because it can reduce recall on flat audio.
    pub local_equalization: bool,
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
            decode_budget_ms: None,
            local_equalization: false,
        }
    }
}
