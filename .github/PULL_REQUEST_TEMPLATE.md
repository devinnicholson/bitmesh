## Scope

Describe the change and the public guarantee or API it affects.

## Verification

List the exact commands and toolchains used.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `cargo rustdoc --all-features --lib -- -D warnings -D missing-docs`
- [ ] `cargo package --locked`

## Certificate compatibility

- [ ] Existing `BMDCERT` v1, `BMCOMPOSE` v1, and `BMDPOSCERT` v1
      bytes and digest fixtures remain unchanged.
- [ ] Proof-kind changes, new formats, and migration requirements are documented.
- [ ] Positive, nearby negative, and adversarial cases cover behavior changes.

## Provenance and contribution terms

- [ ] New fixtures identify their source or derivation.
- [ ] No generated research output, secret, or developer-local path is included.
- [ ] Every commit is signed off under the
      [Developer Certificate of Origin](../DEVELOPER_CERTIFICATE_OF_ORIGIN)
      using `git commit --signoff`.
