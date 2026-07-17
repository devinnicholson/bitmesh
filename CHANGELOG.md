# Changelog

All notable user-visible changes to Bitmesh will be documented in this file.
The project follows [Semantic Versioning](https://semver.org/) for its Rust API.
Canonical certificate encodings are separately versioned as described in the
README.

No formal release has been published yet.

## [Unreleased]

### Added

- An executable FEN certification example.
- Explicit documentation of guarantees, non-claims, invariants, failure modes,
  complexity, compatibility, and versioning.
- Deterministic adversarial and canonical-serialization compatibility tests.
- Citation metadata, contribution guidance, and cross-platform CI.

### Changed

- Scoped decomposition claims to conservative board-local structural and
  one-ply evidence, rather than future game-tree independence.
- Added package metadata and a Rust 1.85 minimum supported version.
