//! Notifications handler — Reddit's bell icon inbox.
//! Generates notifications on comment replies, mentions, upvote milestones, awards, mod actions.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::{auth::AuthUser, errors::ApiError, AppState};

#[derive(Debug, Deserialize)]
pub struct NotifListQuery {
    #[serde(default)]
    pub unread_only: Option<bool>,
    #[serde(default)]
    pub limit: Option<i64>,
}

pub async fn list_notifications(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(q): Query<NotifListQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let unread_only = q.unread_only.unwrap_or(false);
    let sql = if unread_only {
        "SELECT id, kind, target_id, target_type, actor_id, body, is_read, created_at
         FROM notifications WHERE user_id = ? AND is_read = 0
         ORDER BY created_at DESC LIMIT ?"
    } else {
        "SELECT id, kind, target_id, target_type, actor_id, body, is_read, created_at
         FROM notifications WHERE user_id = ?
         ORDER BY created_at DESC LIMIT ?"
    };

    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(sql)
        .bind(&claims.sub)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;

    // Look up actor usernames
    let actor_ids: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            let aid: Option<String> = r.try_get("actor_id").ok().flatten();
            aid
        })
        .collect();
    let mut actor_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for aid in &actor_ids {
        if actor_map.contains_key(aid) {
            continue;
        }
        let row: Option<(String,)> = sqlx::query_as("SELECT username FROM users WHERE id = ?")
            .bind(aid)
            .fetch_optional(&state.pool)
            .await?;
        if let Some((name,)) = row {
            actor_map.insert(aid.clone(), name);
        }
    }

    let notifications: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let id: String = r.try_get("id").unwrap_or_default();
            let kind: String = r.try_get("kind").unwrap_or_else(|_| "unknown".into());
            let target_id: Option<String> = r.try_get("target_id").ok().flatten();
            let target_type: Option<String> = r.try_get("target_type").ok().flatten();
            let actor_id: Option<String> = r.try_get("actor_id").ok().flatten();
            let body: Option<String> = r.try_get("body").ok().flatten();
            let is_read: bool = r.try_get::<i64, _>("is_read").unwrap_or(0) != 0;
            let created: String = r.try_get("created_at").unwrap_or_default();
            let actor_name = actor_id.as_ref().and_then(|aid| actor_map.get(aid)).cloned();
            json!({
                "id": id,
                "kind": kind,
                "target_id": target_id,
                "target_type": target_type,
                "actor_id": actor_id,
                "actor_username": actor_name,
                "body": body,
                "is_read": is_read,
                "created_at": created,
            })
        })
        .collect();

    // Count unread
    let unread_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id = ? AND is_read = 0")
        .bind(&claims.sub)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(json!({
        "notifications": notifications,
        "unread_count": unread_count.0,
    })))
}

pub async fn mark_read(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(notif_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("UPDATE notifications SET is_read = 1 WHERE id = ? AND user_id = ?")
        .bind(&notif_id)
        .bind(&claims.sub)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "marked_read": true, "id": notif_id })))
}

pub async fn mark_all_read(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("UPDATE notifications SET is_read = 1 WHERE user_id = ? AND is_read = 0")
        .bind(&claims.sub)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "marked_all_read": true })))
}

/// Internal helper: create a notification for a user when their comment gets a reply.
/// Called by the comment creation handler.
pub async fn create_notification(
    state: &AppState,
    user_id: &str,
    kind: &str,
    target_id: Option<&str>,
    target_type: Option<&str>,
    actor_id: Option<&str>,
    body: Option<&str>,
) -> anyhow::Result<()> {
    let id = crate::auth::generate_fullname("notif");
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO notifications (id, user_id, kind, target_id, target_type, actor_id, body, is_read, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(kind)
    .bind(target_id)
    .bind(target_type)
    .bind(actor_id)
    .bind(body)
    .bind(&now)
    .execute(&state.pool)
    .await?;
    Ok(())
}

/// Generate notifications for parent comment author on reply.
/// Call this from `comments::create`.
pub async fn notify_comment_reply(
    state: &AppState,
    post_id: &str,
    parent_comment_id: Option<&str>,
    reply_author_id: &str,
    reply_body_preview: &str,
) -> anyhow::Result<()> {
    if let Some(pid) = parent_comment_id {
        let parent: Option<(String,)> =
            sqlx::query_as("SELECT author_id FROM comments WHERE id = ?")
                .bind(pid)
                .fetch_optional(&state.pool)
                .await?;
        if let Some((parent_author_id,)) = parent {
            // Don't notify if you reply to yourself
            if parent_author_id != reply_author_id {
                let body = format!("Replied: {}", &reply_body_preview.chars().take(80).collect::<String>());
                create_notification(
                    state,
                    &parent_author_id,
                    "comment_reply",
                    Some(pid),
                    Some("comment"),
                    Some(reply_author_id),
                    Some(&body),
                )
                .await?;
            }
        }
    }
    Ok(())
}
