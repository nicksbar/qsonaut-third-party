# Upstream catch-up — 2026-10-27

## Scope and immutable revisions

The 0.3.0 adapter update moves `mfsk-core` across 522 commits from
`ef3fc8eaa4ec3936b76bbb21e8befd91728cb8e8` (0.11.1) to
`c21458c90bcb853f20181e9e7cac3c2f9b4bad34` (0.14.0 plus unreleased
public-API corrections). This is an exact Git pin, not a floating branch.

Other dependencies were checked and remain current:

| Dependency | Revision |
|---|---|
| qsonaut-modems 0.1.1 | `a0ef913c4684e8aa4711d13bbbe6775baa436db0` |
| cw-dit | `153fc247ce6e4934c94e0cd2dcbf7887e368ec29` |
| komitoto-sstv | `c98945f7c89f714b3182457a86b15a0c43cb6de6` |

The linked modem library now correctly declares **GPL-3.0-only**, reflecting
its WSJT-X derivation. The independent original N3FJP crate retains
GPL-3.0-or-later. See the repository notices for distribution obligations.

## Same-audio comparison

The `maintenance_snapshot` example captured deterministic 12 kHz f32 audio
using the old adapter before its dependency changed. Each waveform starts at
0.5 seconds in a complete slot, with a reproducible 0.002-amplitude noise
floor. The new adapter replayed those exact saved samples, rather than
comparing two independently generated inputs. All cases returned zero
events on silence and rejected 48 kHz input.

The message is `CQ N7UF DN26`, except WSPR's `K1ABC FN31 30`.
Numbers below are host measurements, not protocol-wide tolerance promises.

| Case | Old → new results |
|---|---|
| FT8 fast | 1 message both; SNR 59.421494 dB, DT 0.005000 s, frequency 1000 Hz unchanged |
| FT8 deep | 1 message both; SNR 39.860100 dB, DT 0.005000 s, frequency 1000 Hz unchanged |
| FT8 fast, gain 0.1 | 1 message both; SNR 59.359253 dB, DT 0.005000 s, frequency 1000 Hz unchanged |
| FT4 normal/deep | 1 message both; SNR 57.118244 dB, DT -0.000500 s, frequency 999.779785 Hz unchanged |
| FT4 normal, gain 0.1 | 1 message both; SNR 57.035023 dB, DT -0.000500 s, frequency 999.771606 Hz unchanged |
| FST4-15 | 1 message both; SNR 27.834841 dB and DT -0.000500 s unchanged; frequency 999.999939 → 1000.000000 Hz |
| WSPR | 1 message both; SNR 42.240650 → 48.658115 dB, DT -0.501333 → -0.488000 s, frequency 1500 → 1502.197266 Hz; drift suffix -0.50 → +0.00 Hz |
| JT9 | Expected message unchanged: SNR 29.040890 dB, DT 0.504000 s, frequency 1000.022278 Hz; three spurious `000AAA 000AAA RA90` rows removed |
| JT65 | Four duplicate expected messages → one expected message; strongest SNR -2.350672 → -2.351128 dB, DT 0.508750 s and frequency 1000.435730 Hz unchanged; no unexpected messages in this fixture |
| Q65-15A | 1 message both; SNR 6.513020 → 6.527782 dB, DT 0.493750 → 0.006250 s, frequency 1000 Hz unchanged |

WSPR now reports the center of the four tones, not tone zero, and uses the
upstream-corrected scan/SNR/time arithmetic. Q65 returns native protocol DT
rather than a buffer-relative start. These are observable corrections, not
RF-frequency adjustments.

The new public JT65 facade initially enabled randomized Chase decoding that
the old `decode_scan` did not use. It produced two unexpected messages in this
fixture. Setting public `ChaseParams.max_trials` to zero preserves the
unconditional hard-RS attempt without arbitrary text filtering. A regression
test fails before this fix and passes on clean/noisy audio at both adapter
depth settings. The facade still runs two passes rather than the old single
scan. Broad 120-candidate probes can produce spurious hard-RS rows on both
old and new versions; this update does not claim to solve JT65 false decodes
in general or to establish weak-signal sensitivity parity.

Synthesis lengths stayed identical in all seven compared families. FT8,
FT4, and FST4 waveform hashes changed with upstream synthesis maintenance;
WSPR, JT9, JT65, and Q65 hashes stayed identical. New-waveform round trips
also pass. A waveform hash change is not presented as bit-identical TX.

## Validation and reproduction

Host toolchain: Rust/Cargo 1.93.1 on Linux. Before migration, all 47 tests
passed. After migration, all 55 tests pass, including deterministic FT8/FT4
depth/gain normalization, JT65 clean/noisy regression, every mode's wrong-rate
and silence/empty handling, and zero-budget behavior.

The checked commands are:

- `cargo fmt --all -- --check`
- `cargo test --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
- `cargo metadata --format-version 1 --locked --no-deps`
- `cargo check --locked -p qsonaut-third-party --target aarch64-linux-android`
- `git diff --check`

For future comparisons, run `cargo run --release -p qsonaut-third-party
--example maintenance_snapshot -- capture /tmp/adapter-audio` on the old
revision with the comparison example, then run the same command with
`replay /tmp/adapter-audio` on the new revision. Keep both outputs and the
original audio unchanged. The example prints per-case message/SNR/DT/frequency
and synthesis length/hash and asserts silence and sample-rate boundaries.

The current tests do not need the temporary captured files. The first-party
`qsonaut-modems` workspace also passes format, all-target tests, strict
all-feature Clippy, and locked metadata at the revision above.

## Integration limits

- Validation is synthetic/host-tested and Android ARM64 **compile-checked**;
  it is not NDK-linked APK, physical Android audio, or live-radio validation.
- The adapter remains stateless per call. Persistent hashes, averaging,
  streaming, AP/QSO hints, new FST4/FST4W modes, and upstream budget telemetry
  require deliberate future contracts; none are silently introduced here.
- Existing 21-mode coverage and frame-only budget/equalizer scope are retained.
  WSPR's native scan ignores requested band bounds.
- `WsjtSyncScale::SyncFraction` is new. Exhaustive consumer matches must
  handle it. Search-default metadata describes adapter requests, not removed
  upstream registry defaults.
- Consumers retain resampling, clocks, cancellation, TX suppression, global
  arm/disarm, PTT, and scheduling. No radio or hardware TX behavior changed.