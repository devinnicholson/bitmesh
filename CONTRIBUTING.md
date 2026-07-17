# Contributing to Bitmesh

Bitmesh is research software with byte-stable provenance formats. Changes need
both ordinary correctness review and a claim/compatibility review.

Before proposing a change:

1. Describe the supported board domain and the claim being changed.
2. Add a positive case, a nearby negative control, and an adversarial case when
   algorithm behavior changes.
3. Do not derive expected research values solely from Bitmesh itself; cite a
   hand derivation or independent oracle where the claim requires one.
4. Update the README and changelog whenever a public guarantee, limitation, API,
   proof kind, or payload changes.

Run the complete local gate:

```console
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
cargo rustdoc --lib -- -D missing-docs
```

## Certificate compatibility

`BMDCERT` v1 and `BMCOMPOSE` v1 are externally consumable byte contracts. Do not
update their golden payloads or digests merely to make a failing test pass.
Classify any proposed difference first:

- An unintended difference is a regression and must be fixed.
- A compatible extension must leave existing v1 payloads unchanged.
- An intentional incompatible format needs a new version or magic, migration
  notes, new fixtures, and explicit maintainer review.

Component ordering must not change canonical payloads. New tests should avoid
hash-map iteration order, wall-clock time, randomness without a recorded seed,
network access, and developer-local paths.

## Pull requests

Keep changes focused and include:

- the exact commands and toolchain used;
- the public claim or API impact;
- fixture provenance and expected output;
- compatibility impact on APIs, proof kinds, payloads, and digests; and
- known false-positive/false-negative risks.

Do not publish a package, create a release, or regenerate version tags as part
of an ordinary contribution.
