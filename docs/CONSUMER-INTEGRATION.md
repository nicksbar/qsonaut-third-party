# Consumer integration plan

QSONaut and QSONoid are intentionally unchanged while this repository is
validated.

## QSONaut

1. Add `qsonaut-modems` and `qsonaut-third-party` on an integration branch.
2. Replace the direct FT8/FT4 protocol calls in the decode worker with the
   adapter functions.
3. Compare old and new results on the existing generated and golden fixtures:
   messages, SNR, delta time, audio frequency, empty-slot behavior, and timing.
4. Keep QSONaut's consumer-owned slot gates, early-decode policy, TX-slot suppression,
   telemetry, PSK reporting, UI models, and TX safety unchanged.

## QSONoid

1. Add the same Rust dependencies to `qsonoid-engine`.
2. Keep Kotlin responsible for `AudioRecord`, permissions, route changes, and
   lifecycle. Convert captured mono audio at the boundary into `AudioBlock`.
3. Run the adapter on a Rust worker and deliver `DecodeEvent` values across the
   existing engine/JNI seam.
4. Validate Android arm64 output and real audio separately; a successful APK
   build is not physical-radio or live-decode proof.

## Acceptance gate before consumer edits

- both repositories pass format, tests, Clippy, and locked metadata checks;
- FT8/FT4 silence and generated-signal tests pass;
- fixture comparison procedure is documented;
- license notices and pinned revisions are reviewed;
- Android cross-compilation is tested for the adapter dependency path.

## WSJT mode mapping

| Consumer mode | Adapter mode | Slot |
|---|---|---:|
| FT8 | `WsjtMode::Ft8` | 15 s |
| FT4 | `WsjtMode::Ft4` | 7.5 s |
| FST4 | `WsjtMode::Fst4(Fst4Submode::...)` | 15–300 s |
| WSPR | `WsjtMode::Wspr` | 120 s |
| JT9 | `WsjtMode::Jt9` | 60 s |
| JT65 | `WsjtMode::Jt65` | 60 s |
| Q65-A30 | `WsjtMode::Q65` | 30 s |
| MSK144 | `WsjtMode::Msk144` | 15 s |

`WsjtDecodeConfig` is the shared adapter configuration. Its frequency range,
sync threshold, time window, score threshold, candidate budget, deep-decode
flag, and optional frequency hint are passed by the consumer; the adapter does
not read GUI profile state. The adapter translates those common controls into
each protocol's native search parameter type, including the asymmetric Q65
window and the WSPR/JT9/JT65 coarse-search controls.
`DecodeBatch::telemetry` reports elapsed decode time, input sample count, and
decoded event count without coupling the adapter to QSONaut's compute-backend
telemetry types.

## Audio boundary clarification

The adapter consumes a modem-specific 12 kHz `AudioBlock`, while a station may
capture and fan out a 48 kHz stream for display, monitoring, recording, or
other decoders. Consumers must use a stateful anti-aliased conversion and must
not replace the full-rate stream with a stateless 4:1 sample reduction. See
[AUDIO-DECODER-CONTRACT.md](AUDIO-DECODER-CONTRACT.md).

## Local checkout workflow

From the sibling checkout root, build this repository directly:

```sh
cd /home/nick/RigForge/qsonaut-third-party
cargo test --workspace --all-targets
```

The standalone repository manifest uses an immutable Git revision of
`qsonaut-modems`, so CI and consumers can check it out without a sibling
directory. During local cross-repository development, a temporary sibling path
override may be used, but it must not be committed or released. Update the Git
revision deliberately when the contract repository changes.
