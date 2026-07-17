use bitmesh::{certify_decomposition, verify_conservative_legal_independence};
use shakmaty::{CastlingMode, Chess, Position, fen::Fen};
use std::{env, process, str::FromStr};

fn main() {
    let Some(fen_text) = env::args().nth(1) else {
        eprintln!("usage: cargo run --example certify_fen -- '<FEN>'");
        process::exit(2);
    };

    let fen = Fen::from_str(&fen_text).unwrap_or_else(|error| {
        eprintln!("invalid FEN: {error}");
        process::exit(2);
    });
    let position: Chess = fen
        .into_position(CastlingMode::Standard)
        .unwrap_or_else(|error| {
            eprintln!("invalid standard-chess position: {error}");
            process::exit(2);
        });

    let certificate = certify_decomposition(position.board());
    println!("status: {:?}", certificate.status);
    println!("components: {}", certificate.active_component_count);
    println!("barrier: {:?}", certificate.barrier);

    match certificate.digest() {
        Ok(digest) => println!("structural_digest: {digest}"),
        Err(error) => println!("structural_digest: unavailable ({error:?})"),
    }

    match verify_conservative_legal_independence(position.board(), &certificate) {
        Ok(proof) => {
            println!("one_ply_screen: accepted");
            println!("proof_kind: {}", proof.proof_kind);
        }
        Err(error) => {
            println!("one_ply_screen: rejected");
            println!("reason: {error:?}");
        }
    }
}
