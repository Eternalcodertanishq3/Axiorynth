use crate::types::{Color, Piece, Square};

const PIECE_SEED: u64 = 0x9d39_249e_3377_6d41;
const SIDE_SEED: u64 = 0x2af7_3948_aa52_0f39;
const CASTLING_SEED: u64 = 0x44db_0150_2462_ba39;
const EN_PASSANT_SEED: u64 = 0x7583_44a1_b5f0_4d19;

pub fn piece_square(piece: Piece, square: Square) -> u64 {
    let color = piece.color.idx() as u64;
    let kind = piece.kind.idx() as u64;
    let index = color * 384 + kind * 64 + square.index() as u64;
    splitmix64(PIECE_SEED ^ index)
}

pub fn side_to_move(color: Color) -> u64 {
    match color {
        Color::White => 0,
        Color::Black => splitmix64(SIDE_SEED),
    }
}

pub fn castling(rights: u8) -> u64 {
    splitmix64(CASTLING_SEED ^ rights as u64)
}

pub fn en_passant(square: Option<Square>) -> u64 {
    square.map_or(0, |square| {
        splitmix64(EN_PASSANT_SEED ^ square.file() as u64)
    })
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut mixed = value;
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^ (mixed >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PieceKind, Square};

    #[test]
    fn zobrist_keys_are_deterministic() {
        let piece = Piece {
            color: Color::White,
            kind: PieceKind::Knight,
        };
        let square = Square::from_name("f3").unwrap();
        assert_eq!(piece_square(piece, square), piece_square(piece, square));
    }

    #[test]
    fn black_side_has_a_side_key() {
        assert_eq!(side_to_move(Color::White), 0);
        assert_ne!(side_to_move(Color::Black), 0);
    }
}
