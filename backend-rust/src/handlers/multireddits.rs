//! Multireddits / custom feeds handler — Reddit's r/user/m/feedname feature.
//! Lets users create personal feeds that aggregate posts from multiple subreddits.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::{auth::AuthUser, errors::ApiError, AppState};

#[derive(Debug, Deserialize)]
pub struct CreateMultiredditRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,  // 'private' | 'public'
    #[serde(default)]
    pub subreddits: Vec<String>,  // subreddit names
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateMultiredditRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 21 {
        return Err(ApiError::BadRequest("name must be 1-21 chars".into()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(ApiError::BadRequest("name may only contain letters, digits, _ or -".into()));
    }
    let visibility = req.visibility.unwrap_or_else(|| "private".into());
    if !matches!(visibility.as_str(), "private" | "public") {
        return Err(ApiError::BadRequest("visibility must be 'private' or 'public'".into()));
    }

    // Resolve subreddit names to IDs
    let mut sub_ids: Vec<String> = Vec::new();
    for sub_name in &req.subreddits {
        let row: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
            .bind(sub_name)
            .fetch_optional(&state.pool)
            .await?;
        if let Some((id,)) = row {
            sub_ids.push(id);
        }
    }
    let subreddit_ids_str = sub_ids.join(",");

    let id = crate::auth::generate_fullname("mr");
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO multireddits (id, user_id, name, description, visibility, subreddit_ids, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(name)
    .bind(&req.description)
    .bind(&visibility)
    .bind(&subreddit_ids_str)
    .bind(&now)
    .bind(&now)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "id": id,
        "name": name,
        "description": req.description,
        "visibility": visibility,
        "subreddits": req.subreddits,
        "created_at": now,
    })))
}

pub async fn list_mine(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, name, COALESCE(description, ''), visibility, subreddit_ids, created_at, updated_at
         FROM multireddits WHERE user_id = ? ORDER BY updated_at DESC",
    )
    .bind(&claims.sub)
    .fetch_all(&state.pool)
    .await?;

    let multireddits: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let id: String = r.try_get("id").unwrap_or_default();
            let name: String = r.try_get("name").unwrap_or_default();
            let desc: String = r.try_get("description").unwrap_or_default();
            let vis: String = r.try_get("visibility").unwrap_or_default();
            let sub_ids: String = r.try_get("subreddit_ids").unwrap_or_default();
            let created: String = r.try_get("created_at").unwrap_or_default();
            let updated: String = r.try_get("updated_at").unwrap_or_default();
            json!({
                "id": id,
                "name": name,
                "description": if desc.is_empty() { None } else { Some(desc) },
                "visibility": vis,
                "subreddit_ids": sub_ids.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
                "created_at": created,
                "updated_at": updated,
            })
        })
        .collect();
    Ok(Json(json!({ "multireddits": multireddits })))
}

pub async fn get_by_name(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, name, COALESCE(description, ''), visibility, subreddit_ids, created_at
         FROM multireddits WHERE user_id = ? AND name = ?",
    )
    .bind(&claims.sub)
    .bind(&name)
    .fetch_optional(&state.pool)
    .await?;
    let Some(r) = row else {
        return Err(ApiError::NotFound(format!("multireddit m/{}", name)));
    };
    let id: String = r.try_get("id").unwrap_or_default();
    let vis: String = r.try_get("visibility").unwrap_or_default();
    let sub_ids: String = r.try_get("subreddit_ids").unwrap_or_default();
    let created: String = r.try_get("created_at").unwrap_or_default();
    Ok(Json(json!({
        "id": id,
        "name": name,
        "visibility": vis,
        "subreddit_ids": sub_ids.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
        "created_at": created,
    })))
}

pub async fn delete_multireddit(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM multireddits WHERE user_id = ? AND name = ?")
        .bind(&claims.sub)
        .bind(&name)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "deleted": true, "name": name })))
}

pub async fn list_posts(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT subreddit_ids FROM multireddits WHERE user_id = ? AND name = ?",
    )
    .bind(&claims.sub)
    .bind(&name)
    .fetch_optional(&state.pool)
    .await?;
    let Some((sub_ids_str,)) = row else {
        return Err(ApiError::NotFound(format!("multireddit m/{}", name)));
    };
    if sub_ids_str.is_empty() {
        return Ok(Json(json!({ "posts": [], "dist": 0 })));
    }
    // Build a parameterized query for posts in any of the subscribed subreddits.
    let ids: Vec<&str> = sub_ids_str.split(',').filter(|s| !s.is_empty()).collect();
    let placeholders: Vec<String> = (0..ids.len()).map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT p.id, s.name AS subreddit, p.title, p.score, p.ups, p.downs, p.num_comments,
                p.kind, p.url, COALESCE(p.body, ''), p.created_at, p.is_sticky, p.is_nsfw
         FROM posts p JOIN subreddits s ON s.id = p.subreddit_id
         WHERE p.subreddit_id IN ({}) AND p.is_removed = 0 AND p.is_deleted = 0
         ORDER BY p.is_sticky DESC, p.hot_score DESC LIMIT 25",
        placeholders.join(", ")
    );
    let mut q = sqlx::query(&sql);
    for id in &ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(&state.pool).await?;
    let posts: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let id: String = r.try_get("id").unwrap_or_default();
            let subreddit: String = r.try_get("subreddit").unwrap_or_default();
            let title: String = r.try_get("title").unwrap_or_default();
            let score: i64 = r.try_get("score").unwrap_or(0);
            let ups: i64 = r.try_get("ups").unwrap_or(0);
            let downs: i64 = r.try_get("downs").unwrap_or(0);
            let num_comments: i64 = r.try_get("num_comments").unwrap_or(0);
            let kind: String = r.try_get("kind").unwrap_or_else(|_| "self".into());
            let url: Option<String> = r.try_get("url").ok().flatten();
            let body: String = r.try_get("body").unwrap_or_default();
            let created: String = r.try_get("created_at").unwrap_or_default();
            let is_sticky: bool = r.try_get::<i64, _>("is_sticky").unwrap_or(0) != 0;
            let is_nsfw: bool = r.try_get::<i64, _>("is_nsfw").unwrap_or(0) != 0;
            json!({
                "id": id, "name": id, "subreddit": subreddit, "subreddit_name_prefixed": format!("r/{}", subreddit),
                "title": title, "score": score, "ups": ups, "downs": downs,
                "num_comments": num_comments, "kind": kind,
                "url": if kind == "self" { None } else { url },
                "body": if kind == "self" { Some(body) } else { None },
                "created_at": created, "is_sticky": is_sticky, "over18": is_nsfw, "is_self": kind == "self",
            })
        })
        .collect();
    let dist = posts.len();
    Ok(Json(json!({
        "kind": "Listing",
        "data": { "after": null, "before": null, "children": posts.into_iter().map(|p| json!({"kind":"t3","data":p})).collect::<Vec<_>>(), "dist": dist }
    })))
}
