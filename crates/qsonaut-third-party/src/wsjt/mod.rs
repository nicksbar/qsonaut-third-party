//! Standard WSJT-family adapter surface.

mod common;
mod config;
mod digital;
mod scans;
mod synthesis;

use common::to_pcm;

pub use config::{Fst4Submode, Q65Submode, WsjtDecodeConfig, WsjtMode};
pub use digital::{decode_ft4, decode_ft8};
pub use scans::{decode_fst4, decode_jt65, decode_jt9, decode_msk144, decode_q65, decode_wspr};
pub use synthesis::{
    synthesize_fst4_standard, synthesize_ft4_standard, synthesize_ft8_standard,
    synthesize_jt65_standard, synthesize_jt9_standard, synthesize_q65_standard,
    synthesize_wspr_type1,
};

/// Minimum 12 kHz audio needed by [`acquire_ft8_slot_phases`]: a 15-second
/// slot plus two additional 5-second acquisition windows.
pub const FT8_SLOT_ACQUISITION_REQUIRED_SAMPLES: usize = mfsk_core::ft8::acquire::REQUIRED_SAMPLES;

/// A candidate FT8 slot phase returned by cold acquisition.
///
/// `delta_time_seconds` is in `(-7.5, 7.5]`; callers should try candidates
/// by decoding at the proposed phase and select the first phase that produces
/// real messages. `weight` ranks the candidates, with larger values first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ft8SlotPhaseCandidate {
    pub delta_time_seconds: f32,
    pub weight: f32,
}

/// Find candidate FT8 slot phases from unaligned audio.
///
/// This is intentionally an acquisition primitive rather than an automatic
/// clock decision: a strong outlier station can produce a coherent but wrong
/// phase, so the caller must validate candidates by decoding. Empty audio or
/// audio shorter than [`FT8_SLOT_ACQUISITION_REQUIRED_SAMPLES`] returns an
/// empty vector.
pub fn acquire_ft8_slot_phases(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<Vec<Ft8SlotPhaseCandidate>, AdapterError> {
    if audio.sample_rate_hz != SAMPLE_RATE_HZ {
        return Err(AdapterError::UnsupportedSampleRate {
            modem: WsjtMode::Ft8.name(),
            expected: SAMPLE_RATE_HZ,
            actual: audio.sample_rate_hz,
        });
    }

    let phases = mfsk_core::ft8::acquire::acquire_slot_phases(
        &to_pcm(&audio.samples),
        config.frequency_min_hz,
        config.frequency_max_hz,
        config.sync_min,
        config.max_candidates,
        8,
    );
    Ok(phases
        .into_iter()
        .map(|(delta_time_seconds, weight)| Ft8SlotPhaseCandidate {
            delta_time_seconds,
            weight,
        })
        .collect())
}

use std::time::Instant;

use qsonaut_modems::{AudioBlock, DecodeBatch};

use crate::AdapterError;

pub const SAMPLE_RATE_HZ: u32 = 12_000;

/// Decode any supported WSJT-family mode using one normalized entry point.
pub fn decode(
    audio: &AudioBlock,
    mode: WsjtMode,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    if audio.sample_rate_hz != SAMPLE_RATE_HZ {
        return Err(AdapterError::UnsupportedSampleRate {
            modem: mode.name(),
            expected: SAMPLE_RATE_HZ,
            actual: audio.sample_rate_hz,
        });
    }
    let started = Instant::now();
    let batch = match mode {
        WsjtMode::Ft8 => decode_ft8(audio, config),
        WsjtMode::Ft4 => decode_ft4(audio, config),
        WsjtMode::Fst4(submode) => decode_fst4(audio, submode, config),
        WsjtMode::Wspr => decode_wspr(audio, config),
        WsjtMode::Jt9 => decode_jt9(audio, config),
        WsjtMode::Jt65 => decode_jt65(audio, config),
        WsjtMode::Q65(submode) => decode_q65(audio, submode, config),
        WsjtMode::Msk144 => decode_msk144(audio, config),
    }?;
    let elapsed = started.elapsed();
    Ok(DecodeBatch {
        telemetry: qsonaut_modems::DecodeTelemetry {
            elapsed,
            input_samples: batch.telemetry.input_samples,
            decoded_events: batch.events.len(),
        },
        ..batch
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn mode_matrix_has_standard_slot_metadata() {
        let modes = [
            WsjtMode::Ft8,
            WsjtMode::Ft4,
            WsjtMode::Fst4(Fst4Submode::S60),
            WsjtMode::Wspr,
            WsjtMode::Jt9,
            WsjtMode::Jt65,
            WsjtMode::Q65(Q65Submode::A30),
            WsjtMode::Msk144,
        ];
        assert_eq!(
            modes.iter().map(|mode| mode.name()).collect::<Vec<_>>(),
            vec!["ft8", "ft4", "fst4-60", "wspr", "jt9", "jt65", "q65-a30", "msk144",]
        );
        assert_eq!(WsjtMode::Ft8.slot(), Duration::from_secs(15));
        assert_eq!(WsjtMode::Ft4.slot(), Duration::from_millis(7_500));
        assert_eq!(WsjtMode::Wspr.slot(), Duration::from_secs(120));
        assert_eq!(
            WsjtMode::Q65(Q65Submode::A30).slot(),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn dispatch_rejects_wrong_audio_rate_before_protocol_work() {
        let audio = AudioBlock::new(48_000, vec![0.0; 128]).unwrap();
        let error = decode(&audio, WsjtMode::Jt9, &WsjtDecodeConfig::default()).unwrap_err();
        assert_eq!(
            error,
            AdapterError::UnsupportedSampleRate {
                modem: "jt9",
                expected: SAMPLE_RATE_HZ,
                actual: 48_000,
            }
        );
    }

    #[test]
    fn ft8_acquisition_requires_the_standard_audio_rate() {
        let audio = AudioBlock::new(48_000, Vec::new()).unwrap();
        let error = acquire_ft8_slot_phases(&audio, &WsjtDecodeConfig::default()).unwrap_err();
        assert_eq!(
            error,
            AdapterError::UnsupportedSampleRate {
                modem: "ft8",
                expected: SAMPLE_RATE_HZ,
                actual: 48_000,
            }
        );
    }

    #[test]
    fn ft8_acquisition_reports_empty_for_a_short_audio_buffer() {
        let audio = AudioBlock::new(SAMPLE_RATE_HZ, Vec::new()).unwrap();
        assert!(
            acquire_ft8_slot_phases(&audio, &WsjtDecodeConfig::default())
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            FT8_SLOT_ACQUISITION_REQUIRED_SAMPLES,
            25 * SAMPLE_RATE_HZ as usize
        );
    }

    #[test]
    fn fst4_submodes_have_distinct_names_and_durations() {
        let submodes = [
            Fst4Submode::S15,
            Fst4Submode::S30,
            Fst4Submode::S60,
            Fst4Submode::S120,
            Fst4Submode::S300,
        ];
        assert_eq!(submodes[0].seconds(), 15);
        assert_eq!(submodes[4].seconds(), 300);
        assert_eq!(
            submodes.iter().map(|mode| mode.name()).collect::<Vec<_>>(),
            vec!["fst4-15", "fst4-30", "fst4-60", "fst4-120", "fst4-300",]
        );
    }

    #[test]
    fn q65_submodes_have_distinct_names_and_slot_durations() {
        let submodes = [
            Q65Submode::A15,
            Q65Submode::A30,
            Q65Submode::A60,
            Q65Submode::B60,
            Q65Submode::C60,
            Q65Submode::D60,
            Q65Submode::E60,
            Q65Submode::D120,
            Q65Submode::E120,
            Q65Submode::A300,
        ];
        assert_eq!(
            submodes.iter().map(|mode| mode.name()).collect::<Vec<_>>(),
            vec![
                "q65-a15", "q65-a30", "q65-a60", "q65-b60", "q65-c60", "q65-d60", "q65-e60",
                "q65-d120", "q65-e120", "q65-a300",
            ]
        );
        assert_eq!(Q65Submode::A15.seconds(), 15);
        assert_eq!(Q65Submode::A30.seconds(), 30);
        assert_eq!(Q65Submode::E60.seconds(), 60);
        assert_eq!(Q65Submode::D120.seconds(), 120);
        assert_eq!(Q65Submode::A300.seconds(), 300);
    }
}
