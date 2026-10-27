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
| JT65A | `WsjtMode::Jt65` | 60 s |
| Q65 | `WsjtMode::Q65(Q65Submode::...)` | 15–300 s |
| MSK144 | `WsjtMode::Msk144` | 15 s |

`WsjtDecodeConfig` is the shared adapter configuration. Its frequency range,
sync threshold, time window, score threshold, candidate budget, deep-decode
flag, optional frequency hint, optional decode wall-clock budget, and optional
local equalization flag are passed by the consumer; the adapter does not read
GUI profile state. The last two options are supported for FT8, FT4, and FST4;
the other protocol families retain their native search behavior. Consumers
should use `wsjt::protocol_capabilities()` to enable controls from capability
metadata rather than assuming every mode accepts every field. The adapter
translates the common controls into each protocol's native search parameter
type, including the asymmetric Q65 window and the WSPR/JT9/JT65 coarse-search
controls.
`DecodeBatch::telemetry` reports elapsed decode time, input sample count, and
decoded event count without coupling the adapter to QSONaut's compute-backend
telemetry types.

`Q65Submode` exposes `A15`, `A30`, `A60`, `B60`, `C60`, `D60`, `E60`, `D120`,
`E120`, and `A300`. JT65 remains JT65A because the pinned backend does not
currently provide usable JT65B or JT65C protocol types.

The upstream registry describes the FT/JT/Q/FST4 geometry. MSK144 is
also returned by `protocol_capabilities()`, but it is represented with
adapter-owned geometry because its separate burst decoder is not part of that
registry and has no FFT1 decode stage. Experimental UVPacket entries are not
returned as WSJT modes.

## Public decoder migration and deliberate limits

The maintenance pin uses `decoder::{AnyDecoder, Decoder, SlotInput}` and
`default_params`, never the now-private request/pipeline modules. Public
adapter function/config/result names remain unchanged. The added
`WsjtSyncScale::SyncFraction` case accurately describes scan scores; consumers
with exhaustive matches must handle it before upgrading.

- A fresh decoder is created per call. No cross-slot averaging, callsign-table
   accumulation, persistent state, period index, or station/QSO hints are added.
   AP is explicitly off, including upstream blind CQ hypotheses.
- FT8 maps `deep_decode=false/true` to Fast/Normal. FT4 retains OSD in both
   settings and explicitly chooses SinglePass/three SIC rounds. JT65 uses Normal
   with `Jt65Extras::chase.max_trials = 0`: legacy `decode_scan` used hard-decision
   RS, not the separate randomized `decode_scan_chase`. Zero trials keeps the
   unconditional zero-erasure RS attempt without enabling new Chase hypotheses
   or changing acceptance gates. The public facade still requires two passes,
   so it cannot reproduce the legacy single-pass scan exactly. At 900–1200 Hz,
   8 candidates, and the unchanged .1 score/±2 s window, clean and deterministic
   low-noise 1000 Hz CQ fixtures return only the expected message (the old pin
   returned four copies). Candidate limits remain caller-owned: 120 candidates
   still produce spurious hard-RS decodes on this fixture, also present in the
   old wide-candidate scan; this correction is not a general false-decode cure.
   Other families use upstream defaults and ignore this adapter flag.
- FT8/FT4/FST4 use the original clamp/round i16 conversion, bypassing upstream
   float RMS normalization. Scan engines continue receiving original f32 audio.
- Budget/equalizer capability flags remain frame-only. Budgets are cooperative,
   not hard deadlines; a caller must still own cancellation and slot policy.
   Frequency hints are wired only for FT4 and MSK144. WSPR ignores requested
   band bounds; this adapter does not pretend otherwise by filtering results.
- Capability geometry still comes from the registry; search defaults now mean
   adapter requests (100–3000 Hz, 120 candidates, .6 frame sync/.1 scan score),
   not deleted upstream registry/GUI defaults. MSK144 keeps zero sentinels;
   no sniper entry point is exposed and sniper defaults are `None`.
- Native corrected SNR, protocol-relative DT, and audio frequency are returned
   directly. WSPR now reports center frequency rather than tone zero. Q65 DT
   no longer reports buffer-relative start. These are intentional observable
   corrections, not RF-frequency calculations. See the [numeric report](UPSTREAM-CATCHUP-2026-10-27.md).

The adapter still exposes 21 modes, not new upstream FST4-900/1800, FST4W,
UVPacket, or every upstream option. `SlotResult` native details, candidate and
budget reports, streaming callbacks, known-signal subtraction, AP/hints,
averaging, and protocol-specific search controls remain behind the boundary.
Promoting them requires a deliberate first-party contract change, not leaking
upstream types. No hardware TX, automatic QSO sequencing, logging, or scheduling
is introduced.

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

The standalone repository manifest uses immutable Git revisions for its
external contracts and modem libraries, so CI and consumers do not need sibling
directories. During local cross-repository development, a temporary sibling
path override may be added locally, but it must not be committed or released.
Update each Git revision deliberately when a contract repository changes.

## N3FJP integration

The `qsonaut-n3fjp` crate is independently usable without modem dependencies.
Follow [N3FJP.md](N3FJP.md) for connection configuration, entry/event mapping,
ambiguous-result handling, and the mandatory consumer transmit/disarm wiring.
QSONaut and QSONoid have not been modified for this adapter.
