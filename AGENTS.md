# qsonaut-third-party repository instructions

## Purpose

This repository isolates adapters around external modem implementations and
their licensing obligations. It translates third-party results into the
first-party contracts from `qsonaut-modems`.

## Source organization

- `crates/qsonaut-third-party/src/lib.rs` is only the public facade and module
  re-export list.
- `errors.rs` owns adapter-level validation errors.
- `wsjt/mod.rs` is the WSJT-family public facade and dispatch entry point.
- `wsjt/config.rs` owns the standard mode matrix and shared decode settings.
- `wsjt/common.rs` owns PCM normalization and normalized result construction.
- `wsjt/digital.rs` owns FT8 and FT4 depth-aware decode paths.
- `wsjt/scans.rs` owns FST4, WSPR, JT9, JT65, Q65, and MSK144 paths.
- `sstv/` owns the extracted VIS, AFC, streaming, image-codec, and test
  components.
- `cw.rs` owns the reusable selected-channel CW DSP adapter and maps upstream
  Morse results to its own `CwDecode` type.
- Add future protocol families as separate modules (`sstv/`, `cw.rs`, or
  separate crates when dependencies or licenses warrant it); do not create a
  massive universal `lib.rs` or protocol module.
- Keep tests beside each adapter module and reserve integration tests for
  cross-adapter contracts or fixture parity.

## Dependency and licensing rules

- Every external modem dependency must be pinned to an immutable revision or
  exact released version before consumer integration.
- Record source, revision, license, purpose, and required attribution in
  `THIRD_PARTY_NOTICES.md`.
- Do not copy external source into this repository without recording its
  provenance and license.
- Do not describe this repository as license-free. Its combined binaries may
  inherit obligations from linked modem libraries.
- The temporary sibling path dependency on `qsonaut-modems` is for local
  development only. Replace it with a published version or immutable Git
  revision before release or consumer migration.

## Adapter design rules

- Return `qsonaut-modems` types; do not leak `mfsk-core`, codec, or upstream
  result types through the public API.
- Accept explicit `AudioBlock` inputs. Adapters do not own `cpal`, Android
  `AudioRecord`, device selection, resampling, or audio threads.
- Reject unsupported rates and malformed inputs explicitly; never silently
  reinterpret samples.
- Preserve protocol-provided SNR, time offset, and audio-frequency offset
  semantics. Do not convert an audio offset into RF frequency here.
- Keep slot scheduling, TX-slot suppression, cancellation, UI state, radio
  state, QSO sequencing, logging, and PSK reporting in consumers.
- Do not add transmit behavior without an explicit safety review and matching
  consumer-side disarm semantics.

## Validation requirements

Run from this repository root:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo metadata --format-version 1 --locked --no-deps
```

Adapter changes require tests for sample-rate rejection, silence/no-decode,
normalization, and at least one deterministic generated or golden waveform
when the upstream library makes one available. Distinguish synthetic fixture
proof from live radio or Android proof in reports.

## Consumer migration gate

Do not edit QSONaut or QSONoid while adapter work is unvalidated. Before
integration, document the pinned dependency revisions, compare old and new
messages/SNR/time/frequency results, validate empty-slot behavior, and verify
the target Android ABI separately. An APK build is not live audio or
physical-radio validation.

## Change procedure

1. Read `THIRD_PARTY_NOTICES.md`, `docs/ARCHITECTURE.md`, and
   `docs/CONSUMER-INTEGRATION.md`.
2. Put implementation in a logical adapter module, not `lib.rs`.
3. Add focused deterministic tests and update notices for dependency changes.
4. Run the complete validation commands.
5. State clearly whether the result is fixture-tested, host-tested, Android
   cross-compiled, or live-radio validated.
