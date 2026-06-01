use crate::board::Board;
use crate::eval::{EvalBreakdown, evaluate};
use crate::movegen::generate_legal_moves;
use crate::search::{SearchControl, SearchLimits, SearchResult, iterative_deepening};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisReport {
    pub fen: String,
    pub legal_moves: Vec<String>,
    pub evaluation: EvalBreakdown,
    pub search: SearchResult,
}

impl AnalysisReport {
    pub fn as_lines(&self) -> Vec<String> {
        let mut lines = vec![
            "Axiorynth analysis report".to_string(),
            format!("fen: {}", self.fen),
            format!("legal move count: {}", self.legal_moves.len()),
            format!("legal moves: {}", self.legal_moves.join(" ")),
            String::new(),
            "Evaluation math".to_string(),
        ];

        lines.extend(self.evaluation.as_math_lines());
        lines.push(String::new());
        lines.push("Search math".to_string());
        lines.extend(self.search.as_math_lines());
        lines
    }
}

pub fn analyze_position(
    board: &Board,
    limits: SearchLimits,
    control: &SearchControl,
) -> AnalysisReport {
    let mut legal_board = board.clone();
    let mut legal_moves = generate_legal_moves(&mut legal_board)
        .into_iter()
        .map(|mv| mv.uci())
        .collect::<Vec<_>>();
    legal_moves.sort();

    let mut search_board = board.clone();
    let search = iterative_deepening(&mut search_board, limits, control);

    AnalysisReport {
        fen: board.to_fen(),
        legal_moves,
        evaluation: evaluate(board),
        search,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_reports_startpos_math_and_moves() {
        let board = Board::startpos().unwrap();
        let control = SearchControl::new();
        let report = analyze_position(
            &board,
            SearchLimits {
                max_depth: 1,
                hash_size_mb: 1,
                ..SearchLimits::default()
            },
            &control,
        );

        assert_eq!(report.legal_moves.len(), 20);
        assert_eq!(report.evaluation.total_white_perspective, 0);
        assert!(report.search.best_move.is_some());
        assert!(report.as_lines().iter().any(|line| line == "Search math"));
    }

    #[test]
    fn analysis_does_not_mutate_input_board() {
        let board = Board::startpos().unwrap();
        let before = board.to_fen();
        let control = SearchControl::new();
        let _ = analyze_position(
            &board,
            SearchLimits {
                max_depth: 1,
                hash_size_mb: 1,
                ..SearchLimits::default()
            },
            &control,
        );
        assert_eq!(board.to_fen(), before);
        assert_eq!(board.hash(), board.compute_hash());
    }
}
