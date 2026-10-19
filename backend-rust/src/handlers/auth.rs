//! Auth handlers — register, login, me.

use axum::{extract::State, Json};
use chrono::Utc;
use serde_json::json;

use crate::{
    auth::{self, AuthUser},
    errors::ApiError,
    models::{AuthResponse, AuthUser as AuthUserModel, LoginRequest, RegisterRequest},
    AppState,
};

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    if req.username.len() < 3 || req.username.len() > 20 {
        return Err(ApiError::BadRequest("username must be 3-20 chars".into()));
    }
    if !req.username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(ApiError::BadRequest("username may only contain letters, digits, _ or -".into()));
    }
    if req.password.len() < 6 {
        return Err(ApiError::BadRequest("password must be at least 6 chars".into()));
    }

    // Check uniqueness
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT username FROM users WHERE username = ?")
            .bind(&req.username)
            .fetch_optional(&state.pool)
            .await?;
    if existing.is_some() {
        return Err(ApiError::Conflict("username already taken".into()));
    }

    let id = auth::generate_fullname("t2");
    let now = Utc::now();
    let hash = auth::hash_password(&req.password)?;

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, post_karma, comment_karma, is_admin, created_at, updated_at)
         VALUES (?, ?, ?, ?, 1, 1, 0, ?, ?)",
    )
    .bind(&id)
    .bind(&req.username)
    .bind(&req.email)
    .bind(&hash)
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(&state.pool)
    .await?;

    let access = auth::issue_access_token(&id, &req.username, false, &state.jwt_secret)?;
    let refresh = auth::generate_fullname("rt");

    sqlx::query(
        "INSERT INTO sessions (id, user_id, refresh_token, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(auth::generate_fullname("s"))
    .bind(&id)
    .bind(&refresh)
    .bind(now.to_rfc3339())
    .bind((now + chrono::Duration::days(30)).to_rfc3339())
    .execute(&state.pool)
    .await?;

    let user = AuthUserModel {
        id: id.clone(),
        username: req.username.clone(),
        email: req.email.clone(),
        is_admin: false,
        post_karma: 1,
        comment_karma: 1,
    };

    Ok(Json(AuthResponse {
        access_token: access,
        refresh_token: refresh,
        expires_in: 900,
        token_type: "Bearer".into(),
        user,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let row: Option<(String, String, String, i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT id, password_hash, username, post_karma, comment_karma, is_admin, 1
         FROM users WHERE username = ? AND is_deleted = 0",
    )
    .bind(&req.username)
    .fetch_optional(&state.pool)
    .await?;

    let Some((id, hash, username, pk, ck, is_admin, _)) = row else {
        return Err(ApiError::Unauthorized("invalid username or password".into()));
    };

    if !auth::verify_password(&req.password, &hash)? {
        return Err(ApiError::Unauthorized("invalid username or password".into()));
    }

    let access = auth::issue_access_token(&id, &username, is_admin != 0, &state.jwt_secret)?;
    let refresh = auth::generate_fullname("rt");
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO sessions (id, user_id, refresh_token, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(auth::generate_fullname("s"))
    .bind(&id)
    .bind(&refresh)
    .bind(now.to_rfc3339())
    .bind((now + chrono::Duration::days(30)).to_rfc3339())
    .execute(&state.pool)
    .await?;

    Ok(Json(AuthResponse {
        access_token: access,
        refresh_token: refresh,
        expires_in: 900,
        token_type: "Bearer".into(),
        user: AuthUserModel {
            id,
            username,
            email: None,
            is_admin: is_admin != 0,
            post_karma: pk,
            comment_karma: ck,
        },
    }))
}

pub async fn me(
    State(_state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(json!({
        "id": claims.sub,
        "username": claims.username,
        "is_admin": claims.is_admin,
    })))
}
