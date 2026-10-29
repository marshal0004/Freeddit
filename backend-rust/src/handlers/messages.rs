//! Messages / Direct Messages handler — Reddit's PM/inbox feature.
//! Conversations are 1:1 (or up to 10 participants for group chats).
//! Messages support markdown body.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::{auth::AuthUser, errors::ApiError, AppState};

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub to: String,           // username
    #[serde(default)]
    pub subject: Option<String>,
    pub body: String,
}

pub async fn start_conversation(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateConversationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.body.trim().is_empty() {
        return Err(ApiError::BadRequest("body is required".into()));
    }
    if req.body.len() > 10000 {
        return Err(ApiError::BadRequest("body too long (max 10000 chars)".into()));
    }

    // Find recipient
    let recipient: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE username = ? AND is_deleted = 0")
        .bind(&req.to)
        .fetch_optional(&state.pool)
        .await?;
    let Some((recipient_id,)) = recipient else {
        return Err(ApiError::NotFound(format!("u/{}", req.to)));
    };
    if recipient_id == claims.sub {
        return Err(ApiError::BadRequest("cannot send a message to yourself".into()));
    }

    let conv_id = crate::auth::generate_fullname("conv");
    let msg_id = crate::auth::generate_fullname("msg");
    let now = Utc::now().to_rfc3339();

    // Create conversation
    sqlx::query("INSERT INTO conversations (id, created_at, last_message_at) VALUES (?, ?, ?)")
        .bind(&conv_id)
        .bind(&now)
        .bind(&now)
        .execute(&state.pool)
        .await?;

    // Add participants
    sqlx::query("INSERT INTO conversation_participants (conversation_id, user_id, joined_at) VALUES (?, ?, ?)")
        .bind(&conv_id).bind(&claims.sub).bind(&now)
        .execute(&state.pool).await?;
    sqlx::query("INSERT INTO conversation_participants (conversation_id, user_id, joined_at) VALUES (?, ?, ?)")
        .bind(&conv_id).bind(&recipient_id).bind(&now)
        .execute(&state.pool).await?;

    // First message
    sqlx::query("INSERT INTO messages (id, conversation_id, sender_id, body, is_read, created_at) VALUES (?, ?, ?, ?, 0, ?)")
        .bind(&msg_id).bind(&conv_id).bind(&claims.sub).bind(&req.body).bind(&now)
        .execute(&state.pool).await?;

    // Create notification for recipient
    crate::handlers::notifications::create_notification(
        &state,
        &recipient_id,
        "private_message",
        Some(&conv_id),
        Some("conversation"),
        Some(&claims.sub),
        Some(&format!("New message: {}", &req.body.chars().take(80).collect::<String>())),
    ).await.ok();

    Ok(Json(json!({
        "conversation_id": conv_id,
        "message_id": msg_id,
        "to": req.to,
        "subject": req.subject,
        "body": req.body,
        "sent_at": now,
    })))
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub body: String,
}

pub async fn send_message(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(conv_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.body.trim().is_empty() {
        return Err(ApiError::BadRequest("body is required".into()));
    }
    // Verify sender is in the conversation
    let participant: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM conversation_participants WHERE conversation_id = ? AND user_id = ?",
    )
    .bind(&conv_id)
    .bind(&claims.sub)
    .fetch_optional(&state.pool)
    .await?;
    if participant.is_none() {
        return Err(ApiError::Forbidden("not a participant in this conversation".into()));
    }

    let msg_id = crate::auth::generate_fullname("msg");
    let now = Utc::now().to_rfc3339();

    sqlx::query("INSERT INTO messages (id, conversation_id, sender_id, body, is_read, created_at) VALUES (?, ?, ?, ?, 0, ?)")
        .bind(&msg_id).bind(&conv_id).bind(&claims.sub).bind(&req.body).bind(&now)
        .execute(&state.pool).await?;

    // Update last_message_at
    sqlx::query("UPDATE conversations SET last_message_at = ? WHERE id = ?")
        .bind(&now).bind(&conv_id)
        .execute(&state.pool).await?;

    // Mark all previous messages in this conv as read for this user (they obviously read them when replying)
    sqlx::query("UPDATE messages SET is_read = 1 WHERE conversation_id = ? AND sender_id != ?")
        .bind(&conv_id).bind(&claims.sub)
        .execute(&state.pool).await?;

    // Notify other participants
    let others: Vec<(String,)> = sqlx::query_as(
        "SELECT user_id FROM conversation_participants WHERE conversation_id = ? AND user_id != ?",
    )
    .bind(&conv_id)
    .bind(&claims.sub)
    .fetch_all(&state.pool)
    .await?;
    let body_preview: String = req.body.chars().take(80).collect();
    for (other_id,) in &others {
        crate::handlers::notifications::create_notification(
            &state,
            other_id,
            "private_message",
            Some(&conv_id),
            Some("conversation"),
            Some(&claims.sub),
            Some(&format!("New message: {}", body_preview)),
        ).await.ok();
    }

    Ok(Json(json!({
        "message_id": msg_id,
        "conversation_id": conv_id,
        "body": req.body,
        "sent_at": now,
    })))
}

#[derive(Debug, Deserialize)]
pub struct ListConvQuery {
    #[serde(default)]
    pub limit: Option<i64>,
}

pub async fn list_conversations(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(q): Query<ListConvQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT c.id, c.last_message_at,
                (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.id AND m.sender_id != ? AND m.is_read = 0) AS unread
         FROM conversations c
         JOIN conversation_participants cp ON cp.conversation_id = c.id
         WHERE cp.user_id = ?
         ORDER BY c.last_message_at DESC LIMIT ?",
    )
    .bind(&claims.sub)
    .bind(&claims.sub)
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;

    let mut convs: Vec<serde_json::Value> = Vec::new();
    for r in &rows {
        let conv_id: String = r.try_get("id").unwrap_or_default();
        let last_msg: String = r.try_get("last_message_at").unwrap_or_default();
        let unread: i64 = r.try_get("unread").unwrap_or(0);

        // Get other participant(s)
        let others: Vec<(String,)> = sqlx::query_as(
            "SELECT u.username FROM conversation_participants cp
             JOIN users u ON u.id = cp.user_id
             WHERE cp.conversation_id = ? AND cp.user_id != ?",
        )
        .bind(&conv_id)
        .bind(&claims.sub)
        .fetch_all(&state.pool)
        .await?;
        let other_names: Vec<String> = others.into_iter().map(|(n,)| n).collect();

        // Get last message preview
        let last_msg_row: Option<(String, String, String)> = sqlx::query_as(
            "SELECT body, sender_id, created_at FROM messages WHERE conversation_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(&conv_id)
        .fetch_optional(&state.pool)
        .await?;
        let preview = last_msg_row.map(|(b, _, _)| b).unwrap_or_default();

        convs.push(json!({
            "id": conv_id,
            "other_participants": other_names,
            "last_message_at": last_msg,
            "unread_count": unread,
            "last_message_preview": preview,
        }));
    }

    Ok(Json(json!({ "conversations": convs })))
}

pub async fn get_conversation(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(conv_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify access
    let participant: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM conversation_participants WHERE conversation_id = ? AND user_id = ?",
    )
    .bind(&conv_id).bind(&claims.sub)
    .fetch_optional(&state.pool).await?;
    if participant.is_none() {
        return Err(ApiError::Forbidden("not a participant".into()));
    }

    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT m.id, m.sender_id, u.username AS sender_name, m.body, m.is_read, m.created_at
         FROM messages m JOIN users u ON u.id = m.sender_id
         WHERE m.conversation_id = ?
         ORDER BY m.created_at ASC LIMIT 200",
    )
    .bind(&conv_id)
    .fetch_all(&state.pool)
    .await?;

    let messages: Vec<serde_json::Value> = rows.iter().map(|r| {
        let id: String = r.try_get("id").unwrap_or_default();
        let sender_id: String = r.try_get("sender_id").unwrap_or_default();
        let sender_name: String = r.try_get("sender_name").unwrap_or_default();
        let body: String = r.try_get("body").unwrap_or_default();
        let is_read: bool = r.try_get::<i64, _>("is_read").unwrap_or(0) != 0;
        let created: String = r.try_get("created_at").unwrap_or_default();
        json!({
            "id": id,
            "sender_id": sender_id,
            "sender_name": sender_name,
            "body": body,
            "is_read": is_read,
            "created_at": created,
            "is_mine": sender_id == claims.sub,
        })
    }).collect();

    // Mark as read for this user
    sqlx::query("UPDATE messages SET is_read = 1 WHERE conversation_id = ? AND sender_id != ?")
        .bind(&conv_id).bind(&claims.sub)
        .execute(&state.pool).await?;

    Ok(Json(json!({
        "conversation_id": conv_id,
        "messages": messages,
    })))
}

pub async fn delete_conversation(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(conv_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Remove the user from participants
    sqlx::query("DELETE FROM conversation_participants WHERE conversation_id = ? AND user_id = ?")
        .bind(&conv_id).bind(&claims.sub)
        .execute(&state.pool).await?;
    Ok(Json(json!({ "deleted": true, "conversation_id": conv_id })))
}
