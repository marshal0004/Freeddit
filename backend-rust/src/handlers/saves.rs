//! Saved/hidden posts handlers — Reddit's "Save" and "Hide" features.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde_json::json;

use crate::{auth::AuthUser, errors::ApiError, AppState};

pub async fn save_post(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(post_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM posts WHERE id = ? AND is_removed = 0 AND is_deleted = 0")
        .bind(&post_id)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_none() {
        return Err(ApiError::NotFound(format!("post {}", post_id)));
    }
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT OR IGNORE INTO saved_posts (user_id, post_id, created_at) VALUES (?, ?, ?)")
        .bind(&claims.sub)
        .bind(&post_id)
        .bind(&now)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "saved": true, "post_id": post_id })))
}

pub async fn unsave_post(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(post_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM saved_posts WHERE user_id = ? AND post_id = ?")
        .bind(&claims.sub)
        .bind(&post_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "saved": false, "post_id": post_id })))
}

pub async fn hide_post(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(post_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT OR IGNORE INTO hidden_posts (user_id, post_id, created_at) VALUES (?, ?, ?)")
        .bind(&claims.sub)
        .bind(&post_id)
        .bind(&now)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "hidden": true, "post_id": post_id })))
}

pub async fn unhide_post(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(post_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM hidden_posts WHERE user_id = ? AND post_id = ?")
        .bind(&claims.sub)
        .bind(&post_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "hidden": false, "post_id": post_id })))
}

pub async fn list_saved(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT p.id FROM saved_posts sp JOIN posts p ON p.id = sp.post_id
         WHERE sp.user_id = ? AND p.is_removed = 0 AND p.is_deleted = 0
         ORDER BY sp.created_at DESC LIMIT 100",
    )
    .bind(&claims.sub)
    .fetch_all(&state.pool)
    .await?;
    let ids: Vec<String> = rows.into_iter().map(|(id,)| id).collect();
    Ok(Json(json!({ "saved_post_ids": ids })))
}

pub async fn list_hidden(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT post_id FROM hidden_posts WHERE user_id = ?")
        .bind(&claims.sub)
        .fetch_all(&state.pool)
        .await?;
    let ids: Vec<String> = rows.into_iter().map(|(id,)| id).collect();
    Ok(Json(json!({ "hidden_post_ids": ids })))
}
