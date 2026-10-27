use qsonaut_modems::AudioBlock;
use qsonaut_third_party::{wsjt::*, AdapterError};

fn generated(mode: WsjtMode) -> AudioBlock {
    let frame = match mode {
        WsjtMode::Ft8 => synthesize_ft8_standard("CQ N7UF DN26", 1_000.0, 18_000),
        WsjtMode::Ft4 => synthesize_ft4_standard("CQ N7UF DN26", 1_000.0, 18_000),
        WsjtMode::Jt65 => synthesize_jt65_standard("CQ N7UF DN26", 1_000.0, 18_000),
        _ => unreachable!(),
    }
    .unwrap();
    let mut samples = vec![0.0; (mode.slot().as_secs_f32() * 12_000.0) as usize];
    for (dst, src) in samples.iter_mut().skip(6_000).zip(frame) {
        *dst = src as f32 / 32_767.0;
    }
    AudioBlock::new(12_000, samples).unwrap()
}

#[test]
fn generated_ft_messages_and_normalized_rows_survive_depth_and_gain() {
    for mode in [WsjtMode::Ft8, WsjtMode::Ft4] {
        let source = generated(mode);
        for (deep_decode, gain) in [(false, 1.0), (true, 1.0), (false, 0.1)] {
            let audio =
                AudioBlock::new(12_000, source.samples.iter().map(|s| s * gain).collect()).unwrap();
            let config = WsjtDecodeConfig {
                deep_decode,
                ..Default::default()
            };
            let batch = decode(&audio, mode, &config).unwrap();
            assert_eq!(batch.events.len(), 1, "{}", mode.name());
            assert_eq!(batch.telemetry.input_samples, audio.samples.len());
            assert_eq!(batch.telemetry.decoded_events, 1);
            let row = &batch.events[0];
            assert_eq!(row.modem, mode.modem_id());
            assert_eq!(row.message, "CQ N7UF DN26");
            assert!(row.snr_db.unwrap().is_finite());
            assert!(row.delta_time_seconds.unwrap().abs() < 0.03);
            assert!((row.audio_frequency_hz.unwrap() - 1_000.0).abs() < 1.0);
        }
    }
}

#[test]
fn generated_jt65_has_only_the_expected_message() {
    let source = generated(WsjtMode::Jt65);
    for noise_amplitude in [0.0, 0.002] {
        // Same deterministic floor and 0.5 s placement as maintenance_snapshot.
        let mut state = 0x1234_5678u32;
        let samples = source
            .samples
            .iter()
            .map(|s| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                s + ((state >> 8) as f32 / 16_777_216.0 - 0.5) * noise_amplitude
            })
            .collect();
        let audio = AudioBlock::new(12_000, samples).unwrap();
        for deep_decode in [false, true] {
            let config = WsjtDecodeConfig {
                frequency_min_hz: 900.0,
                frequency_max_hz: 1_200.0,
                max_candidates: 8,
                deep_decode,
                ..Default::default()
            };
            let batch = decode_jt65(&audio, &config).unwrap();
            assert_eq!(
                batch
                    .events
                    .iter()
                    .map(|e| e.message.as_str())
                    .collect::<Vec<_>>(),
                ["CQ N7UF DN26"],
                "noise={noise_amplitude}, deep={deep_decode}"
            );
            assert_eq!(batch.telemetry.decoded_events, 1);
            assert_eq!(batch.telemetry.input_samples, audio.samples.len());
            let row = &batch.events[0];
            assert_eq!(row.modem, WsjtMode::Jt65.modem_id());
            assert!((row.snr_db.unwrap() - -2.351).abs() < 0.01);
            assert!((row.delta_time_seconds.unwrap() - 0.5).abs() < 0.03);
            assert!((row.audio_frequency_hz.unwrap() - 1_000.0).abs() < 1.0);
        }
    }
}

#[test]
fn every_mode_rejects_wrong_rate_and_accepts_empty_or_silent_input() {
    let wrong = AudioBlock::new(48_000, vec![0.0; 128]).unwrap();
    for info in protocol_capabilities() {
        let config = WsjtDecodeConfig {
            max_candidates: 2,
            ..Default::default()
        };
        assert_eq!(
            decode(&wrong, info.mode, &config).unwrap_err(),
            AdapterError::UnsupportedSampleRate {
                modem: info.mode.name(),
                expected: 12_000,
                actual: 48_000,
            }
        );
        for samples in [Vec::new(), vec![0.0; 12_000]] {
            let audio = AudioBlock::new(12_000, samples).unwrap();
            assert!(decode(&audio, info.mode, &config)
                .unwrap()
                .events
                .is_empty());
        }
    }
    // Public family helpers must enforce the same boundary as dispatch.
    let config = WsjtDecodeConfig::default();
    assert!(decode_ft8(&wrong, &config).is_err());
    assert!(decode_ft4(&wrong, &config).is_err());
    assert!(decode_fst4(&wrong, Fst4Submode::S15, &config).is_err());
    assert!(decode_wspr(&wrong, &config).is_err());
    assert!(decode_jt9(&wrong, &config).is_err());
    assert!(decode_jt65(&wrong, &config).is_err());
    assert!(decode_q65(&wrong, Q65Submode::A15, &config).is_err());
    assert!(decode_msk144(&wrong, &config).is_err());
}

#[test]
fn zero_budget_stops_frame_candidates_without_panics() {
    for mode in [WsjtMode::Ft8, WsjtMode::Ft4] {
        let config = WsjtDecodeConfig {
            decode_budget_ms: Some(0),
            ..Default::default()
        };
        assert!(decode(&generated(mode), mode, &config)
            .unwrap()
            .events
            .is_empty());
    }
}
