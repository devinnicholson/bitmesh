use crate::{
    graph::{get_locked_pawns, partition_board},
    types::*,
};
use shakmaty::{Bitboard, Board, Color, Piece, Role, Square};
use std::collections::{BTreeMap, HashSet};

/// Builds a structural decomposition certificate using locked-pawn barriers.
///
/// A strict result proves only that the detected barrier separates occupied
/// non-barrier squares into at least two 8-connected regions on this board.
/// Call [`verify_conservative_legal_independence`] for the stronger—but still
/// one-ply and board-local—screen.
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

/// Certifies a decomposition and applies the conservative one-ply screen.
///
/// The screen deliberately over-approximates some movement (for example pawn
/// double advances), which can reject uncertain positions. Acceptance covers
/// the supplied board and generated destinations.
pub fn certify_conservative_legal_independence(
    board: &Board,
) -> Result<ConservativeLegalIndependenceProof, ConservativeLegalIndependenceError> {
    let certificate = certify_decomposition(board);
    verify_conservative_legal_independence(board, &certificate)
}

/// Verifies that a strict structural decomposition passes a conservative
/// one-ply movement screen on the supplied board.
///
/// The checker examines both colors and geometric move/capture destinations;
/// its input omits side-to-move, check, castling, and en-passant metadata. It
/// can therefore reject a position that legal move generation would accept.
/// Success is a board-local certificate.
pub fn verify_conservative_legal_independence(
    board: &Board,
    certificate: &DecompositionCertificate,
) -> Result<ConservativeLegalIndependenceProof, ConservativeLegalIndependenceError> {
    certificate.validate().map_err(|error| {
        ConservativeLegalIndependenceError::InvalidDecompositionCertificate { error }
    })?;

    if certificate.status != DecompositionStatus::Strict {
        return Err(
            ConservativeLegalIndependenceError::RequiresStrictDecomposition {
                status: certificate.status,
            },
        );
    }

    let component_by_square = component_index_by_square(certificate);
    verify_frozen_barrier_pawns(board, certificate)?;
    verify_active_masks_match_board(board, certificate)?;
    verify_active_piece_destinations(board, certificate, &component_by_square)?;

    let decomposition_digest = certificate.digest().map_err(|error| {
        ConservativeLegalIndependenceError::InvalidDecompositionCertificate { error }
    })?;

    Ok(ConservativeLegalIndependenceProof {
        decomposition_digest,
        component_count: certificate.active_component_count,
        barrier: certificate.barrier,
        proof_kind: "bitmesh:conservative_legal_independence:v0",
    })
}

fn component_index_by_square(certificate: &DecompositionCertificate) -> [Option<usize>; 64] {
    let mut component_by_square = [None; 64];
    for (component_index, component) in certificate.components.iter().enumerate() {
        for sq in component.mask {
            component_by_square[usize::from(sq)] = Some(component_index);
        }
    }
    component_by_square
}

fn verify_active_masks_match_board(
    board: &Board,
    certificate: &DecompositionCertificate,
) -> Result<(), ConservativeLegalIndependenceError> {
    let certificate_active = certificate
        .components
        .iter()
        .fold(Bitboard::EMPTY, |active, component| {
            active | component.active_mask
        });
    let board_active = board.occupied() & !certificate.barrier;

    if certificate_active != board_active {
        return Err(
            ConservativeLegalIndependenceError::ActiveMaskDoesNotMatchBoard {
                certificate_active,
                board_active,
            },
        );
    }

    Ok(())
}

fn verify_frozen_barrier_pawns(
    board: &Board,
    certificate: &DecompositionCertificate,
) -> Result<(), ConservativeLegalIndependenceError> {
    for square in certificate.barrier {
        let piece = board
            .piece_at(square)
            .ok_or(ConservativeLegalIndependenceError::BarrierSquareIsEmpty { square })?;
        if piece.role != Role::Pawn {
            return Err(ConservativeLegalIndependenceError::BarrierSquareIsNotPawn {
                square,
                role: piece.role,
            });
        }

        let forward_offset = if piece.color == Color::White { 8 } else { -8 };
        if let Some(forward_square) = square.offset(forward_offset)
            && !certificate.barrier.contains(forward_square)
        {
            return Err(ConservativeLegalIndependenceError::BarrierPawnNotFrozen {
                square,
                forward_square: Some(forward_square),
            });
        }

        let captures =
            shakmaty::attacks::pawn_attacks(piece.color, square) & board.by_color(!piece.color);
        if let Some(target) = captures.into_iter().next() {
            return Err(ConservativeLegalIndependenceError::BarrierPawnCanCapture {
                square,
                target,
            });
        }
    }

    Ok(())
}

fn verify_active_piece_destinations(
    board: &Board,
    certificate: &DecompositionCertificate,
    component_by_square: &[Option<usize>; 64],
) -> Result<(), ConservativeLegalIndependenceError> {
    let active = board.occupied() & !certificate.barrier;
    let occupied = board.occupied();

    for from in active {
        let piece = board
            .piece_at(from)
            .expect("occupied non-barrier square must contain a piece");
        let from_component = component_by_square[usize::from(from)].ok_or(
            ConservativeLegalIndependenceError::ActivePieceOutsideCertifiedComponent {
                square: from,
            },
        )?;

        match piece.role {
            Role::Pawn => {
                let captures = shakmaty::attacks::pawn_attacks(piece.color, from)
                    & board.by_color(!piece.color);
                for to in captures {
                    check_conservative_destination(
                        certificate,
                        component_by_square,
                        from,
                        from_component,
                        to,
                    )?;
                }

                for to in conservative_pawn_quiet_destinations(board, from, piece.color) {
                    check_conservative_destination(
                        certificate,
                        component_by_square,
                        from,
                        from_component,
                        to,
                    )?;
                }
            }
            role => {
                let destinations = shakmaty::attacks::attacks(
                    from,
                    Piece {
                        role,
                        color: piece.color,
                    },
                    occupied,
                ) & !board.by_color(piece.color);
                for to in destinations {
                    check_conservative_destination(
                        certificate,
                        component_by_square,
                        from,
                        from_component,
                        to,
                    )?;
                }
            }
        }
    }

    Ok(())
}

fn conservative_pawn_quiet_destinations(board: &Board, from: Square, color: Color) -> Vec<Square> {
    let occupied = board.occupied();
    let forward_offset = if color == Color::White { 8 } else { -8 };
    let mut destinations = Vec::with_capacity(2);

    if let Some(one_step) = from.offset(forward_offset)
        && !occupied.contains(one_step)
    {
        destinations.push(one_step);
        if let Some(two_step) = one_step.offset(forward_offset)
            && !occupied.contains(two_step)
        {
            destinations.push(two_step);
        }
    }

    destinations
}

fn check_conservative_destination(
    certificate: &DecompositionCertificate,
    component_by_square: &[Option<usize>; 64],
    from: Square,
    from_component: usize,
    to: Square,
) -> Result<(), ConservativeLegalIndependenceError> {
    if certificate.barrier.contains(to) {
        return Err(
            ConservativeLegalIndependenceError::BarrierPieceCanBeCaptured {
                attacker_square: from,
                barrier_square: to,
            },
        );
    }

    let to_component = component_by_square[usize::from(to)].ok_or(
        ConservativeLegalIndependenceError::PieceCanEnterUncertifiedFreeSquare {
            from,
            to,
            from_component,
        },
    )?;

    if to_component != from_component {
        return Err(
            ConservativeLegalIndependenceError::PieceCanEnterOtherComponent {
                from,
                to,
                from_component,
                to_component,
            },
        );
    }

    Ok(())
}

/// Finds active structural regions separated by locked-pawn barriers.
///
/// The boolean reports whether more than one region contains non-barrier
/// material. The result describes board-graph connectivity.
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
