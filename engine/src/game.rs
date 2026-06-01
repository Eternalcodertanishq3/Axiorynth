use crate::board::{Board, STARTPOS_FEN};
use crate::eval::evaluate;
use crate::movegen::{find_legal_move_by_uci, generate_legal_moves};
use crate::mv::Move;
use crate::types::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameResult {
    Ongoing,
    WhiteWin,
    BlackWin,
    DrawStalemate,
    DrawFiftyMove,
}

impl GameResult {
    pub fn as_str(self) -> &'static str {
        match self {
            GameResult::Ongoing => "ongoing",
            GameResult::WhiteWin => "white win",
            GameResult::BlackWin => "black win",
            GameResult::DrawStalemate => "draw by stalemate",
            GameResult::DrawFiftyMove => "draw by fifty-move rule",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRecord {
    pub ply: usize,
    pub mv: Move,
    pub uci: String,
    pub fen_before: String,
    pub fen_after: String,
    pub eval_after: i32,
    pub result_after: GameResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    initial_fen: String,
    board: Board,
    records: Vec<GameRecord>,
    result: GameResult,
}

impl Game {
    pub fn new() -> Result<Game, String> {
        Game::from_fen(STARTPOS_FEN)
    }

    pub fn from_fen(fen: &str) -> Result<Game, String> {
        let board = Board::from_fen(fen).map_err(|err| err.to_string())?;
        Ok(Game {
            initial_fen: board.to_fen(),
            board,
            records: Vec::new(),
            result: GameResult::Ongoing,
        })
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn records(&self) -> &[GameRecord] {
        &self.records
    }

    pub fn result(&self) -> GameResult {
        self.result
    }

    pub fn play_uci(&mut self, uci: &str) -> Result<&GameRecord, String> {
        if self.result != GameResult::Ongoing {
            return Err(format!(
                "game is already finished: {}",
                self.result.as_str()
            ));
        }

        let fen_before = self.board.to_fen();
        let mv = find_legal_move_by_uci(&mut self.board, uci)
            .ok_or_else(|| format!("illegal move: {uci}"))?;
        self.board.make_move(mv);
        self.result = detect_result(&self.board);
        let eval_after = evaluate(&self.board).total_white_perspective;
        let record = GameRecord {
            ply: self.records.len() + 1,
            mv,
            uci: mv.uci(),
            fen_before,
            fen_after: self.board.to_fen(),
            eval_after,
            result_after: self.result,
        };
        self.records.push(record);
        Ok(self.records.last().expect("record was just pushed"))
    }

    pub fn replay_position(&self, ply: usize) -> Result<Board, String> {
        if ply > self.records.len() {
            return Err(format!(
                "cannot replay ply {ply}; game has {} plies",
                self.records.len()
            ));
        }

        let mut board = Board::from_fen(&self.initial_fen).map_err(|err| err.to_string())?;
        for record in &self.records[..ply] {
            let mv = find_legal_move_by_uci(&mut board, &record.uci)
                .ok_or_else(|| format!("stored move is no longer legal: {}", record.uci))?;
            board.make_move(mv);
        }
        Ok(board)
    }

    pub fn uci_moves(&self) -> Vec<String> {
        self.records
            .iter()
            .map(|record| record.uci.clone())
            .collect()
    }

    pub fn as_lines(&self) -> Vec<String> {
        let mut lines = vec![
            "Axiorynth game history".to_string(),
            format!("initial fen: {}", self.initial_fen),
            format!("current fen: {}", self.board.to_fen()),
            format!("result: {}", self.result.as_str()),
            format!("plies: {}", self.records.len()),
        ];

        for record in &self.records {
            lines.push(format!(
                "{}. {} eval {:+} result {} fen {}",
                record.ply,
                record.uci,
                record.eval_after,
                record.result_after.as_str(),
                record.fen_after
            ));
        }
        lines
    }
}

fn detect_result(board: &Board) -> GameResult {
    if board.halfmove_clock() >= 100 {
        return GameResult::DrawFiftyMove;
    }

    let mut clone = board.clone();
    let moves = generate_legal_moves(&mut clone);
    if !moves.is_empty() {
        return GameResult::Ongoing;
    }

    if board.in_check(board.side_to_move()) {
        match board.side_to_move().opposite() {
            Color::White => GameResult::WhiteWin,
            Color::Black => GameResult::BlackWin,
        }
    } else {
        GameResult::DrawStalemate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_records_and_replays_moves() {
        let mut game = Game::new().unwrap();
        game.play_uci("e2e4").unwrap();
        game.play_uci("e7e5").unwrap();
        game.play_uci("g1f3").unwrap();

        assert_eq!(game.records().len(), 3);
        assert_eq!(
            game.replay_position(2).unwrap().to_fen(),
            "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2"
        );
        assert!(game.as_lines().iter().any(|line| line.contains("g1f3")));
    }

    #[test]
    fn game_detects_fools_mate() {
        let mut game = Game::new().unwrap();
        for mv in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            game.play_uci(mv).unwrap();
        }

        assert_eq!(game.result(), GameResult::BlackWin);
    }
}
