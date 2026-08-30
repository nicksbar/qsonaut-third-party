use std::time::Instant;

use mfsk_core::{
    ft4::Ft4,
    ft8::{decode::WsjtxDepth, Ft8},
    msg::{decode_request::DecodeRequest, wsjt77::unpack77},
};
use qsonaut_modems::{AudioBlock, DecodeBatch};

use super::{common::*, WsjtDecodeConfig, WsjtMode};
use crate::AdapterError;

pub fn decode_ft8(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    let samples = require_audio(audio);
    let started = Instant::now();
    let pcm = to_pcm(samples);
    let depth = if config.deep_decode {
        WsjtxDepth::D2
    } else {
        WsjtxDepth::D1
    };
    let outcome = DecodeRequest::<Ft8>::wsjtx_depth(
        &pcm,
        config.frequency_min_hz,
        config.frequency_max_hz,
        config.sync_min,
        config.max_candidates,
        depth,
        None,
    )
    .decode();
    let events = outcome
        .results
        .into_iter()
        .filter_map(|result| {
            unpack77(result.message77()).map(|message| {
                event(
                    WsjtMode::Ft8,
                    message,
                    result.snr_db,
                    result.dt_sec,
                    result.freq_hz,
                )
            })
        })
        .collect();
    Ok(DecodeBatch::finish(samples.len(), started, events))
}

pub fn decode_ft4(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    let samples = require_audio(audio);
    let started = Instant::now();
    let pcm = to_pcm(samples);
    let request = DecodeRequest::<Ft4>::new(
        &pcm,
        config.frequency_min_hz,
        config.frequency_max_hz,
        config.sync_min,
        config.max_candidates,
    )
    .freq_hint(config.frequency_hint_hz.unwrap_or(0.0));
    let outcome = if config.deep_decode {
        request.sic_rounds(3).decode()
    } else {
        request.decode()
    };
    let events = outcome
        .results
        .into_iter()
        .filter_map(|result| {
            unpack77(result.message77()).map(|message| {
                event(
                    WsjtMode::Ft4,
                    message,
                    result.snr_db,
                    result.dt_sec,
                    result.freq_hz,
                )
            })
        })
        .collect();
    Ok(DecodeBatch::finish(samples.len(), started, events))
}
