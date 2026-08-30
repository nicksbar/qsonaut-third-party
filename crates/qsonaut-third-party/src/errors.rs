#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AdapterError {
    #[error("{modem} requires {expected} Hz mono audio, got {actual} Hz")]
    UnsupportedSampleRate {
        modem: &'static str,
        expected: u32,
        actual: u32,
    },
}
