# Release checklist

This checklist prepares a Bitmesh release. Publishing to crates.io, creating a
tag, and creating a GitHub release require explicit maintainer approval.

## Contract freeze

- [ ] Public Rust API changes are documented in `CHANGELOG.md`.
- [ ] `BMDCERT` v1, `BMCOMPOSE` v1, and `BMDPOSCERT` v1 golden bytes and
      digests are unchanged.
- [ ] Any new byte contract has distinct magic or a new version byte.
- [ ] The proof-kind string and claim boundary are reviewed.
- [ ] `Cargo.toml`, `Cargo.lock`, README, changelog, and `CITATION.cff`
      agree on the release version and minimum Rust version.

## Reproducibility gate

Run from a clean clone:

```console
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
cargo test --locked --release
cargo rustdoc --all-features --lib -- -D warnings -D missing-docs
cargo run --locked --example certify_fen -- \
  '7k/8/8/p1p1p1p1/PpPpPpPp/1P1P1P1P/8/K7 w - - 0 1'
cargo package --locked --list
cargo publish --locked --dry-run
```

- [ ] CI passes on the minimum Rust version, stable Rust, Linux, macOS, and
      Windows.
- [ ] The packaged crate contains no local paths, generated results, secrets,
      or unrelated files.
- [ ] A clean consumer crate builds against the packaged artifact.
- [ ] Dependency licenses and security advisories have been reviewed.

## Publication

- [ ] The exact commit is approved for publication.
- [ ] `cargo publish --locked` completes successfully.
- [ ] An immutable signed `vX.Y.Z` tag points to the published commit.
- [ ] The GitHub release records checksums and user-visible changes.
- [ ] docs.rs finishes successfully.
- [ ] Downstream compatibility checks run against the registry release.
