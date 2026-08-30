use std::time::Instant;

use mfsk_core::msg::decode_request::DecodeRequest as Fst4Request;
use qsonaut_modems::{AudioBlock, DecodeBatch};

use super::{common::*, Fst4Submode, WsjtDecodeConfig, WsjtMode};
use crate::AdapterError;

macro_rules! fst4_decode {
    ($audio:expr, $config:expr, $mode:expr, $protocol:ty) => {{
        let outcome = Fst4Request::<$protocol>::new(
            $audio,
            $config.frequency_min_hz,
            $config.frequency_max_hz,
            $config.sync_min,
            $config.max_candidates,
        )
        .decode();
        outcome
            .results
            .into_iter()
            .filter_map(|result| {
                mfsk_core::msg::wsjt77::unpack77(result.message77()).map(|message| {
                    event($mode, message, result.snr_db, result.dt_sec, result.freq_hz)
                })
            })
            .collect()
    }};
}

pub fn decode_fst4(
    audio: &AudioBlock,
    submode: Fst4Submode,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    let samples = require_audio(audio);
    let started = Instant::now();
    let pcm = to_pcm(samples);
    let mode = WsjtMode::Fst4(submode);
    let events = match submode {
        Fst4Submode::S15 => fst4_decode!(&pcm, config, mode, mfsk_core::fst4::Fst4s15),
        Fst4Submode::S30 => fst4_decode!(&pcm, config, mode, mfsk_core::fst4::Fst4s30),
        Fst4Submode::S60 => fst4_decode!(&pcm, config, mode, mfsk_core::fst4::Fst4s60),
        Fst4Submode::S120 => fst4_decode!(&pcm, config, mode, mfsk_core::fst4::Fst4s120),
        Fst4Submode::S300 => fst4_decode!(&pcm, config, mode, mfsk_core::fst4::Fst4s300),
    };
    Ok(DecodeBatch::finish(samples.len(), started, events))
}

pub fn decode_wspr(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    let samples = require_audio(audio);
    let started = Instant::now();
    let params = mfsk_core::wspr::search::SearchParams {
        freq_min_hz: config.frequency_min_hz,
        freq_max_hz: config.frequency_max_hz,
        time_tolerance_symbols: (config.time_tolerance_sec / 0.683).ceil().max(0.0) as u32,
        score_threshold: config.score_threshold,
        max_candidates: config.max_candidates,
    };
    let events = mfsk_core::wspr::decode::decode_scan(samples, super::SAMPLE_RATE_HZ, 0, &params)
        .into_iter()
        .map(|result| {
            event(
                WsjtMode::Wspr,
                format!("{} · drift {:+.2} Hz", result.message, result.drift_hz),
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
    let samples = require_audio(audio);
    let started = Instant::now();
    let params = mfsk_core::jt9::search::SearchParams {
        freq_min_hz: config.frequency_min_hz,
        freq_max_hz: config.frequency_max_hz,
        time_tolerance_sec: config.time_tolerance_sec,
        score_threshold: config.score_threshold,
        max_candidates: config.max_candidates,
    };
    let events = mfsk_core::jt9::decode_scan(samples, super::SAMPLE_RATE_HZ, 0, &params)
        .into_iter()
        .map(|result| {
            event(
                WsjtMode::Jt9,
                result.message.to_string(),
                result.snr_db,
                result.start_sample as f32 / super::SAMPLE_RATE_HZ as f32,
                result.freq_hz,
            )
        })
        .collect();
    Ok(DecodeBatch::finish(samples.len(), started, events))
}

pub fn decode_jt65(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    let samples = require_audio(audio);
    let started = Instant::now();
    let params = mfsk_core::jt65::search::SearchParams {
        freq_min_hz: config.frequency_min_hz,
        freq_max_hz: config.frequency_max_hz,
        time_tolerance_sec: config.time_tolerance_sec,
        score_threshold: config.score_threshold,
        max_candidates: config.max_candidates,
    };
    let events = mfsk_core::jt65::decode_scan(samples, super::SAMPLE_RATE_HZ, 0, &params)
        .into_iter()
        .map(|result| {
            event(
                WsjtMode::Jt65,
                result.message.to_string(),
                result.snr_db,
                result.start_sample as f32 / super::SAMPLE_RATE_HZ as f32,
                result.freq_hz,
            )
        })
        .collect();
    Ok(DecodeBatch::finish(samples.len(), started, events))
}

pub fn decode_q65(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    let samples = require_audio(audio);
    let started = Instant::now();
    let request = mfsk_core::q65::DecodeRequest::<mfsk_core::q65::Q65a30>::new(
        samples,
        super::SAMPLE_RATE_HZ,
        0,
        mfsk_core::q65::SearchParams {
            freq_min_hz: config.frequency_min_hz,
            freq_max_hz: config.frequency_max_hz,
            time_tolerance_early_sec: config.time_tolerance_sec,
            time_tolerance_late_sec: config.time_tolerance_sec,
            score_threshold: config.score_threshold,
            max_candidates: config.max_candidates,
        },
    );
    let events = request
        .decode()
        .into_iter()
        .map(|result| {
            event(
                WsjtMode::Q65,
                result.message,
                result.snr_db,
                result.start_sample as f32 / super::SAMPLE_RATE_HZ as f32,
                result.freq_hz,
            )
        })
        .collect();
    Ok(DecodeBatch::finish(samples.len(), started, events))
}

pub fn decode_msk144(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    let samples = require_audio(audio);
    let started = Instant::now();
    let pcm = to_pcm(samples);
    let events = mfsk_core::msk144::decode::decode_slot(
        &pcm,
        config.frequency_hint_hz.unwrap_or(1_500.0),
        200.0,
        mfsk_core::msk144::decode::Depth::Normal,
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
