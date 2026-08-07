# Security policy

Bitmesh validates untrusted certificate structures and caller-supplied board
data. Security reports should identify the affected API, the smallest input
that triggers the issue, the observed resource use or failure, and the Rust
toolchain used.

## Supported versions

Bitmesh has not published a registry release. Security fixes are applied to the
default branch until the first release. After publication, this table will list
the supported release series.

| Version | Supported |
| --- | --- |
| Default branch | Yes |
| Unreleased snapshots | No |

## Reporting a vulnerability

Use GitHub's private vulnerability-reporting flow under **Security →
Advisories → Report a vulnerability**. Do not open a public issue for a
suspected denial of service, panic reachable through an API documented as
non-panicking, certificate acceptance bypass, digest ambiguity, or supply-chain
compromise.

Include:

- affected commit or release;
- a minimal reproducer;
- expected and observed behavior;
- impact and whether untrusted input is required; and
- any proposed embargo constraints.

You should receive an acknowledgement within seven days. A fix will preserve
the versioned certificate byte contracts unless the contract itself is
affected. Contract changes receive a new format version and migration notes.

## Security boundaries

The documented security boundary includes:

- `UnionFind::find`, `union`, and `connected` panic for indices outside
  their active domain;
- conservative decomposition may reject positions it cannot certify;
- Bitmesh does not establish position reachability, descendant independence,
  combinatorial-game values, or correctness of caller-supplied value digests;
  and
- SHA-256 digests provide content identifiers, not signatures or
  authentication.
