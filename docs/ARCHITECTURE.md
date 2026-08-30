# Architecture

```text
consumer audio capture and resampling
              |
              v
qsonaut-third-party adapters
              |
              v
mfsk-core / other external modem libraries
              |
              v
qsonaut-modems normalized contracts
```

The dependency direction is deliberate: third-party code is isolated here,
while first-party consumers depend on stable event and timing contracts. No
external library is allowed to leak its result types into QSONaut or QSONoid.

The SSTV implementation is kept in its own module because it has image and
VIS-specific results rather than WSJT-style text events. Its dependency and
upstream behavior remain documented in `THIRD_PARTY_NOTICES.md`.

WSJT-family modes share `wsjt::WsjtDecodeConfig`, `wsjt::WsjtMode`, and
`wsjt::decode`. Configuration and normalization are separated from the
protocol-family calls in `wsjt/digital.rs` and `wsjt/scans.rs`. The adapter
owns no slot scheduler: consumers use the mode metadata to decide when to
present complete audio to a decoder.

The adapter's 12 kHz requirement applies only to its input `AudioBlock`; it is
not a requirement that a consumer capture, monitor, or record at 12 kHz. See
[AUDIO-DECODER-CONTRACT.md](AUDIO-DECODER-CONTRACT.md) for the full-rate
capture and decoder-stream boundary.
