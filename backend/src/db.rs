use sqlx::SqlitePool;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RatingRow {
    pub user_id: String,
    pub category: String,
    pub rating: f64,
    pub rd: f64,
    pub volatility: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlayerProfile {
    pub id: String,
    pub name: String,
    pub wins: i32,
    pub losses: i32,
    pub draws: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SavedGame {
    pub id: String,
    pub saved_at: String,
    pub moves: String, // JSON array of move UCI strings
    pub result: String,
    pub mode: String,
    pub bot_level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BotMemory {
    pub player_id: String,
    pub opening_tendencies: String, // JSON string
    pub mistake_clusters: String,   // JSON string
    pub bot_adjustments: String,    // JSON string
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameSession {
    pub id: String,
    pub fen: String,
    pub moves: String,     // JSON array of UCI move strings
    pub mode: String,      // "bot" or "self"
    pub bot_level: i32,
    pub result: String,    // "ongoing", "white_win", "black_win", "draw_stalemate", etc.
    pub created_at: String,
    pub updated_at: String,
}

pub async fn init_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePool::connect(database_url).await?;

    sqlx::query("PRAGMA journal_mode=WAL;").execute(&pool).await?;
    sqlx::query("PRAGMA synchronous=NORMAL;").execute(&pool).await?;
    sqlx::query("PRAGMA busy_timeout=5000;").execute(&pool).await?;
    sqlx::query("PRAGMA foreign_keys=ON;").execute(&pool).await?;

    // Create tables
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS player_profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            wins INTEGER NOT NULL DEFAULT 0,
            losses INTEGER NOT NULL DEFAULT 0,
            draws INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS saved_games (
            id TEXT PRIMARY KEY,
            saved_at TEXT NOT NULL,
            moves TEXT NOT NULL,
            result TEXT NOT NULL,
            mode TEXT NOT NULL,
            bot_level INTEGER NOT NULL
        );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS bot_memory (
            player_id TEXT PRIMARY KEY,
            opening_tendencies TEXT NOT NULL,
            mistake_clusters TEXT NOT NULL,
            bot_adjustments TEXT NOT NULL
        );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS game_sessions (
            id TEXT PRIMARY KEY,
            fen TEXT NOT NULL,
            moves TEXT NOT NULL DEFAULT '[]',
            mode TEXT NOT NULL DEFAULT 'bot',
            bot_level INTEGER NOT NULL DEFAULT 3,
            result TEXT NOT NULL DEFAULT 'ongoing',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            rating INTEGER NOT NULL DEFAULT 1200,
            created_at TEXT NOT NULL
        );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS live_games (
            id TEXT PRIMARY KEY,
            white_user_id TEXT NOT NULL,
            black_user_id TEXT NOT NULL,
            fen TEXT NOT NULL,
            moves TEXT NOT NULL DEFAULT '[]',
            result TEXT NOT NULL DEFAULT 'ongoing',
            time_control TEXT NOT NULL DEFAULT 'unlimited',
            created_at TEXT NOT NULL
        );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ratings (
            user_id TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'blitz',
            rating REAL NOT NULL DEFAULT 1500.0,
            rd REAL NOT NULL DEFAULT 350.0,
            volatility REAL NOT NULL DEFAULT 0.06,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (user_id, category)
        );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS moves (
            game_id TEXT NOT NULL,
            ply INTEGER NOT NULL,
            uci TEXT NOT NULL,
            fen_after TEXT NOT NULL,
            clock_ms INTEGER,
            eval_cp INTEGER,
            PRIMARY KEY (game_id, ply)
        );"
    ).execute(&pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);").execute(&pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_live_games_result ON live_games(result, created_at);").execute(&pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_saved_games_saved_at ON saved_games(saved_at DESC);").execute(&pool).await?;

    // Prepopulate a default player if empty
    let count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM player_profiles")
        .fetch_one(&pool)
        .await?;

    if count == 0 {
        sqlx::query(
            "INSERT INTO player_profiles (id, name, wins, losses, draws, created_at)
             VALUES ('default', 'Player 1', 0, 0, 0, CURRENT_TIMESTAMP);"
        ).execute(&pool).await?;

        sqlx::query(
            "INSERT INTO bot_memory (player_id, opening_tendencies, mistake_clusters, bot_adjustments)
             VALUES ('default', '{}', '[]', '{}');"
        ).execute(&pool).await?;
    }

    Ok(pool)
}

// ---------- Player Profiles ----------

pub async fn get_profile(pool: &SqlitePool, id: &str) -> Result<Option<PlayerProfile>, sqlx::Error> {
    sqlx::query_as::<_, PlayerProfile>(
        "SELECT id, name, wins, losses, draws, created_at FROM player_profiles WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn update_profile_stats(
    pool: &SqlitePool,
    id: &str,
    outcome: &str,
) -> Result<(), sqlx::Error> {
    match outcome {
        "wins" => {
            sqlx::query("UPDATE player_profiles SET wins = wins + 1 WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?;
        }
        "losses" => {
            sqlx::query("UPDATE player_profiles SET losses = losses + 1 WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?;
        }
        "draws" => {
            sqlx::query("UPDATE player_profiles SET draws = draws + 1 WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?;
        }
        _ => {}
    }
    Ok(())
}

// ---------- Saved Games ----------

pub async fn save_game(pool: &SqlitePool, game: &SavedGame) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO saved_games (id, saved_at, moves, result, mode, bot_level)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&game.id)
    .bind(&game.saved_at)
    .bind(&game.moves)
    .bind(&game.result)
    .bind(&game.mode)
    .bind(game.bot_level)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_games(pool: &SqlitePool, offset: i64) -> Result<Vec<SavedGame>, sqlx::Error> {
    sqlx::query_as::<_, SavedGame>(
        "SELECT id, saved_at, moves, result, mode, bot_level FROM saved_games ORDER BY saved_at DESC LIMIT 50 OFFSET ?"
    )
    .bind(offset)
    .fetch_all(pool)
    .await
}

// ---------- Bot Memory ----------

pub async fn get_bot_memory(pool: &SqlitePool, player_id: &str) -> Result<Option<BotMemory>, sqlx::Error> {
    sqlx::query_as::<_, BotMemory>(
        "SELECT player_id, opening_tendencies, mistake_clusters, bot_adjustments FROM bot_memory WHERE player_id = ?"
    )
    .bind(player_id)
    .fetch_optional(pool)
    .await
}

pub async fn save_bot_memory(pool: &SqlitePool, memory: &BotMemory) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO bot_memory (player_id, opening_tendencies, mistake_clusters, bot_adjustments)
         VALUES (?, ?, ?, ?)"
    )
    .bind(&memory.player_id)
    .bind(&memory.opening_tendencies)
    .bind(&memory.mistake_clusters)
    .bind(&memory.bot_adjustments)
    .execute(pool)
    .await?;
    Ok(())
}

/// Merge new opening moves into the existing bot_memory opening_tendencies.
/// `opening_moves` is a list of the first N UCI moves from a completed game.
pub async fn record_opening_tendencies(
    pool: &SqlitePool,
    player_id: &str,
    opening_moves: &[String],
) -> Result<(), sqlx::Error> {
    if opening_moves.is_empty() {
        return Ok(());
    }

    let existing = get_bot_memory(pool, player_id).await?;
    let mut tendencies: serde_json::Map<String, serde_json::Value> = match &existing {
        Some(mem) => serde_json::from_str(&mem.opening_tendencies).unwrap_or_default(),
        None => serde_json::Map::new(),
    };

    // Record the first move and opening sequence
    if let Some(first) = opening_moves.first() {
        let count = tendencies.get(first)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        tendencies.insert(first.clone(), serde_json::Value::from(count + 1));
    }

    // Also record the first 2-move and 3-move sequences as composite keys
    if opening_moves.len() >= 2 {
        let key = format!("{}_{}", opening_moves[0], opening_moves[1]);
        let count = tendencies.get(&key).and_then(|v| v.as_i64()).unwrap_or(0);
        tendencies.insert(key, serde_json::Value::from(count + 1));
    }
    if opening_moves.len() >= 3 {
        let key = format!("{}_{}_{}", opening_moves[0], opening_moves[1], opening_moves[2]);
        let count = tendencies.get(&key).and_then(|v| v.as_i64()).unwrap_or(0);
        tendencies.insert(key, serde_json::Value::from(count + 1));
    }

    let tendencies_json = serde_json::to_string(&tendencies).unwrap_or_else(|_| "{}".to_string());

    let memory = BotMemory {
        player_id: player_id.to_string(),
        opening_tendencies: tendencies_json,
        mistake_clusters: existing.as_ref().map_or("[]".to_string(), |m| m.mistake_clusters.clone()),
        bot_adjustments: existing.as_ref().map_or("{}".to_string(), |m| m.bot_adjustments.clone()),
    };

    save_bot_memory(pool, &memory).await
}

pub async fn record_bot_loss_penalties(
    pool: &SqlitePool,
    player_id: &str,
    moves: &[String],
    bot_played_white: bool,
) -> Result<(), sqlx::Error> {
    if moves.is_empty() {
        return Ok(());
    }

    let existing = get_bot_memory(pool, player_id).await?;
    let mut adjustments: serde_json::Map<String, serde_json::Value> = match &existing {
        Some(mem) => serde_json::from_str(&mem.bot_adjustments).unwrap_or_default(),
        None => serde_json::Map::new(),
    };

    let bot_modulo = if bot_played_white { 0 } else { 1 };
    
    let mut context = String::new();
    for (i, mv) in moves.iter().enumerate() {
        if i % 2 == bot_modulo {
            let key = if context.is_empty() {
                mv.clone()
            } else {
                format!("{}_{}", context, mv)
            };
            
            let penalty = adjustments.get(&key).and_then(|v| v.as_i64()).unwrap_or(0);
            adjustments.insert(key, serde_json::Value::from(penalty + 1));
        }
        
        if context.is_empty() {
            context = mv.clone();
        } else {
            context = format!("{}_{}", context, mv);
        }
    }

    let adjustments_json = serde_json::to_string(&adjustments).unwrap_or_else(|_| "{}".to_string());

    let memory = BotMemory {
        player_id: player_id.to_string(),
        opening_tendencies: existing.as_ref().map_or("{}".to_string(), |m| m.opening_tendencies.clone()),
        mistake_clusters: existing.as_ref().map_or("[]".to_string(), |m| m.mistake_clusters.clone()),
        bot_adjustments: adjustments_json,
    };

    save_bot_memory(pool, &memory).await
}

pub async fn record_mistakes(
    pool: &SqlitePool,
    player_id: &str,
    moves: &[String],
    player_played_white: bool,
) -> Result<(), sqlx::Error> {
    if moves.is_empty() {
        return Ok(());
    }
    
    let mut game = axiorynth_engine::game::Game::new().map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?;
    for mv in moves {
        if game.play_uci(mv).is_err() {
            return Ok(());
        }
    }
    
    let color = if player_played_white { axiorynth_engine::types::Color::White } else { axiorynth_engine::types::Color::Black };
    let mistakes = axiorynth_engine::memory::analyze_mistakes(&game, color);
    
    if mistakes.is_empty() {
        return Ok(());
    }
    
    let existing = get_bot_memory(pool, player_id).await?;
    let mut current_mistakes: Vec<axiorynth_engine::memory::MistakeCluster> = match &existing {
        Some(mem) => serde_json::from_str(&mem.mistake_clusters).unwrap_or_default(),
        None => Vec::new(),
    };
    
    current_mistakes.extend(mistakes);
    if current_mistakes.len() > 50 {
        let excess = current_mistakes.len() - 50;
        current_mistakes.drain(0..excess);
    }
    
    let mistakes_json = serde_json::to_string(&current_mistakes).unwrap_or_else(|_| "[]".to_string());
    
    let memory = BotMemory {
        player_id: player_id.to_string(),
        opening_tendencies: existing.as_ref().map_or("{}".to_string(), |m| m.opening_tendencies.clone()),
        mistake_clusters: mistakes_json,
        bot_adjustments: existing.as_ref().map_or("{}".to_string(), |m| m.bot_adjustments.clone()),
    };
    
    save_bot_memory(pool, &memory).await
}

// ---------- Game Sessions ----------

pub async fn create_session(pool: &SqlitePool, session: &GameSession) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO game_sessions (id, fen, moves, mode, bot_level, result, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&session.id)
    .bind(&session.fen)
    .bind(&session.moves)
    .bind(&session.mode)
    .bind(session.bot_level)
    .bind(&session.result)
    .bind(&session.created_at)
    .bind(&session.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_session(pool: &SqlitePool, id: &str) -> Result<Option<GameSession>, sqlx::Error> {
    sqlx::query_as::<_, GameSession>(
        "SELECT id, fen, moves, mode, bot_level, result, created_at, updated_at
         FROM game_sessions WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn update_session(
    pool: &SqlitePool,
    id: &str,
    fen: &str,
    moves: &str,
    result: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE game_sessions SET fen = ?, moves = ?, result = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(fen)
    .bind(moves)
    .bind(result)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_session(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM game_sessions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn list_sessions(pool: &SqlitePool) -> Result<Vec<GameSession>, sqlx::Error> {
    sqlx::query_as::<_, GameSession>(
        "SELECT id, fen, moves, mode, bot_level, result, created_at, updated_at
         FROM game_sessions ORDER BY updated_at DESC"
    )
    .fetch_all(pool)
    .await
}

// ---------- Users ----------

pub async fn create_user(pool: &SqlitePool, user: &crate::auth::User) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, rating, created_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&user.id)
    .bind(&user.username)
    .bind(&user.password_hash)
    .bind(user.rating)
    .bind(&user.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_user_by_username(pool: &SqlitePool, username: &str) -> Result<Option<crate::auth::User>, sqlx::Error> {
    sqlx::query_as::<_, crate::auth::User>(
        "SELECT id, username, password_hash, rating, created_at FROM users WHERE username = ?"
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_id(pool: &SqlitePool, id: &str) -> Result<Option<crate::auth::User>, sqlx::Error> {
    sqlx::query_as::<_, crate::auth::User>(
        "SELECT id, username, password_hash, rating, created_at FROM users WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn update_user_rating(pool: &SqlitePool, user_id: &str, new_rating: i32) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET rating = ? WHERE id = ?")
        .bind(new_rating)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------- Live Games ----------

pub async fn create_live_game(
    pool: &SqlitePool,
    id: &str,
    white_user_id: &str,
    black_user_id: &str,
    time_control: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO live_games (id, white_user_id, black_user_id, fen, moves, result, time_control, created_at)
         VALUES (?, ?, ?, ?, '[]', 'ongoing', ?, datetime('now'))"
    )
    .bind(id)
    .bind(white_user_id)
    .bind(black_user_id)
    .bind(axiorynth_engine::STARTPOS_FEN)
    .bind(time_control)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_live_games(pool: &SqlitePool, offset: i64) -> Result<Vec<LiveGameRow>, sqlx::Error> {
    sqlx::query_as::<_, LiveGameRow>(
        "SELECT id, white_user_id, black_user_id, result, time_control, created_at FROM live_games WHERE result = 'ongoing' ORDER BY created_at DESC LIMIT 50 OFFSET ?"
    )
    .bind(offset)
    .fetch_all(pool)
    .await
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct LiveGameRow {
    pub id: String,
    pub white_user_id: String,
    pub black_user_id: String,
    pub result: String,
    pub time_control: String,
    pub created_at: String,
}

pub async fn update_live_game(
    pool: &SqlitePool,
    id: &str,
    fen: &str,
    moves: &str,
    result: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE live_games SET fen = ?, moves = ?, result = ? WHERE id = ?")
        .bind(fen)
        .bind(moves)
        .bind(result)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------- Migration / Auth ----------

pub async fn update_password_hash(pool: &SqlitePool, user_id: &str, new_hash: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(new_hash)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------- Ratings ----------

pub async fn get_rating(pool: &SqlitePool, user_id: &str, category: &str) -> Result<Option<RatingRow>, sqlx::Error> {
    sqlx::query_as::<_, RatingRow>(
        "SELECT user_id, category, rating, rd, volatility, updated_at FROM ratings WHERE user_id = ? AND category = ?"
    )
    .bind(user_id)
    .bind(category)
    .fetch_optional(pool)
    .await
}

pub async fn upsert_rating(
    pool: &SqlitePool,
    user_id: &str,
    category: &str,
    rating: f64,
    rd: f64,
    volatility: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ratings (user_id, category, rating, rd, volatility, updated_at) 
         VALUES (?, ?, ?, ?, ?, datetime('now')) 
         ON CONFLICT(user_id, category) DO UPDATE SET 
            rating = excluded.rating, 
            rd = excluded.rd, 
            volatility = excluded.volatility, 
            updated_at = excluded.updated_at"
    )
    .bind(user_id)
    .bind(category)
    .bind(rating)
    .bind(rd)
    .bind(volatility)
    .execute(pool)
    .await?;
    Ok(())
}

// ---------- Moves ----------

pub async fn insert_move(
    pool: &SqlitePool,
    game_id: &str,
    ply: i32,
    uci: &str,
    fen_after: &str,
    clock_ms: Option<i64>,
    eval_cp: Option<i32>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO moves (game_id, ply, uci, fen_after, clock_ms, eval_cp) 
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(game_id)
    .bind(ply)
    .bind(uci)
    .bind(fen_after)
    .bind(clock_ms)
    .bind(eval_cp)
    .execute(pool)
    .await?;
    Ok(())
}

