//! Deterministic old/new comparison. Capture once, then replay identical f32 audio.
//! Usage: maintenance_snapshot <capture|replay> <directory>
use std::{fs, path::Path};

use qsonaut_modems::AudioBlock;
use qsonaut_third_party::wsjt::{self, Fst4Submode, Q65Submode, WsjtDecodeConfig, WsjtMode};

fn waveform(mode: WsjtMode) -> Vec<i16> {
    let message = "CQ N7UF DN26";
    match mode {
        WsjtMode::Ft8 => wsjt::synthesize_ft8_standard(message, 1_000.0, 18_000),
        WsjtMode::Ft4 => wsjt::synthesize_ft4_standard(message, 1_000.0, 18_000),
        WsjtMode::Fst4(s) => wsjt::synthesize_fst4_standard(message, s, 1_000.0, 18_000),
        WsjtMode::Wspr => wsjt::synthesize_wspr_type1("K1ABC FN31 30", 1_500.0, 18_000),
        WsjtMode::Jt9 => wsjt::synthesize_jt9_standard(message, 1_000.0, 18_000),
        WsjtMode::Jt65 => wsjt::synthesize_jt65_standard(message, 1_000.0, 18_000),
        WsjtMode::Q65(s) => wsjt::synthesize_q65_standard(message, s, 1_000.0, 18_000),
        WsjtMode::Msk144 => unreachable!(),
    }
    .expect("generated message packs")
}

fn run(capture: bool, directory: &Path) {
    fs::create_dir_all(directory).unwrap();
    for mode in [
        WsjtMode::Ft8,
        WsjtMode::Ft4,
        WsjtMode::Fst4(Fst4Submode::S15),
        WsjtMode::Wspr,
        WsjtMode::Jt9,
        WsjtMode::Jt65,
        WsjtMode::Q65(Q65Submode::A15),
    ] {
        let frame = waveform(mode);
        let checksum = frame.iter().fold(0u64, |h, s| {
            h.wrapping_mul(1_099_511_628_211)
                .wrapping_add(*s as u16 as u64)
        });
        println!(
            "SYNTH {} samples={} hash={checksum:016x}",
            mode.name(),
            frame.len()
        );
        let path = directory.join(format!("{}.f32le", mode.name()));
        let samples = if capture {
            // Frame at 0.5 s, with a reproducible low noise floor over the slot.
            let mut state = 0x1234_5678u32;
            let mut audio: Vec<f32> = (0..(mode.slot().as_secs_f32() * 12_000.0) as usize)
                .map(|_| {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    ((state >> 8) as f32 / 16_777_216.0 - 0.5) * 0.002
                })
                .collect();
            for (dst, src) in audio.iter_mut().skip(6_000).zip(&frame) {
                *dst += *src as f32 / 32_767.0;
            }
            fs::write(
                &path,
                audio
                    .iter()
                    .flat_map(|s| s.to_le_bytes())
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            audio
        } else {
            fs::read(&path)
                .unwrap()
                .chunks_exact(4)
                .map(|s| f32::from_le_bytes(s.try_into().unwrap()))
                .collect()
        };
        let digital = matches!(mode, WsjtMode::Ft8 | WsjtMode::Ft4);
        for &(deep, gain) in if digital {
            &[(false, 1.0), (true, 1.0), (false, 0.1)][..]
        } else {
            &[(false, 1.0)][..]
        } {
            let config = WsjtDecodeConfig {
                // FT4's baseline fit requires off-signal bandwidth (upstream
                // ft4_roundtrip); use the consumer's normal wide band.
                frequency_min_hz: if digital {
                    100.0
                } else if mode == WsjtMode::Wspr {
                    1_400.0
                } else {
                    900.0
                },
                frequency_max_hz: if digital {
                    3_000.0
                } else if mode == WsjtMode::Wspr {
                    1_600.0
                } else {
                    1_200.0
                },
                max_candidates: if digital { 120 } else { 8 },
                deep_decode: deep,
                ..Default::default()
            };
            let audio =
                AudioBlock::new(12_000, samples.iter().map(|s| s * gain).collect()).unwrap();
            let batch = wsjt::decode(&audio, mode, &config).unwrap();
            println!(
                "CASE {} deep={deep} gain={gain} count={}",
                mode.name(),
                batch.events.len()
            );
            for event in &batch.events {
                println!(
                    "ROW {} | {} | snr={:.6} dt={:.6} freq={:.6}",
                    mode.name(),
                    event.message,
                    event.snr_db.unwrap(),
                    event.delta_time_seconds.unwrap(),
                    event.audio_frequency_hz.unwrap()
                );
            }
            if digital || mode == WsjtMode::Jt65 {
                assert_eq!(batch.events.len(), 1);
                assert_eq!(batch.events[0].message, "CQ N7UF DN26");
            }
            let expected = if mode == WsjtMode::Wspr {
                "K1ABC FN31 30 · drift "
            } else {
                "CQ N7UF DN26"
            };
            assert!(batch.events.iter().any(|e| e.message.starts_with(expected)));
            let silence = AudioBlock::new(12_000, vec![0.0; samples.len()]).unwrap();
            let empty = wsjt::decode(&silence, mode, &config).unwrap();
            println!("SILENCE {} count={}", mode.name(), empty.events.len());
            assert!(empty.events.is_empty());
            let wrong_rate = AudioBlock::new(48_000, vec![0.0; 128]).unwrap();
            assert!(wsjt::decode(&wrong_rate, mode, &config).is_err());
        }
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        args.len() == 3 && matches!(args[1].as_str(), "capture" | "replay"),
        "usage: maintenance_snapshot <capture|replay> <directory>"
    );
    run(args[1] == "capture", Path::new(&args[2]));
}
