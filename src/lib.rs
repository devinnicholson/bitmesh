//! `BitMesh`: Fast Graph Theory on Chess Bitboards
//! 
//! This library provides algorithms like Union-Find to decompose
//! a chess position into independent combinatorial game components.

use shakmaty::Bitboard;

/// Standard Union-Find (Disjoint Set) over the 64 squares of a chessboard.
#[derive(Clone, Debug)]
pub struct UnionFind {
    parent: [u8; 64],
    rank: [u8; 64],
}

impl Default for UnionFind {
    fn default() -> Self {
        Self::new()
    }
}

impl UnionFind {
    /// Creates a new Union-Find structure where each square is its own connected component.
    #[must_use] 
    pub fn new() -> Self {
        let mut parent = [0; 64];
        for (i, p) in parent.iter_mut().enumerate() {
            *p = i as u8;
        }
        UnionFind {
            parent,
            rank: [0; 64],
        }
    }

    /// Creates a new Union-Find structure that only initializes squares present in the given mask.
    #[must_use] 
    pub fn with_mask(mask: Bitboard) -> Self {
        let mut parent = [0; 64];
        for sq in mask {
            let i = usize::from(sq);
            parent[i] = i as u8;
        }
        UnionFind {
            parent,
            rank: [0; 64],
        }
    }

    /// Finds the representative of the set containing square `i`, using path compression.
    pub fn find(&mut self, i: usize) -> usize {
        let mut root = i;
        while self.parent[root] as usize != root {
            root = self.parent[root] as usize;
        }

        // Path compression
        let mut curr = i;
        while self.parent[curr] as usize != root {
            let next = self.parent[curr] as usize;
            self.parent[curr] = root as u8;
            curr = next;
        }

        root
    }

    /// Unions the sets containing squares `i` and `j`, using union by rank.
    /// Returns `true` if they were in different sets and are now merged.
    pub fn union(&mut self, i: usize, j: usize) -> bool {
        let root_i = self.find(i);
        let root_j = self.find(j);

        if root_i == root_j {
            return false;
        }

        match self.rank[root_i].cmp(&self.rank[root_j]) {
            std::cmp::Ordering::Less => {
                self.parent[root_i] = root_j as u8;
            }
            std::cmp::Ordering::Greater => {
                self.parent[root_j] = root_i as u8;
            }
            std::cmp::Ordering::Equal => {
                self.parent[root_i] = root_j as u8;
                self.rank[root_j] += 1;
            }
        }
        true
    }

    /// Returns `true` if squares `i` and `j` are in the same component.
    pub fn connected(&mut self, i: usize, j: usize) -> bool {
        self.find(i) == self.find(j)
    }
}

/// Detects topological components of a chessboard given a `barrier` of occupied squares.
///
/// Uses bulletproof bitwise logic to compute 8-way adjacency between non-barrier squares,
/// returning a `UnionFind` structure representing the connected components.
#[must_use] 
pub fn partition_board(barrier: Bitboard) -> UnionFind {
    let free = !barrier;
    let mut uf = UnionFind::with_mask(free);
    let f: u64 = free.into();

    let not_h: u64 = !0x8080808080808080;
    let not_a: u64 = !0x0101010101010101;

    // Compute adjacency masks where a bit at index `i` indicates that square `i`
    // and its neighbor in the given direction are both free.
    let east  = f & (f >> 1) & not_h;
    let north = f & (f >> 8);
    let ne    = f & (f >> 9) & not_h;
    let nw    = f & (f >> 7) & not_a;

    // Apply unions for each connected pair
    for sq in Bitboard::from(east) {
        uf.union(usize::from(sq), usize::from(sq) + 1);
    }
    for sq in Bitboard::from(north) {
        uf.union(usize::from(sq), usize::from(sq) + 8);
    }
    for sq in Bitboard::from(ne) {
        uf.union(usize::from(sq), usize::from(sq) + 9);
    }
    for sq in Bitboard::from(nw) {
        uf.union(usize::from(sq), usize::from(sq) + 7);
    }

    uf
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::Square;

    #[test]
    fn test_empty_board() {
        let mut uf = partition_board(Bitboard::from(0));
        // All squares should be connected.
        for i in 0..64 {
            for j in 0..64 {
                assert!(uf.connected(i, j));
            }
        }
    }

    #[test]
    fn test_locked_pawn_chain_partitions_board() {
        // Create a solid diagonal locked pawn chain from A1 to H8.
        // E.g., White pawns on a1, b2, c3, d4, e5, f6, g7, h8 (wait, pawns can't be on 1 or 8, but for a topological barrier, any pieces work).
        // Let's use squares A2-B3-C4-D5-E6-F7 and A3-B4-C5-D6-E7-F8.
        // Wait, a pawn chain blocking the board could be continuous.
        // A simple horizontal barrier blocks the board 8-way if it's 2-thick, or 1-thick?
        // Let's just create a barrier of A4, B4, C4, D4, E4, F4, G4, H4.
        // Is 1-thick horizontal line blocking 8-way?
        // If row 4 is a barrier:
        // A3 and A5: A3 connects to A4, B4 (both barrier). So no connection.
        // Yes, a 1-thick horizontal or vertical line blocks 8-way!
        // But a 1-thick diagonal line (A1, B2, C3...) does NOT block 8-way, because A2 connects to B1 across the diagonal!
        // So a diagonal barrier needs to be 2-thick to block 8-way.
        // A locked pawn chain is exactly 2-thick! White pawns on b2, c3, d4, e5... Black pawns on b3, c4, d5, e6...
        
        let mut barrier = Bitboard::from(0);
        // White pawns
        barrier.add(Square::B2);
        barrier.add(Square::C3);
        barrier.add(Square::D4);
        barrier.add(Square::E5);
        barrier.add(Square::F6);
        barrier.add(Square::G7);
        // Black pawns
        barrier.add(Square::B3);
        barrier.add(Square::C4);
        barrier.add(Square::D5);
        barrier.add(Square::E6);
        barrier.add(Square::F7);
        barrier.add(Square::G8);
        
        // Let's also block the edges so the chain reaches the walls.
        // To complete the wall from file A to H:
        // A2, A3
        barrier.add(Square::A2);
        barrier.add(Square::A3);
        // H7, H8
        barrier.add(Square::H7);
        barrier.add(Square::H8);

        let mut uf = partition_board(barrier);

        // A1 is connected to H1 via the bottom row, which is entirely empty
        assert!(uf.connected(usize::from(Square::A1), usize::from(Square::H1)));
        
        // A1 (bottom-left) should be separated from A8 (top-left)
        assert!(!uf.connected(usize::from(Square::A1), usize::from(Square::A8)));
        
        // A8 and H1 should be on different sides?
        // Wait, the barrier is from A2/A3 to H7/H8.
        // It separates the bottom-right (H1) from the top-left (A8).
        assert!(!uf.connected(usize::from(Square::A8), usize::from(Square::H1)));

        // A8 and A7 should be connected
        assert!(uf.connected(usize::from(Square::A8), usize::from(Square::A7)));

        // H1 and G1 should be connected
        assert!(uf.connected(usize::from(Square::H1), usize::from(Square::G1)));
    }
}
