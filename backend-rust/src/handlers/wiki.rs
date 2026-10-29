//! Subreddit wiki handler — Reddit's r/sub/wiki feature.
//! Wiki pages support markdown body and have edit history.

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
pub struct CreateWikiPageRequest {
    pub slug: String,
    pub title: String,
    pub content: String,
}

pub async fn list_pages(
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
        "SELECT slug, title, updated_at FROM subreddit_wiki_pages WHERE subreddit_id = ? ORDER BY slug ASC",
    )
    .bind(&sub_id)
    .fetch_all(&state.pool)
    .await?;
    let pages: Vec<serde_json::Value> = rows.iter().map(|r| {
        let slug: String = r.try_get("slug").unwrap_or_default();
        let title: String = r.try_get("title").unwrap_or_default();
        let updated: String = r.try_get("updated_at").unwrap_or_default();
        json!({ "slug": slug, "title": title, "updated_at": updated })
    }).collect();
    Ok(Json(json!({ "subreddit": sub_name, "pages": pages })))
}

pub async fn get_page(
    State(state): State<AppState>,
    Path((sub_name, slug)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sub: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
        .bind(&sub_name)
        .fetch_optional(&state.pool)
        .await?;
    let Some((sub_id,)) = sub else {
        return Err(ApiError::NotFound(format!("r/{}", sub_name)));
    };
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, title, content, history, updated_at FROM subreddit_wiki_pages WHERE subreddit_id = ? AND slug = ?",
    )
    .bind(&sub_id).bind(&slug)
    .fetch_optional(&state.pool)
    .await?;
    let Some(r) = row else {
        return Err(ApiError::NotFound(format!("wiki page: {}", slug)));
    };
    let id: String = r.try_get("id").unwrap_or_default();
    let title: String = r.try_get("title").unwrap_or_default();
    let content: String = r.try_get("content").unwrap_or_default();
    let history: String = r.try_get("history").unwrap_or_default();
    let updated: String = r.try_get("updated_at").unwrap_or_default();
    Ok(Json(json!({
        "id": id,
        "subreddit": sub_name,
        "slug": slug,
        "title": title,
        "content": content,
        "history": history,
        "updated_at": updated,
    })))
}

pub async fn upsert_page(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path((sub_name, _slug_from_path)): Path<(String, String)>,
    Json(req): Json<CreateWikiPageRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let slug = req.slug.trim().to_lowercase();
    if slug.is_empty() || slug.len() > 64 {
        return Err(ApiError::BadRequest("slug must be 1-64 chars".into()));
    }
    if !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(ApiError::BadRequest("slug may only contain letters, digits, -, _".into()));
    }
    if req.title.is_empty() || req.title.len() > 200 {
        return Err(ApiError::BadRequest("title must be 1-200 chars".into()));
    }

    let sub: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
        .bind(&sub_name)
        .fetch_optional(&state.pool)
        .await?;
    let Some((sub_id,)) = sub else {
        return Err(ApiError::NotFound(format!("r/{}", sub_name)));
    };

    // Check if mod (any of: subreddit_moderators, legacy subscriptions table, or admin)
    let is_admin = sqlx::query_as::<_, (i64,)>("SELECT is_admin FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_optional(&state.pool)
        .await?
        .map(|(a,)| a != 0)
        .unwrap_or(false);
    let is_mod: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM subreddit_moderators WHERE subreddit_id = ? AND user_id = ?",
    )
    .bind(&sub_id).bind(&claims.sub)
    .fetch_optional(&state.pool).await?;
    let legacy_mod: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM subscriptions WHERE subreddit_id = ? AND user_id = ? AND role = 'moderator'",
    )
    .bind(&sub_id).bind(&claims.sub)
    .fetch_optional(&state.pool).await?;
    if !is_admin && is_mod.is_none() && legacy_mod.is_none() {
        return Err(ApiError::Forbidden("moderator access required to edit wiki".into()));
    }

    let now = Utc::now().to_rfc3339();
    let id = crate::auth::generate_fullname("wiki");

    // Try INSERT, fall back to UPDATE if exists
    let res = sqlx::query(
        "INSERT INTO subreddit_wiki_pages (id, subreddit_id, slug, title, content, history, last_editor_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, '[]', ?, ?, ?)",
    )
    .bind(&id).bind(&sub_id).bind(&slug).bind(&req.title).bind(&req.content).bind(&claims.sub).bind(&now).bind(&now)
    .execute(&state.pool).await;

    if res.is_err() {
        // Page already exists — append to history and update
        let existing: Option<(String, String, String)> = sqlx::query_as(
            "SELECT title, content, history FROM subreddit_wiki_pages WHERE subreddit_id = ? AND slug = ?",
        )
        .bind(&sub_id).bind(&slug)
        .fetch_optional(&state.pool).await?;
        let Some((prev_title, prev_content, prev_history)) = existing else {
            return Err(ApiError::Internal("wiki page vanished mid-upsert".into()));
        };
        // Append prev revision to history
        let mut history: Vec<serde_json::Value> = serde_json::from_str(&prev_history).unwrap_or_default();
        history.push(json!({
            "title": prev_title,
            "content": prev_content,
            "edited_by": claims.sub,
            "edited_at": now,
        }));
        // Cap at 20 revisions
        if history.len() > 20 {
            history = history.split_off(history.len() - 20);
        }
        let history_str = serde_json::to_string(&history).unwrap_or_else(|_| "[]".into());

        sqlx::query(
            "UPDATE subreddit_wiki_pages SET title = ?, content = ?, history = ?, last_editor_id = ?, updated_at = ? WHERE subreddit_id = ? AND slug = ?",
        )
        .bind(&req.title).bind(&req.content).bind(&history_str).bind(&claims.sub).bind(&now).bind(&sub_id).bind(&slug)
        .execute(&state.pool).await?;
    }

    Ok(Json(json!({
        "id": id,
        "subreddit": sub_name,
        "slug": slug,
        "title": req.title,
        "content": req.content,
        "updated_at": now,
        "editor": claims.sub,
    })))
}

pub async fn delete_page(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path((sub_name, slug)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sub: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
        .bind(&sub_name)
        .fetch_optional(&state.pool)
        .await?;
    let Some((sub_id,)) = sub else {
        return Err(ApiError::NotFound(format!("r/{}", sub_name)));
    };
    sqlx::query("DELETE FROM subreddit_wiki_pages WHERE subreddit_id = ? AND slug = ?")
        .bind(&sub_id).bind(&slug)
        .execute(&state.pool).await?;
    Ok(Json(json!({ "deleted": true, "subreddit": sub_name, "slug": slug })))
}
