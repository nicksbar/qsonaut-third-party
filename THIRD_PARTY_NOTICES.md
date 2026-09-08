# Third-party notices

This repository is a licensing boundary for adapters around external modem
implementations. Each dependency retains its own license and attribution.

## mfsk-core

- Source: https://github.com/jl1nie/mfsk-core
- Revision: `ce9affcfd0c9c5205bbe464224e1c3ca55dafba5`
- The pinned revision corresponds to the `0.10.2` package used by this
  release line; update this entry whenever the dependency is refreshed.
- License: GPL-3.0-or-later
- Used for: WSJT-family digital modem decoding and synthesis.
- Upstream attribution: the project documents its derivation from WSJT-X and
  carries the corresponding attribution and license text.

The adapter crate is distributed under GPL-3.0-or-later because it links to
`mfsk-core`. Consumers must review the terms for the resulting combined
binary. This file is not a substitute for the complete upstream license.

## komitoto-sstv

- Source: https://github.com/IRendy/komitoto
- Revision: `c98945f7c89f714b3182457a86b15a0c43cb6de6`
- License: preserve and verify the upstream license with each release.
- Used for: SSTV image codec implementation.

The SSTV adapter preserves the upstream codec boundary and adds QSONaut's
streaming VIS, auto-target, frequency-offset, and 12 kHz integration logic.

## cw-dit

- Source: https://github.com/swilcox/cw-dit
- Revision: `153fc247ce6e4934c94e0cd2dcbf7887e368ec29`
- License: MIT OR Apache-2.0 (verify against the pinned upstream revision).
- Used for: CW DSP and Morse timing primitives.

The extracted `cw` adapter adds QSONaut's selected-channel filter, envelope
slicing, and streaming accumulation around the upstream IO-free crates.

## RADE (`rade_c`)

- Source: https://github.com/freedv/rade_c
- Development: follow upstream `main`; release builds must record the exact
  immutable commit used by the native build.
- License: BSD-2-Clause. Preserve the upstream license and attribution.
- Used for: RADE V1 and V2 digital-voice modem encoding and decoding.
- Native dependencies: the upstream build includes the FARGAN/Opus speech
  feature and synthesis path; record the exact dependency revisions and
  notices with each release.
- Status: V1 is the initial integration target. V2 is intentionally exposed
  through the same adapter surface, but must remain visibly labeled as an
  upstream-development waveform whose API, weights, and wire behavior may
  change incompatibly.

No RADE source is copied into this repository. The adapter must consume a
reviewed, reproducible `rade_c` checkout and must not fetch unpinned native
source during a normal consumer build.
