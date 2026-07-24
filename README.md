# Bitmesh

Bitmesh is an experimental Rust library for producing conservative structural
decomposition certificates from chess bitboards. It uses locked-pawn barriers
and 8-way board connectivity to identify candidate regions, then optionally
applies a one-ply movement screen.

The central guarantee is deliberately narrow:

> An accepted `ConservativeLegalIndependenceProof` says that, on the supplied
> board, the validated barrier is frozen and the checker found no geometric
> one-ply move or capture that crosses a certified region or removes the
> barrier.

It is **not** a proof that the regions remain independent throughout the future
legal game tree. It does not establish a combinatorial-game sum, calculate a
chess or CGT value, or prove that the input position is reachable.

Bitmesh is pre-1.0 research software. No formal package release has been
published yet. It is licensed [GPL-3.0-or-later](LICENSE), matching its
direct dependency on [Shakmaty](https://github.com/niklasf/shakmaty)
(GPL-3.0).

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
nothing about whether a chess move can alter the barrier or connect regions.

### Conservative one-ply screen

`verify_conservative_legal_independence` additionally requires:

- every barrier square to contain a pawn;
- every barrier pawn to be blocked forward by another barrier pawn (or be on the
  edge of the board);
- no barrier pawn to have an immediate geometric capture;
- every occupied non-barrier square to lie in a certified component; and
- no generated one-ply destination to remove the barrier, enter another
  component, or enter an uncertified free square.

The checker deliberately analyzes both colors and over-approximates selected
movement. For example, pawn quiet moves do not consult side to move, check, pins,
or starting-rank rules. This makes false rejections possible. Because the API
receives only `Board`, it also cannot inspect castling rights, en-passant state,
half-move counters, repetition history, or whose turn it is.

Most importantly, acceptance does not quantify over descendants. A future move
within one region might change the barrier or enable later interaction. Treat
the proof as a deterministic filter for candidate data, not a full game-tree
theorem.

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
SHA-256 structural digest. Component order does not affect that payload.
`position_bound_decomposition_certificate_digest` can additionally bind the
structural digest to caller-supplied canonical position text and a namespace.
Callers are responsible for actually canonicalizing that text.

`CompositionCertificate` exposes a `BMCOMPOSE` v1 payload binding:

- a validated strict decomposition digest;
- exactly one caller-supplied value digest per component root; and
- a caller-supplied digest for the composed result.

`validate_against_decomposition` checks the structural provenance and exact root
coverage. It does not verify the component values, recompute their sum, or prove
the chess/CGT correctness of the result.

Never use an unbound decomposition digest as the identity of a chess position:
different boards can have the same structural fields. Use a position-bound
digest when position identity matters.

## Stability and versioning

Bitmesh follows Semantic Versioning at the crate API level. Before 1.0, a minor
version may change Rust APIs. Patch releases should remain API compatible.

Canonical payload formats have their own explicit magic and version byte. The
checked-in compatibility tests freeze the current `BMDCERT` v1 and `BMCOMPOSE`
v1 byte contracts and digest fixtures. A breaking serialization change must use
a new payload version or magic; it must not silently reinterpret v1. The
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

No latency or throughput claim is made without a versioned benchmark.

## Development

The minimum supported Rust version is 1.88, matching the crate's let-chain
usage and CI floor.
Run the same checks as CI with:

```console
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
cargo rustdoc --lib -- -D missing-docs
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for change and compatibility requirements
and [CHANGELOG.md](CHANGELOG.md) for user-visible changes. Cite the software using
[CITATION.cff](CITATION.cff).
