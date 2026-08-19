use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use sqlx::SqlitePool;

use axiorynth_engine::{
    board::Board,
    bot::{choose_bot_move_with_callback, BotLevel},
    search::SearchControl,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum ClientMessage {
    #[serde(rename = "search")]
    Search {
        fen: String,
        level: u8,
        #[serde(default)]
        moves: Vec<String>,
    },
    #[serde(rename = "cancel")]
    Cancel,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, pool))
}

async fn handle_socket(mut socket: WebSocket, pool: SqlitePool) {
    // Shared search control for cancellation across messages
    let mut active_control: Option<Arc<SearchControl>> = None;

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let req: Result<ClientMessage, _> = serde_json::from_str(&text);
            match req {
                Ok(ClientMessage::Cancel) => {
                    if let Some(ref ctrl) = active_control {
                        ctrl.request_stop();
                    }
                }
                Ok(ClientMessage::Search { fen, level, moves }) => {
                    // Stop any previous search
                    if let Some(ref ctrl) = active_control {
                        ctrl.request_stop();
                    }

                    let control = Arc::new(SearchControl::new());
                    active_control = Some(Arc::clone(&control));

                    let (tx, mut rx) = mpsc::unbounded_channel::<serde_json::Value>();
                    
                    // Asynchronously fetch profile and memory before offloading CPU search
                    let mut custom_config = axiorynth_engine::eval::get_config();
                    let mut adjustments = std::collections::HashMap::new();

                    if let Ok(Some(prof)) = crate::db::get_profile(&pool, "default").await {
                        if prof.wins > prof.losses + 2 {
                            custom_config.center_occupancy = 14;
                            custom_config.mobility_multiplier = 3;
                        } else if prof.losses > prof.wins + 2 {
                            custom_config.center_occupancy = 6;
                            custom_config.mobility_multiplier = 1;
                        }
                    }
                    
                    if let Ok(Some(mem)) = crate::db::get_bot_memory(&pool, "default").await {
                        if let Ok(adj) = serde_json::from_str::<std::collections::HashMap<String, i32>>(&mem.bot_adjustments) {
                            adjustments = adj;
                        }
                    }
                    
                    // Spawn the searching CPU task
                    let fen_clone = fen.clone();
                    let search_control = Arc::clone(&control);
                    let search_thread = tokio::task::spawn_blocking(move || {
                        axiorynth_engine::eval::update_config(custom_config);
                        
                        let history_context = moves.join("_");

                        let board = match Board::from_fen(&fen_clone) {
                            Ok(b) => b,
                            Err(_) => {
                                let _ = tx.send(json!({
                                    "type": "error",
                                    "message": "Invalid FEN string"
                                }));
                                return;
                            }
                        };
                        
                        let bot_lvl = BotLevel::new(level);
                        let start_time = Instant::now();
                        
                        let tx_cb = tx.clone();
                        let bot_move = choose_bot_move_with_callback(
                            &board,
                            bot_lvl,
                            &search_control,
                            &history_context,
                            &adjustments,
                            move |result: &axiorynth_engine::SearchResult, elapsed: Duration| {
                                let total_nodes = result.stats.nodes + result.stats.qnodes;
                                let elapsed_sec = elapsed.as_secs_f64();
                                let nps = if elapsed_sec > 0.0001 {
                                    (total_nodes as f64 / elapsed_sec) as u64
                                } else {
                                    0
                                };
                                
                                let pv_strings: Vec<String> = result.principal_variation
                                    .iter()
                                    .map(|mv| mv.uci())
                                    .collect();
                                
                                let best_move_str = result.best_move.map(|mv| mv.uci());
                                
                                let msg = json!({
                                    "type": "progress",
                                    "depth": result.depth,
                                    "best_move": best_move_str,
                                    "score": result.score,
                                    "pv": pv_strings,
                                    "nodes": result.stats.nodes,
                                    "qnodes": result.stats.qnodes,
                                    "nps": nps,
                                    "tt_hits": result.stats.tt_hits,
                                    "tt_stores": result.stats.tt_stores,
                                    "hashfull": result.stats.hashfull_permill,
                                    "beta_cutoffs": result.stats.beta_cutoffs,
                                    "q_beta_cutoffs": result.stats.q_beta_cutoffs,
                                    "killer_uses": result.stats.killer_uses,
                                    "elapsed_ms": elapsed.as_millis() as u64
                                });
                                
                                let _ = tx_cb.send(msg);
                            }
                        );
                        
                        let total_nodes = bot_move.search.stats.nodes + bot_move.search.stats.qnodes;
                        let elapsed = start_time.elapsed();
                        let elapsed_sec = elapsed.as_secs_f64();
                        let nps = if elapsed_sec > 0.0001 {
                            (total_nodes as f64 / elapsed_sec) as u64
                        } else {
                            0
                        };
                        
                        let selected_move_str = bot_move.selected_move.map(|mv| mv.uci());
                        let best_move_str = bot_move.search.best_move.map(|mv| mv.uci());
                        let pv_strings: Vec<String> = bot_move.search.principal_variation
                            .iter()
                            .map(|mv| mv.uci())
                            .collect();
                        
                        let result_msg = json!({
                            "type": "result",
                            "selected_move": selected_move_str,
                            "best_move": best_move_str,
                            "score": bot_move.search.score,
                            "pv": pv_strings,
                            "nodes": total_nodes,
                            "nps": nps,
                            "elapsed_ms": elapsed.as_millis() as u64
                        });
                        
                        let _ = tx.send(result_msg);
                    });
                    
                    // Consume the channel outputs and send them back to the client
                    while let Some(val) = rx.recv().await {
                        let text_val = val.to_string();
                        if socket.send(Message::Text(text_val.into())).await.is_err() {
                            break;
                        }
                    }
                    
                    let _ = search_thread.await;
                }
                Err(err) => {
                    let err_msg = json!({
                        "type": "error",
                        "message": format!("Invalid message: {}", err)
                    }).to_string();
                    if socket.send(Message::Text(err_msg.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}
