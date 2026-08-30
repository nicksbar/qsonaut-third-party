# qsonaut-third-party

Adapters around third-party amateur-radio modem implementations.

This repository is the licensing boundary for protocol libraries that are not
QSONaut work products. Its adapters translate those libraries into the
UI-independent contracts defined by
[`qsonaut-modems`](https://github.com/nicksbar/qsonaut-modems). During local
development the contract is consumed from the sibling checkout; releases must
replace that path with a pinned immutable Git revision or published version.

The adapters cover FT8, FT4, all FST4 submodes, WSPR, JT9, JT65, Q65-A30, and
MSK144 through the pinned `mfsk-core` Git revision, plus the extracted SSTV
streaming/VIS implementation and selected-channel CW DSP adapter. WSJT modes use
one `WsjtMode`/`WsjtDecodeConfig` dispatch surface and return normalized
`qsonaut-modems` batches.

## Consumer boundary

Consumers own audio capture and resampling. The WSJT adapter accepts mono
12 kHz `f32` audio, performs no device I/O, and returns normalized decode
events plus timing telemetry. Slot policy, TX-slot suppression, cancellation,
UI, radio state, and logging remain consumer-owned.

## Development

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and
[docs/CONSUMER-INTEGRATION.md](docs/CONSUMER-INTEGRATION.md).
