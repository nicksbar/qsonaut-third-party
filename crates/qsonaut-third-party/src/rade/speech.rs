//! RADE speech feature extraction and synthesis over the upstream FARGAN/LPCNet
//! implementation. It links the upstream Opus neural-vocoder build in addition
//! to `librade`.

use qsonaut_modems::{AudioBlock, AudioError};
use std::ffi::c_void;

const SPEECH_SAMPLE_RATE_HZ: u32 = 16_000;
const FRAME_SAMPLES: usize = 160;
const FEATURE_COUNT: usize = 36;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RadeSpeechError {
    #[error("RADE speech bridge returned a null context")]
    NullContext,
    #[error("RADE speech bridge rejected a frame: expected {expected} samples, got {actual}")]
    FrameSamples { expected: usize, actual: usize },
    #[error(
        "RADE speech bridge rejected a feature frame: expected {expected} values, got {actual}"
    )]
    FeatureCount { expected: usize, actual: usize },
    #[error("RADE speech bridge failed with status {0}")]
    Native(i32),
    #[error(transparent)]
    Audio(#[from] AudioError),
}

unsafe extern "C" {
    fn qsonaut_rade_speech_encoder_new() -> *mut c_void;
    fn qsonaut_rade_speech_encoder_free(context: *mut c_void);
    fn qsonaut_rade_speech_encode_frame(
        context: *mut c_void,
        pcm: *const f32,
        features: *mut f32,
    ) -> i32;
    fn qsonaut_rade_speech_decoder_new() -> *mut c_void;
    fn qsonaut_rade_speech_decoder_free(context: *mut c_void);
    fn qsonaut_rade_speech_decode_frame(
        context: *mut c_void,
        features: *const f32,
        pcm: *mut f32,
    ) -> i32;
}

pub struct SpeechEncoder {
    raw: *mut c_void,
}

impl SpeechEncoder {
    pub fn open() -> Result<Self, RadeSpeechError> {
        // SAFETY: the bridge returns an owned opaque context or null.
        let raw = unsafe { qsonaut_rade_speech_encoder_new() };
        if raw.is_null() {
            Err(RadeSpeechError::NullContext)
        } else {
            Ok(Self { raw })
        }
    }

    pub fn encode_frame(&mut self, pcm: &[f32]) -> Result<Vec<f32>, RadeSpeechError> {
        if pcm.len() != FRAME_SAMPLES {
            return Err(RadeSpeechError::FrameSamples {
                expected: FRAME_SAMPLES,
                actual: pcm.len(),
            });
        }
        let mut features = vec![0.0; FEATURE_COUNT];
        // SAFETY: both buffers are valid for the synchronous bridge call.
        let status = unsafe {
            qsonaut_rade_speech_encode_frame(self.raw, pcm.as_ptr(), features.as_mut_ptr())
        };
        if status != 0 {
            return Err(RadeSpeechError::Native(status));
        }
        Ok(features)
    }
}

impl Drop for SpeechEncoder {
    fn drop(&mut self) {
        // SAFETY: raw is owned by this encoder and dropped exactly once.
        unsafe { qsonaut_rade_speech_encoder_free(self.raw) }
    }
}

pub struct SpeechDecoder {
    raw: *mut c_void,
}

impl SpeechDecoder {
    pub fn open() -> Result<Self, RadeSpeechError> {
        // SAFETY: the bridge returns an owned opaque context or null.
        let raw = unsafe { qsonaut_rade_speech_decoder_new() };
        if raw.is_null() {
            Err(RadeSpeechError::NullContext)
        } else {
            Ok(Self { raw })
        }
    }

    pub fn decode_frame(
        &mut self,
        features: &[f32],
    ) -> Result<Option<AudioBlock>, RadeSpeechError> {
        if features.len() != FEATURE_COUNT {
            return Err(RadeSpeechError::FeatureCount {
                expected: FEATURE_COUNT,
                actual: features.len(),
            });
        }
        let mut pcm = vec![0.0; FRAME_SAMPLES];
        // SAFETY: both buffers are valid for the synchronous bridge call.
        let samples = unsafe {
            qsonaut_rade_speech_decode_frame(self.raw, features.as_ptr(), pcm.as_mut_ptr())
        };
        if samples < 0 {
            return Err(RadeSpeechError::Native(samples));
        }
        if samples == 0 {
            return Ok(None);
        }
        pcm.truncate(samples as usize);
        Ok(Some(AudioBlock::new(SPEECH_SAMPLE_RATE_HZ, pcm)?))
    }
}

impl Drop for SpeechDecoder {
    fn drop(&mut self) {
        // SAFETY: raw is owned by this decoder and dropped exactly once.
        unsafe { qsonaut_rade_speech_decoder_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FARGAN_WARMUP_FRAMES: usize = 5;

    #[test]
    fn encoder_extracts_a_feature_frame_from_speech_audio() {
        let mut encoder = SpeechEncoder::open().expect("speech encoder should open");
        let features = encoder
            .encode_frame(&[0.0; FRAME_SAMPLES])
            .expect("speech encoder should accept one 10 ms frame");
        assert_eq!(features.len(), FEATURE_COUNT);
        assert!(features.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn decoder_warms_up_then_returns_speech_audio() {
        let mut decoder = SpeechDecoder::open().expect("speech decoder should open");
        let features = [0.0; FEATURE_COUNT];
        for _ in 0..FARGAN_WARMUP_FRAMES {
            assert!(decoder
                .decode_frame(&features)
                .expect("decoder warm-up should succeed")
                .is_none());
        }
        let audio = decoder
            .decode_frame(&features)
            .expect("decoder should synthesize after warm-up")
            .expect("decoder should return one speech frame");
        assert_eq!(audio.sample_rate_hz, SPEECH_SAMPLE_RATE_HZ);
        assert_eq!(audio.samples.len(), FRAME_SAMPLES);
        assert!(audio.samples.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn normalized_speech_roundtrip_is_not_silent() {
        let mut encoder = SpeechEncoder::open().expect("speech encoder should open");
        let mut decoder = SpeechDecoder::open().expect("speech decoder should open");
        let mut decoded = Vec::new();
        for frame_index in 0..80 {
            let frame = (0..FRAME_SAMPLES)
                .map(|sample_index| {
                    let time = (frame_index * FRAME_SAMPLES + sample_index) as f32 / 16_000.0;
                    (std::f32::consts::TAU * 180.0 * time).sin() * 0.25
                })
                .collect::<Vec<_>>();
            let features = encoder
                .encode_frame(&frame)
                .expect("normalized speech should encode");
            if let Some(audio) = decoder
                .decode_frame(&features)
                .expect("speech features should decode")
            {
                decoded.extend(audio.samples);
            }
        }
        let rms = (decoded
            .iter()
            .map(|sample| f64::from(*sample) * f64::from(*sample))
            .sum::<f64>()
            / decoded.len().max(1) as f64)
            .sqrt();
        assert!(decoded.len() > FRAME_SAMPLES * 10);
        assert!(
            rms > 1e-4,
            "normalized speech roundtrip was silent: rms={rms}"
        );
    }
}
