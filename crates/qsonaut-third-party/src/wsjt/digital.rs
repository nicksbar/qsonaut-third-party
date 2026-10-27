use qsonaut_modems::{AudioBlock, DecodeBatch};

use super::{common::decode_mode, WsjtDecodeConfig, WsjtMode};
use crate::AdapterError;

pub fn decode_ft8(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    decode_mode(audio, WsjtMode::Ft8, config)
}

pub fn decode_ft4(
    audio: &AudioBlock,
    config: &WsjtDecodeConfig,
) -> Result<DecodeBatch, AdapterError> {
    decode_mode(audio, WsjtMode::Ft4, config)
}
