//! Consumer-neutral WSJT protocol discovery.
//!
//! `mfsk-core` 0.11 publishes a detailed protocol registry. This module
//! copies the stable geometry and the capabilities actually exposed by this
//! adapter without leaking upstream types into QSONaut's public contract.

use std::time::Duration;

use super::{Fst4Submode, Q65Submode, WsjtMode};

/// Meaning of the `sync_min` value in a protocol's search defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsjtSyncScale {
    CostasAbsolute,
    BaselineNormalised,
}

/// Default search values published by the upstream protocol registry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WsjtSearchDefaults {
    pub frequency_min_hz: f32,
    pub frequency_max_hz: f32,
    pub sync_min: f32,
    pub max_candidates: u32,
    pub sync_scale: WsjtSyncScale,
    pub sniper_max_candidates: Option<u32>,
}

/// Decode and synthesis capabilities available through this adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WsjtCapabilities {
    pub decode: bool,
    pub synthesize: bool,
    pub deep_decode: bool,
    pub local_equalization: bool,
    pub decode_budget: bool,
    pub frequency_hint: bool,
    pub ft8_slot_acquisition: bool,
}

/// Stable adapter-owned description of one supported WSJT-family mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WsjtProtocolInfo {
    pub mode: WsjtMode,
    pub upstream_name: &'static str,
    pub capabilities: WsjtCapabilities,
    pub defaults: WsjtSearchDefaults,
    pub slot: Duration,
    pub tones: u32,
    pub bits_per_symbol: u32,
    pub samples_per_symbol: u32,
    pub symbol_duration_sec: f32,
    pub tone_spacing_hz: f32,
    /// `None` for modes without an FFT1 decode stage, including MSK144 whose
    /// separate burst decoder is not FFT1-based.
    pub decode_fft1_size: Option<u32>,
}

const MODES: &[WsjtMode] = &[
    WsjtMode::Ft8,
    WsjtMode::Ft4,
    WsjtMode::Fst4(Fst4Submode::S15),
    WsjtMode::Fst4(Fst4Submode::S30),
    WsjtMode::Fst4(Fst4Submode::S60),
    WsjtMode::Fst4(Fst4Submode::S120),
    WsjtMode::Fst4(Fst4Submode::S300),
    WsjtMode::Wspr,
    WsjtMode::Jt9,
    WsjtMode::Jt65,
    WsjtMode::Q65(Q65Submode::A15),
    WsjtMode::Q65(Q65Submode::A30),
    WsjtMode::Q65(Q65Submode::A60),
    WsjtMode::Q65(Q65Submode::B60),
    WsjtMode::Q65(Q65Submode::C60),
    WsjtMode::Q65(Q65Submode::D60),
    WsjtMode::Q65(Q65Submode::E60),
    WsjtMode::Q65(Q65Submode::D120),
    WsjtMode::Q65(Q65Submode::E120),
    WsjtMode::Q65(Q65Submode::A300),
    WsjtMode::Msk144,
];

fn capabilities(mode: WsjtMode) -> WsjtCapabilities {
    let frame_decode = matches!(mode, WsjtMode::Ft8 | WsjtMode::Ft4 | WsjtMode::Fst4(_));
    WsjtCapabilities {
        decode: true,
        synthesize: !matches!(mode, WsjtMode::Msk144),
        deep_decode: matches!(mode, WsjtMode::Ft8 | WsjtMode::Ft4),
        local_equalization: frame_decode,
        decode_budget: frame_decode,
        frequency_hint: matches!(mode, WsjtMode::Ft4),
        ft8_slot_acquisition: matches!(mode, WsjtMode::Ft8),
    }
}

fn sync_scale(scale: mfsk_core::registry::SyncScale) -> WsjtSyncScale {
    match scale {
        mfsk_core::registry::SyncScale::CostasAbsolute => WsjtSyncScale::CostasAbsolute,
        mfsk_core::registry::SyncScale::BaselineNormalised => WsjtSyncScale::BaselineNormalised,
    }
}

fn info(mode: WsjtMode) -> Option<WsjtProtocolInfo> {
    let Some(meta) = mfsk_core::by_name(mode.upstream_name()) else {
        return (mode == WsjtMode::Msk144).then_some(WsjtProtocolInfo {
            mode,
            upstream_name: "MSK144",
            capabilities: capabilities(mode),
            defaults: WsjtSearchDefaults {
                // MSK144's adapter takes a nominal carrier and fixed +/-200 Hz
                // tolerance rather than the registry-style band search.
                frequency_min_hz: 0.0,
                frequency_max_hz: 0.0,
                sync_min: 0.0,
                max_candidates: 0,
                sync_scale: WsjtSyncScale::CostasAbsolute,
                sniper_max_candidates: None,
            },
            slot: mode.slot(),
            tones: 2,
            bits_per_symbol: 1,
            samples_per_symbol: 6,
            symbol_duration_sec: 6.0 / 12_000.0,
            tone_spacing_hz: 1_000.0,
            decode_fft1_size: None,
        });
    };
    Some(WsjtProtocolInfo {
        mode,
        upstream_name: meta.name,
        capabilities: capabilities(mode),
        defaults: WsjtSearchDefaults {
            frequency_min_hz: meta.profile.defaults.freq_min_hz,
            frequency_max_hz: meta.profile.defaults.freq_max_hz,
            sync_min: meta.profile.defaults.sync_min,
            max_candidates: meta.profile.defaults.max_cand,
            sync_scale: sync_scale(meta.profile.sync_scale),
            sniper_max_candidates: meta.profile.sniper_max_cand_cap,
        },
        slot: mode.slot(),
        tones: meta.ntones,
        bits_per_symbol: meta.bits_per_symbol,
        samples_per_symbol: meta.nsps,
        symbol_duration_sec: meta.symbol_dt,
        tone_spacing_hz: meta.tone_spacing_hz,
        decode_fft1_size: (meta.decode_fft1_size > 0).then_some(meta.decode_fft1_size),
    })
}

/// Return every WSJT-family mode supported by this adapter.
///
/// Experimental `mfsk-core` UVPacket entries are intentionally excluded: they
/// are not WSJT messages and do not yet have a first-party packet contract.
pub fn protocol_capabilities() -> Vec<WsjtProtocolInfo> {
    MODES.iter().copied().filter_map(info).collect()
}

/// Look up one supported mode without requiring consumers to duplicate the
/// upstream registry's capitalization or submode names.
pub fn protocol_capability(mode: WsjtMode) -> Option<WsjtProtocolInfo> {
    info(mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_covers_every_adapter_mode_and_excludes_uvpacket() {
        let entries = protocol_capabilities();
        assert_eq!(entries.len(), 21);
        assert!(entries
            .iter()
            .all(|entry| entry.upstream_name != "UvExpress"));
        assert!(entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.mode,
                    WsjtMode::Ft8 | WsjtMode::Ft4 | WsjtMode::Fst4(_)
                )
            })
            .all(|entry| entry.decode_fft1_size.is_some_and(|size| size > 0)));
        for mode in MODES {
            assert_eq!(protocol_capability(*mode).unwrap().mode, *mode);
        }
    }

    #[test]
    fn registry_preserves_submode_names_and_adapter_limits() {
        let entries = protocol_capabilities();
        assert!(entries
            .iter()
            .any(|entry| entry.upstream_name == "FST4-60A"));
        assert!(entries
            .iter()
            .any(|entry| entry.upstream_name == "Q65-120E"));

        let ft8 = protocol_capability(WsjtMode::Ft8).unwrap();
        assert!(ft8.capabilities.decode_budget);
        assert!(ft8.capabilities.ft8_slot_acquisition);

        let msk144 = protocol_capability(WsjtMode::Msk144).unwrap();
        assert!(!msk144.capabilities.synthesize);
        assert!(!msk144.capabilities.decode_budget);
        assert_eq!(msk144.decode_fft1_size, None);
    }
}
