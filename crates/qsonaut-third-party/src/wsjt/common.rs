use qsonaut_modems::{AudioBlock, DecodeEvent};

use super::WsjtMode;

pub fn require_audio(audio: &AudioBlock) -> &[f32] {
    &audio.samples
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
