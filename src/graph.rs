use shakmaty::{Bitboard, Board, Color, Square};

/// Standard Union-Find (Disjoint Set) over the 64 squares of a chessboard.
#[derive(Clone, Debug)]
pub struct UnionFind {
    parent: [u8; 64],
    rank: [u8; 64],
    active: [bool; 64],
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
            active: [true; 64],
        }
    }

    /// Creates a new Union-Find structure that only initializes squares present in the given mask.
    #[must_use]
    pub fn with_mask(mask: Bitboard) -> Self {
        let mut parent = [0; 64];
        let mut active = [false; 64];
        for (i, p) in parent.iter_mut().enumerate() {
            *p = i as u8;
        }
        for sq in mask {
            let i = usize::from(sq);
            parent[i] = i as u8;
            active[i] = true;
        }
        UnionFind {
            parent,
            rank: [0; 64],
            active,
        }
    }

    /// Returns `true` when a square belongs to this union-find domain.
    #[must_use]
    pub fn contains(&self, i: usize) -> bool {
        i < 64 && self.active[i]
    }

    /// Finds the representative of the set containing square `i`, using path compression.
    pub fn find(&mut self, i: usize) -> usize {
        assert!(
            self.contains(i),
            "square index {i} is outside this union-find domain"
        );

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
        assert!(
            self.contains(i),
            "square index {i} is outside this union-find domain"
        );
        assert!(
            self.contains(j),
            "square index {j} is outside this union-find domain"
        );

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
/// Computes 8-way adjacency between non-barrier squares and returns a
/// [`UnionFind`] structure representing the connected components. Barrier
/// selection and semantic validity are the caller's responsibility; this
/// function performs only graph partitioning.
#[must_use]
pub fn partition_board(barrier: Bitboard) -> UnionFind {
    let free = !barrier;
    let mut uf = UnionFind::with_mask(free);
    let f: u64 = free.into();

    let not_h: u64 = !0x8080808080808080;
    let not_a: u64 = !0x0101010101010101;

    // Compute adjacency masks where a bit at index `i` indicates that square `i`
    // and its neighbor in the given direction are both free.
    let east = f & (f >> 1) & not_h;
    let north = f & (f >> 8);
    let ne = f & (f >> 9) & not_h;
    let nw = f & (f >> 7) & not_a;

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

/// Identifies pawns that are blocked and have no immediate capture target.
///
/// This board-only predicate considers both colors. Its inputs omit side to
/// move, check, pins, en passant, castling rights, and move counters.
#[must_use]
pub fn get_locked_pawns(board: &Board) -> Bitboard {
    let occupied = board.occupied();
    let mut locked = Bitboard::EMPTY;

    for sq in board.pawns() {
        let color = board
            .color_at(sq)
            .expect("squares from board.pawns() must contain a pawn");
        let forward_offset = if color == Color::White { 8 } else { -8 };
        let is_blocked = sq
            .offset(forward_offset)
            .is_none_or(|forward_sq| occupied.contains(forward_sq));

        let attacks = shakmaty::attacks::pawn_attacks(color, sq);
        let has_captures = (attacks & board.by_color(!color)).any();

        if is_blocked && !has_captures {
            locked ^= Bitboard::from_square(sq);
        }
    }

    locked
}

pub(crate) const EIGHT_WAY_DELTAS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

pub(crate) fn adjacent_square(sq: Square, file_delta: i32, rank_delta: i32) -> Option<Square> {
    let file = sq.file().offset(file_delta)?;
    let rank = sq.rank().offset(rank_delta)?;
    Some(Square::from_coords(file, rank))
}
