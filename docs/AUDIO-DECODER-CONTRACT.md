# WSJT-family audio contract

The WSJT adapters wrap the pinned `mfsk-core` APIs; they do not change the
upstream modem signal model.

## Input representation

The adapter entry point accepts `qsonaut_modems::AudioBlock` and requires
12,000 Hz mono audio. The adapter converts normalized `f32` samples to the
`i16` PCM representation used by FT8, FT4, FST4, WSPR, JT9, JT65, and MSK144
paths where required. Q65's upstream implementation may operate on `f32`, but
the adapter still validates the shared 12 kHz boundary.

The 12 kHz value is a sample rate, not an instruction to throw away the
station's full-rate capture or to limit the radio/audio path to 12 kHz of
bandwidth. A common consumer arrangement is:

```text
48 kHz capture
    +--> waterfall / monitor / recorder
    +--> stateful anti-aliased resampler --> 12 kHz AudioBlock --> adapter
```

Consumers own that conversion and must retain resampler state across input
chunks. Taking every fourth sample, or averaging four samples independently in
each callback, is not a production replacement: it can alias energy and lose
continuity at chunk boundaries.

## Slot examples

| Mode | Duration | Adapter input at 12 kHz |
|---|---:|---:|
| FT8 | 15 s | 180,000 samples |
| FT4 | 7.5 s | 90,000 samples |
| FST4-15 | 15 s | 180,000 samples |
| WSPR | 120 s | 1,440,000 samples |
| JT9 / JT65 | 60 s | 720,000 samples |
| Q65-A30 | 30 s | 360,000 samples |
| MSK144 | 15 s | 180,000 samples |

These are decoder-window conventions. Slot clocks, early/late decode policy,
TX-slot suppression, capture timestamps, buffering, cancellation, and gap
handling remain consumer responsibilities.

## Adapter guarantees

`wsjt::decode` rejects an incorrect sample rate before invoking a protocol
decoder. It returns normalized messages, SNR, time offset, audio-frequency
offset, and telemetry through `qsonaut-modems`; it does not return RF
frequency, own a radio, or schedule a slot.

See [CONSUMER-INTEGRATION.md](CONSUMER-INTEGRATION.md) for the migration gate
and [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) for pinned sources and
licensing obligations.
