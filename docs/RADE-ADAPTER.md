# RADE adapter

This document defines the isolated RADE integration boundary. It is a design
and build foundation only; QSONaut and QSONoid are not modified by this work.

## Upstream strategy

Development follows the `main` branch of the FreeDV `rade_c` repository so
that the adapter can track both RADE V1 and V2 as the upstream project evolves.
The reproducible native build helper currently pins
`0d5c5f7c27e650e3ca8d9e0f7d4781f2e73ee2b0`; release and consumer integration
must update that pin deliberately with a reviewed native-source provenance
record.

The adapter has one RADE surface with a version selector:

```text
RadeMode::V1
RadeMode::V2
```

V2 is not hidden behind a separate GUI path or compile-time product split. It
is presented through the same adapter API and is labeled in adapter metadata as
an upstream-development waveform. That label is required for consumers to
show status and compatibility information without duplicating their GUI
architecture.

## Boundary

```text
consumer-owned audio capture/resampling
        |
        v
qsonaut-third-party::rade
        |
        +-- Rust FFI and native build for rade_c
        +-- V1/V2 mode selection and status normalization
        +-- sample and buffer validation
        +-- upstream loss-test compatibility
        |
        v
first-party voice/audio contracts
```

The adapter does not own audio devices, threads, radio control, PTT, slot
scheduling, TX cancellation, UI state, QSO automation, or logging.

The native RADE pipeline operates at 8 kHz complex float32 IQ, while speech
input/output is 16 kHz. A real-valued SSB/off-air input requires the documented
real-to-IQ conversion. Device-rate conversion remains consumer-owned.

## Planned Rust surface

The eventual public adapter should normalize, without leaking C types:

- `RadeMode`: V1 or V2;
- `RadeCapabilities`: modem and speech rates, mode status, and TX/RX support;
- `RadeRxStatus`: searching, synchronized, SNR, frequency offset, and
  end-of-over state;
- streaming RX input and decoded speech output;
- streaming TX speech input and modem output;
- explicit validation errors for sample rate, channel layout, buffer shape,
  and native initialization failure.

The first-party `qsonaut-modems` contract crate now provides the generic
voice-audio/status types needed by this surface. The RADE-specific mapping in
`src/rade.rs` uses those types without adding a private parallel contract or
silently placing voice semantics in the GUI.

The upstream C API itself is feature-vector based. A complete speech adapter
still needs the upstream FARGAN/LPCNet feature extraction and synthesis layer;
that layer must be built and tested explicitly rather than misrepresented as
part of the RADE IQ modem ABI.

## Native build

The Rust crate keeps the native dependency opt-in, but the native source is
owned by this third-party integration boundary rather than duplicated in each
consumer. Build the pinned upstream checkout with:

```sh
./tools/build-rade-c.sh
```

Then enable the adapter feature with the printed checkout/build paths:

```sh
RADE_C_BUILD_DIR="${RADE_C_BUILD_DIR:-$RADE_C_DIR/build}"
cmake -S "$RADE_C_DIR" -B "$RADE_C_BUILD_DIR"
cmake --build "$RADE_C_BUILD_DIR"
RADE_C_DIR="$RADE_C_DIR" RADE_C_BUILD_DIR="$RADE_C_BUILD_DIR" \
  RADE_C_LIB_DIR="$RADE_C_BUILD_DIR/src" \
  cargo test -p qsonaut-third-party --features rade-c
```

`RADE_C_DIR` is expected to contain the upstream checkout. `RADE_C_BUILD_DIR`
selects its CMake build directory, and `RADE_C_LIB_DIR` may be used when the
library is installed or built elsewhere. The default workspace feature set
does not link or fetch native RADE code.

On Windows, the bundled build requires MSYS2 in addition to Visual Studio
Build Tools, CMake, and the Windows SDK. Install the MSYS2 `autoconf`,
`automake`, `libtool`, `make`, and `patch` packages. The build selects
`C:\\msys64\\usr\\bin\\bash.exe` automatically; set `MSYS2_BASH` when MSYS2
is installed elsewhere.

Consumers that ship the desktop/native RADE path can use
`--features rade-bundled`. When `RADE_C_DIR` is not supplied, the build script
invokes `tools/build-rade-c.sh`, which checks out the pinned upstream revision
into the shared cache and builds it there. This removes per-consumer source
checkout and environment setup while keeping the generic adapter feature
available for targets that provide their own native library.

The `native` module wraps the upstream feature/IQ API: context lifecycle,
V1/V2 selection, feature-frame TX, IQ-frame RX, end-of-over status, V2 data
symbols, synchronization, SNR, and audio-frequency offset. The optional
`rade-speech` feature adds `rade::speech::SpeechEncoder` and
`SpeechDecoder`, which keep the upstream LPCNet/FARGAN state inside the
adapter and expose 16 kHz `AudioBlock` frames. The decoder owns the required
five-frame FARGAN warm-up and returns no audio until it is ready.

`rade-speech` is separate from `rade-c` because it also links the upstream
Opus neural-vocoder build. It requires `RADE_C_DIR`, not only an installed
`librade`, so the build can locate the upstream FARGAN/LPCNet headers and
static Opus archive. The consumer still owns buffering, device I/O,
resampling, modem frame aggregation, and scheduling.

The helper does not copy RADE source into QSONaut or fetch a moving upstream
branch. It caches one immutable upstream checkout and builds the native
library beside it. The upstream C library is BSD-2-Clause; its required
attribution and the Opus/FARGAN dependency obligations remain part of the
third-party distribution notices.

## Validation gates

Before consumer integration, the adapter must pass:

1. native `rade_c` CTest coverage at the pinned revision;
2. Rust FFI lifecycle, buffer, and error tests;
3. deterministic V1 and V2 software loopbacks;
4. the upstream feature-vector loss baseline, within the upstream tolerance;
5. silence, malformed-input, and end-of-over tests;
6. separate host, Android cross-build, and live-radio evidence labels.

These tests establish software and fixture proof only. They do not claim
physical-radio or live on-air validation.

## Update workflow

Upstream-main tracking is development-only. An update PR must include:

- the new `rade_c` commit;
- native build and license/provenance review;
- V1 and V2 loopback results;
- loss-baseline comparison;
- API/status compatibility notes;
- updated `THIRD_PARTY_NOTICES.md` when the dependency or generated weights
  change.
