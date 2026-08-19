use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    routing::{get, post},
    Json, Router,
    response::IntoResponse,
};
use serde::Deserialize;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use sqlx::SqlitePool;

mod db;
mod ws;
mod auth;
mod matchmaking;
mod live;

use db::{SavedGame, BotMemory, GameSession};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub sessions: auth::SessionStore,
    pub queue: matchmaking::MatchmakingQueue,
    pub live_store: live::LiveGameStore,
    pub book: std::sync::Arc<axiorynth_engine::OpeningBook>,
}

impl axum::extract::FromRef<AppState> for SqlitePool {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.pool.clone()
    }
}
use axiorynth_engine::{
    BotLevel, BotMove, Color, EvalBreakdown, Game, GameRecord, SearchControl, SearchLimits, SearchResult,
    choose_bot_move, evaluate, generate_legal_moves, iterative_deepening,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("axiorynth_backend=info".parse().unwrap()))
        .init();
    tracing::info!("Axiorynth backend starting...");

    // Database setup
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://axiorynth.db".to_string());
    
    // Ensure the SQLite file is created beforehand
    if db_url.starts_with("sqlite://") {
        let db_path = db_url.trim_start_matches("sqlite://");
        if !std::path::Path::new(db_path).exists() {
            if let Some(parent) = std::path::Path::new(db_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::File::create(db_path).ok();
        }
    }

    let pool = db::init_db(&db_url).await.expect("Failed to initialize database");

    // Shared state setup
    let sessions = auth::SessionStore::default();
    let queue = matchmaking::new_queue();
    let live_store = live::new_live_store();
    
    let book_path = "data/books/book.bin";
    if let Some(parent) = std::path::Path::new(book_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let book = axiorynth_engine::OpeningBook::load(book_path)
        .unwrap_or_else(|_| axiorynth_engine::OpeningBook::new());
    let book = std::sync::Arc::new(book);

    let app_state = AppState {
        pool,
        sessions,
        queue,
        live_store,
        book,
    };

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // App routes
    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .route("/api/state", post(get_frontend_state))
        .route("/api/hint", post(get_move_hints))
        .route("/api/profile", get(get_profile))
        .route("/api/profile/result", post(update_profile))
        .route("/api/games", get(list_games).post(save_game))
        .route("/api/bot/memory", get(get_bot_memory).post(save_bot_memory))
        .route("/api/training/recommendations", get(get_training_recommendations))
        // Session-based game management
        .route("/api/session", post(create_session))
        .route("/api/session/{id}", get(get_session).delete(delete_session))
        .route("/api/session/{id}/move", post(session_apply_move))
        // Phase F - Auth & Matchmaking
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/me", get(me_handler))
        .route("/api/auth/logout", post(logout_handler))
        .route("/api/matchmaking/queue", post(join_queue_handler).delete(leave_queue_handler))
        .route("/api/matchmaking/status", get(queue_status_handler))
        .route("/api/live/games", get(list_live_games_handler))
        .route("/ws/live/{game_id}", get(live::live_ws_handler))
        .route("/api/book/probe", get(book_probe_handler))
        .route("/api/tablebase/probe", get(tablebase_probe_handler))
        .route("/api/games/{id}/review", get(game_review_handler))
        .with_state(app_state)
        .layer(cors);

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("Axiorynth Axum backend listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ---------- Stateless State Endpoint (legacy compatibility) ----------

#[derive(Deserialize)]
struct StateRequest {
    moves: Vec<String>,
    #[serde(rename = "botLevel")]
    bot_level: u8,
    depth: u8,
}

async fn get_frontend_state(
    Json(payload): Json<StateRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    tokio::task::spawn_blocking(move || {
        let mut game = Game::new().map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        for mv in &payload.moves {
            game.play_uci(mv).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        }
        
        let board = game.board();
        
        let mut legal_board = board.clone();
        let mut legal_moves = generate_legal_moves(&mut legal_board)
            .into_iter()
            .map(|mv| mv.uci())
            .collect::<Vec<_>>();
        legal_moves.sort();

        let evaluation = evaluate(board);
        let control = SearchControl::new();
        let mut search_board = board.clone();
        let search_depth = payload.depth.min(4);
        let search = iterative_deepening(
            &mut search_board,
            SearchLimits {
                max_depth: search_depth,
                quiescence_depth: search_depth.min(3),
                candidate_count: 4,
                hash_size_mb: 4,
                move_time: Some(std::time::Duration::from_millis(150)),
                ..SearchLimits::default()
            },
            &control,
        );
        let bot = choose_bot_move(board, BotLevel::new(payload.bot_level.min(3)), &control, "", &std::collections::HashMap::new());

        let json_str = frontend_state_json(&game, &legal_moves, &evaluation, &search, &bot);
        
        let response = axum::response::Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(json_str))
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
        Ok(response)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
}

// ---------- Win-Possibility Hints on Piece Selection ----------

#[derive(Deserialize)]
struct HintRequest {
    moves: Vec<String>,
    square: String,
    #[serde(default = "default_hint_depth")]
    depth: u8,
    game_id: Option<String>,
}

fn default_hint_depth() -> u8 {
    4
}

#[derive(serde::Serialize)]
struct MoveHint {
    move_uci: String,
    dest: String,
    score: i32,
    win_pct: i32,
    draw_pct: i32,
    loss_pct: i32,
    reply: Option<String>,
    depth: u8,
}

#[derive(serde::Serialize)]
struct HintResponse {
    square: String,
    candidates: Vec<MoveHint>,
}

async fn get_move_hints(
    Json(payload): Json<HintRequest>,
) -> Result<Json<HintResponse>, (StatusCode, String)> {
    if let Some(ref gid) = payload.game_id {
        if gid.starts_with("live_") {
            // For now, treat all live games as rated and disable hints
            return Err((StatusCode::FORBIDDEN, "Hints are disabled during rated games".to_string()));
        }
    }

    tokio::task::spawn_blocking(move || {
        let mut game = Game::new().map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        for mv in &payload.moves {
            game.play_uci(mv).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        }

        let mut legal_board = game.board().clone();
        let legal_moves = generate_legal_moves(&mut legal_board);

        let matching_moves: Vec<_> = legal_moves
            .into_iter()
            .filter(|m| m.uci().starts_with(&payload.square))
            .collect();

        let control = SearchControl::new();
        let depth = payload.depth.clamp(2, 6);
        let mut candidates = Vec::new();

        for mv in matching_moves {
            let uci = mv.uci();
            let dest = if uci.len() >= 4 { uci[2..4].to_string() } else { "".to_string() };

            let mut after_game = game.clone();
            if after_game.play_uci(&uci).is_ok() {
                let mut search_board = after_game.board().clone();
                let search = iterative_deepening(
                    &mut search_board,
                    SearchLimits {
                        max_depth: depth,
                        quiescence_depth: depth.min(3),
                        candidate_count: 1,
                        hash_size_mb: 2,
                        ..SearchLimits::default()
                    },
                    &control,
                );

                // Note: search.score is from opponent perspective after move
                let player_score = -search.score;
                
                // Calibrated WDL conversion (Logistic model: W = 1 / (1 + 10^(-score/400)))
                let win_prob = 1.0 / (1.0 + 10.0_f64.powf(-player_score as f64 / 400.0));
                let win_pct = ((win_prob * 100.0).round() as i32).clamp(1, 99);
                let draw_pct = 15;
                let loss_pct = (100 - win_pct - draw_pct).max(0);

                let reply = search.principal_variation.first().map(|r| r.uci());

                candidates.push(MoveHint {
                    move_uci: uci,
                    dest,
                    score: player_score,
                    win_pct,
                    draw_pct,
                    loss_pct,
                    reply,
                    depth: search.depth,
                });
            }
        }

        candidates.sort_by(|a, b| b.score.cmp(&a.score));

        Ok(Json(HintResponse {
            square: payload.square,
            candidates,
        }))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
}

// ---------- Player Profiles ----------

async fn get_profile(
    State(state): State<AppState>,
) -> Result<Json<db::PlayerProfile>, (StatusCode, String)> {
    match db::get_profile(&state.pool, "default").await {
        Ok(Some(profile)) => Ok(Json(profile)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Profile not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[derive(Deserialize)]
struct ResultPayload {
    outcome: String, // "wins", "losses", "draws"
}

async fn update_profile(
    State(state): State<AppState>,
    Json(payload): Json<ResultPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    match db::update_profile_stats(&state.pool, "default", &payload.outcome).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// ---------- Saved Games ----------

async fn list_games(
    State(state): State<AppState>,
) -> Result<Json<Vec<SavedGame>>, (StatusCode, String)> {
    match db::list_games(&state.pool, 0).await {
        Ok(games) => Ok(Json(games)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn save_game(
    State(state): State<AppState>,
    Json(game): Json<SavedGame>,
) -> Result<StatusCode, (StatusCode, String)> {
    match db::save_game(&state.pool, &game).await {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// ---------- Bot Memory ----------

async fn get_bot_memory(
    State(state): State<AppState>,
) -> Result<Json<BotMemory>, (StatusCode, String)> {
    match db::get_bot_memory(&state.pool, "default").await {
        Ok(Some(mem)) => Ok(Json(mem)),
        Ok(None) => {
            // Default empty memory
            Ok(Json(BotMemory {
                player_id: "default".to_string(),
                opening_tendencies: "{}".to_string(),
                mistake_clusters: "[]".to_string(),
                bot_adjustments: "{}".to_string(),
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn save_bot_memory(
    State(state): State<AppState>,
    Json(mem): Json<BotMemory>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut clean_mem = mem;
    clean_mem.player_id = "default".to_string(); // Force default
    match db::save_bot_memory(&state.pool, &clean_mem).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// ---------- Training Recommendations ----------

#[derive(serde::Serialize)]
struct TrainingRecommendations {
    notes: Vec<String>,
    mistake_clusters: Vec<axiorynth_engine::memory::MistakeCluster>,
}

async fn get_training_recommendations(
    State(state): State<AppState>,
) -> Result<Json<TrainingRecommendations>, (StatusCode, String)> {
    let mut notes = Vec::new();
    let mut mistake_clusters = Vec::new();
    
    if let Ok(Some(profile)) = db::get_profile(&state.pool, "default").await {
        notes.push(format!("Record: {} wins, {} losses, {} draws", profile.wins, profile.losses, profile.draws));
        if profile.losses > profile.wins {
            notes.push("You are currently losing more games than you win. Consider lowering the bot level to practice tactics.".to_string());
        } else if profile.wins > profile.losses + 2 {
            notes.push("You are winning consistently! The bot will play more aggressively against you.".to_string());
        }
    }
    
    if let Ok(Some(mem)) = db::get_bot_memory(&state.pool, "default").await {
        if let Ok(tendencies) = serde_json::from_str::<std::collections::HashMap<String, i32>>(&mem.opening_tendencies) {
            let mut best_move = None;
            let mut max_count = 0;
            for (k, v) in tendencies.iter() {
                if !k.contains('_') && *v > max_count {
                    max_count = *v;
                    best_move = Some(k.clone());
                }
            }
            if let Some(mv) = best_move {
                notes.push(format!("Your favorite first move is {}, played {} times.", mv, max_count));
            }
        }
        
        if let Ok(clusters) = serde_json::from_str::<Vec<axiorynth_engine::memory::MistakeCluster>>(&mem.mistake_clusters) {
            mistake_clusters = clusters;
            if mistake_clusters.len() > 0 {
                notes.push(format!("Identified {} major blunders in your past games. Review them to avoid hanging pieces.", mistake_clusters.len()));
            }
        }
    }
    
    Ok(Json(TrainingRecommendations { notes, mistake_clusters }))
}

// ---------- Game Sessions ----------

#[derive(Deserialize)]
struct CreateSessionRequest {
    mode: String,
    #[serde(rename = "botLevel", default = "default_bot_level")]
    bot_level: u8,
}

fn default_bot_level() -> u8 { 3 }

async fn create_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<GameSession>), (StatusCode, String)> {
    let id = format!("session_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());

    let now = chrono_now();
    let session = GameSession {
        id: id.clone(),
        fen: axiorynth_engine::STARTPOS_FEN.to_string(),
        moves: "[]".to_string(),
        mode: payload.mode,
        bot_level: payload.bot_level as i32,
        result: "ongoing".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    db::create_session(&state.pool, &session).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Build the initial state response
    Ok((StatusCode::CREATED, Json(session)))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let session = db::get_session(&state.pool, &id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    // Reconstruct full engine state from session
    let state_json = build_session_state(&session)?;
    Ok(Json(state_json))
}

#[derive(Deserialize)]
struct SessionMoveRequest {
    #[serde(rename = "move")]
    uci_move: String,
}

async fn session_apply_move(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SessionMoveRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let session = db::get_session(&state.pool, &id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    if session.result != "ongoing" {
        return Err((StatusCode::BAD_REQUEST, "Game is already over".to_string()));
    }

    // Parse existing moves
    let mut session_moves: Vec<String> = serde_json::from_str(&session.moves)
        .unwrap_or_default();
    session_moves.push(payload.uci_move);

    // Replay the game to get the new state
    let mut game = Game::new().map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    for mv in &session_moves {
        game.play_uci(mv).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    }

    let new_fen = game.board().to_fen();
    let result_str = game.result().as_str().to_string();

    // Update the session in the database
    let moves_json = serde_json::to_string(&session_moves)
        .unwrap_or_else(|_| "[]".to_string());
    db::update_session(&state.pool, &id, &new_fen, &moves_json, &result_str).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // If the game just ended, record opening tendencies and potential bot penalties
    if result_str != "ongoing" {
        let player_openings: Vec<String> = session_moves.iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0) // White's moves
            .take(3)
            .map(|(_, m)| m.clone())
            .collect();
        let _ = db::record_opening_tendencies(&state.pool, "default", &player_openings).await;

        if session.mode == "bot" && result_str == "White wins" {
            // The bot plays Black in the current UI, so if White wins, the bot lost.
            let _ = db::record_bot_loss_penalties(&state.pool, "default", &session_moves, false).await;
        }

        // Record human mistakes (assuming human plays White)
        let _ = db::record_mistakes(&state.pool, "default", &session_moves, true).await;
    }

    // Build state response
    let updated_session = GameSession {
        id: id.clone(),
        fen: new_fen,
        moves: moves_json,
        mode: session.mode,
        bot_level: session.bot_level,
        result: result_str,
        created_at: session.created_at,
        updated_at: chrono_now(),
    };

    let state_json = build_session_state(&updated_session)?;
    Ok(Json(state_json))
}

async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    db::delete_session(&state.pool, &id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- Phase F Auth & Matchmaking Handlers ----------

async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<auth::RegisterRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    auth::register(&state.pool, &state.sessions, payload).await
}

async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<auth::LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    auth::login(&state.pool, &state.sessions, payload).await
}

async fn me_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let auth_header = headers.get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;
    if !auth_header.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, "Invalid token format".to_string()));
    }
    let token = &auth_header[7..];
    auth::get_me(&state.pool, &state.sessions, token).await
}

async fn logout_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let auth_header = headers.get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;
    if !auth_header.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, "Invalid token format".to_string()));
    }
    let token = &auth_header[7..];
    auth::logout(&state.sessions, token).await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct JoinQueueRequest {
    pub time_control: String,
}

async fn join_queue_handler(
    State(state): State<AppState>,
    auth_user: auth::AuthUser,
    axum::extract::Json(payload): axum::extract::Json<JoinQueueRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user = db::get_user_by_id(&state.pool, &auth_user.user_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let entry = matchmaking::QueueEntry {
        user_id: user.id,
        username: user.username,
        rating: user.rating,
        queued_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        time_control: payload.time_control.clone(),
    };

    if let Some(match_result) = matchmaking::join_queue(&state.queue, entry) {
        db::create_live_game(
            &state.pool,
            &match_result.game_id,
            &match_result.white_user_id,
            &match_result.black_user_id,
            &match_result.time_control,
        ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        live::create_live_game(
            &state.live_store,
            match_result.game_id.clone(),
            match_result.white_user_id.clone(),
            match_result.white.clone(),
            match_result.black_user_id.clone(),
            match_result.black.clone(),
            &match_result.time_control,
        );

        Ok((StatusCode::OK, Json(serde_json::json!({
            "status": "matched",
            "match": match_result
        }))))
    } else {
        Ok((StatusCode::OK, Json(serde_json::json!({
            "status": "queued"
        }))))
    }
}

async fn leave_queue_handler(
    State(state): State<AppState>,
    auth_user: auth::AuthUser,
) -> impl IntoResponse {
    matchmaking::leave_queue(&state.queue, &auth_user.user_id);
    StatusCode::OK
}

async fn queue_status_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let entries = matchmaking::queue_status(&state.queue);
    Json(entries)
}

async fn list_live_games_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let games = db::list_live_games(&state.pool, 0).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(games))
}


// ---------- Session State Builder ----------

fn build_session_state(session: &GameSession) -> Result<serde_json::Value, (StatusCode, String)> {
    let session_moves: Vec<String> = serde_json::from_str(&session.moves).unwrap_or_default();

    let mut game = Game::new().map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    for mv in &session_moves {
        game.play_uci(mv).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    }

    let board = game.board();
    let mut legal_board = board.clone();
    let mut legal_moves = generate_legal_moves(&mut legal_board)
        .into_iter()
        .map(|mv| mv.uci())
        .collect::<Vec<_>>();
    legal_moves.sort();

    let evaluation = evaluate(board);
    let control = SearchControl::new();
    let depth = std::cmp::min(session.bot_level as u8, 5).max(2);
    let mut search_board = board.clone();
    let search = iterative_deepening(
        &mut search_board,
        SearchLimits {
            max_depth: depth,
            quiescence_depth: depth.min(4),
            candidate_count: 6,
            hash_size_mb: 4,
            ..SearchLimits::default()
        },
        &control,
    );
    let bot = choose_bot_move(board, BotLevel::new(session.bot_level as u8), &control, "", &std::collections::HashMap::new());

    let side_to_move = match board.side_to_move() {
        Color::White => "white",
        Color::Black => "black",
    };

    Ok(serde_json::json!({
        "session": {
            "id": session.id,
            "mode": session.mode,
            "botLevel": session.bot_level,
            "result": session.result,
            "createdAt": session.created_at,
        },
        "engine": "Axiorynth",
        "ply": game.records().len(),
        "moves": session_moves,
        "result": game.result().as_str(),
        "fen": board.to_fen(),
        "sideToMove": side_to_move,
        "inCheck": board.in_check(board.side_to_move()),
        "legalMoves": legal_moves,
        "evaluation": {
            "materialWhite": evaluation.material_white,
            "materialBlack": evaluation.material_black,
            "materialScore": evaluation.material_score,
            "pieceSquareWhite": evaluation.piece_square_white,
            "pieceSquareBlack": evaluation.piece_square_black,
            "pieceSquareScore": evaluation.piece_square_score,
            "mobilityWhite": evaluation.mobility_white,
            "mobilityBlack": evaluation.mobility_black,
            "mobilityScore": evaluation.mobility_score,
            "centerWhite": evaluation.center_white,
            "centerBlack": evaluation.center_black,
            "centerScore": evaluation.center_score,
            "pawnStructureWhite": evaluation.pawn_structure_white,
            "pawnStructureBlack": evaluation.pawn_structure_black,
            "pawnStructureScore": evaluation.pawn_structure_score,
            "kingSafetyWhite": evaluation.king_safety_white,
            "kingSafetyBlack": evaluation.king_safety_black,
            "kingSafetyScore": evaluation.king_safety_score,
            "totalWhitePerspective": evaluation.total_white_perspective,
            "totalSideToMovePerspective": evaluation.total_side_to_move_perspective,
            "mathLines": evaluation.as_math_lines(),
        },
        "search": {
            "bestMove": search.best_move.map(|mv| mv.uci()),
            "score": search.score,
            "depth": search.depth,
            "nodes": search.stats.nodes,
            "qnodes": search.stats.qnodes,
            "betaCutoffs": search.stats.beta_cutoffs,
            "qBetaCutoffs": search.stats.q_beta_cutoffs,
            "ttHits": search.stats.tt_hits,
            "ttStores": search.stats.tt_stores,
            "hashfullPermill": search.stats.hashfull_permill,
            "killerUses": search.stats.killer_uses,
            "stopped": search.stats.stopped,
            "principalVariation": search.principal_variation.iter().map(|mv| mv.uci()).collect::<Vec<_>>(),
            "candidates": search.candidates.iter().map(|c| serde_json::json!({"move": c.mv.uci(), "score": c.score})).collect::<Vec<_>>(),
            "mathLines": search.as_math_lines(),
        },
        "bot": {
            "level": bot.profile.level.value(),
            "name": bot.profile.name,
            "description": bot.profile.description,
            "selectedMove": bot.selected_move.map(|mv| mv.uci()),
            "searchScore": bot.search.score,
            "searchDepth": bot.search.depth,
            "mathLines": bot.as_lines(),
        },
        "history": game.records().iter().map(|r| serde_json::json!({
            "ply": r.ply,
            "uci": r.uci,
            "evalAfter": r.eval_after,
            "resultAfter": r.result_after.as_str(),
            "fenAfter": r.fen_after,
        })).collect::<Vec<_>>(),
    }))
}

// ---------- Legacy Hand-Rolled JSON (for /api/state compatibility) ----------

fn frontend_state_json(
    game: &Game,
    legal_moves: &[String],
    evaluation: &EvalBreakdown,
    search: &SearchResult,
    bot: &BotMove,
) -> String {
    let board = game.board();
    let side_to_move = match board.side_to_move() {
        Color::White => "white",
        Color::Black => "black",
    };
    let in_check = board.in_check(board.side_to_move());
    let moves = game.uci_moves();

    format!(
        concat!(
            "{{",
            "\"engine\":{},",
            "\"ply\":{},",
            "\"moves\":{},",
            "\"result\":{},",
            "\"fen\":{},",
            "\"sideToMove\":{},",
            "\"inCheck\":{},",
            "\"legalMoves\":{},",
            "\"history\":{},",
            "\"evaluation\":{},",
            "\"search\":{},",
            "\"bot\":{}",
            "}}"
        ),
        json_string("Axiorynth"),
        game.records().len(),
        json_string_array(&moves),
        json_string(game.result().as_str()),
        json_string(&board.to_fen()),
        json_string(side_to_move),
        in_check,
        json_string_array(legal_moves),
        history_json(game.records()),
        evaluation_json(evaluation),
        search_json(search),
        bot_json(bot)
    )
}

fn history_json(records: &[GameRecord]) -> String {
    let rows = records
        .iter()
        .map(|record| {
            format!(
                concat!(
                    "{{",
                    "\"ply\":{},",
                    "\"uci\":{},",
                    "\"evalAfter\":{},",
                    "\"resultAfter\":{},",
                    "\"fenAfter\":{}",
                    "}}"
                ),
                record.ply,
                json_string(&record.uci),
                record.eval_after,
                json_string(record.result_after.as_str()),
                json_string(&record.fen_after)
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", rows.join(","))
}

fn evaluation_json(evaluation: &EvalBreakdown) -> String {
    let math_lines = evaluation.as_math_lines();
    format!(
        concat!(
            "{{",
            "\"materialWhite\":{},",
            "\"materialBlack\":{},",
            "\"materialScore\":{},",
            "\"pieceSquareWhite\":{},",
            "\"pieceSquareBlack\":{},",
            "\"pieceSquareScore\":{},",
            "\"mobilityWhite\":{},",
            "\"mobilityBlack\":{},",
            "\"mobilityScore\":{},",
            "\"centerWhite\":{},",
            "\"centerBlack\":{},",
            "\"centerScore\":{},",
            "\"pawnStructureWhite\":{},",
            "\"pawnStructureBlack\":{},",
            "\"pawnStructureScore\":{},",
            "\"kingSafetyWhite\":{},",
            "\"kingSafetyBlack\":{},",
            "\"kingSafetyScore\":{},",
            "\"totalWhitePerspective\":{},",
            "\"totalSideToMovePerspective\":{},",
            "\"mathLines\":{}",
            "}}"
        ),
        evaluation.material_white,
        evaluation.material_black,
        evaluation.material_score,
        evaluation.piece_square_white,
        evaluation.piece_square_black,
        evaluation.piece_square_score,
        evaluation.mobility_white,
        evaluation.mobility_black,
        evaluation.mobility_score,
        evaluation.center_white,
        evaluation.center_black,
        evaluation.center_score,
        evaluation.pawn_structure_white,
        evaluation.pawn_structure_black,
        evaluation.pawn_structure_score,
        evaluation.king_safety_white,
        evaluation.king_safety_black,
        evaluation.king_safety_score,
        evaluation.total_white_perspective,
        evaluation.total_side_to_move_perspective,
        json_string_array(&math_lines)
    )
}

fn search_json(search: &SearchResult) -> String {
    let math_lines = search.as_math_lines();
    let pv = search
        .principal_variation
        .iter()
        .map(|mv| mv.uci())
        .collect::<Vec<_>>();
    let candidates = search
        .candidates
        .iter()
        .map(|candidate| {
            format!(
                "{{\"move\":{},\"score\":{}}}",
                json_string(&candidate.mv.uci()),
                candidate.score
            )
        })
        .collect::<Vec<_>>();

    format!(
        concat!(
            "{{",
            "\"bestMove\":{},",
            "\"score\":{},",
            "\"depth\":{},",
            "\"nodes\":{},",
            "\"qnodes\":{},",
            "\"betaCutoffs\":{},",
            "\"qBetaCutoffs\":{},",
            "\"ttHits\":{},",
            "\"ttStores\":{},",
            "\"hashfullPermill\":{},",
            "\"killerUses\":{},",
            "\"stopped\":{},",
            "\"principalVariation\":{},",
            "\"candidates\":[{}],",
            "\"mathLines\":{}",
            "}}"
        ),
        optional_move_json(search.best_move),
        search.score,
        search.depth,
        search.stats.nodes,
        search.stats.qnodes,
        search.stats.beta_cutoffs,
        search.stats.q_beta_cutoffs,
        search.stats.tt_hits,
        search.stats.tt_stores,
        search.stats.hashfull_permill,
        search.stats.killer_uses,
        search.stats.stopped,
        json_string_array(&pv),
        candidates.join(","),
        json_string_array(&math_lines)
    )
}

fn bot_json(bot: &BotMove) -> String {
    let math_lines = bot.as_lines();
    format!(
        concat!(
            "{{",
            "\"level\":{},",
            "\"name\":{},",
            "\"description\":{},",
            "\"selectedMove\":{},",
            "\"searchScore\":{},",
            "\"searchDepth\":{},",
            "\"mathLines\":{}",
            "}}"
        ),
        bot.profile.level.value(),
        json_string(bot.profile.name),
        json_string(bot.profile.description),
        optional_move_json(bot.selected_move),
        bot.search.score,
        bot.search.depth,
        json_string_array(&math_lines)
    )
}

fn optional_move_json(mv: Option<axiorynth_engine::Move>) -> String {
    mv.map(|value| json_string(&value.uci()))
        .unwrap_or_else(|| "null".to_string())
}

fn json_string_array(values: &[String]) -> String {
    let escaped = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>();
    format!("[{}]", escaped.join(","))
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Simple ISO-like timestamp without external dep
    format!("{}", secs)
}

// ---------- Phase 3: Intelligence Endpoints ----------

#[derive(Deserialize)]
struct BookProbeQuery {
    #[serde(default)]
    fen: String,
}

#[derive(serde::Serialize)]
struct BookProbeResponse {
    entries: Vec<BookEntryResponse>,
}

#[derive(serde::Serialize)]
struct BookEntryResponse {
    uci_move: String,
    weight: u32,
    score: f64,
}

async fn book_probe_handler(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<BookProbeQuery>,
) -> Result<Json<BookProbeResponse>, (StatusCode, String)> {
    let mut board = axiorynth_engine::Board::startpos().map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if !query.fen.is_empty() {
        board = axiorynth_engine::Board::from_fen(&query.fen).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    }
    
    let hash = board.hash();
    let mut response_entries = Vec::new();
    
    if let Some(entries) = state.book.probe(hash) {
        for e in entries {
            response_entries.push(BookEntryResponse {
                uci_move: e.uci_move.clone(),
                weight: e.weight,
                score: e.score,
            });
        }
    }
    
    response_entries.sort_by(|a, b| b.weight.cmp(&a.weight));
    
    Ok(Json(BookProbeResponse {
        entries: response_entries,
    }))
}

#[derive(serde::Serialize)]
struct TablebaseResponse {
    available: bool,
    wdl: Option<String>,
    dtz: Option<i32>,
}

async fn tablebase_probe_handler(
    axum::extract::Query(query): axum::extract::Query<BookProbeQuery>,
) -> Json<TablebaseResponse> {
    let board = match axiorynth_engine::Board::from_fen(&query.fen) {
        Ok(b) => b,
        Err(_) => return Json(TablebaseResponse { available: false, wdl: None, dtz: None }),
    };
    
    let piece_count = axiorynth_engine::tablebase::total_pieces(&board);
    if piece_count > 7 {
        return Json(TablebaseResponse { available: false, wdl: None, dtz: None });
    }
    
    match axiorynth_engine::probe_tablebase(&board) {
        Some(res) => {
            let wdl = match res {
                axiorynth_engine::WdlResult::Win | axiorynth_engine::WdlResult::BlessedWin => "win",
                axiorynth_engine::WdlResult::Loss | axiorynth_engine::WdlResult::CursedLoss => "loss",
                axiorynth_engine::WdlResult::Draw => "draw",
            }.to_string();
            
            Json(TablebaseResponse {
                available: true,
                wdl: Some(wdl),
                dtz: None,
            })
        }
        None => Json(TablebaseResponse { available: false, wdl: None, dtz: None }),
    }
}

#[derive(serde::Serialize)]
struct GameReviewResponse {
    moves: Vec<MoveReview>,
    white_accuracy: f64,
    black_accuracy: f64,
    critical_moments: Vec<CriticalMoment>,
}

#[derive(serde::Serialize)]
struct MoveReview {
    ply: usize,
    uci: String,
    eval_cp: i32,
    best_move: String,
    best_eval: i32,
    classification: String,
}

#[derive(serde::Serialize)]
struct CriticalMoment {
    ply: usize,
    eval_drop: i32,
    fen: String,
    best_move: String,
    motif: String,
}

async fn game_review_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<GameReviewResponse>, (StatusCode, String)> {
    let mut moves_opt = None;
    // Actually, I can use sqlx::query_scalar
    if let Ok(Some(moves_str)) = sqlx::query_scalar::<_, String>("SELECT moves FROM live_games WHERE id = ?").bind(&id).fetch_optional(&state.pool).await {
        moves_opt = Some(serde_json::from_str::<Vec<String>>(&moves_str).unwrap_or_default());
    } else if let Ok(Some(moves_str)) = sqlx::query_scalar::<_, String>("SELECT moves FROM game_sessions WHERE id = ?").bind(&id).fetch_optional(&state.pool).await {
        moves_opt = Some(serde_json::from_str::<Vec<String>>(&moves_str).unwrap_or_default());
    }
    
    let moves = moves_opt.ok_or((StatusCode::NOT_FOUND, "Game not found".to_string()))?;
    
    let review = tokio::task::spawn_blocking(move || {
        let mut game = axiorynth_engine::Game::new().unwrap();
        let mut move_reviews = Vec::new();
        let control = axiorynth_engine::SearchControl::new();
        
        for (ply, mv_uci) in moves.iter().enumerate() {
            let is_white = ply % 2 == 0;
            let mut search_board = game.board().clone();
            let search = axiorynth_engine::iterative_deepening(
                &mut search_board,
                axiorynth_engine::SearchLimits {
                    max_depth: 8,
                    quiescence_depth: 4,
                    candidate_count: 1,
                    hash_size_mb: 2,
                    ..Default::default()
                },
                &control,
            );
            
            let best_eval_from_side = search.score;
            let best_eval_cp = if is_white { best_eval_from_side } else { -best_eval_from_side };
            let best_move = search.best_move.map(|m| m.uci()).unwrap_or_default();
            
            if game.play_uci(mv_uci).is_err() {
                break; // Invalid move
            }
            
            let mut eval_board = game.board().clone();
            let opp_search = axiorynth_engine::iterative_deepening(
                &mut eval_board,
                axiorynth_engine::SearchLimits {
                    max_depth: 8,
                    quiescence_depth: 4,
                    candidate_count: 1,
                    hash_size_mb: 2,
                    ..Default::default()
                },
                &control,
            );
            
            let actual_eval_from_opp = opp_search.score;
            let eval_after = -actual_eval_from_opp;
            let eval_cp = if is_white { eval_after } else { -eval_after };
            
            let delta = (best_eval_from_side - eval_after).max(0);
            
            let classification = if delta <= 10 {
                "great"
            } else if delta <= 30 {
                "good"
            } else if delta <= 80 {
                "inaccuracy"
            } else if delta <= 200 {
                "mistake"
            } else {
                "blunder"
            }.to_string();
            
            move_reviews.push(MoveReview {
                ply: ply + 1,
                uci: mv_uci.clone(),
                eval_cp,
                best_move,
                best_eval: best_eval_cp,
                classification,
            });
        }
        
        let mut white_acc_sum = 0.0;
        let mut white_count = 0;
        let mut black_acc_sum = 0.0;
        let mut black_count = 0;
        let max_delta = 400.0;
        
        let mut moments = Vec::new();
        let mut replay_game = axiorynth_engine::Game::new().unwrap();
        
        for (i, mr) in move_reviews.iter().enumerate() {
            let is_white = i % 2 == 0;
            let best = if is_white { mr.best_eval } else { -mr.best_eval };
            let actual = if is_white { mr.eval_cp } else { -mr.eval_cp };
            
            let delta = (best - actual).max(0) as f64;
            let acc = (1.0 - delta / max_delta).clamp(0.0, 1.0);
            if is_white {
                white_acc_sum += acc;
                white_count += 1;
            } else {
                black_acc_sum += acc;
                black_count += 1;
            }
            
            let fen_before = replay_game.board().to_fen();
            let _ = replay_game.play_uci(&mr.uci);
            
            moments.push((mr.ply, delta as i32, fen_before, mr.best_move.clone()));
        }
        
        moments.sort_by(|a, b| b.1.cmp(&a.1));
        
        let critical_moments = moments.into_iter().take(3).map(|(ply, drop, fen, bm)| {
            CriticalMoment {
                ply,
                eval_drop: drop,
                fen,
                best_move: bm,
                motif: "Tactical oversight".to_string(),
            }
        }).collect();
        
        let white_accuracy = if white_count > 0 { white_acc_sum / white_count as f64 * 100.0 } else { 100.0 };
        let black_accuracy = if black_count > 0 { black_acc_sum / black_count as f64 * 100.0 } else { 100.0 };
        
        GameReviewResponse {
            moves: move_reviews,
            white_accuracy,
            black_accuracy,
            critical_moments,
        }
    }).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(review))
}

