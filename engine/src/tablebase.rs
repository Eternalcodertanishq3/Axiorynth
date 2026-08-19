use std::sync::{OnceLock, RwLock};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::board::Board;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WdlResult {
    Win,
    BlessedWin,
    Draw,
    CursedLoss,
    Loss,
}

impl WdlResult {
    pub fn from_i32(val: i32) -> Self {
        match val {
            2 => WdlResult::Win,
            1 => WdlResult::BlessedWin,
            0 => WdlResult::Draw,
            -1 => WdlResult::CursedLoss,
            _ => WdlResult::Loss,
        }
    }

    pub fn to_score(self, ply: i32) -> i32 {
        // Map tablebase outcome to search scores.
        // We use scores slightly less than MATE_SCORE to prefer quicker mates
        // and allow search to differentiate between them.
        match self {
            WdlResult::Win => 20000 - ply,
            WdlResult::BlessedWin => 15000 - ply,
            WdlResult::Draw => 0,
            WdlResult::CursedLoss => -15000 + ply,
            WdlResult::Loss => -20000 + ply,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SyzygyResponse {
    wdl: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
struct SyzygyMove {
    uci: String,
    wdl: i32,
    dtz: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct SyzygyRootResponse {
    wdl: i32,
    dtz: i32,
    moves: Vec<SyzygyMove>,
}

static TB_CACHE: OnceLock<RwLock<HashMap<u64, Option<WdlResult>>>> = OnceLock::new();

fn get_cache() -> &'static RwLock<HashMap<u64, Option<WdlResult>>> {
    TB_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Helper to count the total pieces on the board.
pub fn total_pieces(board: &Board) -> usize {
    let mut count = 0;
    for &color in &[crate::types::Color::White, crate::types::Color::Black] {
        for &kind in &crate::types::PieceKind::ALL {
            count += board.piece_count(color, kind);
        }
    }
    count as usize
}

/// Probes the Lichess Syzygy HTTP API for positions with 7 or fewer pieces.
/// Caches results in memory.
pub fn probe_tablebase(board: &Board) -> Option<WdlResult> {
    let piece_count = total_pieces(board);
    if piece_count > 7 {
        return None;
    }

    let hash = board.hash();
    
    // Check cache first
    {
        let cache = get_cache().read().unwrap();
        if let Some(&res) = cache.get(&hash) {
            return res;
        }
    }

    if std::env::var("AXIORYNTH_DISABLE_ONLINE_TB").is_ok() {
        return None;
    }

    // Not in cache, query HTTP
    let fen = board.to_fen();
    let encoded_fen = fen.replace(' ', "%20");
    let url = format!("https://tablebase.lichess.ovh/standard?fen={}", encoded_fen);

    // Make the request with a fast timeout
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_millis(250))
        .build();

    let result = match agent.get(&url).call() {
        Ok(response) => {
            if let Ok(data) = response.into_json::<SyzygyResponse>() {
                data.wdl.map(WdlResult::from_i32)
            } else {
                None
            }
        }
        Err(_) => None, // Timeout, offline, or Lichess API error
    };

    // Cache the result
    {
        let mut cache = get_cache().write().unwrap();
        cache.insert(hash, result);
    }

    result
}

/// Probes the Lichess Syzygy HTTP API at the root.
/// Returns (best_move_uci, score_in_centipawns) if successful.
pub fn probe_root_tablebase(board: &Board) -> Option<(String, i32)> {
    if std::env::var("AXIORYNTH_DISABLE_ONLINE_TB").is_ok() {
        return None;
    }

    let piece_count = total_pieces(board);
    if piece_count > 7 {
        return None;
    }

    let fen = board.to_fen();
    let encoded_fen = fen.replace(' ', "%20");
    let url = format!("https://tablebase.lichess.ovh/standard?fen={}", encoded_fen);

    // Make the request with a fast timeout
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_millis(350))
        .build();

    let response = agent.get(&url).call().ok()?;
    let data: SyzygyRootResponse = response.into_json().ok()?;

    if data.moves.is_empty() {
        return None;
    }

    let mut best_move: Option<&SyzygyMove> = None;

    for mv in &data.moves {
        if let Some(best) = best_move {
            // Compare WDL (lower opponent WDL is better for us)
            if mv.wdl < best.wdl {
                best_move = Some(mv);
            } else if mv.wdl == best.wdl {
                // DTZ tiebreaker
                if mv.wdl < 0 {
                    // We are winning (opponent is losing, so opponent WDL is negative).
                    // We want to minimize plies to zero, meaning we want the absolute DTZ to be smaller.
                    if mv.dtz.abs() < best.dtz.abs() {
                        best_move = Some(mv);
                    }
                } else if mv.wdl > 0 {
                    // We are losing (opponent is winning, so opponent WDL is positive).
                    // We want to maximize plies to zero to delay the loss.
                    if mv.dtz.abs() > best.dtz.abs() {
                        best_move = Some(mv);
                    }
                }
            }
        } else {
            best_move = Some(mv);
        }
    }

    let best = best_move?;
    let wdl_res = WdlResult::from_i32(data.wdl);
    let score = wdl_res.to_score(0);

    Some((best.uci.clone(), score))
}
