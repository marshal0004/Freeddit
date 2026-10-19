//! Reports handler — Reddit's report flow for posts and comments.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;

use crate::{auth::AuthUser, errors::ApiError, AppState};

#[derive(Debug, Deserialize)]
pub struct ReportRequest {
    pub target_id: String,
    pub target_type: String,    // 'post' | 'comment'
    pub reason: String,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub subreddit: Option<String>,
}

pub async fn create_report(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ReportRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !matches!(req.target_type.as_str(), "post" | "comment") {
        return Err(ApiError::BadRequest("target_type must be 'post' or 'comment'".into()));
    }
    if req.reason.trim().is_empty() {
        return Err(ApiError::BadRequest("reason is required".into()));
    }
    if req.reason.len() > 200 {
        return Err(ApiError::BadRequest("reason too long (max 200 chars)".into()));
    }

    // Look up subreddit id if provided
    let sub_id: Option<String> = if let Some(name) = &req.subreddit {
        let row: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
            .bind(name)
            .fetch_optional(&state.pool)
            .await?;
        row.map(|(id,)| id)
    } else { None };

    let id = crate::auth::generate_fullname("rep");
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO reports (id, reporter_id, target_id, target_type, subreddit_id, reason, details, status, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?)",
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&req.target_id)
    .bind(&req.target_type)
    .bind(&sub_id)
    .bind(&req.reason)
    .bind(&req.details)
    .bind(&now)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "report_id": id,
        "status": "pending",
        "target_id": req.target_id,
        "target_type": req.target_type,
    })))
}

pub async fn list_reports_for_subreddit(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(sub_name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify the user is a moderator of this subreddit
    let sub: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
        .bind(&sub_name)
        .fetch_optional(&state.pool)
        .await?;
    let Some((sub_id,)) = sub else {
        return Err(ApiError::NotFound(format!("r/{}", sub_name)));
    };
    let is_mod: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM subreddit_moderators WHERE subreddit_id = ? AND user_id = ?",
    )
    .bind(&sub_id)
    .bind(&claims.sub)
    .fetch_optional(&state.pool)
    .await?;
    if is_mod.is_none() {
        // Also check the legacy subscriptions table
        let sub_check: Option<(String,)> = sqlx::query_as(
            "SELECT user_id FROM subscriptions WHERE subreddit_id = ? AND user_id = ? AND role = 'moderator'",
        )
        .bind(&sub_id)
        .bind(&claims.sub)
        .fetch_optional(&state.pool)
        .await?;
        if sub_check.is_none() {
            return Err(ApiError::Forbidden("moderator access required".into()));
        }
    }

    let rows: Vec<(String, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, target_id, target_type, reason, COALESCE(details, ''), status, created_at
         FROM reports WHERE subreddit_id = ? ORDER BY created_at DESC LIMIT 100",
    )
    .bind(&sub_id)
    .fetch_all(&state.pool)
    .await?;
    let reports: Vec<serde_json::Value> = rows.into_iter().map(|r| json!({
        "id": r.0,
        "target_id": r.1,
        "target_type": r.2,
        "reason": r.3,
        "details": if r.4.is_empty() { None } else { Some(r.4) },
        "status": r.5,
        "created_at": r.6,
    })).collect();
    Ok(Json(json!({ "reports": reports })))
}

pub async fn dismiss_report(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(report_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify mod permission (look up report's subreddit)
    let report: Option<(String,)> = sqlx::query_as("SELECT subreddit_id FROM reports WHERE id = ?")
        .bind(&report_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some((sub_id,)) = report else {
        return Err(ApiError::NotFound(format!("report {}", report_id)));
    };
    if !sub_id.is_empty() {
        let is_mod: Option<(String,)> = sqlx::query_as(
            "SELECT user_id FROM subreddit_moderators WHERE subreddit_id = ? AND user_id = ?",
        )
        .bind(&sub_id)
        .bind(&claims.sub)
        .fetch_optional(&state.pool)
        .await?;
        let sub_mod: Option<(String,)> = sqlx::query_as(
            "SELECT user_id FROM subscriptions WHERE subreddit_id = ? AND user_id = ? AND role = 'moderator'",
        )
        .bind(&sub_id)
        .bind(&claims.sub)
        .fetch_optional(&state.pool)
        .await?;
        if is_mod.is_none() && sub_mod.is_none() {
            return Err(ApiError::Forbidden("moderator access required".into()));
        }
    }
    sqlx::query("UPDATE reports SET status = 'dismissed' WHERE id = ?")
        .bind(&report_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "report_id": report_id, "status": "dismissed" })))
}
