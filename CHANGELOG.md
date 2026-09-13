# Changelog

## 0.2.0 — 2026-09-13

- Refresh the pinned `mfsk-core` integration to upstream `0.11.0` plus the
  post-release binding commit, and keep CI and licensing notices on the same
  immutable revision.
- Expose the upstream WSJT protocol registry through a consumer-neutral
  capability and geometry API, including all FST4 and Q65 submodes.
- Expose the 0.11 caller-owned decode budget and local equalization controls
  for FT8, FT4, and FST4 without leaking `mfsk-core` types.

## Unreleased

- Add independent `qsonaut-n3fjp` crate with the published API 2.2 command
  catalog, bounded framing, opt-in TCP client configuration, version discovery,
  notifications, scored entry submission, and direct ADIF support.
- Add the separate N3FJP station-network codec/client based on YAHAML captures,
  preserving contest fields and unknown messages without automatic log edits.
- Add loopback/fixture validation and explicit transmit authorization/disarm
  integration contracts. No UDP or consumer application changes are included.
