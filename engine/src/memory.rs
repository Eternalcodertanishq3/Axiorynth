use std::collections::BTreeMap;

use crate::game::{Game, GameResult};
use crate::types::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerResult {
    Win,
    Loss,
    Draw,
    Ongoing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMemory {
    pub player_name: String,
    pub games_seen: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    pub unfinished: u32,
    pub first_move_counts: BTreeMap<String, u32>,
    pub early_sequence_counts: BTreeMap<String, u32>,
}

impl PlayerMemory {
    pub fn new(player_name: impl Into<String>) -> PlayerMemory {
        PlayerMemory {
            player_name: player_name.into(),
            games_seen: 0,
            wins: 0,
            losses: 0,
            draws: 0,
            unfinished: 0,
            first_move_counts: BTreeMap::new(),
            early_sequence_counts: BTreeMap::new(),
        }
    }

    pub fn learn_from_game(&mut self, game: &Game, player_color: Color) -> PlayerResult {
        let result = player_result(game.result(), player_color);
        self.games_seen += 1;
        match result {
            PlayerResult::Win => self.wins += 1,
            PlayerResult::Loss => self.losses += 1,
            PlayerResult::Draw => self.draws += 1,
            PlayerResult::Ongoing => self.unfinished += 1,
        }

        if let Some(first) = first_player_move(game, player_color) {
            *self.first_move_counts.entry(first).or_insert(0) += 1;
        }

        let sequence = early_sequence(game, 6);
        if !sequence.is_empty() {
            *self.early_sequence_counts.entry(sequence).or_insert(0) += 1;
        }

        result
    }

    pub fn favorite_first_move(&self) -> Option<(&str, u32)> {
        self.first_move_counts
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(mv, count)| (mv.as_str(), *count))
    }

    pub fn notes(&self) -> Vec<String> {
        let mut notes = vec![format!("games studied: {}", self.games_seen)];
        notes.push(format!(
            "record: {} wins, {} losses, {} draws, {} unfinished",
            self.wins, self.losses, self.draws, self.unfinished
        ));

        if let Some((mv, count)) = self.favorite_first_move() {
            notes.push(format!("favorite first move: {mv} played {count} times"));
            if count >= 3 {
                notes.push(format!(
                    "adaptive hint: prepare a specific response against repeated {mv}"
                ));
            }
        }

        if self.losses > self.wins && self.games_seen >= 3 {
            notes.push("adaptive hint: player is currently losing more than winning".to_string());
        }

        notes
    }

    pub fn as_lines(&self) -> Vec<String> {
        let mut lines = vec![format!("Axiorynth memory for {}", self.player_name)];
        lines.extend(self.notes());
        lines.push("first move counts:".to_string());
        for (mv, count) in &self.first_move_counts {
            lines.push(format!("{mv}: {count}"));
        }
        lines
    }
}

fn player_result(result: GameResult, player_color: Color) -> PlayerResult {
    match (result, player_color) {
        (GameResult::WhiteWin, Color::White) | (GameResult::BlackWin, Color::Black) => {
            PlayerResult::Win
        }
        (GameResult::WhiteWin, Color::Black) | (GameResult::BlackWin, Color::White) => {
            PlayerResult::Loss
        }
        (GameResult::DrawStalemate | GameResult::DrawFiftyMove, _) => PlayerResult::Draw,
        (GameResult::Ongoing, _) => PlayerResult::Ongoing,
    }
}

fn first_player_move(game: &Game, player_color: Color) -> Option<String> {
    let index = match player_color {
        Color::White => 0,
        Color::Black => 1,
    };
    game.records().get(index).map(|record| record.uci.clone())
}

fn early_sequence(game: &Game, max_plies: usize) -> String {
    game.records()
        .iter()
        .take(max_plies)
        .map(|record| record.uci.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_learns_favorite_first_move() {
        let mut memory = PlayerMemory::new("tester");
        for _ in 0..3 {
            let mut game = Game::new().unwrap();
            game.play_uci("e2e4").unwrap();
            memory.learn_from_game(&game, Color::White);
        }

        assert_eq!(memory.favorite_first_move(), Some(("e2e4", 3)));
        assert!(
            memory
                .notes()
                .iter()
                .any(|line| line.contains("adaptive hint"))
        );
    }
}
