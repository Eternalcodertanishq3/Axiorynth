use axum::{
    extract::{FromRequestParts, Json},
    http::{request::Parts, StatusCode},
};
use sqlx::SqlitePool;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::time::SystemTime;

// Session store: maps session_token -> SessionEntry
#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub user_id: String,
    pub expires_at: u64,
}
pub type SessionStore = Arc<RwLock<HashMap<String, SessionEntry>>>;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub rating: i32,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserPublic,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserPublic {
    pub id: String,
    pub username: String,
    pub rating: i32,
}

#[allow(dead_code)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
}

use sha2::{Sha256, Digest};
use rand::RngCore;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2.hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string()
}

fn compute_salted_hash(password: &str, salt_hex: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt_hex.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    let mut result = hasher.finalize();
    for _ in 1..1000 {
        let mut h = Sha256::new();
        h.update(result);
        h.update(salt_hex.as_bytes());
        result = h.finalize();
    }
    hex_encode(&result)
}

pub fn needs_migration(stored: &str) -> bool {
    !stored.starts_with("$argon2id$")
}

pub fn verify_password(password: &str, stored: &str) -> bool {
    if stored.starts_with("$argon2id$") {
        let parsed_hash = match PasswordHash::new(stored) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
    } else if let Some((salt, hash)) = stored.split_once(':') {
        let computed = compute_salted_hash(password, salt);
        computed == hash
    } else {
        // Backward-compatible verification for legacy hashes
        let salted = format!("axiorynth_v1_salt:{}", password);
        let mut hasher = DefaultHasher::new();
        salted.hash(&mut hasher);
        format!("{:016x}", hasher.finish()) == stored
    }
}

pub fn generate_session_token() -> String {
    let mut token_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut token_bytes);
    hex_encode(&token_bytes)
}

#[allow(dead_code)]
pub fn validate_token(sessions: &SessionStore, token: &str) -> Option<String> {
    let guard = sessions.read().ok()?;
    let entry = guard.get(token)?;
    
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
        
    if now > entry.expires_at {
        return None;
    }
    
    Some(entry.user_id.clone())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub async fn register(
    pool: &SqlitePool,
    sessions: &SessionStore,
    payload: RegisterRequest,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    if payload.username.trim().is_empty() || payload.password.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Username and password cannot be empty".to_string()));
    }
    
    if payload.username.len() < 3 || payload.username.len() > 24 {
        return Err((StatusCode::BAD_REQUEST, "Username must be 3-24 characters".to_string()));
    }
    if !payload.username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err((StatusCode::BAD_REQUEST, "Username can only contain alphanumeric characters and underscores".to_string()));
    }
    if payload.password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, "Password must be at least 8 characters".to_string()));
    }

    // Check if user already exists
    let existing = crate::db::get_user_by_username(pool, &payload.username).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if existing.is_some() {
        return Err((StatusCode::CONFLICT, "Username already exists".to_string()));
    }

    let user_id = format!("user_{}", generate_session_token());
    let password_hash = hash_password(&payload.password);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let user = User {
        id: user_id.clone(),
        username: payload.username.clone(),
        password_hash,
        rating: 1200,
        created_at: now.to_string(),
    };

    crate::db::create_user(pool, &user).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let token = generate_session_token();
    let expires_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() + 24 * 3600;
        
    {
        let mut session_guard = sessions.write().map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to acquire session lock".to_string())
        })?;
        session_guard.insert(token.clone(), SessionEntry { user_id: user_id.clone(), expires_at });
    }

    Ok(Json(AuthResponse {
        token,
        user: UserPublic {
            id: user.id,
            username: user.username,
            rating: user.rating,
        },
    }))
}

pub async fn login(
    pool: &SqlitePool,
    sessions: &SessionStore,
    payload: LoginRequest,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let user = crate::db::get_user_by_username(pool, &payload.username).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()))?;

    if !verify_password(&payload.password, &user.password_hash) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()));
    }
    
    if needs_migration(&user.password_hash) {
        let new_hash = hash_password(&payload.password);
        let _ = crate::db::update_password_hash(pool, &user.id, &new_hash).await;
    }

    let token = generate_session_token();
    let expires_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() + 24 * 3600;
        
    {
        let mut session_guard = sessions.write().map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to acquire session lock".to_string())
        })?;
        session_guard.insert(token.clone(), SessionEntry { user_id: user.id.clone(), expires_at });
    }

    Ok(Json(AuthResponse {
        token,
        user: UserPublic {
            id: user.id,
            username: user.username,
            rating: user.rating,
        },
    }))
}

pub async fn get_me(
    pool: &SqlitePool,
    sessions: &SessionStore,
    token: &str,
) -> Result<Json<UserPublic>, (StatusCode, String)> {
    let user_id = {
        let session_guard = sessions.read().map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to acquire session lock".to_string())
        })?;
        let entry = session_guard.get(token).ok_or((StatusCode::UNAUTHORIZED, "Invalid session".to_string()))?;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now > entry.expires_at {
            return Err((StatusCode::UNAUTHORIZED, "Session expired".to_string()));
        }
        entry.user_id.clone()
    };

    let user = crate::db::get_user_by_id(pool, &user_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    Ok(Json(UserPublic {
        id: user.id,
        username: user.username,
        rating: user.rating,
    }))
}

pub async fn logout(
    sessions: &SessionStore,
    token: &str,
) -> Result<(), (StatusCode, String)> {
    let mut session_guard = sessions.write().map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to acquire session lock".to_string())
    })?;
    session_guard.remove(token);
    Ok(())
}

#[axum::async_trait]
impl FromRequestParts<crate::AppState> for AuthUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &crate::AppState) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers.get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err((StatusCode::UNAUTHORIZED, "Invalid Authorization header format".to_string()));
        }

        let token = &auth_header[7..];

        let user_id = {
            let sessions = state.sessions.read().map_err(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read sessions".to_string())
            })?;
            let entry = sessions.get(token).ok_or((StatusCode::UNAUTHORIZED, "Invalid or expired session".to_string()))?;
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if now > entry.expires_at {
                return Err((StatusCode::UNAUTHORIZED, "Session expired".to_string()));
            }
            entry.user_id.clone()
        };

        let user = crate::db::get_user_by_id(&state.pool, &user_id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::UNAUTHORIZED, "User not found".to_string()))?;

        Ok(AuthUser {
            user_id: user.id,
            username: user.username,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "SuperSecretPassword123!";
        let hash = hash_password(password);
        assert!(verify_password(password, &hash));
        assert!(!verify_password("WrongPassword", &hash));
    }

    #[test]
    fn test_session_token_entropy() {
        let token1 = generate_session_token();
        let token2 = generate_session_token();
        assert_ne!(token1, token2);
        assert_eq!(token1.len(), 64);
    }
}
