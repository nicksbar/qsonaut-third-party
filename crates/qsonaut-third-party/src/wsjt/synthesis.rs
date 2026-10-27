use mfsk_core::{engine::tx, fst4, ft4, ft8, msg::wsjt77::pack77, wspr};

use super::{Fst4Submode, Q65Submode};

fn standard_message(compose: &str) -> Option<(String, String, String)> {
    let tokens: Vec<&str> = compose.split_whitespace().collect();
    if tokens.len() != 3 {
        return None;
    }
    Some((
        tokens[0].to_string(),
        tokens[1].to_string(),
        tokens[2].to_string(),
    ))
}

fn pack_standard(compose: &str) -> Option<[u8; 77]> {
    let (destination, source, third) = standard_message(compose)?;
    if destination.eq_ignore_ascii_case("CQ") {
        pack77("CQ", &source, &third)
    } else {
        pack77(&destination, &source, &third)
    }
}

pub fn synthesize_ft8_standard(compose: &str, tone_hz: f32, amplitude: i16) -> Option<Vec<i16>> {
    let bits = pack_standard(compose)?;
    let tones = tx::message_to_tones::<ft8::Ft8>(&bits);
    Some(tx::synthesize_i16::<ft8::Ft8>(
        &tones, 12_000, tone_hz, amplitude,
    ))
}

pub fn synthesize_ft4_standard(compose: &str, tone_hz: f32, amplitude: i16) -> Option<Vec<i16>> {
    let bits = pack_standard(compose)?;
    let tones = tx::message_to_tones::<ft4::Ft4>(&bits);
    Some(tx::synthesize_i16::<ft4::Ft4>(
        &tones, 12_000, tone_hz, amplitude,
    ))
}

pub fn synthesize_fst4_standard(
    compose: &str,
    submode: Fst4Submode,
    tone_hz: f32,
    amplitude: i16,
) -> Option<Vec<i16>> {
    let bits = pack_standard(compose)?;
    macro_rules! synthesize {
        ($protocol:ty) => {
            tx::synthesize_i16::<$protocol>(
                &tx::message_to_tones::<$protocol>(&bits),
                12_000,
                tone_hz,
                amplitude,
            )
        };
    }
    Some(match submode {
        Fst4Submode::S15 => synthesize!(fst4::Fst4s15),
        Fst4Submode::S30 => synthesize!(fst4::Fst4s30),
        Fst4Submode::S60 => synthesize!(fst4::Fst4s60),
        Fst4Submode::S120 => synthesize!(fst4::Fst4s120),
        Fst4Submode::S300 => synthesize!(fst4::Fst4s300),
    })
}

fn standard_waveform(
    compose: &str,
    tone_hz: f32,
    amplitude: i16,
    synthesize: impl FnOnce(&str, &str, &str, u32, f32, f32) -> Option<Vec<f32>>,
) -> Option<Vec<i16>> {
    let (destination, source, third) = standard_message(compose)?;
    synthesize(&destination, &source, &third, 12_000, tone_hz, 1.0).map(|audio| {
        audio
            .into_iter()
            .map(|sample| (sample.clamp(-1.0, 1.0) * amplitude as f32).round() as i16)
            .collect()
    })
}

pub fn synthesize_jt9_standard(compose: &str, tone_hz: f32, amplitude: i16) -> Option<Vec<i16>> {
    standard_waveform(
        compose,
        tone_hz,
        amplitude,
        mfsk_core::jt9::synthesize_standard,
    )
}

pub fn synthesize_jt65_standard(compose: &str, tone_hz: f32, amplitude: i16) -> Option<Vec<i16>> {
    standard_waveform(
        compose,
        tone_hz,
        amplitude,
        mfsk_core::jt65::synthesize_standard,
    )
}

pub fn synthesize_q65_standard(
    compose: &str,
    submode: Q65Submode,
    tone_hz: f32,
    amplitude: i16,
) -> Option<Vec<i16>> {
    macro_rules! q65_synthesize {
        ($protocol:ty) => {
            standard_waveform(compose, tone_hz, amplitude, |a, b, c, rate, freq, amp| {
                mfsk_core::q65::synthesize_standard_for::<$protocol>(a, b, c, rate, freq, amp)
            })
        };
    }
    match submode {
        Q65Submode::A15 => q65_synthesize!(mfsk_core::q65::Q65a15),
        Q65Submode::A30 => q65_synthesize!(mfsk_core::q65::Q65a30),
        Q65Submode::A60 => q65_synthesize!(mfsk_core::q65::Q65a60),
        Q65Submode::B60 => q65_synthesize!(mfsk_core::q65::Q65b60),
        Q65Submode::C60 => q65_synthesize!(mfsk_core::q65::Q65c60),
        Q65Submode::D60 => q65_synthesize!(mfsk_core::q65::Q65d60),
        Q65Submode::E60 => q65_synthesize!(mfsk_core::q65::Q65e60),
        Q65Submode::D120 => q65_synthesize!(mfsk_core::q65::Q65d120),
        Q65Submode::E120 => q65_synthesize!(mfsk_core::q65::Q65e120),
        Q65Submode::A300 => q65_synthesize!(mfsk_core::q65::Q65a300),
    }
}

pub fn synthesize_wspr_type1(compose: &str, tone_hz: f32, amplitude: i16) -> Option<Vec<i16>> {
    let tokens: Vec<&str> = compose.split_whitespace().collect();
    if tokens.len() != 3 {
        return None;
    }
    let power_dbm = tokens[2].parse::<i32>().ok()?;
    wspr::synthesize_type1(tokens[0], tokens[1], power_dbm, 12_000, tone_hz, 0.8).map(|audio| {
        audio
            .into_iter()
            .map(|sample| (sample.clamp(-1.0, 1.0) * amplitude as f32).round() as i16)
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MESSAGE: &str = "CQ W1AW AA00";

    #[test]
    fn standard_waveform_adapters_produce_audio() {
        assert!(!synthesize_ft8_standard(MESSAGE, 1_500.0, 18_000)
            .unwrap()
            .is_empty());
        assert!(!synthesize_ft4_standard(MESSAGE, 1_500.0, 18_000)
            .unwrap()
            .is_empty());
        assert!(
            !synthesize_fst4_standard(MESSAGE, Fst4Submode::S60, 1_500.0, 18_000)
                .unwrap()
                .is_empty()
        );
        assert!(!synthesize_jt9_standard(MESSAGE, 1_500.0, 18_000)
            .unwrap()
            .is_empty());
        assert!(!synthesize_jt65_standard(MESSAGE, 1_500.0, 18_000)
            .unwrap()
            .is_empty());
        assert!(
            !synthesize_q65_standard(MESSAGE, Q65Submode::A30, 1_500.0, 18_000)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn wspr_adapter_rejects_malformed_and_accepts_type_one_messages() {
        assert!(synthesize_wspr_type1("not enough fields", 1_500.0, 18_000).is_none());
        assert!(!synthesize_wspr_type1("K1ABC FN31 30", 1_500.0, 18_000)
            .unwrap()
            .is_empty());
    }
}
