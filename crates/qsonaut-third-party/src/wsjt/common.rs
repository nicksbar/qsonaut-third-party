use std::time::{Duration, Instant};

use mfsk_core::{
    decoder::{
        default_params, AnyDecoder, AnyExtras, ApMode, DecodeParams, Depth, Ft4Strategy,
        SearchTuning, SlotInput, Tuning,
    },
    engine::equalize::EqMode,
    jt65::ChaseParams,
};
use qsonaut_modems::{AudioBlock, DecodeEvent};

use super::{WsjtDecodeConfig, WsjtMode};
use crate::AdapterError;

pub fn validate_audio(audio: &AudioBlock, mode: WsjtMode) -> Result<&[f32], AdapterError> {
    if audio.sample_rate_hz != super::SAMPLE_RATE_HZ {
        return Err(AdapterError::UnsupportedSampleRate {
            modem: mode.name(),
            expected: super::SAMPLE_RATE_HZ,
            actual: audio.sample_rate_hz,
        });
    }
    Ok(&audio.samples)
}

pub fn upstream_mode(mode: WsjtMode) -> mfsk_core::Mode {
    mfsk_core::Mode::from_name(mode.upstream_name()).expect("supported slot mode")
}

pub fn params(mode: WsjtMode, config: &WsjtDecodeConfig) -> DecodeParams {
    let mut params = default_params(upstream_mode(mode));
    params.band_hz = (config.frequency_min_hz, config.frequency_max_hz);
    // The old batch adapter supplied no AP context. Do not silently enable
    // blind CQ hypotheses through the new FT4/FST4 default parameter block.
    params.ap = ApMode::Off;
    match mode {
        WsjtMode::Ft8 => {
            params.depth = if config.deep_decode {
                Depth::Normal
            } else {
                Depth::Fast
            }
        }
        WsjtMode::Ft4 => {
            params.depth = Depth::Deep; // old single pass already had OSD
            params.rx_freq_hz = config.frequency_hint_hz;
        }
        // Avoid the four-pass Deep scan. The legacy hard-RS strategy is
        // selected separately through Jt65Extras::chase below.
        WsjtMode::Jt65 => params.depth = Depth::Normal,
        _ => {} // Other families did not consume the adapter's deep flag.
    }
    params
}

fn frame_tuning<S>(tuning: &mut Tuning<S>, config: &WsjtDecodeConfig) {
    tuning.sync_min = Some(config.sync_min);
    tuning.max_cand = Some(config.max_candidates);
}

pub fn search_tuning(config: &WsjtDecodeConfig) -> SearchTuning {
    let mut tuning = SearchTuning::default();
    tuning.time_tolerance_early_sec = Some(config.time_tolerance_sec);
    tuning.time_tolerance_late_sec = Some(config.time_tolerance_sec);
    tuning.score_threshold = Some(config.score_threshold);
    tuning.max_candidates = Some(config.max_candidates);
    tuning
}

pub(super) fn configured_decoder(mode: WsjtMode, config: &WsjtDecodeConfig) -> AnyDecoder {
    let mut decoder = AnyDecoder::new(upstream_mode(mode), params(mode, config));
    let eq = if config.local_equalization {
        EqMode::Local
    } else {
        EqMode::Off
    };
    match decoder.extras_mut() {
        AnyExtras::Ft8(e) => {
            frame_tuning(&mut e.tuning, config);
            e.eq = eq;
        }
        AnyExtras::Ft4(e) => {
            frame_tuning(&mut e.tuning, config);
            e.tuning.osd = Some(true);
            e.tuning.strategy = Some(if config.deep_decode {
                Ft4Strategy::SicRounds(3)
            } else {
                Ft4Strategy::SinglePass
            });
            e.eq = eq;
        }
        AnyExtras::Fst4(e) => {
            frame_tuning(&mut e.tuning, config);
            e.eq = eq;
        }
        AnyExtras::Wspr(e) => e.search = search_tuning(config),
        AnyExtras::Jt9(e) => e.search = search_tuning(config),
        AnyExtras::Jt65(e) => {
            e.search = search_tuning(config);
            // Legacy decode_scan used hard-decision RS, not decode_scan_chase.
            // The public facade always enters Chase, whose zero-erasure RS
            // attempt runs before the trial loop. Zero trials preserves that
            // strategy without changing acceptance gates or filtering rows.
            let mut chase = ChaseParams::default();
            chase.max_trials = 0;
            e.chase = Some(chase);
        }
        AnyExtras::Q65(e) => e.search = search_tuning(config),
        _ => unreachable!("adapter does not expose this upstream family"),
    }
    decoder
}

pub fn decode_mode(
    audio: &AudioBlock,
    mode: WsjtMode,
    config: &WsjtDecodeConfig,
) -> Result<qsonaut_modems::DecodeBatch, AdapterError> {
    let samples = validate_audio(audio, mode)?;
    let started = Instant::now();
    let mut decoder = configured_decoder(mode, config);
    let frame = matches!(mode, WsjtMode::Ft8 | WsjtMode::Ft4 | WsjtMode::Fst4(_));
    // Preserve the adapter's clamp/round PCM conversion, not the new upstream
    // F32 RMS gain. Float-engine families retain the original f32 samples.
    let pcm = if frame { to_pcm(samples) } else { Vec::new() };
    let mut slot = if frame {
        SlotInput::i16(&pcm)
    } else {
        SlotInput::f32(samples)
    };
    let budget_started = Instant::now();
    let budget = config.decode_budget_ms.map(Duration::from_millis);
    let keep_going = || budget.is_none_or(|limit| budget_started.elapsed() < limit);
    if frame && budget.is_some() {
        slot = slot.budget(&keep_going);
    }
    let events = decoder
        .decode(&slot)
        .rows
        .into_iter()
        .map(|row| {
            let d = row.decoded;
            event(mode, d.text, d.snr_db, d.dt_sec, d.freq_hz)
        })
        .collect();
    Ok(qsonaut_modems::DecodeBatch::finish(
        samples.len(),
        started,
        events,
    ))
}

pub fn to_pcm(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16)
        .collect()
}

pub fn event(
    modem: WsjtMode,
    message: impl Into<String>,
    snr_db: f32,
    delta_time_seconds: f32,
    audio_frequency_hz: f32,
) -> DecodeEvent {
    DecodeEvent {
        modem: modem.modem_id(),
        message: message.into(),
        snr_db: Some(snr_db),
        delta_time_seconds: Some(delta_time_seconds),
        audio_frequency_hz: Some(audio_frequency_hz),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_boundary_keeps_clamping_and_rounding() {
        assert_eq!(
            to_pcm(&[-2.0, -1.0, 0.0, 0.5, 1.0, 2.0]),
            vec![-32767, -32767, 0, 16384, 32767, 32767]
        );
    }

    #[test]
    fn jt65_preserves_hard_rs_and_requested_search_controls() {
        for deep_decode in [false, true] {
            let config = WsjtDecodeConfig {
                deep_decode,
                ..Default::default()
            };
            let mut decoder = configured_decoder(WsjtMode::Jt65, &config);
            let AnyExtras::Jt65(extras) = decoder.extras_mut() else {
                panic!("wrong family")
            };
            assert_eq!(extras.search, search_tuning(&config));
            let chase = extras.chase.as_ref().expect("explicit hard-RS strategy");
            let defaults = ChaseParams::default();
            assert_eq!(chase.max_trials, 0);
            assert_eq!(chase.nd0, defaults.nd0);
            assert_eq!(chase.r0, defaults.r0);
            assert!(extras.known_calls.is_empty());
        }
    }

    #[test]
    fn public_decoder_mapping_preserves_adapter_controls() {
        let config = WsjtDecodeConfig {
            sync_min: 1.25,
            max_candidates: 7,
            frequency_hint_hz: Some(1_234.0),
            local_equalization: true,
            ..Default::default()
        };
        assert_eq!(params(WsjtMode::Ft8, &config).depth, Depth::Fast);
        assert!(matches!(params(WsjtMode::Ft8, &config).ap, ApMode::Off));
        assert_eq!(params(WsjtMode::Ft4, &config).rx_freq_hz, Some(1_234.0));
        assert_eq!(params(WsjtMode::Jt65, &config).depth, Depth::Normal);
        let mut decoder = configured_decoder(WsjtMode::Ft4, &config);
        let AnyExtras::Ft4(extras) = decoder.extras_mut() else {
            panic!("wrong family")
        };
        assert_eq!(extras.tuning.sync_min, Some(1.25));
        assert_eq!(extras.tuning.max_cand, Some(7));
        assert_eq!(extras.tuning.osd, Some(true));
        assert!(matches!(
            extras.tuning.strategy,
            Some(Ft4Strategy::SinglePass)
        ));
        assert!(matches!(extras.eq, EqMode::Local));
        let deep = WsjtDecodeConfig {
            deep_decode: true,
            ..config
        };
        assert_eq!(params(WsjtMode::Ft8, &deep).depth, Depth::Normal);
        let mut decoder = configured_decoder(WsjtMode::Ft4, &deep);
        let AnyExtras::Ft4(extras) = decoder.extras_mut() else {
            panic!("wrong family")
        };
        assert!(matches!(
            extras.tuning.strategy,
            Some(Ft4Strategy::SicRounds(3))
        ));
        let search = search_tuning(&deep);
        assert_eq!(search.max_candidates, Some(7));
        assert_eq!(search.score_threshold, Some(deep.score_threshold));
        assert_eq!(
            search.time_tolerance_early_sec,
            Some(deep.time_tolerance_sec)
        );
        assert_eq!(
            search.time_tolerance_late_sec,
            Some(deep.time_tolerance_sec)
        );
    }
}
