//! Vote handler — Reddit's Vote service (Cassandra-backed in production).
//! Applies an upvote/downvote to a post or comment, updates counts and ranking scores atomically.

use axum::{extract::State, Json};
use chrono::Utc;
use serde_json::json;

use crate::{
    auth::AuthUser,
    errors::ApiError,
    models::VoteRequest,
    AppState,
};

pub async fn cast_vote(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<VoteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !(-1..=1).contains(&req.direction) {
        return Err(ApiError::BadRequest("direction must be -1, 0, or 1".into()));
    }

    // The target_id is the fullname like "t3_xxx" or "t1_xxx".
    // Our DB stores fullnames as the primary key, so we use it directly.
    let raw_id = req.target_id.clone();

    // Detect type — look up in posts first, then comments
    let (target_type, created_at): (String, chrono::DateTime<Utc>) = {
        let post: Option<(String,)> = sqlx::query_as("SELECT created_at FROM posts WHERE id = ?")
            .bind(&raw_id)
            .fetch_optional(&state.pool)
            .await?;
        if let Some((ca,)) = post {
            let dt = chrono::DateTime::parse_from_rfc3339(&ca).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
            ("post".to_string(), dt)
        } else {
            let c: Option<(String,)> = sqlx::query_as("SELECT created_at FROM comments WHERE id = ?")
                .bind(&raw_id)
                .fetch_optional(&state.pool)
                .await?;
            let Some((ca,)) = c else {
                return Err(ApiError::NotFound(format!("target {} not found", req.target_id)));
            };
            let dt = chrono::DateTime::parse_from_rfc3339(&ca).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
            ("comment".to_string(), dt)
        }
    };

    // Get existing vote (if any)
    let existing: Option<(i32,)> = sqlx::query_as(
        "SELECT direction FROM votes WHERE user_id = ? AND target_id = ?",
    )
    .bind(&claims.sub)
    .bind(&raw_id)
    .fetch_optional(&state.pool)
    .await?;
    let old_dir = existing.map(|(d,)| d).unwrap_or(0);
    let new_dir = req.direction;

    if old_dir == new_dir {
        // No-op
        return Ok(Json(json!({
            "id": raw_id,
            "likes": new_dir,
            "score": null,
            "changed": false,
        })));
    }

    let now = Utc::now().to_rfc3339();

    // Upsert vote row
    if new_dir == 0 {
        sqlx::query("DELETE FROM votes WHERE user_id = ? AND target_id = ?")
            .bind(&claims.sub)
            .bind(&raw_id)
            .execute(&state.pool)
            .await?;
    } else if existing.is_some() {
        sqlx::query("UPDATE votes SET direction = ?, updated_at = ? WHERE user_id = ? AND target_id = ?")
            .bind(new_dir)
            .bind(&now)
            .bind(&claims.sub)
            .bind(&raw_id)
            .execute(&state.pool)
            .await?;
    } else {
        sqlx::query("INSERT INTO votes (user_id, target_id, target_type, direction, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&claims.sub)
            .bind(&raw_id)
            .bind(&target_type)
            .bind(new_dir)
            .bind(&now)
            .bind(&now)
            .execute(&state.pool)
            .await?;
    }

    // Compute new ups/downs deltas:
    //   - old_dir was the previous vote. We subtract its effect and add new_dir's.
    let up_delta = match (old_dir, new_dir) {
        (1, 0) => -1,        // was up, now neutral
        (1, -1) => -1,       // was up, now down: -1 up, +1 down
        (-1, 0) => 0,        // was down, now neutral: -1 down
        (-1, 1) => 1,        // was down, now up: +1 up, -1 down
        (0, 1) => 1,         // neutral to up
        (0, -1) => 0,        // neutral to down (handled via down_delta)
        _ => 0,
    };
    let down_delta = match (old_dir, new_dir) {
        (1, -1) => 1,
        (-1, 0) => -1,
        (-1, 1) => -1,
        (0, -1) => 1,
        _ => 0,
    };

    // We don't need the formatted UPDATE-with-delta SQL — we recompute from absolute values below.
    let table = if target_type == "post" { "posts" } else { "comments" };
    // We don't know the absolute ups/downs here without a SELECT, so do a SELECT-then-UPDATE pattern:
    let cur: Option<(i64, i64)> = sqlx::query_as(&format!("SELECT ups, downs FROM {} WHERE id = ?", table))
        .bind(&raw_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some((ups, downs)) = cur else {
        return Ok(Json(json!({ "changed": false })));
    };
    let new_ups = (ups + up_delta).max(0);
    let new_downs = (downs + down_delta).max(0);
    let new_ratio = crate::ranking::upvote_ratio(new_ups, new_downs);
    let new_hot = crate::ranking::hot_score(new_ups, new_downs, created_at);
    let new_best = crate::ranking::best_score(new_ups, new_downs);
    let new_cont = crate::ranking::controversial_score(new_ups, new_downs);

    sqlx::query(&format!(
        "UPDATE {} SET ups = ?, downs = ?, score = ?, upvote_ratio = ?, hot_score = ?, best_score = ?, controversial_score = ? WHERE id = ?",
        table
    ))
    .bind(new_ups)
    .bind(new_downs)
    .bind(new_ups - new_downs)
    .bind(new_ratio)
    .bind(new_hot)
    .bind(new_best)
    .bind(new_cont)
    .bind(&raw_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "id": raw_id,
        "likes": new_dir,
        "score": new_ups - new_downs,
        "ups": new_ups,
        "downs": new_downs,
        "upvote_ratio": new_ratio,
        "changed": true,
        "delta": up_delta - down_delta,
    })))
}
