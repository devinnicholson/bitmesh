//! `BitMesh`: Fast Graph Theory on Chess Bitboards
//!
//! This library provides algorithms like Union-Find to decompose
//! a chess position into independent combinatorial game components.

use shakmaty::{Bitboard, Board, Color, Square};
use std::{
    collections::{BTreeMap, HashSet},
    fmt,
};

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

/// Identifies pawns that are blocked and have no immediate legal capture target.
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

/// Outcome state for a decomposition certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecompositionStatus {
    /// Existing locked-pawn barriers split active material into multiple components.
    Strict,
    /// The position did not produce a strict decomposition certificate.
    Rejected,
}

/// Reason a position did not produce a strict decomposition certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecompositionRejectionReason {
    /// No locked-pawn barrier was found, so no partition proof exists.
    NoLockedBarrier,
    /// Locked barriers exist, but active material does not span at least two components.
    LessThanTwoActiveComponents,
}

/// A single active partition component in a decomposition certificate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecompositionComponent {
    /// Root square index from the underlying union-find partition.
    pub root: u8,
    /// All non-barrier squares in this component.
    pub mask: Bitboard,
    /// Occupied, non-barrier squares in this component.
    pub active_mask: Bitboard,
}

/// Certificate scaffold for locked-pawn barrier decompositions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecompositionCertificate {
    /// Locked-pawn squares used as barriers.
    pub barrier: Bitboard,
    /// Components containing at least one occupied non-barrier square.
    pub components: Vec<DecompositionComponent>,
    /// Number of active components represented in `components`.
    pub active_component_count: u8,
    /// `true` when `status` is [`DecompositionStatus::Strict`].
    pub strict: bool,
    /// Strict/rejected certificate status.
    pub status: DecompositionStatus,
    /// Rejection reason for non-strict certificates.
    pub rejection_reason: Option<DecompositionRejectionReason>,
}

/// Stable structural digest for a decomposition certificate.
///
/// This is the SHA-256 digest of the certificate's versioned canonical payload.
/// It is useful for label provenance and equality checks, but it is still only
/// a digest of the structural certificate fields validated by
/// [`DecompositionCertificate::validate`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DecompositionCertificateDigest([u8; 32]);

impl DecompositionCertificateDigest {
    /// Returns the raw digest bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the raw digest bytes by value.
    #[must_use]
    pub fn into_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Returns the digest encoded as lowercase hexadecimal.
    #[must_use]
    pub fn to_hex(self) -> String {
        let mut hex = String::with_capacity(64);
        for byte in self.0 {
            use fmt::Write as _;
            write!(&mut hex, "{byte:02x}").expect("writing to String cannot fail");
        }
        hex
    }
}

impl fmt::Display for DecompositionCertificateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Exact value assigned to one certified component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionComponentValue {
    /// Component root from the decomposition certificate.
    pub component_root: u8,
    /// Stable digest of the component exact value payload.
    pub value_digest: String,
}

/// Certificate that a result value is composed from independently certified components.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionCertificate {
    /// Digest of the validated decomposition certificate used for components.
    pub decomposition_digest: DecompositionCertificateDigest,
    /// Exact value digest for each component.
    pub component_values: Vec<CompositionComponentValue>,
    /// Stable digest of the composed exact result value payload.
    pub result_value_digest: String,
}

/// Stable structural digest for a composition certificate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompositionCertificateDigest([u8; 32]);

impl CompositionCertificateDigest {
    /// Returns the raw digest bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the digest encoded as lowercase hexadecimal.
    #[must_use]
    pub fn to_hex(self) -> String {
        let mut hex = String::with_capacity(64);
        for byte in self.0 {
            use fmt::Write as _;
            write!(&mut hex, "{byte:02x}").expect("writing to String cannot fail");
        }
        hex
    }
}

impl fmt::Display for CompositionCertificateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Structural validation error for a [`CompositionCertificate`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionCertificateValidationError {
    /// A composition certificate needs at least one component value.
    EmptyComponentValues,
    /// Component exact value digest is empty.
    EmptyComponentValueDigest {
        /// Component root with the empty value digest.
        component_root: u8,
    },
    /// Two component values refer to the same component root.
    DuplicateComponentRoot {
        /// Duplicate root square index.
        component_root: u8,
    },
    /// The composed result exact value digest is empty.
    EmptyResultValueDigest,
}

impl CompositionCertificate {
    /// Returns a versioned canonical byte payload for stable provenance labels.
    pub fn canonical_payload(&self) -> Result<Vec<u8>, CompositionCertificateValidationError> {
        self.validate()?;
        Ok(self.canonical_payload_unchecked())
    }

    /// Returns a stable SHA-256 digest of this certificate's canonical payload.
    pub fn digest(
        &self,
    ) -> Result<CompositionCertificateDigest, CompositionCertificateValidationError> {
        Ok(CompositionCertificateDigest(sha256(
            &self.canonical_payload()?,
        )))
    }

    /// Validates structural invariants for this composition certificate.
    pub fn validate(&self) -> Result<(), CompositionCertificateValidationError> {
        if self.component_values.is_empty() {
            return Err(CompositionCertificateValidationError::EmptyComponentValues);
        }
        if self.result_value_digest.is_empty() {
            return Err(CompositionCertificateValidationError::EmptyResultValueDigest);
        }

        let mut roots = HashSet::new();
        for component in &self.component_values {
            if component.value_digest.is_empty() {
                return Err(
                    CompositionCertificateValidationError::EmptyComponentValueDigest {
                        component_root: component.component_root,
                    },
                );
            }
            if !roots.insert(component.component_root) {
                return Err(
                    CompositionCertificateValidationError::DuplicateComponentRoot {
                        component_root: component.component_root,
                    },
                );
            }
        }
        Ok(())
    }

    fn canonical_payload_unchecked(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BMCOMPOSE\0");
        payload.push(1);
        payload.extend_from_slice(self.decomposition_digest.as_bytes());

        let mut component_values = self.component_values.iter().collect::<Vec<_>>();
        component_values
            .sort_by_key(|component| (component.component_root, component.value_digest.as_str()));
        let component_count =
            u16::try_from(component_values.len()).expect("too many component values");
        payload.extend_from_slice(&component_count.to_le_bytes());
        for component in component_values {
            payload.push(component.component_root);
            push_len_prefixed_bytes(&mut payload, component.value_digest.as_bytes());
        }
        push_len_prefixed_bytes(&mut payload, self.result_value_digest.as_bytes());
        payload
    }
}

/// Structural validation error for a [`DecompositionCertificate`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecompositionCertificateValidationError {
    /// The `strict` convenience flag disagrees with `status`.
    StrictStatusMismatch {
        /// Value stored in [`DecompositionCertificate::strict`].
        strict: bool,
        /// Value stored in [`DecompositionCertificate::status`].
        status: DecompositionStatus,
    },
    /// A component contains at least one barrier square.
    ComponentIntersectsBarrier {
        /// Index of the invalid component.
        component_index: usize,
    },
    /// A component's active mask contains a square outside the component mask.
    ActiveMaskOutsideComponent {
        /// Index of the invalid component.
        component_index: usize,
    },
    /// The declared active component count does not match `components.len()`.
    ActiveComponentCountMismatch {
        /// Value stored in [`DecompositionCertificate::active_component_count`].
        declared: u8,
        /// Number of components actually present.
        actual: usize,
    },
    /// A strict certificate does not contain at least two active components.
    StrictWithTooFewActiveComponents {
        /// Number of active components actually present.
        active_component_count: usize,
    },
    /// A strict certificate cannot be justified without at least one barrier square.
    StrictWithoutBarrier,
    /// A strict certificate carries a rejection reason.
    StrictWithRejectionReason {
        /// Rejection reason present on the strict certificate.
        rejection_reason: DecompositionRejectionReason,
    },
    /// A rejected certificate does not carry a rejection reason.
    RejectedWithoutRejectionReason,
    /// A `NoLockedBarrier` rejection cannot contain barrier squares.
    NoLockedBarrierRejectionWithBarrier,
    /// A `NoLockedBarrier` rejection cannot describe multiple active components.
    NoLockedBarrierRejectionWithMultipleActiveComponents {
        /// Number of active components actually present.
        active_component_count: usize,
    },
    /// A `LessThanTwoActiveComponents` rejection still needs a barrier.
    LessThanTwoActiveComponentsRejectionWithoutBarrier,
    /// A `LessThanTwoActiveComponents` rejection cannot have two or more active components.
    LessThanTwoActiveComponentsRejectionWithTooManyActiveComponents {
        /// Number of active components actually present.
        active_component_count: usize,
    },
    /// A component has no squares in its component mask.
    EmptyComponentMask {
        /// Index of the invalid component.
        component_index: usize,
    },
    /// A certified active component has no active squares.
    ComponentWithoutActiveSquares {
        /// Index of the invalid component.
        component_index: usize,
    },
    /// A component's root square is not inside its component mask.
    ComponentRootOutsideMask {
        /// Index of the invalid component.
        component_index: usize,
        /// Root square index stored on the invalid component.
        root: u8,
    },
    /// Two component masks overlap.
    ComponentMasksOverlap {
        /// Index of the first overlapping component.
        first_component_index: usize,
        /// Index of the second overlapping component.
        second_component_index: usize,
    },
    /// Two components use the same root square.
    DuplicateComponentRoot {
        /// Index of the first component using this root.
        first_component_index: usize,
        /// Index of the second component using this root.
        second_component_index: usize,
        /// Duplicate root square index.
        root: u8,
    },
    /// Two distinct certified components have adjacent non-barrier squares.
    CrossComponentAdjacency {
        /// Index of the first adjacent component.
        first_component_index: usize,
        /// Index of the second adjacent component.
        second_component_index: usize,
        /// Square in the first component.
        first_square: Square,
        /// Adjacent square in the second component.
        second_square: Square,
    },
}

impl DecompositionCertificate {
    /// Returns a versioned canonical byte payload for stable provenance labels.
    ///
    /// Components are serialized in sorted order, so equivalent certificates do
    /// not depend on the caller's component vector order. Validation is run
    /// before serialization. The resulting payload is a structural certificate,
    /// not a proof of full legal-chess dynamic independence.
    pub fn canonical_payload(&self) -> Result<Vec<u8>, DecompositionCertificateValidationError> {
        self.validate()?;
        Ok(self.canonical_payload_unchecked())
    }

    /// Returns a stable SHA-256 digest of this certificate's canonical payload.
    ///
    /// Validation is run before hashing. The digest is deterministic across
    /// process runs for the same validated structural certificate.
    pub fn digest(
        &self,
    ) -> Result<DecompositionCertificateDigest, DecompositionCertificateValidationError> {
        Ok(DecompositionCertificateDigest(sha256(
            &self.canonical_payload()?,
        )))
    }

    /// Validates bounded structural invariants for this certificate.
    ///
    /// This checks mask/status consistency and audits 8-way adjacency between
    /// certified component masks. It does not prove full legal-chess dynamic
    /// independence.
    pub fn validate(&self) -> Result<(), DecompositionCertificateValidationError> {
        let expected_strict = self.status == DecompositionStatus::Strict;
        if self.strict != expected_strict {
            return Err(
                DecompositionCertificateValidationError::StrictStatusMismatch {
                    strict: self.strict,
                    status: self.status,
                },
            );
        }

        if usize::from(self.active_component_count) != self.components.len() {
            return Err(
                DecompositionCertificateValidationError::ActiveComponentCountMismatch {
                    declared: self.active_component_count,
                    actual: self.components.len(),
                },
            );
        }

        match self.status {
            DecompositionStatus::Strict => {
                if self.components.len() < 2 {
                    return Err(
                        DecompositionCertificateValidationError::StrictWithTooFewActiveComponents {
                            active_component_count: self.components.len(),
                        },
                    );
                }
                if self.barrier.is_empty() {
                    return Err(DecompositionCertificateValidationError::StrictWithoutBarrier);
                }
                if let Some(rejection_reason) = self.rejection_reason {
                    return Err(
                        DecompositionCertificateValidationError::StrictWithRejectionReason {
                            rejection_reason,
                        },
                    );
                }
            }
            DecompositionStatus::Rejected => {
                use DecompositionCertificateValidationError as Error;

                match self.rejection_reason {
                    Some(DecompositionRejectionReason::NoLockedBarrier) => {
                        if !self.barrier.is_empty() {
                            return Err(Error::NoLockedBarrierRejectionWithBarrier);
                        }
                        if self.components.len() > 1 {
                            return Err(
                                Error::NoLockedBarrierRejectionWithMultipleActiveComponents {
                                    active_component_count: self.components.len(),
                                },
                            );
                        }
                    }
                    Some(DecompositionRejectionReason::LessThanTwoActiveComponents) => {
                        if self.barrier.is_empty() {
                            return Err(Error::LessThanTwoActiveComponentsRejectionWithoutBarrier);
                        }
                        if self.components.len() >= 2 {
                            return Err(
                                Error::LessThanTwoActiveComponentsRejectionWithTooManyActiveComponents {
                                    active_component_count: self.components.len(),
                                },
                            );
                        }
                    }
                    None => {
                        return Err(Error::RejectedWithoutRejectionReason);
                    }
                }
            }
        }

        let mut component_by_square = [None; 64];
        let mut component_by_root = [None; 64];
        for (component_index, component) in self.components.iter().enumerate() {
            if component.mask.is_empty() {
                return Err(
                    DecompositionCertificateValidationError::EmptyComponentMask { component_index },
                );
            }

            if component.active_mask.is_empty() {
                return Err(
                    DecompositionCertificateValidationError::ComponentWithoutActiveSquares {
                        component_index,
                    },
                );
            }

            if !component.mask.is_disjoint(self.barrier) {
                return Err(
                    DecompositionCertificateValidationError::ComponentIntersectsBarrier {
                        component_index,
                    },
                );
            }

            if !component.active_mask.is_subset(component.mask) {
                return Err(
                    DecompositionCertificateValidationError::ActiveMaskOutsideComponent {
                        component_index,
                    },
                );
            }

            let root_bit = 1u64.checked_shl(u32::from(component.root)).unwrap_or(0);
            if bitboard_bits(component.mask) & root_bit == 0 {
                return Err(
                    DecompositionCertificateValidationError::ComponentRootOutsideMask {
                        component_index,
                        root: component.root,
                    },
                );
            }

            let root_index = usize::from(component.root);
            if let Some(first_component_index) = component_by_root[root_index] {
                return Err(
                    DecompositionCertificateValidationError::DuplicateComponentRoot {
                        first_component_index,
                        second_component_index: component_index,
                        root: component.root,
                    },
                );
            }
            component_by_root[root_index] = Some(component_index);

            for sq in component.mask {
                let square_index = usize::from(sq);
                if let Some(first_component_index) = component_by_square[square_index] {
                    return Err(
                        DecompositionCertificateValidationError::ComponentMasksOverlap {
                            first_component_index,
                            second_component_index: component_index,
                        },
                    );
                }
                component_by_square[square_index] = Some(component_index);
            }
        }

        for (component_index, component) in self.components.iter().enumerate() {
            for sq in component.mask {
                for (file_delta, rank_delta) in EIGHT_WAY_DELTAS {
                    if let Some(adjacent) = adjacent_square(sq, file_delta, rank_delta) {
                        let adjacent_index = usize::from(adjacent);
                        if let Some(adjacent_component_index) = component_by_square[adjacent_index]
                            && adjacent_component_index != component_index
                        {
                            return Err(
                                DecompositionCertificateValidationError::CrossComponentAdjacency {
                                    first_component_index: component_index,
                                    second_component_index: adjacent_component_index,
                                    first_square: sq,
                                    second_square: adjacent,
                                },
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn canonical_payload_unchecked(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(22 + self.components.len() * 17);
        payload.extend_from_slice(b"BMDCERT\0");
        payload.push(1);
        payload.push(decomposition_status_tag(self.status));
        payload.push(u8::from(self.strict));
        payload.push(decomposition_rejection_reason_tag(self.rejection_reason));
        payload.push(self.active_component_count);
        payload.extend_from_slice(&bitboard_bits(self.barrier).to_le_bytes());
        payload.push(self.components.len() as u8);

        let mut components: Vec<_> = self.components.iter().collect();
        components.sort_by_key(|component| {
            (
                component.root,
                bitboard_bits(component.mask),
                bitboard_bits(component.active_mask),
            )
        });

        for component in components {
            payload.push(component.root);
            payload.extend_from_slice(&bitboard_bits(component.mask).to_le_bytes());
            payload.extend_from_slice(&bitboard_bits(component.active_mask).to_le_bytes());
        }

        payload
    }
}

fn bitboard_bits(mask: Bitboard) -> u64 {
    mask.into()
}

fn decomposition_status_tag(status: DecompositionStatus) -> u8 {
    match status {
        DecompositionStatus::Strict => 1,
        DecompositionStatus::Rejected => 2,
    }
}

fn decomposition_rejection_reason_tag(reason: Option<DecompositionRejectionReason>) -> u8 {
    match reason {
        None => 0,
        Some(DecompositionRejectionReason::NoLockedBarrier) => 1,
        Some(DecompositionRejectionReason::LessThanTwoActiveComponents) => 2,
    }
}

fn push_len_prefixed_bytes(payload: &mut Vec<u8>, bytes: &[u8]) {
    let length = u16::try_from(bytes.len()).expect("certificate field is too large");
    payload.extend_from_slice(&length.to_le_bytes());
    payload.extend_from_slice(bytes);
}

const SHA256_INITIAL_HASH: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

const SHA256_ROUND_CONSTANTS: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256(payload: &[u8]) -> [u8; 32] {
    let mut state = SHA256_INITIAL_HASH;
    let bit_len = (payload.len() as u64).wrapping_mul(8);
    let mut padded = payload.to_vec();

    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for block in padded.chunks_exact(64) {
        sha256_compress(&mut state, block);
    }

    let mut digest = [0; 32];
    for (chunk, word) in digest.chunks_exact_mut(4).zip(state) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    digest
}

fn sha256_compress(state: &mut [u32; 8], block: &[u8]) {
    let mut words = [0; 64];

    for (word, bytes) in words.iter_mut().take(16).zip(block.chunks_exact(4)) {
        *word = u32::from_be_bytes(
            bytes
                .try_into()
                .expect("SHA-256 block chunks are exactly four bytes"),
        );
    }

    for i in 16..64 {
        let s0 =
            words[i - 15].rotate_right(7) ^ words[i - 15].rotate_right(18) ^ (words[i - 15] >> 3);
        let s1 =
            words[i - 2].rotate_right(17) ^ words[i - 2].rotate_right(19) ^ (words[i - 2] >> 10);
        words[i] = words[i - 16]
            .wrapping_add(s0)
            .wrapping_add(words[i - 7])
            .wrapping_add(s1);
    }

    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;

    for (&word, &round_constant) in words.iter().zip(SHA256_ROUND_CONSTANTS.iter()) {
        let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let temp1 = h
            .wrapping_add(sum1)
            .wrapping_add(ch)
            .wrapping_add(round_constant)
            .wrapping_add(word);
        let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = sum0.wrapping_add(maj);

        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
    }

    for (current, compressed) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *current = current.wrapping_add(compressed);
    }
}

const EIGHT_WAY_DELTAS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

fn adjacent_square(sq: Square, file_delta: i32, rank_delta: i32) -> Option<Square> {
    let file = sq.file().offset(file_delta)?;
    let rank = sq.rank().offset(rank_delta)?;
    Some(Square::from_coords(file, rank))
}

/// Builds a decomposition certificate using locked-pawn barriers and the board partition.
#[must_use]
pub fn certify_decomposition(board: &Board) -> DecompositionCertificate {
    let barrier = get_locked_pawns(board);
    let mobile_pieces = board.occupied() & !barrier;
    let mut uf = partition_board(barrier);
    let mut components_by_root = BTreeMap::new();

    for sq in mobile_pieces {
        let root = uf.find(usize::from(sq));
        components_by_root
            .entry(root)
            .or_insert_with(|| DecompositionComponent {
                root: root as u8,
                mask: Bitboard::EMPTY,
                active_mask: Bitboard::EMPTY,
            })
            .active_mask
            .add(sq);
    }

    for sq in !barrier {
        let root = uf.find(usize::from(sq));
        if let Some(component) = components_by_root.get_mut(&root) {
            component.mask.add(sq);
        }
    }

    let components: Vec<_> = components_by_root.into_values().collect();
    let active_component_count = components.len() as u8;
    let rejection_reason = if barrier.is_empty() {
        Some(DecompositionRejectionReason::NoLockedBarrier)
    } else if active_component_count < 2 {
        Some(DecompositionRejectionReason::LessThanTwoActiveComponents)
    } else {
        None
    };
    let strict = rejection_reason.is_none();
    let status = if strict {
        DecompositionStatus::Strict
    } else {
        DecompositionStatus::Rejected
    };

    DecompositionCertificate {
        barrier,
        components,
        active_component_count,
        strict,
        status,
        rejection_reason,
    }
}

/// Finds active topological subsystems separated by locked-pawn barriers.
#[must_use]
pub fn find_subsystems(board: &Board) -> (bool, u8) {
    let barrier = get_locked_pawns(board);
    let mobile_pieces = board.occupied() & !barrier;
    let mut uf = partition_board(barrier);
    let mut active_components = HashSet::new();

    for sq in mobile_pieces {
        active_components.insert(uf.find(usize::from(sq)));
    }

    let num_components = active_components.len().min(u8::MAX as usize) as u8;
    (num_components > 1, num_components)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{Board, CastlingMode, Chess, Color, Position, Rank, Square, fen::Fen};
    use std::str::FromStr;

    fn locked_horizontal_chain_board() -> Board {
        let mut board = Board::empty();

        for sq in [
            Square::A4,
            Square::B4,
            Square::C4,
            Square::D4,
            Square::E4,
            Square::F4,
            Square::G4,
            Square::H4,
        ] {
            board.set_piece_at(sq, Color::White.pawn());
        }

        for sq in [
            Square::A5,
            Square::B5,
            Square::C5,
            Square::D5,
            Square::E5,
            Square::F5,
            Square::G5,
            Square::H5,
        ] {
            board.set_piece_at(sq, Color::White.pawn());
        }

        board.set_piece_at(Square::A1, Color::White.knight());
        board.set_piece_at(Square::H8, Color::Black.knight());
        board
    }

    #[test]
    fn test_sha256_known_answer() {
        assert_eq!(
            DecompositionCertificateDigest(sha256(b"abc")).to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

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

    #[test]
    fn test_barrier_squares_are_outside_partition_domain() {
        let barrier = Bitboard::from_square(Square::A4);
        let uf = partition_board(barrier);

        assert!(!uf.contains(usize::from(Square::A4)));
        assert!(uf.contains(usize::from(Square::A3)));
    }

    #[test]
    fn test_certificate_rejects_without_locked_barrier() {
        let certificate = certify_decomposition(&Board::new());

        assert_eq!(certificate.status, DecompositionStatus::Rejected);
        assert!(!certificate.strict);
        assert_eq!(
            certificate.rejection_reason,
            Some(DecompositionRejectionReason::NoLockedBarrier)
        );
        assert!(certificate.barrier.is_empty());
        assert_eq!(certificate.active_component_count, 1);
    }

    #[test]
    fn test_locked_chain_produces_strict_multi_component_certificate() {
        let board = locked_horizontal_chain_board();
        let certificate = certify_decomposition(&board);
        let certified_active = certificate
            .components
            .iter()
            .fold(Bitboard::EMPTY, |acc, component| {
                acc | component.active_mask
            });

        assert_eq!(certificate.status, DecompositionStatus::Strict);
        assert!(certificate.strict);
        assert_eq!(certificate.rejection_reason, None);
        assert_eq!(certificate.barrier, Bitboard::from_rank(Rank::Fourth));
        assert_eq!(certificate.active_component_count, 2);
        assert_eq!(certificate.components.len(), 2);
        assert_eq!(certified_active, board.occupied() & !certificate.barrier);
    }

    #[test]
    fn test_certificate_excludes_barrier_squares_from_components() {
        let certificate = certify_decomposition(&locked_horizontal_chain_board());

        for component in &certificate.components {
            assert!(component.mask.is_disjoint(certificate.barrier));
            assert!(component.active_mask.is_disjoint(certificate.barrier));
            assert!(component.active_mask.is_subset(component.mask));
        }
    }

    #[test]
    fn test_certificate_validation_accepts_certifier_outputs() {
        let strict_certificate = certify_decomposition(&locked_horizontal_chain_board());
        let rejected_certificate = certify_decomposition(&Board::new());

        assert_eq!(strict_certificate.validate(), Ok(()));
        assert_eq!(rejected_certificate.validate(), Ok(()));
    }

    #[test]
    fn test_certificate_canonical_payload_and_digest_are_stable() {
        let certificate = certify_decomposition(&locked_horizontal_chain_board());
        certificate.validate().unwrap();

        let payload = certificate.canonical_payload().unwrap();
        let digest = certificate.digest().unwrap();

        assert_eq!(payload, certificate.canonical_payload().unwrap());
        assert_eq!(digest, certificate.digest().unwrap());
        assert_eq!(digest.as_bytes().len(), 32);
        assert_eq!(digest.to_string(), digest.to_hex());
    }

    #[test]
    fn test_certificate_canonical_payload_ignores_component_order() {
        let certificate = certify_decomposition(&locked_horizontal_chain_board());
        let mut reordered = certificate.clone();
        reordered.components.reverse();

        certificate.validate().unwrap();
        reordered.validate().unwrap();

        assert_eq!(
            certificate.canonical_payload().unwrap(),
            reordered.canonical_payload().unwrap()
        );
        assert_eq!(certificate.digest().unwrap(), reordered.digest().unwrap());
    }

    #[test]
    fn test_structurally_different_certificates_hash_differently() {
        let certificate = certify_decomposition(&locked_horizontal_chain_board());
        let mut changed = certificate.clone();
        let component = changed
            .components
            .first_mut()
            .expect("strict test certificate has components");
        let mut added_square = None;

        for sq in component.mask {
            if !component.active_mask.contains(sq) {
                component.active_mask.add(sq);
                added_square = Some(sq);
                break;
            }
        }

        assert!(added_square.is_some());
        certificate.validate().unwrap();
        changed.validate().unwrap();

        assert_ne!(
            certificate.canonical_payload().unwrap(),
            changed.canonical_payload().unwrap()
        );
        assert_ne!(certificate.digest().unwrap(), changed.digest().unwrap());
    }

    #[test]
    fn test_certificate_digest_rejects_invalid_certificate() {
        let mut certificate = certify_decomposition(&locked_horizontal_chain_board());
        certificate.active_component_count = 1;

        assert_eq!(
            certificate.digest(),
            Err(
                DecompositionCertificateValidationError::ActiveComponentCountMismatch {
                    declared: 1,
                    actual: 2,
                },
            )
        );
    }

    fn sample_composition_certificate() -> CompositionCertificate {
        let decomposition = certify_decomposition(&locked_horizontal_chain_board());
        let decomposition_digest = decomposition.digest().unwrap();
        let mut roots = decomposition
            .components
            .iter()
            .map(|component| component.root)
            .collect::<Vec<_>>();
        roots.sort();

        CompositionCertificate {
            decomposition_digest,
            component_values: vec![
                CompositionComponentValue {
                    component_root: roots[0],
                    value_digest: "thermograph:left-component".to_owned(),
                },
                CompositionComponentValue {
                    component_root: roots[1],
                    value_digest: "thermograph:right-component".to_owned(),
                },
            ],
            result_value_digest: "thermograph:sum-result".to_owned(),
        }
    }

    #[test]
    fn test_composition_certificate_payload_and_digest_are_stable() {
        let certificate = sample_composition_certificate();
        let payload = certificate.canonical_payload().unwrap();
        let digest = certificate.digest().unwrap();

        assert_eq!(payload, certificate.canonical_payload().unwrap());
        assert_eq!(digest, certificate.digest().unwrap());
        assert_eq!(digest.as_bytes().len(), 32);
        assert_eq!(digest.to_string(), digest.to_hex());
    }

    #[test]
    fn test_composition_certificate_ignores_component_value_order() {
        let certificate = sample_composition_certificate();
        let mut reordered = certificate.clone();
        reordered.component_values.reverse();

        assert_eq!(
            certificate.canonical_payload().unwrap(),
            reordered.canonical_payload().unwrap()
        );
        assert_eq!(certificate.digest().unwrap(), reordered.digest().unwrap());
    }

    #[test]
    fn test_composition_certificate_rejects_duplicate_component_root() {
        let mut certificate = sample_composition_certificate();
        let duplicate_root = certificate.component_values[0].component_root;
        certificate.component_values[1].component_root = duplicate_root;

        assert_eq!(
            certificate.validate(),
            Err(
                CompositionCertificateValidationError::DuplicateComponentRoot {
                    component_root: duplicate_root,
                },
            )
        );
    }

    #[test]
    fn test_composition_certificate_rejects_missing_value_digest() {
        let mut certificate = sample_composition_certificate();
        let component_root = certificate.component_values[0].component_root;
        certificate.component_values[0].value_digest.clear();

        assert_eq!(
            certificate.digest(),
            Err(
                CompositionCertificateValidationError::EmptyComponentValueDigest { component_root },
            )
        );
    }

    #[test]
    fn test_certificate_validation_rejects_strict_status_mismatch() {
        let mut certificate = certify_decomposition(&locked_horizontal_chain_board());
        certificate.strict = false;

        assert_eq!(
            certificate.validate(),
            Err(
                DecompositionCertificateValidationError::StrictStatusMismatch {
                    strict: false,
                    status: DecompositionStatus::Strict,
                },
            )
        );
    }

    #[test]
    fn test_certificate_validation_rejects_strict_without_barrier() {
        let mut certificate = certify_decomposition(&locked_horizontal_chain_board());
        certificate.barrier = Bitboard::EMPTY;

        assert_eq!(
            certificate.validate(),
            Err(DecompositionCertificateValidationError::StrictWithoutBarrier)
        );
    }

    #[test]
    fn test_certificate_validation_rejects_component_barrier_overlap() {
        let mut certificate = certify_decomposition(&locked_horizontal_chain_board());
        certificate.components[0].mask.add(Square::A4);

        assert_eq!(
            certificate.validate(),
            Err(
                DecompositionCertificateValidationError::ComponentIntersectsBarrier {
                    component_index: 0,
                },
            )
        );
    }

    #[test]
    fn test_certificate_validation_rejects_active_mask_outside_component() {
        let mut certificate = certify_decomposition(&locked_horizontal_chain_board());
        let outside_square = certificate.components[1]
            .mask
            .into_iter()
            .next()
            .expect("test certificate has a second component");
        certificate.components[0].active_mask.add(outside_square);

        assert_eq!(
            certificate.validate(),
            Err(
                DecompositionCertificateValidationError::ActiveMaskOutsideComponent {
                    component_index: 0,
                },
            )
        );
    }

    #[test]
    fn test_certificate_validation_rejects_empty_active_component() {
        let mut certificate = certify_decomposition(&locked_horizontal_chain_board());
        certificate.components[0].active_mask = Bitboard::EMPTY;

        assert_eq!(
            certificate.validate(),
            Err(
                DecompositionCertificateValidationError::ComponentWithoutActiveSquares {
                    component_index: 0,
                },
            )
        );
    }

    #[test]
    fn test_certificate_validation_rejects_component_root_outside_mask() {
        let mut certificate = certify_decomposition(&locked_horizontal_chain_board());
        certificate.components[0].root = 64;

        assert_eq!(
            certificate.validate(),
            Err(
                DecompositionCertificateValidationError::ComponentRootOutsideMask {
                    component_index: 0,
                    root: 64,
                },
            )
        );
    }

    #[test]
    fn test_certificate_validation_rejects_active_component_count_mismatch() {
        let mut certificate = certify_decomposition(&locked_horizontal_chain_board());
        certificate.active_component_count = 1;

        assert_eq!(
            certificate.validate(),
            Err(
                DecompositionCertificateValidationError::ActiveComponentCountMismatch {
                    declared: 1,
                    actual: 2,
                },
            )
        );
    }

    #[test]
    fn test_certificate_validation_rejects_overlapping_component_masks() {
        let overlap = Bitboard::from_square(Square::A1);
        let second = overlap | Bitboard::from_square(Square::B1);
        let certificate = DecompositionCertificate {
            barrier: Bitboard::from_square(Square::H8),
            components: vec![
                DecompositionComponent {
                    root: usize::from(Square::A1) as u8,
                    mask: overlap,
                    active_mask: overlap,
                },
                DecompositionComponent {
                    root: usize::from(Square::B1) as u8,
                    mask: second,
                    active_mask: Bitboard::from_square(Square::B1),
                },
            ],
            active_component_count: 2,
            strict: true,
            status: DecompositionStatus::Strict,
            rejection_reason: None,
        };

        assert_eq!(
            certificate.validate(),
            Err(
                DecompositionCertificateValidationError::ComponentMasksOverlap {
                    first_component_index: 0,
                    second_component_index: 1,
                },
            )
        );
    }

    #[test]
    fn test_certificate_validation_rejects_cross_component_adjacency() {
        let a1 = Bitboard::from_square(Square::A1);
        let b1 = Bitboard::from_square(Square::B1);
        let certificate = DecompositionCertificate {
            barrier: Bitboard::from_square(Square::H8),
            components: vec![
                DecompositionComponent {
                    root: usize::from(Square::A1) as u8,
                    mask: a1,
                    active_mask: a1,
                },
                DecompositionComponent {
                    root: usize::from(Square::B1) as u8,
                    mask: b1,
                    active_mask: b1,
                },
            ],
            active_component_count: 2,
            strict: true,
            status: DecompositionStatus::Strict,
            rejection_reason: None,
        };

        assert_eq!(
            certificate.validate(),
            Err(
                DecompositionCertificateValidationError::CrossComponentAdjacency {
                    first_component_index: 0,
                    second_component_index: 1,
                    first_square: Square::A1,
                    second_square: Square::B1,
                },
            )
        );
    }

    #[test]
    fn test_readme_subsystems_example() {
        let fen =
            Fen::from_str("rnbqkbnr/pp3ppp/4p3/2ppP3/3P4/8/PPP2PPP/RNBQKBNR w KQkq - 0 4").unwrap();
        let pos: Chess = fen.into_position(CastlingMode::Standard).unwrap();

        let (is_decomposable, num_components) = find_subsystems(pos.board());

        assert!(!is_decomposable);
        assert_eq!(num_components, 1);
    }
}
