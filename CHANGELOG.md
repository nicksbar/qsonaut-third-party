# Changelog

## Unreleased

- Add independent `qsonaut-n3fjp` crate with the published API 2.2 command
  catalog, bounded framing, opt-in TCP client configuration, version discovery,
  notifications, scored entry submission, and direct ADIF support.
- Add the separate N3FJP station-network codec/client based on YAHAML captures,
  preserving contest fields and unknown messages without automatic log edits.
- Add loopback/fixture validation and explicit transmit authorization/disarm
  integration contracts. No UDP or consumer application changes are included.
