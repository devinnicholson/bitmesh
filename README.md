# Bitmesh

[![CI](https://github.com/devinnicholson/bitmesh/actions/workflows/ci.yml/badge.svg)](https://github.com/devinnicholson/bitmesh/actions/workflows/ci.yml)

Bitmesh is an experimental Rust library for producing conservative structural
decomposition certificates from chess bitboards. It uses locked-pawn barriers
and 8-way board connectivity to identify candidate regions, then optionally
applies a one-ply movement screen.

The central guarantee is deliberately narrow:

> An accepted `ConservativeLegalIndependenceProof` says that, on the supplied
> board, the certificate's active masks match the occupied non-barrier squares,
> the validated barrier is frozen, and every generated geometric one-ply
> destination stays inside its certified region while preserving the barrier.

The guarantee ends at the supplied board. Descendant search, combinatorial-game
sums and values, and position reachability lie outside the contract.

Bitmesh is pre-1.0 research software. No formal package release has been
published yet. It is licensed [GPL-3.0-or-later](LICENSE), matching its
direct dependency on [Shakmaty](https://github.com/niklasf/shakmaty)
(GPL-3.0).

## Status and role

| Item | Current state |
| --- | --- |
| Crate | `bitmesh` 0.1.0 research candidate |
| Input | A caller-supplied `shakmaty::Board` |
| Output | Structural, position-bound, and composition-provenance certificates |
| Minimum Rust | 1.88 |
| License | GPL-3.0-or-later |
| Registry release | Pending |
| Research snapshot | [`v0.1.0-alpha.1`](https://github.com/devinnicholson/bitmesh/releases/tag/v0.1.0-alpha.1) |

## Start here

| Goal | Entry point |
| --- | --- |
| Verify the repository | Run the [five-minute source check](#five-minute-source-check) |
| Evaluate one position | Run the [FEN example](#executable-fen-example) |
| Integrate the API | Follow the [library example](#library-api) |
| Audit a certificate | Read [what each layer guarantees](#what-each-layer-guarantees) and [failure modes](#failure-modes) |
| Prepare a release | Follow the [release checklist](docs/RELEASE_CHECKLIST.md) |

Bitmesh is the optional chess-structure layer in the
[Partizan](https://github.com/devinnicholson/partizan) stack. It can support
[Astralbase](https://github.com/devinnicholson/astralbase) domain gates and
bind caller-supplied component digests into composition provenance.
[Thermograph](https://github.com/devinnicholson/thermograph) owns finite-game
values and comparison. Bitmesh never computes a component value or proves a
disjunctive sum; Partizan must verify those claims independently before
promoting a result.

## Five-minute source check

Rust 1.88 or newer is required.

```console
git clone https://github.com/devinnicholson/bitmesh.git
cd bitmesh
cargo test --locked
```

The crate has no registry release yet. Pin integrations to an audited commit
until a release appears on crates.io.

## Executable FEN example

The checked-in example accepts one FEN argument, prints the structural
certificate, and reports whether the conservative one-ply screen accepted it:

```console
cargo run --example certify_fen -- \
  '7k/8/8/p1p1p1p1/PpPpPpPp/1P1P1P1P/8/K7 w - - 0 1'
```

This hand-checkable position contains a zig-zag wall of mutually blocked pawns
from the a-file to the h-file. The kings occupy different structural regions.
The command reports two components and an accepted
`bitmesh:conservative_legal_independence:v0` proof.

## Library API

Library users can run the same operation directly:

```rust
use bitmesh::{DecompositionStatus, certify_decomposition,
              verify_conservative_legal_independence};
use shakmaty::{CastlingMode, Chess, Position, fen::Fen};
use std::str::FromStr;

let fen = Fen::from_str(
    "7k/8/8/p1p1p1p1/PpPpPpPp/1P1P1P1P/8/K7 w - - 0 1",
)
.expect("example FEN parses");
let position: Chess = fen
    .into_position(CastlingMode::Standard)
    .expect("example is a valid standard-chess position");
let certificate = certify_decomposition(position.board());

assert_eq!(certificate.status, DecompositionStatus::Strict);
let proof = verify_conservative_legal_independence(position.board(), &certificate)
    .expect("example passes the conservative screen");
assert_eq!(proof.component_count, 2);
```

The FEN parser checks ordinary chess position validity before Bitmesh receives
the board. The Bitmesh API itself accepts a `shakmaty::Board`, including boards
constructed without a reachability or legality check.

## What each layer guarantees

### Structural partition

- `partition_board` treats the supplied barrier squares as absent and computes
  8-connected components among the remaining squares.
- `get_locked_pawns` selects pawns whose forward square is occupied or off-board
  and that have no opposing piece on a pawn-attack square.
- `certify_decomposition` returns `Strict` only when the selected barrier leaves
  occupied non-barrier material in at least two regions.
- `DecompositionCertificate::validate` checks status/mask consistency, component
  roots, disjointness, active-square containment, and closure under 8-way
  adjacency.

These are board-graph claims. A strict structural certificate alone says
only that the selected barrier separates the recorded occupied material.

### Conservative one-ply screen

`verify_conservative_legal_independence` additionally requires:

- every barrier square to contain a pawn;
- every barrier pawn to be blocked forward by another barrier pawn (or be on the
  edge of the board);
- no barrier pawn to have an immediate geometric capture;
- the union of certificate active masks to equal the occupied non-barrier
  squares on the supplied board;
- every occupied non-barrier square to lie in a certified component; and
- no generated one-ply destination to remove the barrier, enter another
  component, or enter an uncertified free square.

The checker deliberately analyzes both colors and over-approximates selected
movement. Pawn quiet moves, for example, are generated without side-to-move,
check, pin, or starting-rank information. The `Board` input also omits castling
rights, en-passant state, half-move counters, repetition history, and turn.
These choices can produce false rejections.

Acceptance quantifies over the supplied board. A move within one region can
change the barrier or enable later interaction, so research pipelines should
use the proof as a deterministic candidate-data filter.

## Failure modes

Expected negative results are returned as data:

- `DecompositionStatus::Rejected` distinguishes no detected locked barrier from
  fewer than two active regions.
- `DecompositionCertificateValidationError` identifies malformed or internally
  inconsistent structural certificates.
- `ConservativeLegalIndependenceError` identifies an invalid/non-strict
  certificate, a missing/non-pawn/mobile/capturable barrier square, material
  outside certified regions, or a crossing/uncertified destination.
- `CompositionCertificateValidationError` identifies incomplete, duplicated,
  stale, or structurally inconsistent composition provenance.

`UnionFind::find`, `union`, and `connected` panic when given an index outside
their active domain. FEN parsing and position validation errors occur before the
Bitmesh API is called in the example.

## Composition and provenance certificates

`DecompositionCertificate` exposes a `BMDCERT` v1 canonical byte payload and a
SHA-256 structural digest. Component ordering leaves that payload unchanged.
`position_bound_decomposition_certificate_digest` can additionally bind the
structural digest to caller-supplied canonical position text and a namespace.
Callers are responsible for actually canonicalizing that text.

`CompositionCertificate` exposes a `BMCOMPOSE` v1 payload binding:

- a validated strict decomposition digest;
- exactly one caller-supplied value digest per component root; and
- a caller-supplied digest for the composed result.

`validate_against_decomposition` checks the structural provenance and exact root
coverage. Component-value verification, sum recomputation, and chess/CGT
correctness remain the caller's responsibility.

An unbound decomposition digest identifies structural certificate fields.
Different boards can share those fields. Position-sensitive pipelines should
use the position-bound digest with a documented canonical text format and
namespace.

`BMCOMPOSE` v1 and `BMDPOSCERT` v1 store caller text with unsigned 16-bit length
prefixes. Each value digest, canonical position, and context namespace is
limited to 65,535 UTF-8 bytes. Oversized fields return typed validation errors.

## Stability and versioning

Bitmesh follows Semantic Versioning at the crate API level. Before 1.0, a minor
version may change Rust APIs. Patch releases should remain API compatible.

Canonical payload formats have their own explicit magic and version byte. The
checked-in compatibility tests freeze the current `BMDCERT` v1,
`BMCOMPOSE` v1, and `BMDPOSCERT` v1 byte contracts and digest fixtures. A
breaking serialization change must use a new payload version or magic; v1
semantics remain fixed. The
`bitmesh:conservative_legal_independence:v0` proof kind remains experimental and
must be matched exactly by downstream manifests.

## Complexity

Chess fixes the graph at 64 vertices, so all public operations have a small
constant bound in practice. Expressed for a generalized `V`-square graph:

- union-find construction and partitioning are `O(V alpha(V))` time and `O(V)`
  space, with at most eight adjacency checks per square;
- certificate construction and validation are `O(V alpha(V))` time and `O(V)`
  space for the fixed-degree board graph;
- the conservative screen visits each occupied square and its bounded chess
  attack set, so it is `O(V)` time and space on the board; and
- canonical serialization and hashing are linear in the number of certificate
  components and digest text bytes.

The repository currently contains no versioned latency or throughput benchmark.

## Development

The minimum supported Rust version is 1.88, matching the crate's let-chain
usage and CI floor.
Run the same checks as CI with:

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

See [CONTRIBUTING.md](CONTRIBUTING.md) for change and compatibility requirements
and [CHANGELOG.md](CHANGELOG.md) for user-visible changes. Security reports
follow [SECURITY.md](SECURITY.md); usage support follows
[SUPPORT.md](SUPPORT.md). Cite the software using [CITATION.cff](CITATION.cff).
