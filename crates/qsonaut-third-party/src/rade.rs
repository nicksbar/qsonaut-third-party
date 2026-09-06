//! RADE V1/V2 mode metadata and the native-adapter boundary.
//!
//! The upstream `rade_c` API consumes FARGAN feature vectors and produces
//! complex 8 kHz IQ. Speech feature extraction and synthesis are a separate
//! native dependency layer; this module deliberately does not pretend that
//! the modem API itself accepts speech `AudioBlock` values.

use qsonaut_modems::{ModemId, VoiceModemCapabilities};

#[cfg(feature = "rade-c")]
pub mod native;

pub const RADE_V1: ModemId = ModemId("rade-v1");
pub const RADE_V2: ModemId = ModemId("rade-v2");

/// RADE waveform selected for a native modem context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadeMode {
    V1,
    V2,
}

impl RadeMode {
    pub const fn modem_id(self) -> ModemId {
        match self {
            Self::V1 => RADE_V1,
            Self::V2 => RADE_V2,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::V1 => "RADE V1",
            Self::V2 => "RADE V2 (upstream development)",
        }
    }

    pub const fn development(self) -> bool {
        matches!(self, Self::V2)
    }

    /// Flags for `rade_open`, excluding verbosity and test flags.
    pub const fn native_flags(self) -> i32 {
        const RADE_USE_C_ENCODER: i32 = 0x1;
        const RADE_USE_C_DECODER: i32 = 0x2;
        const RADE_MODE_V2: i32 = 0x10;

        let flags = RADE_USE_C_ENCODER | RADE_USE_C_DECODER;
        match self {
            Self::V1 => flags,
            Self::V2 => flags | RADE_MODE_V2,
        }
    }

    pub const fn capabilities(self) -> VoiceModemCapabilities {
        VoiceModemCapabilities {
            modem: self.modem_id(),
            label: self.label(),
            modem_input_rate_hz: 8_000,
            modem_output_rate_hz: 8_000,
            speech_input_rate_hz: 16_000,
            speech_output_rate_hz: 16_000,
            supports_receive: true,
            supports_transmit: true,
            development: self.development(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_and_v2_share_the_adapter_surface() {
        assert_eq!(RadeMode::V1.capabilities().speech_input_rate_hz, 16_000);
        assert_eq!(RadeMode::V2.capabilities().speech_output_rate_hz, 16_000);
        assert_ne!(RadeMode::V1.native_flags(), RadeMode::V2.native_flags());
        assert!(!RadeMode::V1.development());
        assert!(RadeMode::V2.development());
    }
}
