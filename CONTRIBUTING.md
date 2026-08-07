# Contributing to Bitmesh

Bitmesh is research software with byte-stable provenance formats. Changes need
both ordinary correctness review and a claim/compatibility review.

Before proposing a change:

1. Describe the supported board domain and the claim being changed.
2. Add a positive case, a nearby negative control, and an adversarial case when
   algorithm behavior changes.
3. Ground expected research values in a cited hand derivation or independent
   oracle where the claim requires one.
4. Update the README and changelog whenever a public guarantee, limitation, API,
   proof kind, or payload changes.

## Developer Certificate of Origin

Contributions use the
[Developer Certificate of Origin 1.1](DEVELOPER_CERTIFICATE_OF_ORIGIN). Sign
off every commit:

```console
git commit --signoff
```

The sign-off records that you have the right to submit the contribution under
this repository's license. Keep authorship and third-party provenance in the
commit history.

Run the complete local gate:

```console
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
cargo test --locked --release
cargo rustdoc --all-features --lib -- -D warnings -D missing-docs
cargo run --locked --example certify_fen -- \
  '7k/8/8/p1p1p1p1/PpPpPpPp/1P1P1P1P/8/K7 w - - 0 1'
cargo package --locked
cargo publish --locked --dry-run
```

## Certificate compatibility

`BMDCERT` v1, `BMCOMPOSE` v1, and `BMDPOSCERT` v1 are externally
consumable byte contracts. Classify every proposed golden-payload or digest
difference before editing a fixture:

- An unintended difference is a regression and must be fixed.
- A compatible extension must leave existing v1 payloads unchanged.
- An intentional incompatible format needs a new version or magic, migration
  notes, new fixtures, and explicit maintainer review.

Preserve canonical payloads across component orderings. New tests should avoid
hash-map iteration order, wall-clock time, randomness without a recorded seed,
network access, and developer-local paths.

## Pull requests

Keep changes focused and include:

- the exact commands and toolchain used;
- the public claim or API impact;
- fixture provenance and expected output;
- compatibility impact on APIs, proof kinds, payloads, and digests; and
- known false-positive/false-negative risks.

Package publication, releases, and version tags belong to the maintainer release
process.
