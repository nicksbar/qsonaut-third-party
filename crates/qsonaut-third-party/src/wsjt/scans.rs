use std::time::Instant;

use mfsk_core::decoder::{Decoder, SlotInput};
use qsonaut_modems::{AudioBlock, DecodeBatch};

use super::{common::*, Fst4Submode, Q65Submode, WsjtDecodeConfig, WsjtMode};
use crate::AdapterError;

pub fn decode_fst4(
    audio: &AudioBlock,
    submode: Fst4Submode,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    decode_mode(audio, WsjtMode::Fst4(submode), config)
}

pub fn decode_wspr(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    let samples = validate_audio(audio, WsjtMode::Wspr)?;
    let started = Instant::now();
    // Keep the native row only here: the public adapter text includes drift.
    let mut decoder = Decoder::<mfsk_core::Wspr>::new(params(WsjtMode::Wspr, config));
    decoder.extras_mut().search = search_tuning(config);
    let events = decoder
        .decode(&SlotInput::f32(samples))
        .rows
        .into_iter()
        .map(|row| {
            let result = row.decoded;
            event(
                WsjtMode::Wspr,
                format!("{} · drift {:+.2} Hz", result.text, row.native.drift_hz),
                result.snr_db,
                result.dt_sec,
                result.freq_hz,
            )
        })
        .collect();
    Ok(DecodeBatch::finish(samples.len(), started, events))
}

pub fn decode_jt9(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    decode_mode(audio, WsjtMode::Jt9, config)
}

pub fn decode_jt65(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    decode_mode(audio, WsjtMode::Jt65, config)
}

pub fn decode_q65(
    audio: &AudioBlock,
    submode: Q65Submode,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    decode_mode(audio, WsjtMode::Q65(submode), config)
}

pub fn decode_msk144(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    let samples = validate_audio(audio, WsjtMode::Msk144)?;
    let started = Instant::now();
    let pcm = to_pcm(samples);
    let events = mfsk_core::msk144::decode::decode_slot(
        &pcm,
        config.frequency_hint_hz.unwrap_or(1_500.0),
        200.0,
        mfsk_core::decoder::Depth::Normal,
    )
    .into_iter()
    .map(|result| {
        event(
            WsjtMode::Msk144,
            result.message,
            result.snr_db as f32,
            result.tsec,
            result.freq_hz,
        )
    })
    .collect();
    Ok(DecodeBatch::finish(samples.len(), started, events))
}
