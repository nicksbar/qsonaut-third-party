//! FFI wrapper for the public `rade_c` API.
//!
//! Enable with the crate's `rade-c` feature and provide either
//! `RADE_C_LIB_DIR` or `RADE_C_DIR` (with `build/src/librade.so`). The native
//! library is intentionally external: it is a moving development dependency
//! during upstream tracking and an immutable source/build artifact at release.

use std::ffi::CString;
use std::ptr::NonNull;

use qsonaut_modems::{VoiceRxStatus, VoiceSyncState};

use super::RadeMode;

const RADE_VERBOSE_0: i32 = 0x8;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RadeIqSample {
    pub real: f32,
    pub imag: f32,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RadeNativeError {
    #[error("rade_c returned a null context")]
    NullContext,
    #[error("rade_c requires {expected} feature values, got {actual}")]
    FeatureCount { expected: usize, actual: usize },
    #[error("rade_c requires {expected} IQ samples, got {actual}")]
    InputSampleCount { expected: usize, actual: usize },
    #[error("rade_c returned an invalid output count {actual}")]
    InvalidOutputCount { actual: i32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RadeRxResult {
    pub features: Option<Vec<f32>>,
    pub status: VoiceRxStatus,
    pub end_of_over: bool,
    pub data_symbol: Option<f32>,
}

#[allow(improper_ctypes)]
extern "C" {
    fn rade_initialize();
    fn rade_finalize();
    fn rade_open(model_file: *mut std::os::raw::c_char, flags: i32) -> *mut std::ffi::c_void;
    fn rade_close(context: *mut std::ffi::c_void);
    fn rade_n_tx_out(context: *mut std::ffi::c_void) -> i32;
    fn rade_n_tx_eoo_out(context: *mut std::ffi::c_void) -> i32;
    fn rade_nin(context: *mut std::ffi::c_void) -> i32;
    fn rade_n_features_in_out(context: *mut std::ffi::c_void) -> i32;
    fn rade_n_eoo_bits(context: *mut std::ffi::c_void) -> i32;
    fn rade_tx(
        context: *mut std::ffi::c_void,
        output: *mut RadeIqSample,
        features: *mut f32,
    ) -> i32;
    fn rade_tx_eoo(context: *mut std::ffi::c_void, output: *mut RadeIqSample) -> i32;
    fn rade_rx(
        context: *mut std::ffi::c_void,
        features: *mut f32,
        has_eoo: *mut i32,
        eoo: *mut f32,
        input: *mut RadeIqSample,
    ) -> i32;
    fn rade_sync(context: *mut std::ffi::c_void) -> i32;
    fn rade_freq_offset(context: *mut std::ffi::c_void) -> f32;
    fn rade_snrdB_3k_est(context: *mut std::ffi::c_void) -> f32;
    fn rade_rx_get_data_symbol(context: *mut std::ffi::c_void) -> f32;
}

/// One RADE C context containing one transmitter and one receiver.
///
/// The upstream API currently supports only one context per process. This
/// wrapper therefore remains deliberately non-`Send`/non-`Sync`.
pub struct RadeContext {
    raw: NonNull<std::ffi::c_void>,
    mode: RadeMode,
}

impl RadeContext {
    pub fn open(mode: RadeMode) -> Result<Self, RadeNativeError> {
        let model = CString::new("").expect("empty model path has no nul byte");
        // SAFETY: the caller has selected the optional native backend; the C
        // library requires global initialization before opening a context.
        unsafe {
            rade_initialize();
            let raw = rade_open(
                model.as_ptr() as *mut std::os::raw::c_char,
                mode.native_flags() | RADE_VERBOSE_0,
            );
            let Some(raw) = NonNull::new(raw) else {
                rade_finalize();
                return Err(RadeNativeError::NullContext);
            };
            Ok(Self { raw, mode })
        }
    }

    pub fn mode(&self) -> RadeMode {
        self.mode
    }

    pub fn feature_count(&self) -> usize {
        // SAFETY: raw is a live context owned by self.
        unsafe { rade_n_features_in_out(self.raw.as_ptr()) as usize }
    }

    pub fn tx_features(&mut self, features: &[f32]) -> Result<Vec<RadeIqSample>, RadeNativeError> {
        let expected = self.feature_count();
        if features.len() != expected {
            return Err(RadeNativeError::FeatureCount {
                expected,
                actual: features.len(),
            });
        }
        // SAFETY: output and input buffers are sized from the live context.
        unsafe {
            let count = rade_n_tx_out(self.raw.as_ptr());
            if count < 0 {
                return Err(RadeNativeError::InvalidOutputCount { actual: count });
            }
            let mut output = vec![RadeIqSample::default(); count as usize];
            let actual = rade_tx(
                self.raw.as_ptr(),
                output.as_mut_ptr(),
                features.as_ptr() as *mut f32,
            );
            if actual < 0 || actual as usize > output.len() {
                return Err(RadeNativeError::InvalidOutputCount { actual });
            }
            output.truncate(actual as usize);
            Ok(output)
        }
    }

    pub fn tx_end_of_over(&mut self) -> Result<Vec<RadeIqSample>, RadeNativeError> {
        // SAFETY: output is sized using the live context.
        unsafe {
            let count = rade_n_tx_eoo_out(self.raw.as_ptr());
            if count < 0 {
                return Err(RadeNativeError::InvalidOutputCount { actual: count });
            }
            let mut output = vec![RadeIqSample::default(); count as usize];
            let actual = rade_tx_eoo(self.raw.as_ptr(), output.as_mut_ptr());
            if actual < 0 || actual as usize > output.len() {
                return Err(RadeNativeError::InvalidOutputCount { actual });
            }
            output.truncate(actual as usize);
            Ok(output)
        }
    }

    pub fn rx_iq(&mut self, input: &[RadeIqSample]) -> Result<RadeRxResult, RadeNativeError> {
        // SAFETY: raw is a live context.
        let expected = unsafe { rade_nin(self.raw.as_ptr()) };
        if expected < 0 {
            return Err(RadeNativeError::InvalidOutputCount { actual: expected });
        }
        let expected = expected as usize;
        if input.len() != expected {
            return Err(RadeNativeError::InputSampleCount {
                expected,
                actual: input.len(),
            });
        }
        // SAFETY: all buffers are sized from the live context and remain
        // valid for the duration of the synchronous C call.
        unsafe {
            let feature_count = self.feature_count();
            let mut features = vec![0.0; feature_count];
            let mut eoo = vec![
                0.0;
                if matches!(self.mode, RadeMode::V1) {
                    let count = rade_n_eoo_bits(self.raw.as_ptr());
                    if count < 0 {
                        return Err(RadeNativeError::InvalidOutputCount { actual: count });
                    }
                    count as usize
                } else {
                    0
                }
            ];
            let mut has_eoo = 0;
            let eoo_ptr = if eoo.is_empty() {
                std::ptr::null_mut()
            } else {
                eoo.as_mut_ptr()
            };
            let actual = rade_rx(
                self.raw.as_ptr(),
                features.as_mut_ptr(),
                &mut has_eoo,
                eoo_ptr,
                input.as_ptr() as *mut RadeIqSample,
            );
            if actual < 0 || actual as usize > features.len() {
                return Err(RadeNativeError::InvalidOutputCount { actual });
            }
            let synchronized = rade_sync(self.raw.as_ptr()) != 0;
            let status = VoiceRxStatus {
                state: if has_eoo != 0 {
                    VoiceSyncState::EndOfOver
                } else if synchronized {
                    VoiceSyncState::Synchronized
                } else {
                    VoiceSyncState::Searching
                },
                snr_db: synchronized.then(|| rade_snrdB_3k_est(self.raw.as_ptr())),
                audio_frequency_offset_hz: synchronized
                    .then(|| rade_freq_offset(self.raw.as_ptr())),
                frame_index: None,
            };
            let data_symbol = (actual > 0 && matches!(self.mode, RadeMode::V2))
                .then(|| rade_rx_get_data_symbol(self.raw.as_ptr()));
            features.truncate(actual as usize);
            Ok(RadeRxResult {
                features: (actual > 0).then_some(features),
                status,
                end_of_over: has_eoo != 0,
                data_symbol,
            })
        }
    }
}

impl Drop for RadeContext {
    fn drop(&mut self) {
        // SAFETY: raw was returned by rade_open and is dropped exactly once.
        unsafe { rade_close(self.raw.as_ptr()) }
    }
}

/// Finalize the process-global RADE library after all contexts are dropped.
pub fn finalize() {
    // SAFETY: callers must ensure no RadeContext remains alive.
    unsafe { rade_finalize() }
}
