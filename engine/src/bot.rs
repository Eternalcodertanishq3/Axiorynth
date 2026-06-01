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

pub fn choose_bot_move(board: &Board, level: BotLevel, control: &SearchControl) -> BotMove {
    let profile = profile_for_level(level);
    let mut search_board = board.clone();
    let search = iterative_deepening(&mut search_board, profile.limits, control);
    let selected_move = pick_candidate(board, &search, profile.candidate_pick);

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
    BotProfile {
        level,
        name,
        description,
        limits: SearchLimits {
            max_depth: depth,
            quiescence_depth: qdepth,
            candidate_count: candidates,
            hash_size_mb: if level.value() >= 8 { 32 } else { 8 },
            ..SearchLimits::default()
        },
        candidate_pick,
    }
}

fn pick_candidate(board: &Board, search: &SearchResult, candidate_pick: usize) -> Option<Move> {
    let legal = {
        let mut clone = board.clone();
        generate_legal_moves(&mut clone)
    };

    let candidate = search
        .candidates
        .get(candidate_pick)
        .or_else(|| search.candidates.last())
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
        let bot_move = choose_bot_move(&board, BotLevel::new(3), &control);
        let selected = bot_move.selected_move.unwrap();
        let mut clone = board.clone();
        assert!(generate_legal_moves(&mut clone).contains(&selected));
    }
}
