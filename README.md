# Bitmesh

[![Crates.io](https://img.shields.io/crates/v/bitmesh.svg)](https://crates.io/crates/bitmesh)
[![Docs.rs](https://docs.rs/bitmesh/badge.svg)](https://docs.rs/bitmesh)

Bitmesh is an extension of standard chess bitboards to support high-performance graph theory and topological partitioning.

## Overview

Traditional chess engines evaluate positions as monolithic game states. In Combinatorial Game Theory (CGT), games are often evaluated as sums of independent, non-interacting sub-games. Bitmesh provides the mathematical primitives required to decouple a single 8x8 chessboard into independent topological components using bitwise operations.

Built on top of the `shakmaty` crate, Bitmesh algorithms execute in microseconds, making the library suitable for high-throughput node evaluation and large-scale retrograde analysis.

## Features

- **Reachability Analysis**: Compute the transitive closure of piece mobility across barrier structures (e.g., locked pawn chains).
- **Disjoint-Set (Union-Find) on Bitboards**: Fast grouping of reachable squares into isolated graph components.
- **Topological Barrier Detection**: Mathematically identify strictly impassable board structures.

## Example Usage

```rust
use bitmesh::{UnionFind, get_locked_pawns, find_subsystems};
use shakmaty::{Chess, fen::Fen, Position};
use std::str::FromStr;

let fen = Fen::from_str("rnbqkbnr/pp3ppp/4p3/2ppP3/3P4/8/PPP2PPP/RNBQKBNR w KQkq - 0 4").unwrap();
let pos: Chess = fen.into_position(shakmaty::CastlingMode::Standard).unwrap();

let (is_decomposable, num_components) = find_subsystems(pos.board());

assert_eq!(is_decomposable, false);
assert_eq!(num_components, 1);
```

## Research Context

Bitmesh was developed to generate training data for Game-Theoretic Representation Learning models, providing the structural decoupling required to sum sub-games in complex endgames.

## Composition Certificates

Bitmesh exposes a BMCOMPOSE v1 provenance certificate for composed exact labels.
A `CompositionCertificate` binds:

- the digest of a validated strict `DecompositionCertificate`
- one component value digest per decomposition component root
- the digest of the composed result value payload

Before a downstream generator promotes a composed row to exact supervision, it
should call `CompositionCertificate::validate_against_decomposition(&cert)`.
That check verifies structural composition metadata: strict decomposition status,
fresh decomposition digest, duplicate-root rejection, and exact component-root
coverage. It does not prove chess value correctness by itself; the generator
must still verify component values and recompute the composed result value.
