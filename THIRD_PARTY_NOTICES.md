# Third-party notices

This repository is a licensing boundary for adapters around external modem
implementations. Each dependency retains its own license and attribution.

## mfsk-core

- Source: https://github.com/jl1nie/mfsk-core
- Revision: `4f2f678eda44f13f88c8e43a3f71adb892e3b84b`
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
