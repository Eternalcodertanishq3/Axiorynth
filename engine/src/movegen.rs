use crate::board::{
    Board, CASTLE_BLACK_KING, CASTLE_BLACK_QUEEN, CASTLE_WHITE_KING, CASTLE_WHITE_QUEEN,
    bit_squares, king_targets, knight_targets, step_square,
};
use crate::mv::{Move, MoveKind};
use crate::types::{Color, Piece, PieceKind, Square};

pub fn generate_legal_moves(board: &mut Board) -> Vec<Move> {
    let us = board.side_to_move();
    let pseudo_moves = generate_pseudo_legal_moves(board);
    let mut legal_moves = Vec::with_capacity(pseudo_moves.len());

    for mv in pseudo_moves {
        let undo = board.make_move(mv);
        if !board.in_check(us) {
            legal_moves.push(mv);
        }
        board.undo_move(undo);
    }

    legal_moves
}

pub fn find_legal_move_by_uci(board: &mut Board, uci: &str) -> Option<Move> {
    generate_legal_moves(board)
        .into_iter()
        .find(|mv| mv.uci() == uci)
}

pub fn generate_pseudo_legal_moves(board: &Board) -> Vec<Move> {
    let mut moves = Vec::with_capacity(64);
    let us = board.side_to_move();

    generate_pawns(board, us, &mut moves);
    generate_leapers(board, us, PieceKind::Knight, &mut moves);
    generate_sliders(
        board,
        us,
        PieceKind::Bishop,
        &[(-1, -1), (-1, 1), (1, -1), (1, 1)],
        &mut moves,
    );
    generate_sliders(
        board,
        us,
        PieceKind::Rook,
        &[(-1, 0), (1, 0), (0, -1), (0, 1)],
        &mut moves,
    );
    generate_sliders(
        board,
        us,
        PieceKind::Queen,
        &[
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1),
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
        ],
        &mut moves,
    );
    generate_king(board, us, &mut moves);

    moves
}

fn generate_pawns(board: &Board, us: Color, moves: &mut Vec<Move>) {
    let direction: i8 = match us {
        Color::White => 1,
        Color::Black => -1,
    };
    let start_rank = match us {
        Color::White => 1,
        Color::Black => 6,
    };
    let promotion_from_rank = match us {
        Color::White => 6,
        Color::Black => 1,
    };

    for from in bit_squares(board.pieces(us, PieceKind::Pawn)) {
        if let Some(to) = step_square(from, 0, direction) {
            if board.piece_at(to).is_none() {
                if from.rank() == promotion_from_rank {
                    push_promotions(moves, from, to, MoveKind::Quiet);
                } else {
                    moves.push(Move::new(from, to, None, MoveKind::Quiet));

                    if from.rank() == start_rank {
                        if let Some(double_to) = step_square(to, 0, direction) {
                            if board.piece_at(double_to).is_none() {
                                moves.push(Move::new(
                                    from,
                                    double_to,
                                    None,
                                    MoveKind::DoublePawnPush,
                                ));
                            }
                        }
                    }
                }
            }
        }

        for file_delta in [-1, 1] {
            let Some(to) = step_square(from, file_delta, direction) else {
                continue;
            };

            let target_piece = board.piece_at(to);
            if target_piece.is_some_and(|piece| piece.color == us.opposite()) {
                if from.rank() == promotion_from_rank {
                    push_promotions(moves, from, to, MoveKind::Capture);
                } else {
                    moves.push(Move::new(from, to, None, MoveKind::Capture));
                }
            }

            if board.en_passant() == Some(to) {
                moves.push(Move::new(from, to, None, MoveKind::EnPassant));
            }
        }
    }
}

fn push_promotions(moves: &mut Vec<Move>, from: Square, to: Square, kind: MoveKind) {
    for promotion in PieceKind::PROMOTIONS {
        moves.push(Move::new(from, to, Some(promotion), kind));
    }
}

fn generate_leapers(board: &Board, us: Color, kind: PieceKind, moves: &mut Vec<Move>) {
    for from in bit_squares(board.pieces(us, kind)) {
        let targets = match kind {
            PieceKind::Knight => knight_targets(from),
            PieceKind::King => king_targets(from),
            _ => unreachable!("leaper generation supports knight and king only"),
        };

        push_target_bits(board, us, from, targets, moves);
    }
}

fn generate_sliders(
    board: &Board,
    us: Color,
    kind: PieceKind,
    directions: &[(i8, i8)],
    moves: &mut Vec<Move>,
) {
    for from in bit_squares(board.pieces(us, kind)) {
        for (df, dr) in directions {
            let mut current = from;
            while let Some(to) = step_square(current, *df, *dr) {
                current = to;
                match board.piece_at(to) {
                    Some(Piece { color, .. }) if color == us => break,
                    Some(_) => {
                        moves.push(Move::new(from, to, None, MoveKind::Capture));
                        break;
                    }
                    None => moves.push(Move::new(from, to, None, MoveKind::Quiet)),
                }
            }
        }
    }
}

fn generate_king(board: &Board, us: Color, moves: &mut Vec<Move>) {
    generate_leapers(board, us, PieceKind::King, moves);
    generate_castles(board, us, moves);
}

fn push_target_bits(board: &Board, us: Color, from: Square, targets: u64, moves: &mut Vec<Move>) {
    let own = board.color_occupancy(us);
    for to in bit_squares(targets & !own) {
        let kind = if board.piece_at(to).is_some() {
            MoveKind::Capture
        } else {
            MoveKind::Quiet
        };
        moves.push(Move::new(from, to, None, kind));
    }
}

fn generate_castles(board: &Board, us: Color, moves: &mut Vec<Move>) {
    if board.in_check(us) {
        return;
    }

    let rank = match us {
        Color::White => 0,
        Color::Black => 7,
    };
    let king_from = sq(4, rank);
    let enemy = us.opposite();

    let king_right = match us {
        Color::White => CASTLE_WHITE_KING,
        Color::Black => CASTLE_BLACK_KING,
    };
    if board.castling_rights() & king_right != 0 {
        let rook_from = sq(7, rank);
        let f = sq(5, rank);
        let g = sq(6, rank);
        if board.piece_at(king_from)
            == Some(Piece {
                color: us,
                kind: PieceKind::King,
            })
            && board.piece_at(rook_from)
                == Some(Piece {
                    color: us,
                    kind: PieceKind::Rook,
                })
            && board.piece_at(f).is_none()
            && board.piece_at(g).is_none()
            && !board.is_square_attacked(f, enemy)
            && !board.is_square_attacked(g, enemy)
        {
            moves.push(Move::new(king_from, g, None, MoveKind::KingCastle));
        }
    }

    let queen_right = match us {
        Color::White => CASTLE_WHITE_QUEEN,
        Color::Black => CASTLE_BLACK_QUEEN,
    };
    if board.castling_rights() & queen_right != 0 {
        let rook_from = sq(0, rank);
        let b = sq(1, rank);
        let c = sq(2, rank);
        let d = sq(3, rank);
        if board.piece_at(king_from)
            == Some(Piece {
                color: us,
                kind: PieceKind::King,
            })
            && board.piece_at(rook_from)
                == Some(Piece {
                    color: us,
                    kind: PieceKind::Rook,
                })
            && board.piece_at(b).is_none()
            && board.piece_at(c).is_none()
            && board.piece_at(d).is_none()
            && !board.is_square_attacked(c, enemy)
            && !board.is_square_attacked(d, enemy)
        {
            moves.push(Move::new(king_from, c, None, MoveKind::QueenCastle));
        }
    }
}

fn sq(file: u8, rank: u8) -> Square {
    Square::from_coords(file, rank).expect("valid board square")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Board;

    #[test]
    fn startpos_has_twenty_legal_moves() {
        let mut board = Board::startpos().unwrap();
        assert_eq!(generate_legal_moves(&mut board).len(), 20);
    }

    #[test]
    fn castling_moves_are_generated_when_legal() {
        let mut board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let moves: Vec<String> = generate_legal_moves(&mut board)
            .into_iter()
            .map(Move::uci)
            .collect();

        assert!(moves.contains(&"e1g1".to_string()));
        assert!(moves.contains(&"e1c1".to_string()));
    }

    #[test]
    fn en_passant_removes_the_captured_pawn() {
        let mut board = Board::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        let mv = find_legal_move_by_uci(&mut board, "e5d6").unwrap();

        let undo = board.make_move(mv);
        assert_eq!(board.piece_at(Square::from_name("d5").unwrap()), None);
        assert_eq!(
            board.piece_at(Square::from_name("d6").unwrap()),
            Some(Piece {
                color: Color::White,
                kind: PieceKind::Pawn,
            })
        );
        board.undo_move(undo);
        assert_eq!(board.to_fen(), "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1");
    }

    #[test]
    fn uci_lookup_rejects_illegal_moves() {
        let mut board = Board::startpos().unwrap();
        assert!(find_legal_move_by_uci(&mut board, "e2e4").is_some());
        assert!(find_legal_move_by_uci(&mut board, "e2e5").is_none());
    }
}
