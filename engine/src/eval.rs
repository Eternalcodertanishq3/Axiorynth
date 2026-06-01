use crate::board::{Board, bit_squares, king_targets};
use crate::movegen::generate_legal_moves;
use crate::types::{Color, PieceKind, Square};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalBreakdown {
    pub material_white: i32,
    pub material_black: i32,
    pub material_score: i32,
    pub piece_square_white: i32,
    pub piece_square_black: i32,
    pub piece_square_score: i32,
    pub mobility_white: i32,
    pub mobility_black: i32,
    pub mobility_score: i32,
    pub center_white: i32,
    pub center_black: i32,
    pub center_score: i32,
    pub pawn_structure_white: i32,
    pub pawn_structure_black: i32,
    pub pawn_structure_score: i32,
    pub king_safety_white: i32,
    pub king_safety_black: i32,
    pub king_safety_score: i32,
    pub total_white_perspective: i32,
    pub total_side_to_move_perspective: i32,
}

impl EvalBreakdown {
    pub fn as_math_lines(&self) -> Vec<String> {
        vec![
            format!(
                "material: {} - {} = {:+}",
                self.material_white, self.material_black, self.material_score
            ),
            format!(
                "piece-square: {} - {} = {:+}",
                self.piece_square_white, self.piece_square_black, self.piece_square_score
            ),
            format!(
                "mobility: ({} - {}) * 2 = {:+}",
                self.mobility_white, self.mobility_black, self.mobility_score
            ),
            format!(
                "center: {} - {} = {:+}",
                self.center_white, self.center_black, self.center_score
            ),
            format!(
                "pawn structure: {} - {} = {:+}",
                self.pawn_structure_white, self.pawn_structure_black, self.pawn_structure_score
            ),
            format!(
                "king safety: {} - {} = {:+}",
                self.king_safety_white, self.king_safety_black, self.king_safety_score
            ),
            format!(
                "total: {:+} centipawns from White perspective",
                self.total_white_perspective
            ),
            format!(
                "side-to-move total: {:+} centipawns",
                self.total_side_to_move_perspective
            ),
        ]
    }
}

pub fn evaluate(board: &Board) -> EvalBreakdown {
    let material_white = material(board, Color::White);
    let material_black = material(board, Color::Black);
    let material_score = material_white - material_black;

    let piece_square_white = piece_square(board, Color::White);
    let piece_square_black = piece_square(board, Color::Black);
    let piece_square_score = piece_square_white - piece_square_black;

    let mobility_white = mobility(board, Color::White);
    let mobility_black = mobility(board, Color::Black);
    let mobility_score = (mobility_white - mobility_black) * 2;

    let center_white = center_control(board, Color::White);
    let center_black = center_control(board, Color::Black);
    let center_score = center_white - center_black;

    let pawn_structure_white = pawn_structure(board, Color::White);
    let pawn_structure_black = pawn_structure(board, Color::Black);
    let pawn_structure_score = pawn_structure_white - pawn_structure_black;

    let king_safety_white = king_safety(board, Color::White);
    let king_safety_black = king_safety(board, Color::Black);
    let king_safety_score = king_safety_white - king_safety_black;

    let total_white_perspective = material_score
        + piece_square_score
        + mobility_score
        + center_score
        + pawn_structure_score
        + king_safety_score;
    let total_side_to_move_perspective = match board.side_to_move() {
        Color::White => total_white_perspective,
        Color::Black => -total_white_perspective,
    };

    EvalBreakdown {
        material_white,
        material_black,
        material_score,
        piece_square_white,
        piece_square_black,
        piece_square_score,
        mobility_white,
        mobility_black,
        mobility_score,
        center_white,
        center_black,
        center_score,
        pawn_structure_white,
        pawn_structure_black,
        pawn_structure_score,
        king_safety_white,
        king_safety_black,
        king_safety_score,
        total_white_perspective,
        total_side_to_move_perspective,
    }
}

pub fn evaluate_side_to_move(board: &Board) -> i32 {
    evaluate(board).total_side_to_move_perspective
}

fn material(board: &Board, color: Color) -> i32 {
    PieceKind::ALL
        .iter()
        .map(|kind| board.piece_count(color, *kind) as i32 * kind.material_value())
        .sum()
}

fn piece_square(board: &Board, color: Color) -> i32 {
    PieceKind::ALL
        .iter()
        .map(|kind| {
            bit_squares(board.pieces(color, *kind))
                .map(|square| table_value(*kind, color, square))
                .sum::<i32>()
        })
        .sum()
}

fn table_value(kind: PieceKind, color: Color, square: Square) -> i32 {
    let index = match color {
        Color::White => square.index(),
        Color::Black => (7 - square.rank() as usize) * 8 + square.file() as usize,
    };

    match kind {
        PieceKind::Pawn => PAWN_TABLE[index],
        PieceKind::Knight => KNIGHT_TABLE[index],
        PieceKind::Bishop => BISHOP_TABLE[index],
        PieceKind::Rook => ROOK_TABLE[index],
        PieceKind::Queen => QUEEN_TABLE[index],
        PieceKind::King => KING_TABLE[index],
    }
}

fn mobility(board: &Board, color: Color) -> i32 {
    let mut clone = board.clone();
    clone.set_side_to_move(color);
    generate_legal_moves(&mut clone).len() as i32
}

fn center_control(board: &Board, color: Color) -> i32 {
    ["d4", "e4", "d5", "e5"]
        .iter()
        .map(|name| Square::from_name(name).expect("valid center square"))
        .map(|square| {
            let attack_score = if board.is_square_attacked(square, color) {
                8
            } else {
                0
            };
            let occupancy_score = if board
                .piece_at(square)
                .is_some_and(|piece| piece.color == color)
            {
                10
            } else {
                0
            };
            attack_score + occupancy_score
        })
        .sum()
}

fn pawn_structure(board: &Board, color: Color) -> i32 {
    let mut file_counts = [0i32; 8];
    for pawn in bit_squares(board.pieces(color, PieceKind::Pawn)) {
        file_counts[pawn.file() as usize] += 1;
    }

    let doubled_penalty: i32 = file_counts
        .iter()
        .filter(|count| **count > 1)
        .map(|count| (*count - 1) * 12)
        .sum();

    let mut isolated_penalty = 0;
    let mut passed_bonus = 0;
    for pawn in bit_squares(board.pieces(color, PieceKind::Pawn)) {
        let file = pawn.file() as i32;
        let has_left = file > 0 && file_counts[(file - 1) as usize] > 0;
        let has_right = file < 7 && file_counts[(file + 1) as usize] > 0;
        if !has_left && !has_right {
            isolated_penalty += 10;
        }

        if is_passed_pawn(board, color, pawn) {
            let advancement = match color {
                Color::White => pawn.rank() as i32,
                Color::Black => 7 - pawn.rank() as i32,
            };
            passed_bonus += advancement * 5;
        }
    }

    passed_bonus - doubled_penalty - isolated_penalty
}

fn is_passed_pawn(board: &Board, color: Color, pawn: Square) -> bool {
    let enemy_pawns = board.pieces(color.opposite(), PieceKind::Pawn);
    let file = pawn.file() as i32;
    for enemy in bit_squares(enemy_pawns) {
        let enemy_file = enemy.file() as i32;
        if (enemy_file - file).abs() > 1 {
            continue;
        }

        let blocks_promotion_path = match color {
            Color::White => enemy.rank() > pawn.rank(),
            Color::Black => enemy.rank() < pawn.rank(),
        };
        if blocks_promotion_path {
            return false;
        }
    }

    true
}

fn king_safety(board: &Board, color: Color) -> i32 {
    let Some(king) = board.king_square(color) else {
        return 0;
    };

    let attacked_ring = bit_squares(king_targets(king))
        .filter(|square| board.is_square_attacked(*square, color.opposite()))
        .count() as i32;

    let shield_rank_delta = match color {
        Color::White => 1,
        Color::Black => -1,
    };
    let shield = [-1, 0, 1]
        .iter()
        .filter_map(|df| crate::board::step_square(king, *df, shield_rank_delta))
        .filter(|square| {
            board
                .piece_at(*square)
                .is_some_and(|piece| piece.color == color && piece.kind == PieceKind::Pawn)
        })
        .count() as i32;

    shield * 6 - attacked_ring * 8
}

const PAWN_TABLE: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 10, -15, -15, 10, 10, 5, 4, 8, 8, 12, 12, 8, 8, 4, 2, 5, 7, 18,
    18, 7, 5, 2, 1, 2, 4, 14, 14, 4, 2, 1, 2, 2, 3, -6, -6, 3, 2, 2, 4, 5, 5, -10, -10, 5, 5, 4, 0,
    0, 0, 0, 0, 0, 0, 0,
];

const KNIGHT_TABLE: [i32; 64] = [
    -50, -35, -25, -20, -20, -25, -35, -50, -35, -15, 0, 5, 5, 0, -15, -35, -25, 5, 12, 18, 18, 12,
    5, -25, -20, 8, 18, 24, 24, 18, 8, -20, -20, 5, 18, 24, 24, 18, 5, -20, -25, 0, 12, 18, 18, 12,
    0, -25, -35, -15, 0, 5, 5, 0, -15, -35, -50, -35, -25, -20, -20, -25, -35, -50,
];

const BISHOP_TABLE: [i32; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20, -10, 8, 0, 0, 0, 0, 8, -10, -10, 10, 10, 12, 12, 10,
    10, -10, -10, 0, 12, 16, 16, 12, 0, -10, -10, 5, 8, 16, 16, 8, 5, -10, -10, 0, 8, 10, 10, 8, 0,
    -10, -10, 4, 0, 0, 0, 0, 4, -10, -20, -10, -10, -10, -10, -10, -10, -20,
];

const ROOK_TABLE: [i32; 64] = [
    0, 0, 0, 6, 6, 0, 0, 0, -2, 0, 0, 0, 0, 0, 0, -2, -4, 0, 0, 0, 0, 0, 0, -4, -4, 0, 0, 0, 0, 0,
    0, -4, -4, 0, 0, 0, 0, 0, 0, -4, -4, 0, 0, 0, 0, 0, 0, -4, 8, 12, 12, 12, 12, 12, 12, 8, 0, 0,
    0, 4, 4, 0, 0, 0,
];

const QUEEN_TABLE: [i32; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20, -10, 0, 4, 0, 0, 0, 0, -10, -10, 4, 8, 8, 8, 8, 0, -10,
    -5, 0, 8, 8, 8, 8, 0, -5, 0, 0, 8, 8, 8, 8, 0, -5, -10, 8, 8, 8, 8, 8, 0, -10, -10, 0, 8, 0, 0,
    0, 0, -10, -20, -10, -10, -5, -5, -10, -10, -20,
];

const KING_TABLE: [i32; 64] = [
    20, 30, 10, 0, 0, 10, 30, 20, 20, 20, 0, 0, 0, 0, 20, 20, -10, -20, -20, -20, -20, -20, -20,
    -10, -20, -30, -30, -40, -40, -30, -30, -20, -30, -40, -40, -50, -50, -40, -40, -30, -30, -40,
    -40, -50, -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40, -50, -50,
    -40, -40, -30,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_evaluates_to_zero_material() {
        let board = Board::startpos().unwrap();
        let eval = evaluate(&board);
        assert_eq!(eval.material_score, 0);
        assert_eq!(eval.total_white_perspective, 0);
    }

    #[test]
    fn material_advantage_is_visible_in_centipawns() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1").unwrap();
        let eval = evaluate(&board);
        assert_eq!(eval.material_score, 900);
        assert!(eval.total_white_perspective > 800);
    }

    #[test]
    fn math_lines_include_phase_2_terms() {
        let board = Board::startpos().unwrap();
        let lines = evaluate(&board).as_math_lines();
        assert!(lines.iter().any(|line| line.starts_with("piece-square:")));
        assert!(lines.iter().any(|line| line.starts_with("king safety:")));
    }
}
