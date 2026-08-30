use mfsk_core::{fst4, ft4, ft8, msg::wsjt77::pack77, wspr};

use super::Fst4Submode;

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
    let tones = ft8::wave_gen::message_to_tones(&bits);
    Some(ft8::wave_gen::tones_to_i16(&tones, tone_hz, amplitude))
}

pub fn synthesize_ft4_standard(compose: &str, tone_hz: f32, amplitude: i16) -> Option<Vec<i16>> {
    let bits = pack_standard(compose)?;
    let tones = ft4::encode::message_to_tones(&bits);
    Some(ft4::encode::tones_to_i16(&tones, tone_hz, amplitude))
}

pub fn synthesize_fst4_standard(
    compose: &str,
    submode: Fst4Submode,
    tone_hz: f32,
    amplitude: i16,
) -> Option<Vec<i16>> {
    let bits = pack_standard(compose)?;
    let tones = fst4::encode::message_to_tones(&bits);
    let gfsk = match submode {
        Fst4Submode::S15 => &fst4::encode::FST4_15_GFSK,
        Fst4Submode::S30 => &fst4::encode::FST4_30_GFSK,
        Fst4Submode::S60 => &fst4::encode::FST4_60A_GFSK,
        Fst4Submode::S120 => &fst4::encode::FST4_120_GFSK,
        Fst4Submode::S300 => &fst4::encode::FST4_300_GFSK,
    };
    Some(fst4::encode::tones_to_i16_with_gfsk(
        &tones, tone_hz, amplitude, gfsk,
    ))
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

pub fn synthesize_q65_standard(compose: &str, tone_hz: f32, amplitude: i16) -> Option<Vec<i16>> {
    standard_waveform(
        compose,
        tone_hz,
        amplitude,
        mfsk_core::q65::synthesize_standard,
    )
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
