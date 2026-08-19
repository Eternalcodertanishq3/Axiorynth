use crate::board::Board;
use crate::movegen::generate_legal_moves;
use crate::mv::Move;
use crate::search::{SearchControl, SearchLimits, SearchResult, iterative_deepening};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BotLevel(u8);

impl BotLevel {
    pub fn new(level: u8) -> BotLevel {
        BotLevel(level.clamp(1, 10))
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotProfile {
    pub level: BotLevel,
    pub name: &'static str,
    pub description: &'static str,
    pub limits: SearchLimits,
    pub candidate_pick: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotMove {
    pub profile: BotProfile,
    pub selected_move: Option<Move>,
    pub search: SearchResult,
}

impl BotMove {
    pub fn as_lines(&self) -> Vec<String> {
        let selected = self
            .selected_move
            .map(|mv| mv.uci())
            .unwrap_or_else(|| "(none)".to_string());
        let mut lines = vec![
            format!("bot level: {}", self.profile.level.value()),
            format!("bot name: {}", self.profile.name),
            format!("selected move: {selected}"),
            format!("description: {}", self.profile.description),
        ];
        lines.extend(self.search.as_math_lines());
        lines
    }
}

pub fn profile_for_level(level: BotLevel) -> BotProfile {
    match level.value() {
        1 => profile(
            level,
            "Axiorynth Seed",
            "shallow and intentionally forgiving",
            1,
            0,
            5,
            4,
        ),
        2 => profile(
            level,
            "Axiorynth Spark",
            "basic tactics with light mistakes",
            1,
            1,
            5,
            3,
        ),
        3 => profile(
            level,
            "Axiorynth Pulse",
            "sees simple one-move tactics",
            2,
            1,
            5,
            2,
        ),
        4 => profile(
            level,
            "Axiorynth Lens",
            "more stable two-ply play",
            2,
            2,
            5,
            1,
        ),
        5 => profile(
            level,
            "Axiorynth Forge",
            "balanced beginner engine",
            3,
            3,
            5,
            0,
        ),
        6 => profile(
            level,
            "Axiorynth Vector",
            "deeper and less tactical blindness",
            3,
            4,
            6,
            0,
        ),
        7 => profile(
            level,
            "Axiorynth Nexus",
            "stronger search and candidates",
            4,
            4,
            6,
            0,
        ),
        8 => profile(
            level,
            "Axiorynth Oracle",
            "deeper search with larger hash",
            5,
            5,
            7,
            0,
        ),
        9 => profile(
            level,
            "Axiorynth Apex",
            "advanced local engine setting",
            6,
            6,
            8,
            0,
        ),
        _ => profile(
            level,
            "Axiorynth Zenith",
            "full current strength",
            7,
            6,
            10,
            0,
        ),
    }
}

pub fn all_bot_profiles() -> Vec<BotProfile> {
    (1..=10)
        .map(|level| profile_for_level(BotLevel::new(level)))
        .collect()
}

pub fn choose_bot_move(
    board: &Board,
    level: BotLevel,
    control: &SearchControl,
    history_context: &str,
    penalties: &std::collections::HashMap<String, i32>,
) -> BotMove {
    let profile = profile_for_level(level);
    let mut search_board = board.clone();
    let search = iterative_deepening(&mut search_board, profile.limits, control);
    let selected_move = pick_candidate(board, &search, profile.candidate_pick, history_context, penalties);

    BotMove {
        profile,
        selected_move,
        search,
    }
}

pub fn choose_bot_move_with_callback<F>(
    board: &Board,
    level: BotLevel,
    control: &SearchControl,
    history_context: &str,
    penalties: &std::collections::HashMap<String, i32>,
    on_depth: F,
) -> BotMove
where
    F: FnMut(&SearchResult, std::time::Duration),
{
    let profile = profile_for_level(level);
    let mut search_board = board.clone();
    let search = crate::search::iterative_deepening_with_callback(&mut search_board, profile.limits, control, on_depth);
    let selected_move = pick_candidate(board, &search, profile.candidate_pick, history_context, penalties);

    BotMove {
        profile,
        selected_move,
        search,
    }
}

fn profile(
    level: BotLevel,
    name: &'static str,
    description: &'static str,
    depth: u8,
    qdepth: u8,
    candidates: usize,
    candidate_pick: usize,
) -> BotProfile {
    let move_time = match level.value() {
        10 => Some(std::time::Duration::from_millis(1500)),
        9 => Some(std::time::Duration::from_millis(1200)),
        8 => Some(std::time::Duration::from_millis(1000)),
        7 => Some(std::time::Duration::from_millis(800)),
        6 => Some(std::time::Duration::from_millis(600)),
        5 => Some(std::time::Duration::from_millis(450)),
        4 => Some(std::time::Duration::from_millis(300)),
        3 => Some(std::time::Duration::from_millis(200)),
        2 => Some(std::time::Duration::from_millis(150)),
        _ => Some(std::time::Duration::from_millis(100)),
    };

    BotProfile {
        level,
        name,
        description,
        limits: SearchLimits {
            max_depth: depth,
            quiescence_depth: qdepth,
            candidate_count: candidates.min(5),
            hash_size_mb: if level.value() >= 8 { 32 } else { 8 },
            move_time,
            ..SearchLimits::default()
        },
        candidate_pick,
    }
}

fn pick_candidate(
    board: &Board,
    search: &SearchResult,
    candidate_pick: usize,
    history_context: &str,
    penalties: &std::collections::HashMap<String, i32>,
) -> Option<Move> {
    let legal = {
        let mut clone = board.clone();
        generate_legal_moves(&mut clone)
    };

    let mut candidates = search.candidates.clone();
    for candidate in &mut candidates {
        let key = if history_context.is_empty() {
            candidate.mv.uci()
        } else {
            format!("{}_{}", history_context, candidate.mv.uci())
        };
        
        if let Some(&penalty_count) = penalties.get(&key) {
            candidate.score -= penalty_count * 300; // Penalize 3 pawns per recorded failure
        }
    }
    
    candidates.sort_by(|a, b| b.score.cmp(&a.score));

    if candidates.is_empty() {
        return search.best_move.filter(|mv| legal.contains(mv));
    }

    let best_score = candidates[0].score;
    let selected_idx = if candidate_pick == 0 || best_score >= 10_000 {
        0
    } else {
        candidate_pick.min(candidates.len() - 1)
    };

    let candidate = candidates
        .get(selected_idx)
        .map(|candidate| candidate.mv)
        .or(search.best_move);

    candidate.filter(|mv| legal.contains(mv))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_ten_bot_profiles() {
        let profiles = all_bot_profiles();
        assert_eq!(profiles.len(), 10);
        assert_eq!(profiles[0].level.value(), 1);
        assert_eq!(profiles[9].level.value(), 10);
    }

    #[test]
    fn bot_level_selects_legal_move() {
        let board = Board::startpos().unwrap();
        let control = SearchControl::new();
        let bot_move = choose_bot_move(&board, BotLevel::new(3), &control, "", &std::collections::HashMap::new());
        let selected = bot_move.selected_move.unwrap();
        let mut clone = board.clone();
        assert!(generate_legal_moves(&mut clone).contains(&selected));
    }
}
