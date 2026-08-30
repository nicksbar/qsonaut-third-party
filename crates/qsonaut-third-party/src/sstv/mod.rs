//! Reusable analog SSTV streaming, VIS, and codec integration.
//!
//! Live audio is mono 12 kHz PCM. Automatic VIS selection and explicit receive
//! filtering cover the pinned backend's Martin, Scottie, Robot, and PD modes.

use std::f32::consts::TAU;

use image::{DynamicImage, ImageBuffer, Rgb};

pub use komitoto_sstv::SstvMode;

pub const SAMPLE_RATE_HZ: u32 = 12_000;
pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 256;
pub const VIS_CODE_MARTIN_M1: u8 = 0x2c;
pub const MULTIMODE_SAMPLE_RATE_HZ: u32 = 48_000;
pub const AUTO_TARGET_MIN_OFFSET_HZ: i32 = -900;
pub const AUTO_TARGET_MAX_OFFSET_HZ: i32 = 700;
pub const AUTO_TARGET_CANDIDATES_PER_WINDOW: usize = 16;

const LEADER_MS: f64 = 300.0;
const VIS_BREAK_MS: f64 = 10.0;
const VIS_BIT_MS: f64 = 30.0;
const SYNC_MS: f64 = 4.862;
const GAP_MS: f64 = 0.572;
const CHANNEL_MS: f64 = 146.432;
const LINE_MS: f64 = SYNC_MS + 4.0 * GAP_MS + 3.0 * CHANNEL_MS;
const HEADER_MS: f64 = LEADER_MS * 2.0 + VIS_BREAK_MS + VIS_BIT_MS * 10.0;
const IMAGE_MS: f64 = LINE_MS * HEIGHT as f64;
const STREAM_DECODE_GUARD_MS: f64 = 20.0;
const AUTO_REACQUIRE_TIMEOUT_MS: f64 = 2_000.0;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SstvError {
    #[error("Martin M1 needs exactly 320 x 256 RGB pixels")]
    InvalidImage,
    #[error("audio does not contain a complete Martin M1 image")]
    IncompleteAudio,
    #[error("VIS header is not Martin M1")]
    UnsupportedVis,
    #[error("SSTV image dimensions do not match its RGB buffer")]
    InvalidDimensions,
    #[error("SSTV codec failed: {0}")]
    Codec(String),
}

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}

/// Modes supplied by the pinned multi-mode codec backend.
pub fn supported_modes() -> &'static [SstvMode] {
    SstvMode::all()
}

pub fn mode_duration_seconds(mode: SstvMode) -> f32 {
    komitoto_sstv::spec::from_mode(mode).total_samples() as f32 / MULTIMODE_SAMPLE_RATE_HZ as f32
}

/// Map a parity-stripped VIS value to a codec mode.
pub fn mode_from_vis(vis: u8) -> Option<SstvMode> {
    match vis {
        0x2c => Some(SstvMode::MartinM1),
        0x28 => Some(SstvMode::MartinM2),
        0x3c => Some(SstvMode::ScottieS1),
        0x38 => Some(SstvMode::ScottieS2),
        0x08 => Some(SstvMode::Robot36),
        0x0c => Some(SstvMode::Robot72),
        0x5d => Some(SstvMode::Pd50),
        0x63 => Some(SstvMode::Pd90),
        0x5f => Some(SstvMode::Pd120),
        0x62 => Some(SstvMode::Pd160),
        0x60 => Some(SstvMode::Pd180),
        0x61 => Some(SstvMode::Pd240),
        0x5e => Some(SstvMode::Pd290),
        _ => None,
    }
}

/// Encode arbitrary RGB pixels with the pinned multi-mode codec.
///
/// The input is resized to the selected mode's native dimensions. Returned
/// PCM is normalized to signed 16-bit samples at 48 kHz.
pub fn encode_rgb_mode(
    mode: SstvMode,
    width: u32,
    height: u32,
    rgb: &[u8],
) -> Result<Vec<i16>, SstvError> {
    let source = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, rgb.to_vec())
        .ok_or(SstvError::InvalidDimensions)?;
    let (target_width, target_height) = mode.resolution();
    let prepared = komitoto_sstv::image_proc::prepare_image(
        &DynamicImage::ImageRgb8(source),
        target_width,
        target_height,
        komitoto_sstv::image_proc::ResizeStrategy::Crop,
    );
    komitoto_sstv::SstvEncoder::new(mode)
        .encode(&prepared)
        .map(|samples| {
            samples
                .into_iter()
                .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16)
                .collect()
        })
        .map_err(|error| SstvError::Codec(error.to_string()))
}

/// Encode a selected mode for a consumer's native 12 kHz transmit path.
pub fn encode_rgb_mode_12k(
    mode: SstvMode,
    width: u32,
    height: u32,
    rgb: &[u8],
) -> Result<Vec<i16>, SstvError> {
    Ok(encode_rgb_mode(mode, width, height, rgb)?
        .into_iter()
        .step_by((MULTIMODE_SAMPLE_RATE_HZ / SAMPLE_RATE_HZ) as usize)
        .collect())
}

/// Decode a complete, already mode-selected 48 kHz SSTV recording.
pub fn decode_mode(mode: SstvMode, audio: &[f32]) -> Result<DecodedImage, SstvError> {
    let image = komitoto_sstv::SstvDecoder::new(mode)
        .decode(audio)
        .map_err(|error| SstvError::Codec(error.to_string()))?
        .to_rgb8();
    Ok(DecodedImage {
        width: image.width() as usize,
        height: image.height() as usize,
        rgb: image.into_raw(),
    })
}

fn mode_sample_count_12k(mode: SstvMode) -> usize {
    komitoto_sstv::spec::from_mode(mode)
        .total_samples()
        .div_ceil((MULTIMODE_SAMPLE_RATE_HZ / SAMPLE_RATE_HZ) as usize)
}

fn decode_mode_12k(
    mode: SstvMode,
    audio: &[f32],
    frequency_offset_hz: f32,
) -> Result<DecodedImage, SstvError> {
    let corrected = if frequency_offset_hz.abs() >= 1.0 {
        let frequencies = komitoto_sstv::dsp::fm_demodulate(audio, SAMPLE_RATE_HZ);
        let mut phase = 0.0_f64;
        frequencies
            .into_iter()
            .map(|frequency| {
                phase +=
                    TAU as f64 * (frequency - frequency_offset_hz as f64) / SAMPLE_RATE_HZ as f64;
                phase.sin() as f32
            })
            .collect::<Vec<_>>()
    } else {
        audio.to_vec()
    };
    let factor = (MULTIMODE_SAMPLE_RATE_HZ / SAMPLE_RATE_HZ) as usize;
    let mut upsampled = Vec::with_capacity(corrected.len() * factor);
    for (index, &sample) in corrected.iter().enumerate() {
        let next = corrected.get(index + 1).copied().unwrap_or(sample);
        for step in 0..factor {
            let fraction = step as f32 / factor as f32;
            upsampled.push(sample + (next - sample) * fraction);
        }
    }
    decode_mode(mode, &upsampled)
}

// Keep the public facade and protocol-independent definitions together while
// separating the implementation by responsibility. These files are included
// in one module so the existing private helper relationships remain intact.
include!("signal.rs");
include!("vis.rs");
include!("codec.rs");
include!("martin.rs");
include!("stream.rs");

#[cfg(test)]
include!("tests.rs");
