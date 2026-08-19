use crate::board::{Board, bit_squares, king_targets};
use crate::types::{Color, PieceKind, Square};
use std::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvalConfig {
    pub pawn_val: i32,
    pub knight_val: i32,
    pub bishop_val: i32,
    pub rook_val: i32,
    pub queen_val: i32,
    pub center_attack: i32,
    pub center_occupancy: i32,
    pub pawn_doubled_penalty: i32,
    pub pawn_isolated_penalty: i32,
    pub pawn_passed_bonus: i32,
    pub king_safety_shield: i32,
    pub king_safety_attacked_ring: i32,
    pub mobility_multiplier: i32,
}

impl Default for EvalConfig {
    fn default() -> Self {
        EvalConfig {
            pawn_val: 100,
            knight_val: 320,
            bishop_val: 330,
            rook_val: 500,
            queen_val: 900,
            center_attack: 8,
            center_occupancy: 10,
            pawn_doubled_penalty: 12,
            pawn_isolated_penalty: 10,
            pawn_passed_bonus: 5,
            king_safety_shield: 6,
            king_safety_attacked_ring: 8,
            mobility_multiplier: 2,
        }
    }
}

pub static EVAL_CONFIG: RwLock<EvalConfig> = RwLock::new(EvalConfig {
    pawn_val: 100,
    knight_val: 320,
    bishop_val: 330,
    rook_val: 500,
    queen_val: 900,
    center_attack: 8,
    center_occupancy: 10,
    pawn_doubled_penalty: 12,
    pawn_isolated_penalty: 10,
    pawn_passed_bonus: 5,
    king_safety_shield: 6,
    king_safety_attacked_ring: 8,
    mobility_multiplier: 2,
});

pub fn get_config() -> EvalConfig {
    *EVAL_CONFIG.read().unwrap_or_else(|e| e.into_inner())
}

pub fn update_config(config: EvalConfig) {
    if let Ok(mut lock) = EVAL_CONFIG.write() {
        *lock = config;
    }
}

/// HalfKP feature extractor for NNUE training inputs.
/// Maps the board state into a sparse vector of active feature indices for the given perspective.
/// Own king square is K. For each other piece of color C and kind P on square S:
/// feature_index = K * 640 + C_offset * 320 + P_offset * 64 + S
pub fn get_half_kp_features(board: &Board, perspective: Color) -> Vec<usize> {
    let mut features = Vec::new();
    let Some(king_sq) = board.king_square(perspective) else {
        return features;
    };
    
    let k_index = if perspective == Color::White {
        king_sq.index()
    } else {
        (7 - king_sq.rank() as usize) * 8 + king_sq.file() as usize
    };

    for sq_idx in 0..64 {
        let square = Square::from_index(sq_idx);
        if let Some(piece) = board.piece_at(square) {
            if piece.kind == PieceKind::King {
                continue;
            }
            
            let is_opponent = if piece.color == perspective { 0 } else { 1 };
            
            let kind_idx = match piece.kind {
                PieceKind::Pawn => 0,
                PieceKind::Knight => 1,
                PieceKind::Bishop => 2,
                PieceKind::Rook => 3,
                PieceKind::Queen => 4,
                PieceKind::King => continue,
            };
            
            let s_index = if perspective == Color::White {
                sq_idx
            } else {
                (7 - square.rank() as usize) * 8 + square.file() as usize
            };
            
            let feature_idx = k_index * 640 + is_opponent * 320 + kind_idx * 64 + s_index;
            features.push(feature_idx);
        }
    }
    
    features
}

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
        let config = get_config();
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
                "mobility: ({} - {}) * {} = {:+}",
                self.mobility_white, self.mobility_black, config.mobility_multiplier, self.mobility_score
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
    let config = get_config();

    let material_white = material(board, Color::White, &config);
    let material_black = material(board, Color::Black, &config);
    let material_score = material_white - material_black;

    let phase = game_phase(board);
    let piece_square_white = piece_square(board, Color::White, phase);
    let piece_square_black = piece_square(board, Color::Black, phase);
    let piece_square_score = piece_square_white - piece_square_black;

    let mobility_white = mobility(board, Color::White);
    let mobility_black = mobility(board, Color::Black);
    let mobility_score = (mobility_white - mobility_black) * config.mobility_multiplier;

    let center_white = center_control(board, Color::White, &config);
    let center_black = center_control(board, Color::Black, &config);
    let center_score = center_white - center_black;

    let pawn_structure_white = pawn_structure(board, Color::White, &config);
    let pawn_structure_black = pawn_structure(board, Color::Black, &config);
    let pawn_structure_score = pawn_structure_white - pawn_structure_black;

    let king_safety_white = king_safety(board, Color::White, &config);
    let king_safety_black = king_safety(board, Color::Black, &config);
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

static NNUE: RwLock<Option<crate::nnue::NnueNetwork>> = RwLock::new(None);

pub fn load_nnue(path: &str) -> std::io::Result<()> {
    let net = crate::nnue::NnueNetwork::load(path)?;
    let mut lock = NNUE.write().unwrap();
    *lock = Some(net);
    Ok(())
}

pub fn unload_nnue() {
    let mut lock = NNUE.write().unwrap();
    *lock = None;
}

pub fn evaluate_nnue(board: &Board) -> Option<i32> {
    let lock = NNUE.read().unwrap();
    lock.as_ref().map(|net| net.evaluate_board(board))
}

pub fn evaluate_side_to_move(board: &Board) -> i32 {
    if let Some(score) = evaluate_nnue(board) {
        score
    } else {
        evaluate(board).total_side_to_move_perspective
    }
}

fn material(board: &Board, color: Color, config: &EvalConfig) -> i32 {
    PieceKind::ALL
        .iter()
        .map(|kind| {
            let val = match kind {
                PieceKind::Pawn => config.pawn_val,
                PieceKind::Knight => config.knight_val,
                PieceKind::Bishop => config.bishop_val,
                PieceKind::Rook => config.rook_val,
                PieceKind::Queen => config.queen_val,
                PieceKind::King => 0,
            };
            board.piece_count(color, *kind) as i32 * val
        })
        .sum()
}

pub fn game_phase(board: &Board) -> i32 {
    let knights = board.piece_count(Color::White, PieceKind::Knight) + board.piece_count(Color::Black, PieceKind::Knight);
    let bishops = board.piece_count(Color::White, PieceKind::Bishop) + board.piece_count(Color::Black, PieceKind::Bishop);
    let rooks = board.piece_count(Color::White, PieceKind::Rook) + board.piece_count(Color::Black, PieceKind::Rook);
    let queens = board.piece_count(Color::White, PieceKind::Queen) + board.piece_count(Color::Black, PieceKind::Queen);
    (knights as i32 + bishops as i32 + rooks as i32 * 2 + queens as i32 * 4).min(24)
}

fn piece_square(board: &Board, color: Color, phase: i32) -> i32 {
    PieceKind::ALL
        .iter()
        .map(|kind| {
            bit_squares(board.pieces(color, *kind))
                .map(|square| table_value(*kind, color, square, phase))
                .sum::<i32>()
        })
        .sum()
}

fn table_value(kind: PieceKind, color: Color, square: Square, phase: i32) -> i32 {
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
        PieceKind::King => {
            let mg_val = KING_TABLE[index];
            let eg_val = KING_ENDGAME_TABLE[index];
            (mg_val * phase + eg_val * (24 - phase)) / 24
        }
    }
}

fn mobility(board: &Board, color: Color) -> i32 {
    let own = board.color_occupancy(color);
    let mut count = 0;

    for sq in bit_squares(board.pieces(color, PieceKind::Knight)) {
        count += (crate::board::knight_targets(sq) & !own).count_ones() as i32;
    }

    let bishop_dirs = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
    for sq in bit_squares(board.pieces(color, PieceKind::Bishop)) {
        count += count_slider_targets(board, sq, &bishop_dirs, own);
    }

    let rook_dirs = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for sq in bit_squares(board.pieces(color, PieceKind::Rook)) {
        count += count_slider_targets(board, sq, &rook_dirs, own);
    }

    let queen_dirs = [
        (-1, -1), (-1, 1), (1, -1), (1, 1),
        (-1, 0), (1, 0), (0, -1), (0, 1),
    ];
    for sq in bit_squares(board.pieces(color, PieceKind::Queen)) {
        count += count_slider_targets(board, sq, &queen_dirs, own);
    }

    count
}

fn count_slider_targets(board: &Board, from: Square, directions: &[(i8, i8)], own: u64) -> i32 {
    let mut count = 0;
    for (df, dr) in directions {
        let mut current = from;
        while let Some(next) = crate::board::step_square(current, *df, *dr) {
            current = next;
            if (next.bit() & own) != 0 {
                break;
            }
            count += 1;
            if board.piece_at(next).is_some() {
                break;
            }
        }
    }
    count
}

fn center_control(board: &Board, color: Color, config: &EvalConfig) -> i32 {
    ["d4", "e4", "d5", "e5"]
        .iter()
        .map(|name| Square::from_name(name).expect("valid center square"))
        .map(|square| {
            let attack_score = if board.is_square_attacked(square, color) {
                config.center_attack
            } else {
                0
            };
            let occupancy_score = if board
                .piece_at(square)
                .is_some_and(|piece| piece.color == color)
            {
                config.center_occupancy
            } else {
                0
            };
            attack_score + occupancy_score
        })
        .sum()
}

fn pawn_structure(board: &Board, color: Color, config: &EvalConfig) -> i32 {
    let mut file_counts = [0i32; 8];
    for pawn in bit_squares(board.pieces(color, PieceKind::Pawn)) {
        file_counts[pawn.file() as usize] += 1;
    }

    let doubled_penalty: i32 = file_counts
        .iter()
        .filter(|count| **count > 1)
        .map(|count| (*count - 1) * config.pawn_doubled_penalty)
        .sum();

    let mut isolated_penalty = 0;
    let mut passed_bonus = 0;
    for pawn in bit_squares(board.pieces(color, PieceKind::Pawn)) {
        let file = pawn.file() as i32;
        let has_left = file > 0 && file_counts[(file - 1) as usize] > 0;
        let has_right = file < 7 && file_counts[(file + 1) as usize] > 0;
        if !has_left && !has_right {
            isolated_penalty += config.pawn_isolated_penalty;
        }

        if is_passed_pawn(board, color, pawn) {
            let advancement = match color {
                Color::White => pawn.rank() as i32,
                Color::Black => 7 - pawn.rank() as i32,
            };
            passed_bonus += advancement * config.pawn_passed_bonus;
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

fn king_safety(board: &Board, color: Color, config: &EvalConfig) -> i32 {
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

    shield * config.king_safety_shield - attacked_ring * config.king_safety_attacked_ring
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

const KING_ENDGAME_TABLE: [i32; 64] = [
    -50, -40, -30, -20, -20, -30, -40, -50,
    -30, -20, -10,   0,   0, -10, -20, -30,
    -30, -10,  20,  30,  30,  20, -10, -30,
    -30, -10,  30,  40,  40,  30, -10, -30,
    -30, -10,  30,  40,  40,  30, -10, -30,
    -30, -10,  20,  30,  30,  20, -10, -30,
    -30, -30,   0,   0,   0,   0, -30, -30,
    -50, -30, -30, -30, -30, -30, -30, -50,
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
        let expected_queen = get_config().queen_val;
        let eval = evaluate(&board);
        assert_eq!(eval.material_score, expected_queen);
        assert!(eval.total_white_perspective > expected_queen - 100);
    }

    #[test]
    fn math_lines_include_phase_2_terms() {
        let board = Board::startpos().unwrap();
        let lines = evaluate(&board).as_math_lines();
        assert!(lines.iter().any(|line| line.starts_with("piece-square:")));
        assert!(lines.iter().any(|line| line.starts_with("king safety:")));
    }

    #[test]
    fn spsa_updates_evaluation_values() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1").unwrap();
        let original_config = get_config();
        
        let mut custom = original_config;
        custom.queen_val = 1500;
        update_config(custom);
        
        let eval = evaluate(&board);
        assert_eq!(eval.material_score, 1500);
        
        // Reset to avoid side effects on other tests
        update_config(original_config);
    }

    #[test]
    fn half_kp_features_are_extracted_correctly() {
        let board = Board::startpos().unwrap();
        let white_features = get_half_kp_features(&board, Color::White);
        let black_features = get_half_kp_features(&board, Color::Black);
        
        // Active features should not be empty (we have a board full of pieces)
        assert!(!white_features.is_empty());
        assert!(!black_features.is_empty());
        
        // Each feature index should be within bounds of our NNUE input size (64 * 640 = 40,960)
        for &feat in &white_features {
            assert!(feat < 40960, "Feature index {} is out of bounds", feat);
        }
        for &feat in &black_features {
            assert!(feat < 40960, "Feature index {} is out of bounds", feat);
        }
    }
}
