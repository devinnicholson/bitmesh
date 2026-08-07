//! Conservative structural decomposition certificates for chess bitboards.
//!
//! Bitmesh identifies connected regions separated by selected locked-pawn
//! barriers and can apply a conservative, one-ply movement screen to those
//! regions. An accepted screen is evidence about the supplied board only. Its
//! scope ends before future descendants, combinatorial-game sums, and component
//! values.
//!
//! See the repository README for the complete guarantee, boundaries, failure
//! modes, serialization policy, and an executable FEN example.

mod certificates;
mod graph;
mod serialization;
mod types;
mod validation;

pub use certificates::{
    certify_conservative_legal_independence, certify_decomposition, find_subsystems,
    verify_conservative_legal_independence,
};
pub use graph::{UnionFind, get_locked_pawns, partition_board};
pub use types::{
    CertificateDigestParseError, CompositionCertificate, CompositionCertificateDigest,
    CompositionCertificateValidationError, CompositionComponentValue,
    ConservativeLegalIndependenceError, ConservativeLegalIndependenceProof,
    DecompositionCertificate, DecompositionCertificateDigest,
    DecompositionCertificateValidationError, DecompositionComponent, DecompositionRejectionReason,
    DecompositionStatus, PositionBoundDecompositionCertificateDigest,
    position_bound_decomposition_certificate_digest,
};

#[cfg(test)]
use serialization::{MAX_CERTIFICATE_TEXT_BYTES, sha256};

#[cfg(test)]
mod tests;
