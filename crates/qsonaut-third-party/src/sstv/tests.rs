#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn martin_m1_round_trip_preserves_color_structure() {
        let mut source = vec![0_u8; WIDTH * HEIGHT * 3];
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let pixel = &mut source[(y * WIDTH + x) * 3..][..3];
                pixel.copy_from_slice(&[x as u8, y as u8, ((x + y) / 2) as u8]);
            }
        }
        let pcm = encode_martin_m1(&source).unwrap();
        let audio: Vec<f32> = pcm
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .collect();
        let decoded = decode_martin_m1(&audio).unwrap();
        assert_eq!((decoded.width, decoded.height), (WIDTH, HEIGHT));
        let mean_error = source
            .iter()
            .zip(&decoded.rgb)
            .map(|(a, b)| a.abs_diff(*b) as u64)
            .sum::<u64>() as f64
            / source.len() as f64;
        assert!(mean_error < 35.0, "mean channel error was {mean_error}");
    }

    #[test]
    fn streaming_receiver_reports_progress_and_finishes() {
        let source = vec![127_u8; WIDTH * HEIGHT * 3];
        let pcm = encode_martin_m1(&source).unwrap();
        let audio: Vec<f32> = pcm
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .collect();
        let mut receiver = MartinM1Receiver::default();
        let mut result = None;
        for chunk in audio.chunks(4096) {
            result = receiver.push(chunk).or(result);
        }
        assert!(result.is_some());
    }

    #[test]
    fn streaming_receiver_reports_an_unsupported_vis_mode() {
        let mut pcm = Vec::new();
        let mut phase = 0.0_f64;
        let mut remainder = 0.0_f64;
        let mut tone = |frequency: f64, duration_ms: f64, out: &mut Vec<i16>| {
            remainder += duration_ms * SAMPLE_RATE_HZ as f64 / 1000.0;
            let count = remainder.floor() as usize;
            remainder -= count as f64;
            let step = std::f64::consts::TAU * frequency / SAMPLE_RATE_HZ as f64;
            for _ in 0..count {
                out.push((phase.sin() * 18_000.0).round() as i16);
                phase = (phase + step) % std::f64::consts::TAU;
            }
        };
        append_vis_header(0x3c, &mut tone, &mut pcm);
        let audio: Vec<f32> = pcm
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .collect();
        let mut receiver = MartinM1Receiver::default();
        assert!(receiver.push(&audio).is_none());
        assert_eq!(receiver.detected_vis(), Some(0x3c));
        assert_eq!(vis_mode_name(0x3c), "Scottie S1");
        assert!(receiver.progress().is_none());
    }

    #[test]
    fn martin_m1_afc_accepts_a_frequency_shifted_signal() {
        let source = vec![127_u8; WIDTH * HEIGHT * 3];
        let pcm = encode_martin_m1_with_offset(&source, 60.0).unwrap();
        let audio: Vec<f32> = pcm
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .collect();
        let mut receiver = MartinM1Receiver::default();
        let mut result = None;
        for chunk in audio.chunks(4096) {
            result = receiver.push(chunk).or(result);
        }
        let decoded = result.expect("shifted Martin M1 should decode");
        let mean = decoded
            .rgb
            .iter()
            .map(|value| u64::from(*value))
            .sum::<u64>() as f64
            / decoded.rgb.len() as f64;
        assert!((mean - 127.0).abs() < 35.0, "decoded mean was {mean}");
    }

    #[test]
    fn streaming_receiver_manual_tuning_accepts_a_shift_beyond_afc() {
        let source = vec![96_u8; WIDTH * HEIGHT * 3];
        let pcm = encode_martin_m1_with_offset(&source, 420.0).unwrap();
        let audio: Vec<f32> = pcm
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .collect();
        let mut receiver = MartinM1Receiver::default();
        receiver.set_tuning_offset_hz(400.0);
        let mut result = None;
        for chunk in audio.chunks(4096) {
            result = receiver.push(chunk).or(result);
        }
        assert!(result.is_some());
        assert_eq!(receiver.tuning_offset_hz, 400.0);
    }

    #[test]
    fn multimode_backend_vis_mapping_covers_every_supported_mode() {
        assert_eq!(supported_modes().len(), 13);
        for &mode in supported_modes() {
            let vis = komitoto_sstv::spec::from_mode(mode).vis_code() & 0x7f;
            assert_eq!(mode_from_vis(vis), Some(mode), "missing {}", mode.name());
        }
    }

    #[test]
    fn multimode_adapter_round_trips_martin_m2() {
        let source = vec![112_u8; WIDTH * HEIGHT * 3];
        let pcm = encode_rgb_mode(SstvMode::MartinM2, WIDTH as u32, HEIGHT as u32, &source)
            .expect("Martin M2 encode should succeed");
        let audio: Vec<f32> = pcm
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .collect();
        let decoded =
            decode_mode(SstvMode::MartinM2, &audio).expect("Martin M2 decode should succeed");
        assert_eq!((decoded.width, decoded.height), (WIDTH, HEIGHT));
        let mean = decoded
            .rgb
            .iter()
            .map(|value| u64::from(*value))
            .sum::<u64>() as f64
            / decoded.rgb.len() as f64;
        assert!((mean - 112.0).abs() < 25.0, "decoded mean was {mean}");
    }

    #[test]
    fn streaming_multimode_receiver_auto_decodes_every_mode_at_12k() {
        let source = vec![112_u8; WIDTH * HEIGHT * 3];
        for &mode in supported_modes() {
            let pcm = encode_rgb_mode_12k(mode, WIDTH as u32, HEIGHT as u32, &source)
                .unwrap_or_else(|error| panic!("{} encode failed: {error}", mode.name()));
            let mut audio: Vec<f32> = pcm
                .iter()
                .map(|sample| *sample as f32 / i16::MAX as f32)
                .collect();
            audio.resize(audio.len() + ms_samples(STREAM_DECODE_GUARD_MS), 0.0);
            let mut receiver = MultiModeReceiver::default();
            let mut result = None;
            for chunk in audio.chunks(4096) {
                result = receiver.push(chunk).or(result);
            }
            let decoded = result.unwrap_or_else(|| panic!("{} did not decode", mode.name()));
            let (width, height) = mode.resolution();
            assert_eq!(
                (decoded.width, decoded.height),
                (width as usize, height as usize),
                "{} dimensions",
                mode.name()
            );
            assert_eq!(receiver.take_completed_mode(), Some(mode));
        }
    }

    #[test]
    fn auto_target_acquires_a_shifted_unaligned_transmission() {
        let source = vec![96_u8; WIDTH * HEIGHT * 3];
        let pcm = encode_martin_m1_with_offset(&source, 420.0).unwrap();
        let mut audio = vec![0.0_f32; ms_samples(7.0)];
        audio.extend(pcm.iter().map(|sample| *sample as f32 / i16::MAX as f32));
        audio.resize(audio.len() + ms_samples(STREAM_DECODE_GUARD_MS), 0.0);
        let mut receiver = MultiModeReceiver::default();
        receiver.set_auto_target(true);
        let mut result = None;
        for chunk in audio.chunks(4096) {
            result = receiver.push(chunk).or(result);
        }
        assert!(result.is_some(), "shifted auto-target image should decode");
        assert_eq!(receiver.take_completed_mode(), Some(SstvMode::MartinM1));
    }

    #[test]
    fn auto_target_rejects_silence_and_keeps_its_buffer_bounded() {
        let audio = vec![0.0_f32; SAMPLE_RATE_HZ as usize * 2];
        let mut receiver = MultiModeReceiver::default();
        receiver.set_auto_target(true);
        for chunk in audio.chunks(2048) {
            assert!(receiver.push(chunk).is_none());
        }
        assert!(receiver.detected_vis().is_none());
        assert!(receiver.buffer.len() <= ms_samples(HEADER_MS + 400.0));
    }

    #[test]
    fn auto_target_validates_ranked_candidates_beyond_the_strongest_tone() {
        let mut pcm = Vec::new();
        let mut phase = 0.0_f64;
        let mut remainder = 0.0_f64;
        let mut shifted_tone = |frequency: f64, duration_ms: f64, out: &mut Vec<i16>| {
            remainder += duration_ms * SAMPLE_RATE_HZ as f64 / 1000.0;
            let count = remainder.floor() as usize;
            remainder -= count as f64;
            let step = std::f64::consts::TAU * (frequency + 420.0) / SAMPLE_RATE_HZ as f64;
            for _ in 0..count {
                out.push((phase.sin() * 12_000.0).round() as i16);
                phase = (phase + step) % std::f64::consts::TAU;
            }
        };
        append_vis_header(VIS_CODE_MARTIN_M1, &mut shifted_tone, &mut pcm);
        let mut audio: Vec<f32> = pcm
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .collect();
        let start = ms_samples(40.0);
        let end = ms_samples(260.0);
        for (index, sample) in audio[start..end].iter_mut().enumerate() {
            *sample += (TAU * 1_100.0 * index as f32 / SAMPLE_RATE_HZ as f32).sin() * 0.8;
        }

        let scan = decode_vis_header_auto(&audio);
        assert_eq!(scan.detection.map(|(vis, _)| vis), Some(VIS_CODE_MARTIN_M1));
        assert!(
            scan.strongest_offset_hz.unwrap_or_default() < -700.0,
            "the interference should be the strongest leader candidate"
        );
    }

    #[test]
    fn manual_receive_mode_filters_a_different_vis_header() {
        let mut pcm = Vec::new();
        let mut phase = 0.0_f64;
        let mut remainder = 0.0_f64;
        let mut tone = |frequency: f64, duration_ms: f64, out: &mut Vec<i16>| {
            remainder += duration_ms * SAMPLE_RATE_HZ as f64 / 1000.0;
            let count = remainder.floor() as usize;
            remainder -= count as f64;
            let step = std::f64::consts::TAU * frequency / SAMPLE_RATE_HZ as f64;
            for _ in 0..count {
                out.push((phase.sin() * 18_000.0).round() as i16);
                phase = (phase + step) % std::f64::consts::TAU;
            }
        };
        append_vis_header(0x3c, &mut tone, &mut pcm);
        let audio: Vec<f32> = pcm
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .collect();
        let mut receiver = MultiModeReceiver::default();
        receiver.set_selected_mode(Some(SstvMode::MartinM1));
        assert!(receiver.push(&audio).is_none());
        assert_eq!(receiver.detected_vis(), Some(0x3c));
        assert_eq!(receiver.active_mode(), None);
    }

    #[test]
    fn auto_target_expires_a_vis_that_does_not_match_the_receive_filter() {
        let mut pcm = Vec::new();
        let mut phase = 0.0_f64;
        let mut remainder = 0.0_f64;
        let mut tone = |frequency: f64, duration_ms: f64, out: &mut Vec<i16>| {
            remainder += duration_ms * SAMPLE_RATE_HZ as f64 / 1000.0;
            let count = remainder.floor() as usize;
            remainder -= count as f64;
            let step = std::f64::consts::TAU * frequency / SAMPLE_RATE_HZ as f64;
            for _ in 0..count {
                out.push((phase.sin() * 18_000.0).round() as i16);
                phase = (phase + step) % std::f64::consts::TAU;
            }
        };
        append_vis_header(0x3c, &mut tone, &mut pcm);
        let audio = pcm
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .collect::<Vec<_>>();
        let mut receiver = MultiModeReceiver::default();
        receiver.set_auto_target(true);
        receiver.set_selected_mode(Some(SstvMode::MartinM1));
        assert!(receiver.push(&audio).is_none());
        assert_eq!(receiver.detected_vis(), Some(0x3c));

        let silence = vec![0.0; ms_samples(AUTO_REACQUIRE_TIMEOUT_MS)];
        for chunk in silence.chunks(1024) {
            assert!(receiver.push(chunk).is_none());
        }
        assert!(receiver.detected_vis().is_none());
        assert!(receiver.locked_offset_hz().is_none());
        assert!(receiver.take_auto_reacquired());
    }
}
