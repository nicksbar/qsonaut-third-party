# qsonaut-third-party

Adapters around third-party amateur-radio modems and N3FJP logging protocols.

This repository is the licensing boundary for protocol libraries that are not
QSONaut work products. Its adapters translate those libraries into the
UI-independent contracts defined by
[`qsonaut-modems`](https://github.com/nicksbar/qsonaut-modems). The contract is
consumed from immutable Git revisions. Local sibling checkouts can be used by
an uncommitted development-only patch when cross-repository work requires it.

The adapters cover FT8, FT4, FST4's 15–300 s submodes, WSPR, JT9, JT65A, every wired
Q65 submode, and MSK144 through the pinned `mfsk-core` Git revision, plus the
extracted SSTV streaming/VIS implementation and selected-channel CW DSP
adapter. WSJT modes use one `WsjtMode`/`WsjtDecodeConfig` dispatch surface and
return normalized `qsonaut-modems` batches. Consumers can use
`wsjt::protocol_capabilities()` to discover exact mode geometry and adapter
features instead of maintaining a second hardcoded mode matrix.

## N3FJP logging protocols

The standalone [`qsonaut-n3fjp`](crates/qsonaut-n3fjp) crate implements the
published N3FJP API 2.2 and the separate captured station-network protocol. It
provides configurable, opt-in TCP clients without modem dependencies. See
[protocol coverage and validation boundaries](docs/N3FJP.md).

## Consumer boundary

Consumers own audio capture and resampling. The WSJT adapter accepts mono
12 kHz `f32` audio, performs no device I/O, and returns normalized decode
events plus timing telemetry. Slot policy, TX-slot suppression, cancellation,
UI, radio state, and logging remain consumer-owned.

The frame decoders also accept an optional caller-owned wall-clock
budget and opt-in local Costas-pilot equalization through
`WsjtDecodeConfig`. These options apply to FT8, FT4, and FST4 only; capability
discovery reports that scope explicitly. UVPacket is an upstream experimental
packet family and remains outside this WSJT message adapter until a separate
first-party packet contract exists.

The maintenance pin is `mfsk-core` `c21458c90bcb853f20181e9e7cac3c2f9b4bad34`
(0.14.0 plus unreleased public-API corrections). The linked modem crate is
GPL-3.0-only; the independent original N3FJP crate remains GPL-3.0-or-later.
See the [maintenance validation report](docs/UPSTREAM-CATCHUP-2026-10-27.md)
for numeric comparisons, behavior changes, and remaining integration limits.

## Development

```sh
cargo fmt --all -- --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo metadata --format-version 1 --locked --no-deps
cargo check --locked -p qsonaut-third-party --target aarch64-linux-android
```

See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and
[docs/CONSUMER-INTEGRATION.md](docs/CONSUMER-INTEGRATION.md).

For the full-rate capture versus 12 kHz WSJT decoder boundary, see
[docs/AUDIO-DECODER-CONTRACT.md](docs/AUDIO-DECODER-CONTRACT.md).
