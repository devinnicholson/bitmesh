use super::*;
use shakmaty::{
    Bitboard, Board, CastlingMode, Chess, Color, Position, Rank, Role, Square, fen::Fen,
};
use std::{collections::HashSet, str::FromStr};

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

fn d_file_barrier_squares() -> [Square; 8] {
    [
        Square::D1,
        Square::D2,
        Square::D3,
        Square::D4,
        Square::D5,
        Square::D6,
        Square::D7,
        Square::D8,
    ]
}

fn d_file_frozen_barrier() -> Bitboard {
    let mut barrier = Bitboard::EMPTY;
    for sq in d_file_barrier_squares() {
        barrier.add(sq);
    }
    barrier
}

fn frozen_vertical_wall_board() -> Board {
    let mut board = Board::empty();
    for (index, sq) in d_file_barrier_squares().into_iter().enumerate() {
        let color = if index % 2 == 0 {
            Color::White
        } else {
            Color::Black
        };
        board.set_piece_at(sq, color.pawn());
    }
    board
}

fn left_of_d_file_mask() -> Bitboard {
    let barrier = d_file_frozen_barrier();
    let mut mask = Bitboard::EMPTY;
    for sq in !barrier {
        if usize::from(sq) % 8 < 3 {
            mask.add(sq);
        }
    }
    mask
}

fn right_of_d_file_mask() -> Bitboard {
    let barrier = d_file_frozen_barrier();
    let mut mask = Bitboard::EMPTY;
    for sq in !barrier {
        if usize::from(sq) % 8 > 3 {
            mask.add(sq);
        }
    }
    mask
}

fn frozen_vertical_wall_certificate(
    left_active: Bitboard,
    right_active: Bitboard,
) -> DecompositionCertificate {
    DecompositionCertificate {
        barrier: d_file_frozen_barrier(),
        components: vec![
            DecompositionComponent {
                root: usize::from(Square::A1) as u8,
                mask: left_of_d_file_mask(),
                active_mask: left_active,
            },
            DecompositionComponent {
                root: usize::from(Square::E1) as u8,
                mask: right_of_d_file_mask(),
                active_mask: right_active,
            },
        ],
        active_component_count: 2,
        strict: true,
        status: DecompositionStatus::Strict,
        rejection_reason: None,
    }
}

fn readme_fen_position() -> Chess {
    Fen::from_str("7k/8/8/p1p1p1p1/PpPpPpPp/1P1P1P1P/8/K7 w - - 0 1")
        .unwrap()
        .into_position(CastlingMode::Standard)
        .unwrap()
}

#[test]
fn test_sha256_known_answer() {
    assert_eq!(
        DecompositionCertificateDigest(sha256(b"abc")).to_hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn test_certificate_digest_hex_parsing_roundtrips() {
    let decomposition_digest = DecompositionCertificateDigest(sha256(b"decomposition"));
    let decomposition_hex = decomposition_digest.to_hex();
    assert_eq!(
        DecompositionCertificateDigest::from_hex(&decomposition_hex),
        Ok(decomposition_digest)
    );
    assert_eq!(
        decomposition_hex.parse::<DecompositionCertificateDigest>(),
        Ok(decomposition_digest)
    );
    assert_eq!(
        DecompositionCertificateDigest::from_hex(&decomposition_hex.to_uppercase()),
        Ok(decomposition_digest)
    );

    let composition_digest = CompositionCertificateDigest(sha256(b"composition"));
    let composition_hex = composition_digest.to_hex();
    assert_eq!(
        CompositionCertificateDigest::from_hex(&composition_hex),
        Ok(composition_digest)
    );
    assert_eq!(
        composition_hex.parse::<CompositionCertificateDigest>(),
        Ok(composition_digest)
    );
    assert_eq!(
        CompositionCertificateDigest::from_hex(&composition_hex.to_uppercase()),
        Ok(composition_digest)
    );
}

#[test]
fn test_certificate_digest_hex_parsing_rejects_invalid_input() {
    assert_eq!(
        DecompositionCertificateDigest::from_hex("abc"),
        Err(CertificateDigestParseError::InvalidLength { actual: 3 })
    );

    let invalid_hex = format!("{}z", "0".repeat(63));
    assert_eq!(
        CompositionCertificateDigest::from_hex(&invalid_hex),
        Err(CertificateDigestParseError::InvalidHexByte {
            index: 63,
            byte: b'z',
        })
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
    assert_eq!(&payload[..9], b"BMDCERT\0\x01");
    assert_eq!(
        digest.to_hex(),
        "cf721c7f1fb6fdf02de27735da2a9af56a55518f5c062d9653f41be0f447576e"
    );
}

#[test]
fn test_position_bound_decomposition_digest_includes_position_context() {
    let certificate = certify_decomposition(&locked_horizontal_chain_board());
    let position = "8/8/8/PPPPPPPP/PPPPPPPP/8/8/N6n w - - 0 1";
    let context = "bitmesh:test-fen";

    let digest =
        position_bound_decomposition_certificate_digest(&certificate, position, context).unwrap();
    assert_eq!(
        digest,
        position_bound_decomposition_certificate_digest(&certificate, position, context).unwrap()
    );
    assert_ne!(
        digest,
        position_bound_decomposition_certificate_digest(
            &certificate,
            "8/8/8/PPPPPPPP/PPPPPPPP/8/8/NN5n w - - 0 1",
            context,
        )
        .unwrap()
    );
    assert_ne!(
        digest,
        position_bound_decomposition_certificate_digest(
            &certificate,
            position,
            "bitmesh:alternate-context",
        )
        .unwrap()
    );
    assert_eq!(digest.as_bytes().len(), 32);
    assert_eq!(digest.to_string(), digest.to_hex());
    assert_eq!(
        digest.to_hex(),
        "a1d0a0c5e9a26ed5d5f96b9801b127d1a479b19dac11dfdcb2b520d59992c874"
    );
}

#[test]
fn test_position_bound_digest_rejects_oversized_text_fields() {
    let certificate = certify_decomposition(&locked_horizontal_chain_board());
    let oversized = "x".repeat(MAX_CERTIFICATE_TEXT_BYTES + 1);

    assert_eq!(
        position_bound_decomposition_certificate_digest(
            &certificate,
            &oversized,
            "bitmesh:test-fen",
        ),
        Err(
            DecompositionCertificateValidationError::PositionContextTooLong {
                actual: MAX_CERTIFICATE_TEXT_BYTES + 1,
                maximum: MAX_CERTIFICATE_TEXT_BYTES,
            }
        )
    );
    assert_eq!(
        position_bound_decomposition_certificate_digest(
            &certificate,
            "8/8/8/PPPPPPPP/PPPPPPPP/8/8/N6n w - - 0 1",
            &oversized,
        ),
        Err(
            DecompositionCertificateValidationError::PositionContextNamespaceTooLong {
                actual: MAX_CERTIFICATE_TEXT_BYTES + 1,
                maximum: MAX_CERTIFICATE_TEXT_BYTES,
            }
        )
    );
}

#[test]
fn test_conservative_legal_independence_accepts_frozen_vertical_wall() {
    let mut board = frozen_vertical_wall_board();
    board.set_piece_at(Square::A1, Color::White.knight());
    board.set_piece_at(Square::H8, Color::Black.knight());
    let certificate = frozen_vertical_wall_certificate(
        Bitboard::from_square(Square::A1),
        Bitboard::from_square(Square::H8),
    );

    let proof = verify_conservative_legal_independence(&board, &certificate).unwrap();

    assert_eq!(proof.component_count, 2);
    assert_eq!(proof.barrier, d_file_frozen_barrier());
    assert_eq!(
        proof.proof_kind,
        "bitmesh:conservative_legal_independence:v0"
    );
    assert_eq!(
        proof.decomposition_digest,
        certificate.digest().expect("test certificate is valid"),
    );
}

#[test]
fn test_conservative_legal_independence_rejects_stale_active_masks() {
    let mut board = frozen_vertical_wall_board();
    board.set_piece_at(Square::A1, Color::White.knight());
    board.set_piece_at(Square::H8, Color::Black.knight());
    let mut certificate = frozen_vertical_wall_certificate(
        Bitboard::from_square(Square::A1),
        Bitboard::from_square(Square::H8),
    );
    certificate.components[0].active_mask = Bitboard::from_square(Square::B1);

    assert_eq!(
        verify_conservative_legal_independence(&board, &certificate),
        Err(
            ConservativeLegalIndependenceError::ActiveMaskDoesNotMatchBoard {
                certificate_active: Bitboard::from_square(Square::B1)
                    | Bitboard::from_square(Square::H8),
                board_active: Bitboard::from_square(Square::A1) | Bitboard::from_square(Square::H8),
            }
        )
    );
}

#[test]
fn test_readme_fen_certification_is_deterministic() {
    let position = readme_fen_position();
    let certificate = certify_decomposition(position.board());

    assert_eq!(certificate.status, DecompositionStatus::Strict);
    assert_eq!(certificate.active_component_count, 2);
    assert_eq!(certificate.barrier.count(), 16);
    assert_eq!(
        certificate.digest().unwrap().to_hex(),
        "7e300b3e0b7901942161a58eb284b87134be7f26484561f2bbc142d1010465bf"
    );

    let first = verify_conservative_legal_independence(position.board(), &certificate).unwrap();
    let second = verify_conservative_legal_independence(position.board(), &certificate).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.component_count, 2);
    assert_eq!(
        first.proof_kind,
        "bitmesh:conservative_legal_independence:v0"
    );
}

#[test]
fn test_conservative_legal_independence_rejects_missing_barrier_square() {
    let mut board = frozen_vertical_wall_board();
    board.set_piece_at(Square::A1, Color::White.knight());
    board.set_piece_at(Square::H8, Color::Black.knight());
    let certificate = frozen_vertical_wall_certificate(
        Bitboard::from_square(Square::A1),
        Bitboard::from_square(Square::H8),
    );
    board.discard_piece_at(Square::D4);

    assert_eq!(
        verify_conservative_legal_independence(&board, &certificate),
        Err(ConservativeLegalIndependenceError::BarrierSquareIsEmpty { square: Square::D4 })
    );
}

#[test]
fn test_conservative_legal_independence_rejects_non_pawn_barrier_square() {
    let mut board = frozen_vertical_wall_board();
    board.set_piece_at(Square::A1, Color::White.knight());
    board.set_piece_at(Square::H8, Color::Black.knight());
    let certificate = frozen_vertical_wall_certificate(
        Bitboard::from_square(Square::A1),
        Bitboard::from_square(Square::H8),
    );
    board.set_piece_at(Square::D4, Color::Black.knight());

    assert_eq!(
        verify_conservative_legal_independence(&board, &certificate),
        Err(ConservativeLegalIndependenceError::BarrierSquareIsNotPawn {
            square: Square::D4,
            role: Role::Knight,
        })
    );
}

#[test]
fn test_conservative_legal_independence_rejects_mobile_blocker_wall() {
    let board = locked_horizontal_chain_board();
    let certificate = certify_decomposition(&board);
    assert_eq!(certificate.status, DecompositionStatus::Strict);

    assert_eq!(
        verify_conservative_legal_independence(&board, &certificate),
        Err(ConservativeLegalIndependenceError::BarrierPawnNotFrozen {
            square: Square::A4,
            forward_square: Some(Square::A5),
        },)
    );
}

#[test]
fn test_conservative_legal_independence_rejects_barrier_pawn_capture() {
    let mut board = frozen_vertical_wall_board();
    board.set_piece_at(Square::C1, Color::White.knight());
    board.set_piece_at(Square::H8, Color::Black.knight());
    let certificate = frozen_vertical_wall_certificate(
        Bitboard::from_square(Square::C1),
        Bitboard::from_square(Square::H8),
    );

    assert_eq!(
        verify_conservative_legal_independence(&board, &certificate),
        Err(ConservativeLegalIndependenceError::BarrierPawnCanCapture {
            square: Square::D2,
            target: Square::C1,
        },)
    );
}

#[test]
fn test_conservative_legal_independence_rejects_knight_crossing_wall() {
    let mut board = frozen_vertical_wall_board();
    board.set_piece_at(Square::C2, Color::White.knight());
    board.set_piece_at(Square::H8, Color::Black.knight());
    let certificate = frozen_vertical_wall_certificate(
        Bitboard::from_square(Square::C2),
        Bitboard::from_square(Square::H8),
    );

    assert_eq!(
        verify_conservative_legal_independence(&board, &certificate),
        Err(
            ConservativeLegalIndependenceError::PieceCanEnterOtherComponent {
                from: Square::C2,
                to: Square::E1,
                from_component: 0,
                to_component: 1,
            },
        )
    );
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

fn sample_decomposition_certificate() -> DecompositionCertificate {
    certify_decomposition(&locked_horizontal_chain_board())
}

fn sample_composition_certificate_for(
    decomposition: &DecompositionCertificate,
) -> CompositionCertificate {
    let decomposition_digest = decomposition.digest().unwrap();
    let mut roots = decomposition
        .components
        .iter()
        .map(|component| component.root)
        .collect::<Vec<_>>();
    roots.sort();

    CompositionCertificate {
        decomposition_digest,
        component_values: roots
            .iter()
            .map(|root| CompositionComponentValue {
                component_root: *root,
                value_digest: format!("thermograph:component-{root}"),
            })
            .collect(),
        result_value_digest: "thermograph:sum-result".to_owned(),
    }
}

fn sample_composition_certificate() -> CompositionCertificate {
    let decomposition = certify_decomposition(&locked_horizontal_chain_board());
    sample_composition_certificate_for(&decomposition)
}

fn component_root_not_in(decomposition: &DecompositionCertificate) -> u8 {
    let roots = decomposition
        .components
        .iter()
        .map(|component| component.root)
        .collect::<HashSet<_>>();
    (0..64)
        .find(|root| !roots.contains(root))
        .expect("test decomposition leaves at least one root unused")
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
    assert_eq!(&payload[..11], b"BMCOMPOSE\0\x01");
    assert_eq!(
        digest.to_hex(),
        "5fefeec7de17e312bae7661f6745b74fc1001d49ad447c0d6a1985ff70f7b525"
    );
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
        Err(CompositionCertificateValidationError::EmptyComponentValueDigest { component_root },)
    );
}

#[test]
fn test_composition_certificate_rejects_oversized_value_digests() {
    let oversized = "x".repeat(MAX_CERTIFICATE_TEXT_BYTES + 1);
    let mut component_certificate = sample_composition_certificate();
    let component_root = component_certificate.component_values[0].component_root;
    component_certificate.component_values[0].value_digest = oversized.clone();

    assert_eq!(
        component_certificate.canonical_payload(),
        Err(
            CompositionCertificateValidationError::ComponentValueDigestTooLong {
                component_root,
                actual: MAX_CERTIFICATE_TEXT_BYTES + 1,
                maximum: MAX_CERTIFICATE_TEXT_BYTES,
            }
        )
    );

    let mut result_certificate = sample_composition_certificate();
    result_certificate.result_value_digest = oversized;
    assert_eq!(
        result_certificate.canonical_payload(),
        Err(
            CompositionCertificateValidationError::ResultValueDigestTooLong {
                actual: MAX_CERTIFICATE_TEXT_BYTES + 1,
                maximum: MAX_CERTIFICATE_TEXT_BYTES,
            }
        )
    );
}

#[test]
fn test_composition_certificate_validates_against_decomposition() {
    let decomposition = sample_decomposition_certificate();
    let certificate = sample_composition_certificate_for(&decomposition);

    assert_eq!(
        certificate.validate_against_decomposition(&decomposition),
        Ok(())
    );
}

#[test]
fn test_composition_certificate_rejects_stale_decomposition_digest() {
    let decomposition = sample_decomposition_certificate();
    let mut certificate = sample_composition_certificate_for(&decomposition);
    let stale_digest = DecompositionCertificateDigest(sha256(b"stale-decomposition"));
    certificate.decomposition_digest = stale_digest;

    assert_eq!(
        certificate.validate_against_decomposition(&decomposition),
        Err(
            CompositionCertificateValidationError::DecompositionDigestMismatch {
                certificate_digest: decomposition.digest().unwrap(),
                composition_digest: stale_digest,
            },
        )
    );
}

#[test]
fn test_composition_certificate_rejects_missing_decomposition_root() {
    let decomposition = sample_decomposition_certificate();
    let mut certificate = sample_composition_certificate_for(&decomposition);
    let missing_root = certificate
        .component_values
        .pop()
        .expect("sample composition has component values")
        .component_root;

    assert_eq!(
        certificate.validate_against_decomposition(&decomposition),
        Err(
            CompositionCertificateValidationError::MissingComponentRoot {
                component_root: missing_root,
            }
        )
    );
}

#[test]
fn test_composition_certificate_rejects_unexpected_component_root() {
    let decomposition = sample_decomposition_certificate();
    let mut certificate = sample_composition_certificate_for(&decomposition);
    let unexpected_root = component_root_not_in(&decomposition);
    certificate
        .component_values
        .push(CompositionComponentValue {
            component_root: unexpected_root,
            value_digest: "thermograph:unexpected-component".to_owned(),
        });

    assert_eq!(
        certificate.validate_against_decomposition(&decomposition),
        Err(
            CompositionCertificateValidationError::UnexpectedComponentRoot {
                component_root: unexpected_root,
            }
        )
    );
}

#[test]
fn test_composition_certificate_requires_strict_decomposition() {
    let rejected_decomposition = certify_decomposition(&Board::empty());
    let mut certificate = sample_composition_certificate();
    certificate.decomposition_digest = rejected_decomposition.digest().unwrap();

    assert_eq!(
        certificate.validate_against_decomposition(&rejected_decomposition),
        Err(
            CompositionCertificateValidationError::CompositionRequiresStrictDecomposition {
                status: DecompositionStatus::Rejected,
            },
        )
    );
}

#[test]
fn test_composition_certificate_rejects_invalid_decomposition() {
    let mut decomposition = sample_decomposition_certificate();
    let certificate = sample_composition_certificate_for(&decomposition);
    decomposition.active_component_count = 1;

    assert_eq!(
        certificate.validate_against_decomposition(&decomposition),
        Err(
            CompositionCertificateValidationError::InvalidDecompositionCertificate {
                error: DecompositionCertificateValidationError::ActiveComponentCountMismatch {
                    declared: 1,
                    actual: 2,
                },
            },
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
fn test_certificate_validation_rejects_omitted_free_bridge_square() {
    let a1 = Bitboard::from_square(Square::A1);
    let c1 = Bitboard::from_square(Square::C1);
    let certificate = DecompositionCertificate {
        barrier: Bitboard::from_square(Square::H8),
        components: vec![
            DecompositionComponent {
                root: usize::from(Square::A1) as u8,
                mask: a1,
                active_mask: a1,
            },
            DecompositionComponent {
                root: usize::from(Square::C1) as u8,
                mask: c1,
                active_mask: c1,
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
            DecompositionCertificateValidationError::StrictComponentMaskNotClosed {
                component_index: 0,
                square: Square::A1,
                omitted_square: Square::B1,
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

#[test]
fn digest_parsers_reject_non_hex_at_every_byte_position() {
    for index in 0..64 {
        let mut malformed = vec![b'0'; 64];
        malformed[index] = b'g';
        let malformed = String::from_utf8(malformed).expect("test bytes are UTF-8");

        assert_eq!(
            DecompositionCertificateDigest::from_hex(&malformed),
            Err(CertificateDigestParseError::InvalidHexByte { index, byte: b'g' })
        );
        assert_eq!(
            CompositionCertificateDigest::from_hex(&malformed),
            Err(CertificateDigestParseError::InvalidHexByte { index, byte: b'g' })
        );
    }
}

#[test]
fn v1_text_fields_accept_the_exact_u16_boundary() {
    let maximum = "x".repeat(MAX_CERTIFICATE_TEXT_BYTES);
    let decomposition = sample_decomposition_certificate();

    let position_digest =
        position_bound_decomposition_certificate_digest(&decomposition, &maximum, &maximum)
            .expect("BMDPOSCERT v1 accepts an exact u16-length field");
    assert_eq!(position_digest.as_bytes().len(), 32);

    let mut composition = sample_composition_certificate_for(&decomposition);
    composition.component_values[0].value_digest = maximum.clone();
    composition.result_value_digest = maximum;
    let payload = composition
        .canonical_payload()
        .expect("BMCOMPOSE v1 accepts exact u16-length fields");
    assert!(payload.len() > MAX_CERTIFICATE_TEXT_BYTES);
}

#[test]
fn structural_mutations_return_errors_without_panicking() {
    let baseline = sample_decomposition_certificate();
    let mut mutations = Vec::new();

    let mut mutation = baseline.clone();
    mutation.active_component_count = 0;
    mutations.push(mutation);

    let mut mutation = baseline.clone();
    mutation.strict = false;
    mutations.push(mutation);

    let mut mutation = baseline.clone();
    mutation.rejection_reason = Some(DecompositionRejectionReason::NoLockedBarrier);
    mutations.push(mutation);

    let mut mutation = baseline.clone();
    mutation.components[0].root = u8::MAX;
    mutations.push(mutation);

    let mut mutation = baseline.clone();
    mutation.components[0].active_mask = Bitboard::EMPTY;
    mutations.push(mutation);

    let mut mutation = baseline.clone();
    mutation.components[0].mask |= mutation.barrier;
    mutations.push(mutation);

    for mutation in mutations {
        let result = std::panic::catch_unwind(|| mutation.validate());
        assert!(result.is_ok(), "untrusted certificate validation panicked");
        assert!(
            result.expect("panic result checked").is_err(),
            "structural mutation unexpectedly validated"
        );
    }
}

#[test]
fn union_find_out_of_domain_panics_are_deterministic() {
    let mut full = UnionFind::new();
    assert!(std::panic::catch_unwind(move || full.find(64)).is_err());

    let mut masked = UnionFind::with_mask(Bitboard::from_square(Square::A1));
    assert!(
        std::panic::catch_unwind(move || masked.connected(usize::from(Square::A1), 64)).is_err()
    );

    let mut inactive = UnionFind::with_mask(Bitboard::from_square(Square::A1));
    assert!(std::panic::catch_unwind(move || inactive.find(usize::from(Square::B1))).is_err());
}

#[test]
fn position_bindings_change_for_each_context_input() {
    let certificate = sample_decomposition_certificate();
    let baseline =
        position_bound_decomposition_certificate_digest(&certificate, "position-a", "fen:v1")
            .unwrap();
    let changed_position =
        position_bound_decomposition_certificate_digest(&certificate, "position-b", "fen:v1")
            .unwrap();
    let changed_namespace =
        position_bound_decomposition_certificate_digest(&certificate, "position-a", "epd:v1")
            .unwrap();

    assert_ne!(baseline, changed_position);
    assert_ne!(baseline, changed_namespace);
    assert_ne!(changed_position, changed_namespace);
}
