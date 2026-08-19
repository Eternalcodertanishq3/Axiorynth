use std::fmt;

use crate::mv::{Move, MoveKind};
use crate::types::{COLOR_COUNT, Color, PIECE_KIND_COUNT, Piece, PieceKind, Square};
use crate::zobrist;

pub const STARTPOS_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub const CASTLE_WHITE_KING: u8 = 0b0001;
pub const CASTLE_WHITE_QUEEN: u8 = 0b0010;
pub const CASTLE_BLACK_KING: u8 = 0b0100;
pub const CASTLE_BLACK_QUEEN: u8 = 0b1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pieces: [[u64; PIECE_KIND_COUNT]; COLOR_COUNT],
    side_to_move: Color,
    castling_rights: u8,
    en_passant: Option<Square>,
    halfmove_clock: u16,
    fullmove_number: u16,
    hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoState {
    mv: Move,
    moving_piece: Piece,
    captured: Option<(Square, Piece)>,
    rook_move: Option<(Square, Square, Piece)>,
    side_to_move: Color,
    castling_rights: u8,
    en_passant: Option<Square>,
    halfmove_clock: u16,
    fullmove_number: u16,
    hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardError {
    FenFieldCount(usize),
    InvalidPiecePlacement(String),
    InvalidSideToMove(String),
    InvalidCastlingRights(String),
    InvalidEnPassant(String),
    InvalidHalfmoveClock(String),
    InvalidFullmoveNumber(String),
    InvalidKingCount { white: u32, black: u32 },
}

impl fmt::Display for BoardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoardError::FenFieldCount(count) => {
                write!(f, "FEN must have 6 fields, found {count}")
            }
            BoardError::InvalidPiecePlacement(value) => {
                write!(f, "invalid FEN piece placement: {value}")
            }
            BoardError::InvalidSideToMove(value) => {
                write!(f, "invalid FEN side to move: {value}")
            }
            BoardError::InvalidCastlingRights(value) => {
                write!(f, "invalid FEN castling rights: {value}")
            }
            BoardError::InvalidEnPassant(value) => {
                write!(f, "invalid FEN en passant square: {value}")
            }
            BoardError::InvalidHalfmoveClock(value) => {
                write!(f, "invalid FEN halfmove clock: {value}")
            }
            BoardError::InvalidFullmoveNumber(value) => {
                write!(f, "invalid FEN fullmove number: {value}")
            }
            BoardError::InvalidKingCount { white, black } => {
                write!(
                    f,
                    "FEN must contain one king per side, found white={white}, black={black}"
                )
            }
        }
    }
}

impl std::error::Error for BoardError {}

impl Board {
    pub fn empty() -> Board {
        Board {
            pieces: [[0; PIECE_KIND_COUNT]; COLOR_COUNT],
            side_to_move: Color::White,
            castling_rights: 0,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            hash: 0,
        }
    }

    pub fn startpos() -> Result<Board, BoardError> {
        Board::from_fen(STARTPOS_FEN)
    }

    pub fn from_fen(fen: &str) -> Result<Board, BoardError> {
        let fields: Vec<&str> = fen.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(BoardError::FenFieldCount(fields.len()));
        }

        let mut board = Board::empty();
        board.parse_piece_placement(fields[0])?;

        board.side_to_move = match fields[1] {
            "w" => Color::White,
            "b" => Color::Black,
            other => return Err(BoardError::InvalidSideToMove(other.to_string())),
        };

        board.castling_rights = parse_castling_rights(fields[2])?;
        board.en_passant = if fields[3] == "-" {
            None
        } else {
            Some(
                Square::from_name(fields[3])
                    .ok_or_else(|| BoardError::InvalidEnPassant(fields[3].to_string()))?,
            )
        };
        board.halfmove_clock = fields[4]
            .parse()
            .map_err(|_| BoardError::InvalidHalfmoveClock(fields[4].to_string()))?;
        board.fullmove_number = fields[5]
            .parse()
            .map_err(|_| BoardError::InvalidFullmoveNumber(fields[5].to_string()))?;
        board.recompute_hash();

        let white_kings = board.piece_count(Color::White, PieceKind::King);
        let black_kings = board.piece_count(Color::Black, PieceKind::King);
        if white_kings != 1 || black_kings != 1 {
            return Err(BoardError::InvalidKingCount {
                white: white_kings,
                black: black_kings,
            });
        }

        Ok(board)
    }

    fn parse_piece_placement(&mut self, placement: &str) -> Result<(), BoardError> {
        let ranks: Vec<&str> = placement.split('/').collect();
        if ranks.len() != 8 {
            return Err(BoardError::InvalidPiecePlacement(placement.to_string()));
        }

        for (rank_from_top, rank_text) in ranks.iter().enumerate() {
            let rank = 7u8 - rank_from_top as u8;
            let mut file = 0u8;

            for ch in rank_text.chars() {
                if ch.is_ascii_digit() {
                    let empty_count = ch
                        .to_digit(10)
                        .ok_or_else(|| BoardError::InvalidPiecePlacement(placement.to_string()))?
                        as u8;
                    if empty_count == 0 || empty_count > 8 {
                        return Err(BoardError::InvalidPiecePlacement(placement.to_string()));
                    }
                    file += empty_count;
                    if file > 8 {
                        return Err(BoardError::InvalidPiecePlacement(placement.to_string()));
                    }
                    continue;
                }

                let piece = Piece::from_fen(ch)
                    .ok_or_else(|| BoardError::InvalidPiecePlacement(placement.to_string()))?;
                let square = Square::from_coords(file, rank)
                    .ok_or_else(|| BoardError::InvalidPiecePlacement(placement.to_string()))?;
                self.set_piece(square, piece);
                file += 1;
            }

            if file != 8 {
                return Err(BoardError::InvalidPiecePlacement(placement.to_string()));
            }
        }

        Ok(())
    }

    pub fn to_fen(&self) -> String {
        let mut placement = String::new();
        for rank in (0..8u8).rev() {
            if rank != 7 {
                placement.push('/');
            }

            let mut empty = 0u8;
            for file in 0..8u8 {
                let square = Square::from_coords(file, rank).expect("valid board square");
                if let Some(piece) = self.piece_at(square) {
                    if empty > 0 {
                        placement.push(char::from(b'0' + empty));
                        empty = 0;
                    }
                    placement.push(piece.to_fen());
                } else {
                    empty += 1;
                }
            }

            if empty > 0 {
                placement.push(char::from(b'0' + empty));
            }
        }

        let side = self.side_to_move.fen();
        let castling = castling_rights_to_fen(self.castling_rights);
        let en_passant = self
            .en_passant
            .map(|square| square.to_string())
            .unwrap_or_else(|| "-".to_string());

        format!(
            "{placement} {side} {castling} {en_passant} {} {}",
            self.halfmove_clock, self.fullmove_number
        )
    }

    #[inline]
    pub fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    #[inline]
    pub fn set_side_to_move(&mut self, color: Color) {
        if self.side_to_move != color {
            self.hash ^= zobrist::side_to_move(self.side_to_move);
            self.hash ^= zobrist::side_to_move(color);
        }
        self.side_to_move = color;
    }

    #[inline]
    pub fn castling_rights(&self) -> u8 {
        self.castling_rights
    }

    #[inline]
    pub fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    #[inline]
    pub fn halfmove_clock(&self) -> u16 {
        self.halfmove_clock
    }

    #[inline]
    pub fn fullmove_number(&self) -> u16 {
        self.fullmove_number
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    #[inline]
    pub fn pieces(&self, color: Color, kind: PieceKind) -> u64 {
        self.pieces[color.idx()][kind.idx()]
    }

    pub fn piece_count(&self, color: Color, kind: PieceKind) -> u32 {
        self.pieces(color, kind).count_ones()
    }

    pub fn color_occupancy(&self, color: Color) -> u64 {
        PieceKind::ALL
            .iter()
            .map(|kind| self.pieces(color, *kind))
            .fold(0, |acc, bits| acc | bits)
    }

    pub fn occupancy(&self) -> u64 {
        self.color_occupancy(Color::White) | self.color_occupancy(Color::Black)
    }

    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        let bit = square.bit();
        for color in Color::ALL {
            for kind in PieceKind::ALL {
                if self.pieces(color, kind) & bit != 0 {
                    return Some(Piece { color, kind });
                }
            }
        }
        None
    }

    pub fn set_piece(&mut self, square: Square, piece: Piece) {
        self.pieces[piece.color.idx()][piece.kind.idx()] |= square.bit();
        self.hash ^= zobrist::piece_square(piece, square);
    }

    pub fn remove_piece_at(&mut self, square: Square) -> Option<Piece> {
        let bit = square.bit();
        for color in Color::ALL {
            for kind in PieceKind::ALL {
                let slot = &mut self.pieces[color.idx()][kind.idx()];
                if *slot & bit != 0 {
                    *slot &= !bit;
                    let piece = Piece { color, kind };
                    self.hash ^= zobrist::piece_square(piece, square);
                    return Some(piece);
                }
            }
        }
        None
    }

    pub fn recompute_hash(&mut self) {
        self.hash = self.compute_hash();
    }

    pub fn compute_hash(&self) -> u64 {
        let mut hash = 0;
        for color in Color::ALL {
            for kind in PieceKind::ALL {
                for square in bit_squares(self.pieces(color, kind)) {
                    hash ^= zobrist::piece_square(Piece { color, kind }, square);
                }
            }
        }
        hash ^= zobrist::side_to_move(self.side_to_move);
        hash ^= zobrist::castling(self.castling_rights);
        hash ^= zobrist::en_passant(self.en_passant);
        hash
    }

    pub fn king_square(&self, color: Color) -> Option<Square> {
        let kings = self.pieces(color, PieceKind::King);
        if kings == 0 {
            None
        } else {
            Some(Square::from_index(kings.trailing_zeros() as usize))
        }
    }

    pub fn in_check(&self, color: Color) -> bool {
        let Some(king_square) = self.king_square(color) else {
            return false;
        };
        self.is_square_attacked(king_square, color.opposite())
    }

    pub fn is_square_attacked(&self, square: Square, by_color: Color) -> bool {
        if self.is_attacked_by_pawn(square, by_color) {
            return true;
        }

        for from in bit_squares(self.pieces(by_color, PieceKind::Knight)) {
            if knight_targets(from) & square.bit() != 0 {
                return true;
            }
        }

        for from in bit_squares(self.pieces(by_color, PieceKind::King)) {
            if king_targets(from) & square.bit() != 0 {
                return true;
            }
        }

        let bishop_like = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
        if self.is_attacked_on_rays(
            square,
            by_color,
            &bishop_like,
            &[PieceKind::Bishop, PieceKind::Queen],
        ) {
            return true;
        }

        let rook_like = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        self.is_attacked_on_rays(
            square,
            by_color,
            &rook_like,
            &[PieceKind::Rook, PieceKind::Queen],
        )
    }

    fn is_attacked_by_pawn(&self, square: Square, by_color: Color) -> bool {
        let pawn_sources = match by_color {
            Color::White => [(-1, -1), (1, -1)],
            Color::Black => [(-1, 1), (1, 1)],
        };

        pawn_sources.iter().any(|(df, dr)| {
            step_square(square, *df, *dr).is_some_and(|source| {
                self.piece_at(source)
                    == Some(Piece {
                        color: by_color,
                        kind: PieceKind::Pawn,
                    })
            })
        })
    }

    fn is_attacked_on_rays(
        &self,
        square: Square,
        by_color: Color,
        directions: &[(i8, i8)],
        attackers: &[PieceKind],
    ) -> bool {
        for (df, dr) in directions {
            let mut current = square;
            while let Some(next) = step_square(current, *df, *dr) {
                current = next;
                if let Some(piece) = self.piece_at(current) {
                    if piece.color == by_color && attackers.contains(&piece.kind) {
                        return true;
                    }
                    break;
                }
            }
        }

        false
    }

    pub fn make_move(&mut self, mv: Move) -> UndoState {
        let old_side_to_move = self.side_to_move;
        let old_castling_rights = self.castling_rights;
        let old_en_passant = self.en_passant;
        let old_halfmove_clock = self.halfmove_clock;
        let old_fullmove_number = self.fullmove_number;
        let old_hash = self.hash;

        self.hash ^= zobrist::side_to_move(self.side_to_move);
        self.hash ^= zobrist::castling(self.castling_rights);
        self.hash ^= zobrist::en_passant(self.en_passant);

        let moving_piece = self
            .remove_piece_at(mv.from)
            .expect("move source must contain a piece");

        let captured = if mv.kind == MoveKind::EnPassant {
            let capture_rank = match moving_piece.color {
                Color::White => mv.to.rank() - 1,
                Color::Black => mv.to.rank() + 1,
            };
            let capture_square =
                Square::from_coords(mv.to.file(), capture_rank).expect("valid en passant square");
            self.remove_piece_at(capture_square)
                .map(|piece| (capture_square, piece))
        } else {
            self.remove_piece_at(mv.to).map(|piece| (mv.to, piece))
        };
        let captured_piece = captured.map(|(_, piece)| piece);

        let placed_kind = mv.promotion.unwrap_or(moving_piece.kind);
        self.set_piece(
            mv.to,
            Piece {
                color: moving_piece.color,
                kind: placed_kind,
            },
        );

        let rook_move = match mv.kind {
            MoveKind::KingCastle => Some(self.move_castling_rook(moving_piece.color, true)),
            MoveKind::QueenCastle => Some(self.move_castling_rook(moving_piece.color, false)),
            _ => None,
        };

        self.update_castling_rights(moving_piece, mv, captured_piece);

        self.en_passant = None;
        if moving_piece.kind == PieceKind::Pawn && mv.kind == MoveKind::DoublePawnPush {
            let ep_rank = (mv.from.rank() + mv.to.rank()) / 2;
            self.en_passant = Square::from_coords(mv.from.file(), ep_rank);
        }

        if moving_piece.kind == PieceKind::Pawn || captured.is_some() {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        if moving_piece.color == Color::Black {
            self.fullmove_number += 1;
        }

        self.side_to_move = self.side_to_move.opposite();
        self.hash ^= zobrist::side_to_move(self.side_to_move);
        self.hash ^= zobrist::castling(self.castling_rights);
        self.hash ^= zobrist::en_passant(self.en_passant);

        UndoState {
            mv,
            moving_piece,
            captured,
            rook_move,
            side_to_move: old_side_to_move,
            castling_rights: old_castling_rights,
            en_passant: old_en_passant,
            halfmove_clock: old_halfmove_clock,
            fullmove_number: old_fullmove_number,
            hash: old_hash,
        }
    }

    pub fn make_null_move(&mut self) -> UndoState {
        let old_side_to_move = self.side_to_move;
        let old_castling_rights = self.castling_rights;
        let old_en_passant = self.en_passant;
        let old_halfmove_clock = self.halfmove_clock;
        let old_fullmove_number = self.fullmove_number;
        let old_hash = self.hash;

        self.hash ^= zobrist::side_to_move(self.side_to_move);
        self.hash ^= zobrist::en_passant(self.en_passant);

        self.en_passant = None;
        self.halfmove_clock += 1;

        if self.side_to_move == Color::Black {
            self.fullmove_number += 1;
        }

        self.side_to_move = self.side_to_move.opposite();
        self.hash ^= zobrist::side_to_move(self.side_to_move);
        self.hash ^= zobrist::en_passant(self.en_passant);

        UndoState {
            mv: Move::new(Square::from_index(0), Square::from_index(0), None, MoveKind::Quiet),
            moving_piece: Piece { color: old_side_to_move, kind: PieceKind::King },
            captured: None,
            rook_move: None,
            side_to_move: old_side_to_move,
            castling_rights: old_castling_rights,
            en_passant: old_en_passant,
            halfmove_clock: old_halfmove_clock,
            fullmove_number: old_fullmove_number,
            hash: old_hash,
        }
    }

    pub fn undo_null_move(&mut self, undo: UndoState) {
        self.side_to_move = undo.side_to_move;
        self.castling_rights = undo.castling_rights;
        self.en_passant = undo.en_passant;
        self.halfmove_clock = undo.halfmove_clock;
        self.fullmove_number = undo.fullmove_number;
        self.hash = undo.hash;
    }

    pub fn undo_move(&mut self, undo: UndoState) {
        self.side_to_move = undo.side_to_move;
        self.castling_rights = undo.castling_rights;
        self.en_passant = undo.en_passant;
        self.halfmove_clock = undo.halfmove_clock;
        self.fullmove_number = undo.fullmove_number;

        if let Some((rook_from, rook_to, rook)) = undo.rook_move {
            self.remove_piece_at(rook_to)
                .expect("castling rook must be on undo target");
            self.set_piece(rook_from, rook);
        }

        self.remove_piece_at(undo.mv.to)
            .expect("moved piece must be on undo target");
        self.set_piece(undo.mv.from, undo.moving_piece);

        if let Some((square, piece)) = undo.captured {
            self.set_piece(square, piece);
        }

        self.hash = undo.hash;
    }

    fn move_castling_rook(&mut self, color: Color, king_side: bool) -> (Square, Square, Piece) {
        let rank = match color {
            Color::White => 0,
            Color::Black => 7,
        };
        let (rook_from_file, rook_to_file) = if king_side { (7, 5) } else { (0, 3) };
        let rook_from = Square::from_coords(rook_from_file, rank).expect("valid rook square");
        let rook_to = Square::from_coords(rook_to_file, rank).expect("valid rook square");
        let rook = self
            .remove_piece_at(rook_from)
            .expect("castling rook must exist");
        self.set_piece(rook_to, rook);
        (rook_from, rook_to, rook)
    }

    fn update_castling_rights(&mut self, moving_piece: Piece, mv: Move, captured: Option<Piece>) {
        match moving_piece.kind {
            PieceKind::King => match moving_piece.color {
                Color::White => self.castling_rights &= !(CASTLE_WHITE_KING | CASTLE_WHITE_QUEEN),
                Color::Black => self.castling_rights &= !(CASTLE_BLACK_KING | CASTLE_BLACK_QUEEN),
            },
            PieceKind::Rook => self.clear_rook_castling_right(moving_piece.color, mv.from),
            _ => {}
        }

        if captured.is_some_and(|piece| piece.kind == PieceKind::Rook) {
            self.clear_rook_castling_right(moving_piece.color.opposite(), mv.to);
        }
    }

    fn clear_rook_castling_right(&mut self, color: Color, square: Square) {
        match (color, square.file(), square.rank()) {
            (Color::White, 7, 0) => self.castling_rights &= !CASTLE_WHITE_KING,
            (Color::White, 0, 0) => self.castling_rights &= !CASTLE_WHITE_QUEEN,
            (Color::Black, 7, 7) => self.castling_rights &= !CASTLE_BLACK_KING,
            (Color::Black, 0, 7) => self.castling_rights &= !CASTLE_BLACK_QUEEN,
            _ => {}
        }
    }
}

fn parse_castling_rights(value: &str) -> Result<u8, BoardError> {
    if value == "-" {
        return Ok(0);
    }

    let mut rights = 0u8;
    for ch in value.chars() {
        let bit = match ch {
            'K' => CASTLE_WHITE_KING,
            'Q' => CASTLE_WHITE_QUEEN,
            'k' => CASTLE_BLACK_KING,
            'q' => CASTLE_BLACK_QUEEN,
            _ => return Err(BoardError::InvalidCastlingRights(value.to_string())),
        };
        if rights & bit != 0 {
            return Err(BoardError::InvalidCastlingRights(value.to_string()));
        }
        rights |= bit;
    }

    Ok(rights)
}

fn castling_rights_to_fen(rights: u8) -> String {
    if rights == 0 {
        return "-".to_string();
    }

    let mut out = String::new();
    if rights & CASTLE_WHITE_KING != 0 {
        out.push('K');
    }
    if rights & CASTLE_WHITE_QUEEN != 0 {
        out.push('Q');
    }
    if rights & CASTLE_BLACK_KING != 0 {
        out.push('k');
    }
    if rights & CASTLE_BLACK_QUEEN != 0 {
        out.push('q');
    }
    out
}

pub(crate) fn bit_squares(mut bits: u64) -> impl Iterator<Item = Square> {
    std::iter::from_fn(move || {
        if bits == 0 {
            None
        } else {
            let index = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            Some(Square::from_index(index))
        }
    })
}

pub(crate) fn step_square(square: Square, df: i8, dr: i8) -> Option<Square> {
    let file = square.file() as i8 + df;
    let rank = square.rank() as i8 + dr;
    if (0..=7).contains(&file) && (0..=7).contains(&rank) {
        Square::from_coords(file as u8, rank as u8)
    } else {
        None
    }
}

pub(crate) fn knight_targets(square: Square) -> u64 {
    const OFFSETS: [(i8, i8); 8] = [
        (-2, -1),
        (-2, 1),
        (-1, -2),
        (-1, 2),
        (1, -2),
        (1, 2),
        (2, -1),
        (2, 1),
    ];

    OFFSETS
        .iter()
        .filter_map(|(df, dr)| step_square(square, *df, *dr))
        .fold(0, |acc, target| acc | target.bit())
}

pub(crate) fn king_targets(square: Square) -> u64 {
    const OFFSETS: [(i8, i8); 8] = [
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
    ];

    OFFSETS
        .iter()
        .filter_map(|(df, dr)| step_square(square, *df, *dr))
        .fold(0, |acc, target| acc | target.bit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_round_trips_through_fen() {
        let board = Board::startpos().unwrap();
        assert_eq!(board.to_fen(), STARTPOS_FEN);
        assert_eq!(board.hash(), board.compute_hash());
    }

    #[test]
    fn rejects_missing_kings() {
        let err = Board::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").unwrap_err();
        assert!(matches!(err, BoardError::InvalidKingCount { .. }));
    }

    #[test]
    fn compact_undo_restores_fen_and_hash() {
        let mut board = Board::startpos().unwrap();
        let before_fen = board.to_fen();
        let before_hash = board.hash();
        let mv = crate::movegen::find_legal_move_by_uci(&mut board, "e2e4").unwrap();
        let undo = board.make_move(mv);
        assert_ne!(board.hash(), before_hash);
        board.undo_move(undo);
        assert_eq!(board.to_fen(), before_fen);
        assert_eq!(board.hash(), before_hash);
        assert_eq!(board.hash(), board.compute_hash());
    }
}
