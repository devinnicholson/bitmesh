use crate::serialization::{
    MAX_CERTIFICATE_TEXT_BYTES, parse_sha256_hex, push_len_prefixed_bytes, sha256,
};
use shakmaty::{Bitboard, Role, Square};
use std::{fmt, str::FromStr};

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

/// Parse error for fixed-width SHA-256 certificate digests encoded as hex.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateDigestParseError {
    /// A SHA-256 digest must be exactly 64 hexadecimal bytes.
    InvalidLength {
        /// Number of bytes in the supplied string.
        actual: usize,
    },
    /// The supplied string contains a non-hex byte.
    InvalidHexByte {
        /// Byte offset of the invalid byte.
        index: usize,
        /// Invalid byte value.
        byte: u8,
    },
}

impl fmt::Display for CertificateDigestParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CertificateDigestParseError::InvalidLength { actual } => {
                write!(
                    f,
                    "SHA-256 certificate digest must be 64 hex bytes, got {actual}"
                )
            }
            CertificateDigestParseError::InvalidHexByte { index, byte } => {
                write!(f, "invalid hex byte 0x{byte:02x} at byte index {index}")
            }
        }
    }
}

impl std::error::Error for CertificateDigestParseError {}

/// Stable structural digest for a decomposition certificate.
///
/// This is the SHA-256 digest of the certificate's versioned canonical payload.
/// It is useful for label provenance and equality checks, but it is still only
/// a digest of the structural certificate fields validated by
/// [`DecompositionCertificate::validate`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DecompositionCertificateDigest(pub(crate) [u8; 32]);

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

    /// Parses a fixed-width SHA-256 digest from hexadecimal text.
    pub fn from_hex(hex: &str) -> Result<Self, CertificateDigestParseError> {
        parse_sha256_hex(hex).map(Self)
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

impl FromStr for DecompositionCertificateDigest {
    type Err = CertificateDigestParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
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
pub struct CompositionCertificateDigest(pub(crate) [u8; 32]);

impl CompositionCertificateDigest {
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

    /// Parses a fixed-width SHA-256 digest from hexadecimal text.
    pub fn from_hex(hex: &str) -> Result<Self, CertificateDigestParseError> {
        parse_sha256_hex(hex).map(Self)
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

impl FromStr for CompositionCertificateDigest {
    type Err = CertificateDigestParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
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

/// Stable digest for a decomposition certificate bound to concrete position text.
///
/// This hashes the validated structural certificate digest together with
/// caller-supplied canonical position text and context. The context should
/// describe the position text format or producer namespace.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PositionBoundDecompositionCertificateDigest(pub(crate) [u8; 32]);

impl PositionBoundDecompositionCertificateDigest {
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

impl fmt::Display for PositionBoundDecompositionCertificateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Hashes a validated decomposition certificate with canonical position context.
///
/// This is an additive helper for provenance labels that need to bind the
/// structural certificate to a concrete position string without changing the
/// existing structural certificate payload. `BMDPOSCERT` v1 accepts at most
/// 65,535 UTF-8 bytes in each caller-supplied text field.
pub fn position_bound_decomposition_certificate_digest(
    certificate: &DecompositionCertificate,
    canonical_position: &str,
    context: &str,
) -> Result<PositionBoundDecompositionCertificateDigest, DecompositionCertificateValidationError> {
    let certificate_digest = certificate.digest()?;
    if canonical_position.len() > MAX_CERTIFICATE_TEXT_BYTES {
        return Err(
            DecompositionCertificateValidationError::PositionContextTooLong {
                actual: canonical_position.len(),
                maximum: MAX_CERTIFICATE_TEXT_BYTES,
            },
        );
    }
    if context.len() > MAX_CERTIFICATE_TEXT_BYTES {
        return Err(
            DecompositionCertificateValidationError::PositionContextNamespaceTooLong {
                actual: context.len(),
                maximum: MAX_CERTIFICATE_TEXT_BYTES,
            },
        );
    }
    let mut payload = Vec::new();
    payload.extend_from_slice(b"BMDPOSCERT\0");
    payload.push(1);
    payload.extend_from_slice(certificate_digest.as_bytes());
    push_len_prefixed_bytes(&mut payload, canonical_position.as_bytes());
    push_len_prefixed_bytes(&mut payload, context.as_bytes());
    Ok(PositionBoundDecompositionCertificateDigest(sha256(
        &payload,
    )))
}

/// Additive proof that a strict structural decomposition also passes a
/// conservative one-ply legal-independence screen.
///
/// The proof is intentionally conservative: it accepts only supplied boards
/// whose barrier pawns are frozen by other barrier pawns and whose generated
/// geometric destinations stay inside their certified component and preserve
/// the barrier. The proof contract ends after this board-local screen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConservativeLegalIndependenceProof {
    /// Digest of the validated structural decomposition certificate.
    pub decomposition_digest: DecompositionCertificateDigest,
    /// Number of certified active components.
    pub component_count: u8,
    /// Frozen pawn barrier that separates the components.
    pub barrier: Bitboard,
    /// Stable proof contract identifier for downstream manifests.
    pub proof_kind: &'static str,
}

/// Conservative legal-independence proof failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConservativeLegalIndependenceError {
    /// The cited structural decomposition certificate is invalid.
    InvalidDecompositionCertificate {
        /// Structural validation error.
        error: DecompositionCertificateValidationError,
    },
    /// Conservative legal independence requires a strict structural certificate.
    RequiresStrictDecomposition {
        /// Status of the structural certificate.
        status: DecompositionStatus,
    },
    /// A barrier square has no piece in the supplied board.
    BarrierSquareIsEmpty {
        /// Empty barrier square.
        square: Square,
    },
    /// A barrier square is not occupied by a pawn.
    BarrierSquareIsNotPawn {
        /// Invalid barrier square.
        square: Square,
        /// Role occupying that square.
        role: Role,
    },
    /// A barrier pawn can become mobile because its forward blocker is not also
    /// part of the certified barrier.
    BarrierPawnNotFrozen {
        /// Mobile barrier pawn.
        square: Square,
        /// Forward square that is absent from the barrier. `None` is not used by
        /// the current checker, because off-board pawns are treated as frozen.
        forward_square: Option<Square>,
    },
    /// A barrier pawn has an immediate geometric capture, so the screen cannot
    /// certify the wall as frozen.
    BarrierPawnCanCapture {
        /// Mobile barrier pawn.
        square: Square,
        /// Capturable opposing piece.
        target: Square,
    },
    /// A non-barrier occupied square is absent from all certified components.
    ActivePieceOutsideCertifiedComponent {
        /// Occupied non-barrier square.
        square: Square,
    },
    /// The certificate's active masks differ from the occupied non-barrier
    /// squares on the supplied board.
    ActiveMaskDoesNotMatchBoard {
        /// Union of active masks stored in the certificate.
        certificate_active: Bitboard,
        /// Occupied non-barrier squares on the supplied board.
        board_active: Bitboard,
    },
    /// A generated geometric capture can remove a barrier piece.
    BarrierPieceCanBeCaptured {
        /// Attacking non-barrier piece.
        attacker_square: Square,
        /// Capturable barrier square.
        barrier_square: Square,
    },
    /// A generated geometric destination lies in a different component.
    PieceCanEnterOtherComponent {
        /// Moving piece origin.
        from: Square,
        /// Destination in another component.
        to: Square,
        /// Origin component index.
        from_component: usize,
        /// Destination component index.
        to_component: usize,
    },
    /// A generated geometric destination is a non-barrier square outside the
    /// structural certificate.
    PieceCanEnterUncertifiedFreeSquare {
        /// Moving piece origin.
        from: Square,
        /// Uncertified destination.
        to: Square,
        /// Origin component index.
        from_component: usize,
    },
}

/// Structural validation error for a [`CompositionCertificate`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionCertificateValidationError {
    /// A composition certificate needs at least one component value.
    EmptyComponentValues,
    /// The referenced decomposition certificate is structurally invalid.
    InvalidDecompositionCertificate {
        /// Validation error returned by the decomposition certificate.
        error: DecompositionCertificateValidationError,
    },
    /// The decomposition digest does not match the referenced certificate.
    DecompositionDigestMismatch {
        /// Digest of the referenced decomposition certificate.
        certificate_digest: DecompositionCertificateDigest,
        /// Digest carried by the composition certificate.
        composition_digest: DecompositionCertificateDigest,
    },
    /// Composition certificates only support strict decomposition certificates.
    CompositionRequiresStrictDecomposition {
        /// Status of the referenced decomposition certificate.
        status: DecompositionStatus,
    },
    /// Component exact value digest is empty.
    EmptyComponentValueDigest {
        /// Component root with the empty value digest.
        component_root: u8,
    },
    /// A component value digest exceeds the `BMCOMPOSE` v1 length field.
    ComponentValueDigestTooLong {
        /// Component root with the oversized value digest.
        component_root: u8,
        /// UTF-8 byte length supplied by the caller.
        actual: usize,
        /// Largest UTF-8 byte length accepted by v1.
        maximum: usize,
    },
    /// A strict decomposition component is missing from the composition.
    MissingComponentRoot {
        /// Missing root square index.
        component_root: u8,
    },
    /// A composition component root is not present in the decomposition.
    UnexpectedComponentRoot {
        /// Unexpected root square index.
        component_root: u8,
    },
    /// Two component values refer to the same component root.
    DuplicateComponentRoot {
        /// Duplicate root square index.
        component_root: u8,
    },
    /// The composed result exact value digest is empty.
    EmptyResultValueDigest,
    /// The result value digest exceeds the `BMCOMPOSE` v1 length field.
    ResultValueDigestTooLong {
        /// UTF-8 byte length supplied by the caller.
        actual: usize,
        /// Largest UTF-8 byte length accepted by v1.
        maximum: usize,
    },
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
    /// A strict component mask omits a non-barrier square adjacent to it.
    StrictComponentMaskNotClosed {
        /// Index of the component with the incomplete mask.
        component_index: usize,
        /// Square in the component mask.
        square: Square,
        /// Adjacent non-barrier square omitted from all component masks.
        omitted_square: Square,
    },
    /// Canonical position text exceeds the `BMDPOSCERT` v1 length field.
    PositionContextTooLong {
        /// UTF-8 byte length supplied by the caller.
        actual: usize,
        /// Largest UTF-8 byte length accepted by v1.
        maximum: usize,
    },
    /// Position-context namespace text exceeds the `BMDPOSCERT` v1 length
    /// field.
    PositionContextNamespaceTooLong {
        /// UTF-8 byte length supplied by the caller.
        actual: usize,
        /// Largest UTF-8 byte length accepted by v1.
        maximum: usize,
    },
}
