use crate::game::Game;
use crate::memory::PlayerMemory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingGameRow {
    pub game_index: usize,
    pub result: String,
    pub plies: usize,
    pub final_fen: String,
    pub moves: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingReport {
    pub games: Vec<TrainingGameRow>,
    pub memory_lines: Vec<String>,
}

impl TrainingReport {
    pub fn as_lines(&self) -> Vec<String> {
        let mut lines = vec!["Axiorynth training report".to_string()];
        lines.push(format!("games exported: {}", self.games.len()));
        lines.push("memory summary:".to_string());
        lines.extend(self.memory_lines.clone());
        lines.push("game rows:".to_string());
        lines.extend(self.csv_lines());
        lines
    }

    pub fn csv_lines(&self) -> Vec<String> {
        let mut lines = vec!["game_index,result,plies,final_fen,moves".to_string()];
        for row in &self.games {
            lines.push(format!(
                "{},{},{},{},{}",
                row.game_index,
                escape_csv(&row.result),
                row.plies,
                escape_csv(&row.final_fen),
                escape_csv(&row.moves)
            ));
        }
        lines
    }
}

pub fn build_training_report(games: &[Game], memory: &PlayerMemory) -> TrainingReport {
    let rows = games
        .iter()
        .enumerate()
        .map(|(idx, game)| TrainingGameRow {
            game_index: idx + 1,
            result: game.result().as_str().to_string(),
            plies: game.records().len(),
            final_fen: game.board().to_fen(),
            moves: game.uci_moves().join(" "),
        })
        .collect();

    TrainingReport {
        games: rows,
        memory_lines: memory.as_lines(),
    }
}

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains(' ') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::memory::PlayerMemory;
    use crate::types::Color;

    #[test]
    fn training_report_exports_games_and_memory() {
        let mut game = Game::new().unwrap();
        game.play_uci("e2e4").unwrap();
        game.play_uci("e7e5").unwrap();
        let mut memory = PlayerMemory::new("tester");
        memory.learn_from_game(&game, Color::White);

        let report = build_training_report(&[game], &memory);
        assert_eq!(report.games.len(), 1);
        assert!(report.csv_lines()[1].contains("e2e4 e7e5"));
        assert!(
            report
                .as_lines()
                .iter()
                .any(|line| line.contains("memory summary"))
        );
    }
}
