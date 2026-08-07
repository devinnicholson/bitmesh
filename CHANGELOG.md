# Changelog

All notable user-visible changes to Bitmesh will be documented in this file.
The project follows [Semantic Versioning](https://semver.org/) for its Rust API.
Canonical certificate encodings are separately versioned as described in the
README.

No formal release has been published yet.

## [Unreleased]

### Added

- An executable FEN certification example.
- Explicit documentation of guarantees, boundaries, invariants, failure modes,
  complexity, compatibility, and versioning.
- Deterministic adversarial and canonical-serialization compatibility tests.
- Citation metadata, contribution guidance, and cross-platform CI.
- Typed validation errors for text fields that exceed the unsigned 16-bit
  length prefixes in `BMCOMPOSE` v1 and `BMDPOSCERT` v1.
- A board-binding check that requires certificate active masks to match the
  occupied non-barrier squares before a one-ply proof is accepted.
- Separate modules for board connectivity, certificate operations, public
  types, validation, canonical serialization, and tests.
- Deterministic boundary, mutation, stale-binding, and panic-contract tests.
- Security and support policies, DCO contribution terms, issue forms,
  dependency update configuration, and a release checklist.
- Scheduled advisory, license, duplicate-dependency, and dependency-source
  checks through `cargo-deny`.

### Changed

- Scoped decomposition claims to conservative board-local structural and
  one-ply evidence.
- Added package metadata and a Rust 1.88 minimum supported version.
- Licensed the crate GPL-3.0-or-later, matching its direct Shakmaty (GPL-3.0)
  dependency.
- Added example execution and package assembly to the stable Linux CI job.
- Limited packaged crate contents to source, examples, metadata, licensing,
  governance, and user-facing release documents.
- Added optimized tests and a crates.io publication dry run to CI.

### Compatibility

- Existing `BMDCERT` v1, `BMCOMPOSE` v1, and `BMDPOSCERT` v1 payloads
  and digest fixtures are unchanged.
- `bitmesh:conservative_legal_independence:v0` now rejects stale active masks.
