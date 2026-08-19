use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub user_id: String,
    pub username: String,
    pub rating: i32,
    pub queued_at: u64, // epoch millis
    pub time_control: String,
}

pub type MatchmakingQueue = Arc<RwLock<VecDeque<QueueEntry>>>;

#[derive(Debug, Clone, Serialize)]
pub struct MatchResult {
    pub game_id: String,
    pub white: String, // username
    pub black: String,
    pub white_user_id: String,
    pub black_user_id: String,
    pub time_control: String,
}

pub fn new_queue() -> MatchmakingQueue {
    Arc::new(RwLock::new(VecDeque::new()))
}

pub fn join_queue(queue: &MatchmakingQueue, entry: QueueEntry) -> Option<MatchResult> {
    let mut q = queue.write().ok()?;
    
    // Prevent duplicate entries
    if q.iter().any(|e| e.user_id == entry.user_id) {
        return None;
    }
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Evict stale entries (> 60 seconds)
    q.retain(|e| now.saturating_sub(e.queued_at) <= 60_000);

    let wait_seconds = now.saturating_sub(entry.queued_at) / 1000;
    let window = (100 + (wait_seconds / 2) * 25).min(400) as i32;

    let mut match_idx = None;
    for (i, other) in q.iter().enumerate() {
        if other.time_control == entry.time_control && (other.rating - entry.rating).abs() <= window {
            match_idx = Some(i);
            break;
        }
    }

    if let Some(idx) = match_idx {
        let other = q.remove(idx).unwrap();
        // We have a match!
        let choose_entry_white = if entry.rating > other.rating {
            now % 2 == 0
        } else if other.rating > entry.rating {
            now % 2 != 0
        } else {
            now % 2 == 0
        };

        let (white_username, white_user_id, black_username, black_user_id) = if choose_entry_white {
            (entry.username.clone(), entry.user_id.clone(), other.username.clone(), other.user_id.clone())
        } else {
            (other.username.clone(), other.user_id.clone(), entry.username.clone(), entry.user_id.clone())
        };

        let game_id = format!("live_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis());

        Some(MatchResult {
            game_id,
            white: white_username,
            black: black_username,
            white_user_id,
            black_user_id,
            time_control: entry.time_control.clone(),
        })
    } else {
        q.push_back(entry);
        None
    }
}

pub fn leave_queue(queue: &MatchmakingQueue, user_id: &str) {
    if let Ok(mut q) = queue.write() {
        q.retain(|e| e.user_id != user_id);
    }
}

pub fn queue_status(queue: &MatchmakingQueue) -> Vec<QueueEntry> {
    if let Ok(q) = queue.read() {
        q.iter().cloned().collect()
    } else {
        Vec::new()
    }
}
