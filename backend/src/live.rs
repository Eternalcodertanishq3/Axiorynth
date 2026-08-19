use axum::{extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State, Path, Query}, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Instant, Duration};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TimeControl {
    Bullet1_0,
    Bullet2_1,
    Blitz3_0,
    Blitz5_3,
    Rapid10_0,
    Rapid15_10,
    Unlimited,
}

impl TimeControl {
    pub fn initial_ms(&self) -> Option<i64> {
        match self {
            Self::Bullet1_0 => Some(60_000),
            Self::Bullet2_1 => Some(120_000),
            Self::Blitz3_0 => Some(180_000),
            Self::Blitz5_3 => Some(300_000),
            Self::Rapid10_0 => Some(600_000),
            Self::Rapid15_10 => Some(900_000),
            Self::Unlimited => None,
        }
    }
    pub fn increment_ms(&self) -> i64 {
        match self {
            Self::Bullet1_0 => 0,
            Self::Bullet2_1 => 1_000,
            Self::Blitz3_0 => 0,
            Self::Blitz5_3 => 3_000,
            Self::Rapid10_0 => 0,
            Self::Rapid15_10 => 10_000,
            Self::Unlimited => 0,
        }
    }
    pub fn category(&self) -> &'static str {
        match self {
            Self::Bullet1_0 | Self::Bullet2_1 => "bullet",
            Self::Blitz3_0 | Self::Blitz5_3 => "blitz",
            Self::Rapid10_0 | Self::Rapid15_10 => "rapid",
            Self::Unlimited => "unlimited",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Bullet1_0 => "1+0",
            Self::Bullet2_1 => "2+1",
            Self::Blitz3_0 => "3+0",
            Self::Blitz5_3 => "5+3",
            Self::Rapid10_0 => "10+0",
            Self::Rapid15_10 => "15+10",
            Self::Unlimited => "∞",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "1+0" => Self::Bullet1_0,
            "2+1" => Self::Bullet2_1,
            "3+0" => Self::Blitz3_0,
            "5+3" => Self::Blitz5_3,
            "10+0" => Self::Rapid10_0,
            "15+10" => Self::Rapid15_10,
            _ => Self::Unlimited,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiveClock {
    pub white_remaining_ms: i64,
    pub black_remaining_ms: i64,
    pub increment_ms: i64,
    pub last_move_at: Instant,
    pub active_color: bool, // true = white's clock running
    pub is_ticking: bool,
}

#[derive(Debug, Clone)]
pub struct LiveGame {
    pub id: String,
    pub white_user_id: String,
    pub black_user_id: String,
    pub white_username: String,
    pub black_username: String,
    pub fen: String,
    pub moves: Vec<String>,
    pub result: String,
    pub time_control: TimeControl,
    pub clock: Option<LiveClock>,
    pub draw_offered_by: Option<String>,
    pub disconnect_deadline: Option<(String, Instant)>,
    pub last_heartbeat: HashMap<String, Instant>,
    pub sender: broadcast::Sender<String>,
}

pub type LiveGameStore = Arc<RwLock<HashMap<String, LiveGame>>>;

#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
#[allow(dead_code)]
pub enum LiveClientMessage {
    #[serde(rename = "move")]
    MakeMove { uci_move: String, user_id: Option<String> },
    #[serde(rename = "resign")]
    Resign { user_id: Option<String> },
    #[serde(rename = "draw_offer")]
    DrawOffer { user_id: Option<String> },
    #[serde(rename = "draw_accept")]
    DrawAccept { user_id: Option<String> },
    #[serde(rename = "draw_decline")]
    DrawDecline { user_id: Option<String> },
    #[serde(rename = "heartbeat")]
    Heartbeat { user_id: Option<String> },
}

pub fn new_live_store() -> LiveGameStore {
    Arc::new(RwLock::new(HashMap::new()))
}

pub fn create_live_game(
    store: &LiveGameStore,
    game_id: String,
    white_user_id: String,
    white_username: String,
    black_user_id: String,
    black_username: String,
    tc: &str,
) -> LiveGame {
    let (tx, _rx) = broadcast::channel(100);
    let time_control = TimeControl::from_str(tc);
    
    let clock = time_control.initial_ms().map(|ms| LiveClock {
        white_remaining_ms: ms,
        black_remaining_ms: ms,
        increment_ms: time_control.increment_ms(),
        last_move_at: Instant::now(),
        active_color: true,
        is_ticking: false,
    });
    
    let game = LiveGame {
        id: game_id.clone(),
        white_user_id,
        black_user_id,
        white_username,
        black_username,
        fen: axiorynth_engine::STARTPOS_FEN.to_string(),
        moves: Vec::new(),
        result: "ongoing".to_string(),
        time_control,
        clock,
        draw_offered_by: None,
        disconnect_deadline: None,
        last_heartbeat: HashMap::new(),
        sender: tx,
    };
    if let Ok(mut s) = store.write() {
        s.insert(game_id, game.clone());
    }
    game
}

pub async fn live_ws_handler(
    ws: WebSocketUpgrade,
    Path(game_id): Path<String>,
    Query(query): Query<WsAuthQuery>,
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let auth_user_id = query.token.as_deref().and_then(|t| {
        crate::auth::validate_token(&state.sessions, t)
    });

    ws.on_upgrade(move |socket| handle_live_socket(socket, game_id, auth_user_id, state.live_store, state.pool))
}

async fn handle_live_socket(
    mut socket: WebSocket,
    game_id: String,
    auth_user_id: Option<String>,
    store: LiveGameStore,
    pool: sqlx::SqlitePool,
) {
    let (rx_opt, initial_state_opt) = {
        let store_guard = match store.read() {
            Ok(g) => g,
            Err(_) => return,
        };
        match store_guard.get(&game_id) {
            Some(game) => {
                let mut clock_json = serde_json::Value::Null;
                if let Some(c) = &game.clock {
                    let mut w_ms = c.white_remaining_ms;
                    let mut b_ms = c.black_remaining_ms;
                    if c.is_ticking {
                        let elapsed = c.last_move_at.elapsed().as_millis() as i64;
                        if c.active_color { w_ms -= elapsed; } else { b_ms -= elapsed; }
                    }
                    clock_json = json!({
                        "white_ms": w_ms.max(0),
                        "black_ms": b_ms.max(0),
                        "active": if c.active_color { "white" } else { "black" }
                    });
                }
                
                let initial_state = json!({
                    "type": "game_state",
                    "id": game.id,
                    "white_user_id": game.white_user_id,
                    "black_user_id": game.black_user_id,
                    "white_username": game.white_username,
                    "black_username": game.black_username,
                    "fen": game.fen,
                    "moves": game.moves,
                    "result": game.result,
                    "clock": clock_json,
                }).to_string();
                (Some(game.sender.subscribe()), Some(initial_state))
            }
            None => {
                let err_msg = json!({ "type": "error", "message": "Game not found" }).to_string();
                (None, Some(err_msg))
            }
        }
    };

    let mut rx = match rx_opt {
        Some(rx) => {
            if let Some(initial_state) = initial_state_opt {
                if socket.send(Message::Text(initial_state.into())).await.is_err() {
                    return;
                }
            }
            rx
        }
        None => {
            if let Some(err_msg) = initial_state_opt {
                let _ = socket.send(Message::Text(err_msg.into())).await;
            }
            return;
        }
    };

    if let Some(uid) = &auth_user_id {
        if let Ok(mut g) = store.write() {
            if let Some(game) = g.get_mut(&game_id) {
                game.disconnect_deadline = None;
                game.last_heartbeat.insert(uid.clone(), Instant::now());
                let msg = json!({ "type": "player_reconnected", "user_id": uid }).to_string();
                let _ = game.sender.send(msg);
            }
        }
    }

    loop {
        tokio::select! {
            val_res = tokio::time::timeout(Duration::from_secs(1), socket.recv()) => {
                let msg = match val_res {
                    Ok(Some(Ok(m))) => m,
                    Ok(_) => break, // Connection closed or error
                    Err(_) => {
                        // Timeout tick, check disconnect deadlines
                        let mut auto_forfeit = None;
                        if let Ok(g) = store.read() {
                            if let Some(game) = g.get(&game_id) {
                                if let Some((uid, deadline)) = &game.disconnect_deadline {
                                    if Instant::now() > *deadline {
                                        auto_forfeit = Some(uid.clone());
                                    }
                                }
                            }
                        }
                        
                        if let Some(uid) = auto_forfeit {
                            // Player forfeits on time / disconnect
                            let _ = handle_resign(&game_id, &uid, &store, &pool).await;
                        }
                        continue;
                    }
                };
                
                if let Message::Text(text) = msg {
                    let client_msg: Result<LiveClientMessage, _> = serde_json::from_str(&text);
                    match client_msg {
                        Ok(LiveClientMessage::MakeMove { uci_move, .. }) => {
                            let sender_id = match &auth_user_id {
                                Some(id) => id,
                                None => {
                                    let err_msg = json!({ "type": "error", "message": "Authentication required to make moves" }).to_string();
                                    let _ = socket.send(Message::Text(err_msg.into())).await;
                                    continue;
                                }
                            };
                            match handle_make_move(&game_id, &uci_move, sender_id, &store, &pool).await {
                                Ok(_) => {}
                                Err(err_str) => {
                                    let err_msg = json!({ "type": "error", "message": err_str }).to_string();
                                    let _ = socket.send(Message::Text(err_msg.into())).await;
                                }
                            }
                        }
                        Ok(LiveClientMessage::Resign { .. }) => {
                            let sender_id = match &auth_user_id {
                                Some(id) => id,
                                None => {
                                    let err_msg = json!({ "type": "error", "message": "Authentication required to resign" }).to_string();
                                    let _ = socket.send(Message::Text(err_msg.into())).await;
                                    continue;
                                }
                            };
                            match handle_resign(&game_id, sender_id, &store, &pool).await {
                                Ok(_) => {}
                                Err(err_str) => {
                                    let err_msg = json!({ "type": "error", "message": err_str }).to_string();
                                    let _ = socket.send(Message::Text(err_msg.into())).await;
                                }
                            }
                        }
                        Ok(LiveClientMessage::DrawOffer { .. }) => {
                            if let Some(uid) = &auth_user_id {
                                if let Ok(mut g) = store.write() {
                                    if let Some(game) = g.get_mut(&game_id) {
                                        game.draw_offered_by = Some(uid.clone());
                                        let by_color = if uid == &game.white_user_id { "white" } else { "black" };
                                        let msg = json!({ "type": "draw_offered", "by": by_color }).to_string();
                                        let _ = game.sender.send(msg);
                                    }
                                }
                            }
                        }
                        Ok(LiveClientMessage::DrawAccept { .. }) => {
                            if let Some(uid) = &auth_user_id {
                                let mut accepted = false;
                                let mut white_uid = String::new();
                                let mut black_uid = String::new();
                                let mut category = String::new();
                                let mut sender_opt = None;
                                if let Ok(mut g) = store.write() {
                                    if let Some(game) = g.get_mut(&game_id) {
                                        if let Some(offerer) = &game.draw_offered_by {
                                            if offerer != uid {
                                                game.result = "Draw by agreement".to_string();
                                                white_uid = game.white_user_id.clone();
                                                black_uid = game.black_user_id.clone();
                                                category = game.time_control.category().to_string();
                                                sender_opt = Some(game.sender.clone());
                                                accepted = true;
                                                let msg = json!({ "type": "game_state", "result": game.result }).to_string();
                                                let _ = game.sender.send(msg);
                                            }
                                        }
                                    }
                                }
                                if accepted {
                                    if let Some(sender) = sender_opt {
                                        let _ = handle_game_over(&pool, &white_uid, &black_uid, "Draw by agreement", &category, &sender).await;
                                    }
                                }
                            }
                        }
                        Ok(LiveClientMessage::DrawDecline { .. }) => {
                            if let Some(uid) = &auth_user_id {
                                if let Ok(mut g) = store.write() {
                                    if let Some(game) = g.get_mut(&game_id) {
                                        if let Some(offerer) = &game.draw_offered_by {
                                            if offerer != uid {
                                                game.draw_offered_by = None;
                                                let msg = json!({ "type": "draw_declined" }).to_string();
                                                let _ = game.sender.send(msg);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Ok(LiveClientMessage::Heartbeat { .. }) => {
                            if let Some(uid) = &auth_user_id {
                                if let Ok(mut g) = store.write() {
                                    if let Some(game) = g.get_mut(&game_id) {
                                        game.last_heartbeat.insert(uid.clone(), Instant::now());
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            let err_msg = json!({ "type": "error", "message": format!("Invalid message: {}", err) }).to_string();
                            let _ = socket.send(Message::Text(err_msg.into())).await;
                        }
                    }
                }
            }
            
            broadcast_msg = rx.recv() => {
                match broadcast_msg {
                    Ok(msg_text) => {
                        if socket.send(Message::Text(msg_text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Continue
                    }
                }
            }
        }
    }

    if let Some(uid) = auth_user_id {
        if let Ok(mut g) = store.write() {
            if let Some(game) = g.get_mut(&game_id) {
                game.disconnect_deadline = Some((uid.clone(), Instant::now() + Duration::from_secs(30)));
                let msg = json!({ "type": "player_disconnected", "user_id": uid }).to_string();
                let _ = game.sender.send(msg);
            }
        }
    }
}

async fn handle_make_move(
    game_id: &str,
    uci_move: &str,
    user_id: &str,
    store: &LiveGameStore,
    pool: &sqlx::SqlitePool,
) -> Result<String, String> {
    let (game_fen, game_moves, game_result, state_json, sender, white_user_id, black_user_id, category) = {
        let mut store_guard = store.write().map_err(|_| "Failed to lock store".to_string())?;
        let game = store_guard.get_mut(game_id).ok_or_else(|| "Game not found".to_string())?;
        
        if game.result != "ongoing" {
            return Err("Game has already ended".to_string());
        }
        
        // Determine whose turn it is
        let is_white_turn = game.moves.len() % 2 == 0;
        let expected_user_id = if is_white_turn {
            &game.white_user_id
        } else {
            &game.black_user_id
        };
        
        if user_id != expected_user_id {
            return Err("Not your turn".to_string());
        }
        
        // Handle Clock
        let mut flagged = false;
        if let Some(clock) = &mut game.clock {
            if clock.is_ticking {
                let elapsed = clock.last_move_at.elapsed().as_millis() as i64;
                if is_white_turn {
                    clock.white_remaining_ms -= elapsed;
                    if clock.white_remaining_ms <= -150 {
                        flagged = true;
                        game.result = "Black wins".to_string();
                    } else {
                        clock.white_remaining_ms += clock.increment_ms;
                    }
                } else {
                    clock.black_remaining_ms -= elapsed;
                    if clock.black_remaining_ms <= -150 {
                        flagged = true;
                        game.result = "White wins".to_string();
                    } else {
                        clock.black_remaining_ms += clock.increment_ms;
                    }
                }
            }
            
            if !flagged {
                // start clock if it wasn't ticking and move is made
                // standard: start ticking after white's first move
                if !clock.is_ticking && is_white_turn {
                    clock.is_ticking = true;
                }
                clock.active_color = !is_white_turn;
                clock.last_move_at = Instant::now();
            }
        }
        
        let mut clock_json = serde_json::Value::Null;
        
        if flagged {
            // Reconstruct Game just for broadcast fen/moves parity
            if let Some(c) = &game.clock {
                clock_json = json!({
                    "white_ms": c.white_remaining_ms.max(0),
                    "black_ms": c.black_remaining_ms.max(0),
                    "active": if c.active_color { "white" } else { "black" }
                });
            }
        } else {
            // Reconstruct Game
            let mut engine_game = axiorynth_engine::Game::new().map_err(|e| e.to_string())?;
            for mv in &game.moves {
                engine_game.play_uci(mv).map_err(|e| e.to_string())?;
            }
            
            // Apply new move
            engine_game.play_uci(uci_move).map_err(|e| format!("Illegal move: {}", e))?;
            
            game.fen = engine_game.board().to_fen();
            game.moves.push(uci_move.to_string());
            
            // Check game status
            let engine_result = engine_game.result();
            let result_str = engine_result.as_str().to_string();
            game.result = result_str;
            
            // Auto decline draw
            game.draw_offered_by = None;
            
            if let Some(c) = &game.clock {
                clock_json = json!({
                    "white_ms": c.white_remaining_ms,
                    "black_ms": c.black_remaining_ms,
                    "active": if c.active_color { "white" } else { "black" }
                });
            }
        }

        let category = game.time_control.category();
        // Clone/copy necessary info before dropping the lock
        let state_json = json!({
            "type": "game_state",
            "id": game.id,
            "white_user_id": game.white_user_id,
            "black_user_id": game.black_user_id,
            "white_username": game.white_username,
            "black_username": game.black_username,
            "fen": game.fen,
            "moves": game.moves,
            "result": game.result,
            "clock": clock_json,
        }).to_string();

        (
            game.fen.clone(),
            game.moves.clone(),
            game.result.clone(),
            state_json,
            game.sender.clone(),
            game.white_user_id.clone(),
            game.black_user_id.clone(),
            category
        )
    }; // Lock guard is dropped here

    // Perform database and async operations outside lock
    let moves_json = serde_json::to_string(&game_moves).unwrap_or_else(|_| "[]".to_string());
    let _ = crate::db::update_live_game(pool, game_id, &game_fen, &moves_json, &game_result).await;
    
    if game_result != "ongoing" {
        // Game over, let's update ratings!
        let _ = handle_game_over(pool, &white_user_id, &black_user_id, &game_result, category, &sender).await;
    }
    
    // Broadcast state
    let _ = sender.send(state_json.clone());
    
    Ok(state_json)
}

async fn handle_resign(
    game_id: &str,
    user_id: &str,
    store: &LiveGameStore,
    pool: &sqlx::SqlitePool,
) -> Result<(), String> {
    let (game_fen, game_moves, game_result, state_json, sender, white_user_id, black_user_id, category) = {
        let mut store_guard = store.write().map_err(|_| "Failed to lock store".to_string())?;
        let game = store_guard.get_mut(game_id).ok_or_else(|| "Game not found".to_string())?;
        
        if game.result != "ongoing" {
            return Err("Game has already ended".to_string());
        }
        
        let result_str = if user_id == game.white_user_id {
            "Black wins" // White resigned
        } else if user_id == game.black_user_id {
            "White wins" // Black resigned
        } else {
            return Err("Not a player in this game".to_string());
        };
        
        game.result = result_str.to_string();

        let category = game.time_control.category();
        let state_json = json!({
            "type": "game_state",
            "id": game.id,
            "white_user_id": game.white_user_id,
            "black_user_id": game.black_user_id,
            "white_username": game.white_username,
            "black_username": game.black_username,
            "fen": game.fen,
            "moves": game.moves,
            "result": game.result,
        }).to_string();

        (
            game.fen.clone(),
            game.moves.clone(),
            game.result.clone(),
            state_json,
            game.sender.clone(),
            game.white_user_id.clone(),
            game.black_user_id.clone(),
            category
        )
    }; // Lock guard is dropped here

    // Perform database and async operations outside lock
    let moves_json = serde_json::to_string(&game_moves).unwrap_or_else(|_| "[]".to_string());
    let _ = crate::db::update_live_game(pool, game_id, &game_fen, &moves_json, &game_result).await;
    
    // Update ratings
    let _ = handle_game_over(pool, &white_user_id, &black_user_id, &game_result, category, &sender).await;
    
    // Broadcast state
    let _ = sender.send(state_json);
    Ok(())
}

fn glicko2_update(
    rating: f64, rd: f64, volatility: f64,
    opponent_rating: f64, opponent_rd: f64,
    score: f64,
) -> (f64, f64, f64) {
    let tau = 0.5f64;
    let mu = (rating - 1500.0) / 173.7178;
    let phi = rd / 173.7178;
    
    let mu_j = (opponent_rating - 1500.0) / 173.7178;
    let phi_j = opponent_rd / 173.7178;
    
    let g = |phi: f64| 1.0 / (1.0 + 3.0 * phi * phi / std::f64::consts::PI.powi(2)).sqrt();
    let e = |mu: f64, mu_j: f64, phi_j: f64| 1.0 / (1.0 + (-g(phi_j) * (mu - mu_j)).exp());
    
    let e_j = e(mu, mu_j, phi_j);
    let v = 1.0 / (g(phi_j).powi(2) * e_j * (1.0 - e_j));
    let delta = v * g(phi_j) * (score - e_j);
    
    let a = volatility.powi(2).ln();
    let f = |x: f64| -> f64 {
        let e_x = x.exp();
        let num = e_x * (delta.powi(2) - phi.powi(2) - v - e_x);
        let den = 2.0 * (phi.powi(2) + v + e_x).powi(2);
        (num / den) - (x - a) / tau.powi(2)
    };
    
    let epsilon = 0.000001;
    let mut a_bound = a;
    let mut b_bound = if delta.powi(2) > phi.powi(2) + v {
        (delta.powi(2) - phi.powi(2) - v).ln()
    } else {
        let mut k = 1.0;
        while f(a - k * tau) < 0.0 {
            k += 1.0;
        }
        a - k * tau
    };
    
    let mut f_a = f(a_bound);
    let mut f_b = f(b_bound);
    
    while (b_bound - a_bound).abs() > epsilon {
        let c = a_bound + (a_bound - b_bound) * f_a / (f_b - f_a);
        let f_c = f(c);
        if f_c * f_b <= 0.0 {
            a_bound = b_bound;
            f_a = f_b;
        } else {
            f_a /= 2.0;
        }
        b_bound = c;
        f_b = f_c;
    }
    
    let sig_prime = (a_bound).exp().sqrt();
    let phi_star = (phi.powi(2) + sig_prime.powi(2)).sqrt();
    let phi_prime = 1.0 / (1.0 / phi_star.powi(2) + 1.0 / v).sqrt();
    let mu_prime = mu + phi_prime.powi(2) * g(phi_j) * (score - e_j);
    
    let new_rating = 173.7178 * mu_prime + 1500.0;
    let new_rd = 173.7178 * phi_prime;
    
    (new_rating, new_rd, sig_prime)
}

async fn handle_game_over(
    pool: &sqlx::SqlitePool,
    white_user_id: &str,
    black_user_id: &str,
    result: &str,
    category: &str,
    sender: &broadcast::Sender<String>,
) -> Result<(), String> {
    let w_row = crate::db::get_rating(pool, white_user_id, category).await.unwrap_or(None);
    let b_row = crate::db::get_rating(pool, black_user_id, category).await.unwrap_or(None);
    
    let (w_r, w_rd, w_v) = w_row.map(|r| (r.rating, r.rd, r.volatility)).unwrap_or((1500.0, 350.0, 0.06));
    let (b_r, b_rd, b_v) = b_row.map(|r| (r.rating, r.rd, r.volatility)).unwrap_or((1500.0, 350.0, 0.06));
    
    let (sa, sb) = match result {
        "White wins" => (1.0, 0.0),
        "Black wins" => (0.0, 1.0),
        _ => (0.5, 0.5), // Draw
    };
    
    let (new_w_r, new_w_rd, new_w_v) = glicko2_update(w_r, w_rd, w_v, b_r, b_rd, sa);
    let (new_b_r, new_b_rd, new_b_v) = glicko2_update(b_r, b_rd, b_v, w_r, w_rd, sb);
    
    let _ = crate::db::upsert_rating(pool, white_user_id, category, new_w_r, new_w_rd, new_w_v).await;
    let _ = crate::db::upsert_rating(pool, black_user_id, category, new_b_r, new_b_rd, new_b_v).await;
    
    // Update main rating (simplistic average or just overwrite) for the user display
    let _ = crate::db::update_user_rating(pool, white_user_id, new_w_r.round() as i32).await;
    let _ = crate::db::update_user_rating(pool, black_user_id, new_b_r.round() as i32).await;
    
    let msg = json!({
        "type": "rating_update",
        "white_delta": new_w_r.round() as i32 - w_r.round() as i32,
        "black_delta": new_b_r.round() as i32 - b_r.round() as i32,
        "white_new": new_w_r.round() as i32,
        "black_new": new_b_r.round() as i32,
    }).to_string();
    let _ = sender.send(msg);
    
    Ok(())
}

