## Summary

<!-- What changed, and why? -->

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo metadata --locked --no-deps`
- [ ] `cargo test --workspace --all-targets --locked`
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`

## Licensing and compatibility

- [ ] Third-party source/license changes are reflected in `THIRD_PARTY_NOTICES.md`.
- [ ] Upstream dependency revisions remain pinned and are reviewed.
- [ ] Consumer impact is described for QSONaut and QSONoid.
- [ ] Application-specific orchestration and platform code remain out of this repo.
