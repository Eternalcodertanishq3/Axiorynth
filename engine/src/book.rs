use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::board::Board;
use crate::types::Color;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookEntry {
    pub uci_move: String,
    pub weight: u32, // how many times this was played
    pub score: f64,  // win rate when this was played
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpeningBook {
    /// Maps position hash (as hex string) -> list of book moves
    entries: HashMap<String, Vec<BookEntry>>,
}

impl OpeningBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn probe(&self, hash: u64) -> Option<&[BookEntry]> {
        let key = format!("{:x}", hash);
        self.entries.get(&key).map(|v| v.as_slice())
    }

    pub fn best_move(&self, hash: u64) -> Option<&BookEntry> {
        let entries = self.probe(hash)?;
        entries.iter().max_by_key(|e| e.weight)
    }

    pub fn add_position(&mut self, hash: u64, uci_move: &str, won: bool) {
        let key = format!("{:x}", hash);
        let entry_list = self.entries.entry(key).or_default();
        let outcome = if won { 1.0 } else { 0.0 };
        if let Some(existing) = entry_list.iter_mut().find(|e| e.uci_move == uci_move) {
            let old_weight = existing.weight as f64;
            existing.weight += 1;
            existing.score = (existing.score * old_weight + outcome) / (existing.weight as f64);
        } else {
            entry_list.push(BookEntry {
                uci_move: uci_move.to_string(),
                weight: 1,
                score: outcome,
            });
        }
    }

    pub fn generate_from_games(games: &[(Vec<String>, &str)]) -> Self {
        let mut temp: HashMap<String, HashMap<String, (u32, f64)>> = HashMap::new();
        
        for (moves, result_str) in games {
            let mut board = Board::startpos().expect("valid startpos");
            
            // Parse winner
            let winner = match *result_str {
                "1-0" | "white win" | "WhiteWin" => Some(Color::White),
                "0-1" | "black win" | "BlackWin" => Some(Color::Black),
                _ => None, // Draw or other
            };
            
            let plies_to_record = moves.len().min(12);
            for i in 0..plies_to_record {
                let hash = board.hash();
                let uci_move = &moves[i];
                let side_who_played = board.side_to_move();
                
                // Determine outcome
                let outcome = if let Some(w) = winner {
                    if w == side_who_played {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    0.5
                };
                
                let key = format!("{:x}", hash);
                let moves_map = temp.entry(key).or_default();
                let entry = moves_map.entry(uci_move.clone()).or_insert((0, 0.0));
                entry.0 += 1;
                entry.1 += outcome;
                
                // Play move to advance board state
                if let Some(mv) = crate::movegen::find_legal_move_by_uci(&mut board, uci_move) {
                    board.make_move(mv);
                } else {
                    break;
                }
            }
        }
        
        let mut entries = HashMap::new();
        for (hash_str, moves_map) in temp {
            let mut book_entries = Vec::new();
            for (uci_move, (weight, total_score)) in moves_map {
                let score = total_score / weight as f64;
                book_entries.push(BookEntry {
                    uci_move,
                    weight,
                    score,
                });
            }
            entries.insert(hash_str, book_entries);
        }
        
        OpeningBook { entries }
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    pub fn load(path: &str) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let book = serde_json::from_reader(file)?;
        Ok(book)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_basics() {
        let mut book = OpeningBook::new();
        assert_eq!(book.len(), 0);
        assert!(book.probe(12345).is_none());
        assert!(book.best_move(12345).is_none());

        book.add_position(12345, "e2e4", true);
        assert_eq!(book.len(), 1);
        
        let entries = book.probe(12345).expect("should find entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uci_move, "e2e4");
        assert_eq!(entries[0].weight, 1);
        assert_eq!(entries[0].score, 1.0);

        book.add_position(12345, "e2e4", false);
        let entries = book.probe(12345).expect("should find entries");
        assert_eq!(entries[0].weight, 2);
        assert_eq!(entries[0].score, 0.5); // (1.0 + 0.0) / 2
        
        let best = book.best_move(12345).expect("should find best move");
        assert_eq!(best.uci_move, "e2e4");
    }

    #[test]
    fn test_generate_from_games() {
        // e2e4 e7e5
        let game1 = (vec!["e2e4".to_string(), "e7e5".to_string()], "1-0"); // white wins
        let game2 = (vec!["e2e4".to_string(), "e7e5".to_string()], "1/2-1/2"); // draw
        let games = vec![game1, game2];

        let book = OpeningBook::generate_from_games(&games);
        // Start position hash
        let start_board = Board::startpos().unwrap();
        let start_hash = start_board.hash();

        let start_entries = book.probe(start_hash).expect("startpos entries");
        assert_eq!(start_entries.len(), 1);
        assert_eq!(start_entries[0].uci_move, "e2e4");
        assert_eq!(start_entries[0].weight, 2);
        // Game 1: winner is White (side who played e2e4). outcome = 1.0
        // Game 2: winner is None. outcome = 0.5
        // Average score = (1.0 + 0.5) / 2 = 0.75
        assert_eq!(start_entries[0].score, 0.75);

        // Next position: after e2e4
        let mut board_after_e4 = start_board;
        let mv = crate::movegen::find_legal_move_by_uci(&mut board_after_e4, "e2e4").unwrap();
        board_after_e4.make_move(mv);
        let hash_after_e4 = board_after_e4.hash();

        let entries_after_e4 = book.probe(hash_after_e4).expect("after e4 entries");
        assert_eq!(entries_after_e4.len(), 1);
        assert_eq!(entries_after_e4[0].uci_move, "e7e5");
        assert_eq!(entries_after_e4[0].weight, 2);
        // Game 1: side who played e7e5 is Black. winner is White. outcome = 0.0
        // Game 2: winner is None. outcome = 0.5
        // Average score = (0.0 + 0.5) / 2 = 0.25
        assert_eq!(entries_after_e4[0].score, 0.25);
    }

    #[test]
    fn test_save_load() {
        let mut book = OpeningBook::new();
        book.add_position(42, "d2d4", true);
        
        let path = "test_book.json";
        book.save(path).unwrap();
        
        let loaded = OpeningBook::load(path).unwrap();
        std::fs::remove_file(path).ok();

        assert_eq!(loaded.len(), 1);
        let entries = loaded.probe(42).unwrap();
        assert_eq!(entries[0].uci_move, "d2d4");
        assert_eq!(entries[0].weight, 1);
        assert_eq!(entries[0].score, 1.0);
    }
}

