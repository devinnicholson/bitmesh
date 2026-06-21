//! BitMesh: Fast Graph Theory on Chess Bitboards
//! 
//! This library provides algorithms like Union-Find to decompose
//! a chess position into independent combinatorial game components.

use shakmaty::{Bitboard, Board, Color, Square};
use std::collections::HashSet;

/// Standard Union-Find (Disjoint Set) over the 64 squares of a chessboard.
pub struct UnionFind {
    parent: [usize; 64],
}

impl UnionFind {
    pub fn new() -> Self {
        let mut parent = [0; 64];
        for i in 0..64 {
            parent[i] = i;
        }
        UnionFind { parent }
    }

    pub fn find(&mut self, i: usize) -> usize {
        if self.parent[i] == i {
            i
        } else {
            let root = self.find(self.parent[i]);
            self.parent[i] = root;
            root
        }
    }

    pub fn union(&mut self, i: usize, j: usize) {
        let root_i = self.find(i);
        let root_j = self.find(j);
        if root_i != root_j {
            self.parent[root_i] = root_j;
        }
    }
}
