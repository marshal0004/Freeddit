//! Mod tools handler — sticky/lock/remove posts, subreddit rules, moderator management.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::{auth::AuthUser, errors::ApiError, AppState};

async fn check_mod(state: &AppState, user_id: &str, sub_name: &str) -> Result<String, ApiError> {
    let sub: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
        .bind(sub_name)
        .fetch_optional(&state.pool)
        .await?;
    let Some((sub_id,)) = sub else {
        return Err(ApiError::NotFound(format!("r/{}", sub_name)));
    };
    let mod_check: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM subreddit_moderators WHERE subreddit_id = ? AND user_id = ?",
    )
    .bind(&sub_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;
    if mod_check.is_none() {
        let legacy: Option<(String,)> = sqlx::query_as(
            "SELECT user_id FROM subscriptions WHERE subreddit_id = ? AND user_id = ? AND role = 'moderator'",
        )
        .bind(&sub_id)
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await?;
        if legacy.is_none() {
            return Err(ApiError::Forbidden("moderator access required".into()));
        }
    }
    Ok(sub_id)
}

#[derive(Debug, Deserialize)]
pub struct ModActionRequest {
    pub subreddit: String,
    #[serde(default)]
    pub action: Option<String>,  // sticky | unsticky | lock | unlock | remove | approve
}

pub async fn mod_action(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(post_id): Path<String>,
    Json(req): Json<ModActionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _sub_id = check_mod(&state, &claims.sub, &req.subreddit).await?;
    let action = req.action.unwrap_or_else(|| "sticky".into());
    let (field, value): (&str, i64) = match action.as_str() {
        "sticky" => ("is_sticky", 1),
        "unsticky" => ("is_sticky", 0),
        "lock" => ("is_locked", 1),
        "unlock" => ("is_locked", 0),
        "remove" => ("is_removed", 1),
        "approve" => ("is_removed", 0),
        _ => return Err(ApiError::BadRequest(format!("unknown action: {}", action))),
    };
    let sql = format!("UPDATE posts SET {} = ? WHERE id = ?", field);
    sqlx::query(&sql)
        .bind(value)
        .bind(&post_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "post_id": post_id, "action": action, "applied": true })))
}

pub async fn sticky_comment(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(comment_id): Path<String>,
    Json(req): Json<ModActionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _sub_id = check_mod(&state, &claims.sub, &req.subreddit).await?;
    let value: i64 = match req.action.as_deref().unwrap_or("sticky") {
        "unsticky" => 0,
        _ => 1,
    };
    sqlx::query("UPDATE comments SET is_sticky = ? WHERE id = ?")
        .bind(value)
        .bind(&comment_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "comment_id": comment_id, "is_sticky": value != 0 })))
}

// ===== Subreddit rules =====

#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub short_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub priority: Option<i32>,
}

pub async fn list_rules(
    State(state): State<AppState>,
    Path(sub_name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sub: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
        .bind(&sub_name)
        .fetch_optional(&state.pool)
        .await?;
    let Some((sub_id,)) = sub else {
        return Err(ApiError::NotFound(format!("r/{}", sub_name)));
    };
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, short_name, COALESCE(description, ''), priority, created_at FROM subreddit_rules WHERE subreddit_id = ? ORDER BY priority ASC",
    )
    .bind(&sub_id)
    .fetch_all(&state.pool)
    .await?;
    let rules: Vec<serde_json::Value> = rows.iter().map(|r| {
        let id: String = r.try_get("id").unwrap_or_default();
        let name: String = r.try_get("short_name").unwrap_or_default();
        let desc: String = r.try_get("description").unwrap_or_default();
        let prio: i32 = r.try_get("priority").unwrap_or(0);
        let created: String = r.try_get("created_at").unwrap_or_default();
        json!({
            "id": id,
            "short_name": name,
            "description": if desc.is_empty() { None } else { Some(desc) },
            "priority": prio,
            "created_at": created,
        })
    }).collect();
    Ok(Json(json!({ "subreddit": sub_name, "rules": rules })))
}

pub async fn create_rule(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(sub_name): Path<String>,
    Json(req): Json<CreateRuleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sub_id = check_mod(&state, &claims.sub, &sub_name).await?;
    if req.short_name.trim().is_empty() || req.short_name.len() > 100 {
        return Err(ApiError::BadRequest("short_name must be 1-100 chars".into()));
    }
    let id = crate::auth::generate_fullname("rule");
    let now = Utc::now().to_rfc3339();
    let prio = req.priority.unwrap_or(0);
    sqlx::query(
        "INSERT INTO subreddit_rules (id, subreddit_id, short_name, description, priority, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&sub_id)
    .bind(&req.short_name)
    .bind(&req.description)
    .bind(prio)
    .bind(&now)
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({
        "id": id,
        "short_name": req.short_name,
        "description": req.description,
        "priority": prio,
        "created_at": now,
    })))
}

pub async fn delete_rule(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path((sub_name, rule_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _sub_id = check_mod(&state, &claims.sub, &sub_name).await?;
    sqlx::query("DELETE FROM subreddit_rules WHERE id = ?")
        .bind(&rule_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "deleted": true, "rule_id": rule_id })))
}

// ===== Moderators list =====

pub async fn list_moderators(
    State(state): State<AppState>,
    Path(sub_name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sub: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
        .bind(&sub_name)
        .fetch_optional(&state.pool)
        .await?;
    let Some((sub_id,)) = sub else {
        return Err(ApiError::NotFound(format!("r/{}", sub_name)));
    };

    // Try the explicit moderators table first
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT u.username, COALESCE(sm.permissions, 'all'), sm.added_at
         FROM subreddit_moderators sm JOIN users u ON u.id = sm.user_id
         WHERE sm.subreddit_id = ? ORDER BY sm.added_at ASC",
    )
    .bind(&sub_id)
    .fetch_all(&state.pool)
    .await?;

    let mods: Vec<serde_json::Value> = if !rows.is_empty() {
        rows.into_iter().map(|r| json!({
            "username": r.0,
            "permissions": r.1,
            "added_at": r.2,
        })).collect()
    } else {
        // Fallback to the legacy subscriptions table (moderators from seed)
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT u.username, s.created_at FROM subscriptions s JOIN users u ON u.id = s.user_id
             WHERE s.subreddit_id = ? AND s.role = 'moderator' ORDER BY s.created_at ASC",
        )
        .bind(&sub_id)
        .fetch_all(&state.pool)
        .await?;
        rows.into_iter().map(|r| json!({
            "username": r.0,
            "permissions": "all",
            "added_at": r.1,
        })).collect()
    };
    Ok(Json(json!({ "subreddit": sub_name, "moderators": mods })))
}
