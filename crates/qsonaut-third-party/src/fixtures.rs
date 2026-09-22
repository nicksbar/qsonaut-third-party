//! Deterministic, generated adapter fixtures for consumer integration tests.

use qsonaut_modems::{FixtureDecode, FixtureProvenance, ModemFixture};

use crate::wsjt::WsjtMode;

const GENERATED: FixtureProvenance = FixtureProvenance {
    source: "qsonaut-third-party generated waveform",
    license: "GPL-3.0-or-later",
};

const FT8_DECODE: &[FixtureDecode] = &[FixtureDecode {
    modem: WsjtMode::Ft8.modem_id(),
    message: "CQ N7UF DN26",
}];

const FT4_DECODE: &[FixtureDecode] = &[FixtureDecode {
    modem: WsjtMode::Ft4.modem_id(),
    message: "CQ N7UF DN26",
}];

/// A generated WSJT fixture. Consumers retain audio storage and execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WsjtFixture {
    pub mode: WsjtMode,
    pub compose: &'static str,
    pub tone_hz: u16,
    pub contract: ModemFixture,
}

/// Deterministic, provenance-carrying fixtures for basic FT8 and FT4 paths.
pub const WSJT_FIXTURES: &[WsjtFixture] = &[
    WsjtFixture {
        mode: WsjtMode::Ft8,
        compose: "CQ N7UF DN26",
        tone_hz: 1_000,
        contract: ModemFixture {
            name: "generated-ft8-cq",
            modem: WsjtMode::Ft8.modem_id(),
            sample_rate_hz: 12_000,
            channels: 1,
            provenance: GENERATED,
            expected_decodes: FT8_DECODE,
        },
    },
    WsjtFixture {
        mode: WsjtMode::Ft4,
        compose: "CQ N7UF DN26",
        tone_hz: 1_000,
        contract: ModemFixture {
            name: "generated-ft4-cq",
            modem: WsjtMode::Ft4.modem_id(),
            sample_rate_hz: 12_000,
            channels: 1,
            provenance: GENERATED,
            expected_decodes: FT4_DECODE,
        },
    },
];

#[cfg(test)]
mod tests {
    use qsonaut_modems::AudioBlock;

    use crate::{wsjt, AdapterError};

    use super::*;

    #[test]
    fn generated_fixtures_keep_provenance_and_normalized_expectations() {
        assert_eq!(WSJT_FIXTURES.len(), 2);
        assert!(WSJT_FIXTURES.iter().all(|fixture| {
            fixture.contract.provenance.source.contains("generated")
                && fixture.contract.expected_decodes.len() == 1
        }));
    }

    #[test]
    fn fixture_modes_reject_wrong_rate_and_silence_decodes_cleanly() {
        let wrong_rate = AudioBlock::new(48_000, Vec::new()).unwrap();
        let error = wsjt::decode(&wrong_rate, WsjtMode::Ft8, &Default::default()).unwrap_err();
        assert_eq!(
            error,
            AdapterError::UnsupportedSampleRate {
                modem: "ft8",
                expected: 12_000,
                actual: 48_000,
            }
        );
        let silence = AudioBlock::new(12_000, vec![0.0; 12_000]).unwrap();
        assert!(wsjt::decode(&silence, WsjtMode::Ft4, &Default::default())
            .unwrap()
            .events
            .is_empty());
    }
}
