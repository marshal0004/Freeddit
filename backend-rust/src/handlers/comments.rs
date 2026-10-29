//! Comments handlers — list, get, create. Supports nested replies via materialized path.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::Row;
use std::collections::HashMap;

use crate::{
    auth::{self, AuthUser, MaybeAuthUser},
    errors::ApiError,
    models::{Comment, CreateCommentRequest, Listing, ListingData, Thing},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

const COMMENT_SELECT: &str = r#"
    SELECT c.id, c.post_id, c.subreddit_id, s.name, u.username, c.author_id,
           c.parent_id, c.depth, c.body, c.score, c.ups, c.downs, c.upvote_ratio,
           c.num_replies, c.is_sticky, c.is_locked, c.is_op,
           c.is_removed, c.is_deleted, c.created_at
    FROM comments c
    JOIN subreddits s ON s.id = c.subreddit_id
    JOIN users u ON u.id = c.author_id
    WHERE c.is_removed = 0 AND c.is_deleted = 0
"#;

fn row_to_comment(r: &sqlx::sqlite::SqliteRow, likes: Option<i32>) -> Comment {
    let id: String = r.try_get("id").unwrap_or_default();
    let post_id: String = r.try_get("post_id").unwrap_or_default();
    let subreddit: String = r.try_get("name").unwrap_or_default();
    let author: String = r.try_get("username").unwrap_or_default();
    let author_id: String = r.try_get("author_id").unwrap_or_default();
    let parent_id: Option<String> = r.try_get("parent_id").ok().flatten();
    let depth: i32 = r.try_get("depth").unwrap_or(0);
    let body: String = r.try_get("body").unwrap_or_default();
    let score: i64 = r.try_get("score").unwrap_or(0);
    let ups: i64 = r.try_get("ups").unwrap_or(0);
    let downs: i64 = r.try_get("downs").unwrap_or(0);
    let ratio: f64 = r.try_get("upvote_ratio").unwrap_or(1.0);
    let num_replies: i64 = r.try_get("num_replies").unwrap_or(0);
    let is_sticky: bool = r.try_get::<i64, _>("is_sticky").unwrap_or(0) != 0;
    let is_locked: bool = r.try_get::<i64, _>("is_locked").unwrap_or(0) != 0;
    let is_op: bool = r.try_get::<i64, _>("is_op").unwrap_or(0) != 0;
    let is_deleted: bool = r.try_get::<i64, _>("is_deleted").unwrap_or(0) != 0;
    let created_str: String = r.try_get("created_at").unwrap_or_default();
    let created = chrono::DateTime::parse_from_rfc3339(&created_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let final_body = if is_deleted { "[deleted]".to_string() } else { body };
    let final_author = if is_deleted { "[deleted]".to_string() } else { author };

    Comment {
        id: id.clone(),
        name: id.clone(),  // id already includes the t1_ prefix
        post_id,
        subreddit_id: r.try_get("subreddit_id").unwrap_or_default(),
        subreddit,
        author: final_author,
        author_id,
        parent_id,
        depth,
        body: final_body,
        score,
        ups,
        downs,
        upvote_ratio: ratio,
        num_replies,
        is_sticky,
        is_locked,
        is_op,
        created_at: created,
        edited_at: None,
        likes,
        replies: Vec::new(),
    }
}

pub async fn list_for_post(
    State(state): State<AppState>,
    MaybeAuthUser(maybe_claims): MaybeAuthUser,
    Path(id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Listing<Comment>>, ApiError> {
    list_for_post_sorted_inner(state, maybe_claims, id, q.sort.as_deref().unwrap_or("best")).await
}

pub async fn list_for_post_sorted(
    State(state): State<AppState>,
    MaybeAuthUser(maybe_claims): MaybeAuthUser,
    Path((id, sort)): Path<(String, String)>,
) -> Result<Json<Listing<Comment>>, ApiError> {
    list_for_post_sorted_inner(state, maybe_claims, id, &sort).await
}

async fn list_for_post_sorted_inner(
    state: AppState,
    maybe_claims: Option<crate::auth::Claims>,
    post_id: String,
    sort: &str,
) -> Result<Json<Listing<Comment>>, ApiError> {
    let order = match sort {
        "top" => "c.score DESC, c.ups DESC",
        "new" => "c.created_at DESC",
        "controversial" => "c.controversial_score DESC",
        "old" => "c.created_at ASC",
        "qa" => "(CASE WHEN c.is_op = 1 THEN 0 ELSE 1 END), c.score DESC",
        _ => "c.best_score DESC, c.score DESC",
    };
    let sql = format!(
        "{} AND c.post_id = ? ORDER BY c.depth ASC, c.path ASC, {}",
        COMMENT_SELECT, order
    );
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(&sql)
        .bind(&post_id)
        .fetch_all(&state.pool)
        .await?;

    let likes_map = load_likes(&state, &maybe_claims, &rows.iter().map(|r| r.try_get::<String, _>("id").unwrap_or_default()).collect::<Vec<_>>()).await;

    let mut comments: Vec<Comment> = rows
        .iter()
        .map(|r| {
            let id: String = r.try_get("id").unwrap_or_default();
            let likes = likes_map.get(&id).copied().flatten();
            row_to_comment(r, likes)
        })
        .collect();

    // Build nested tree — group by parent_id
    let mut by_parent: HashMap<Option<String>, Vec<Comment>> = HashMap::new();
    for c in std::mem::take(&mut comments) {
        by_parent.entry(c.parent_id.clone()).or_default().push(c);
    }
    let mut roots = by_parent.remove(&None).unwrap_or_default();
    sort_comments(&mut roots, sort);
    fn attach_children(parent: &mut Comment, by_parent: &mut HashMap<Option<String>, Vec<Comment>>, sort: &str) {
        let pid = Some(parent.id.clone());
        if let Some(mut children) = by_parent.remove(&pid) {
            sort_comments(&mut children, sort);
            for c in children.iter_mut() {
                attach_children(c, by_parent, sort);
            }
            parent.replies = children;
        }
    }
    for r in roots.iter_mut() {
        attach_children(r, &mut by_parent, sort);
    }
    let dist = roots.len() as i64;
    Ok(Json(Listing {
        kind: "Listing".into(),
        data: ListingData {
            after: None,
            before: None,
            children: roots.into_iter().map(|c| Thing { kind: "t1".into(), data: c }).collect(),
            dist,
        },
    }))
}

fn sort_comments(comments: &mut Vec<Comment>, sort: &str) {
    match sort {
        "top" => comments.sort_by(|a, b| b.score.cmp(&a.score)),
        "new" => comments.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        "old" => comments.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
        "controversial" => comments.sort_by(|a, b| {
            let ca = crate::ranking::controversial_score(a.ups, a.downs);
            let cb = crate::ranking::controversial_score(b.ups, b.downs);
            cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
        }),
        "qa" => comments.sort_by(|a, b| {
            (b.is_op as i32).cmp(&(a.is_op as i32)).then(b.score.cmp(&a.score))
        }),
        _ => comments.sort_by(|a, b| {
            let ba = crate::ranking::best_score(a.ups, a.downs);
            let bb = crate::ranking::best_score(b.ups, b.downs);
            bb.partial_cmp(&ba)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.score.cmp(&a.score))
        }),
    }
}

pub async fn get_by_id(
    State(state): State<AppState>,
    MaybeAuthUser(maybe_claims): MaybeAuthUser,
    Path(id): Path<String>,
) -> Result<Json<Thing<Comment>>, ApiError> {
    let sql = format!("{} AND c.id = ? LIMIT 1", COMMENT_SELECT);
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(r) = row else {
        return Err(ApiError::NotFound(format!("comment t1_{}", id)));
    };
    let cid: String = r.try_get("id").unwrap_or_default();
    let likes = if let Some(claims) = maybe_claims {
        let v: Option<(i32,)> = sqlx::query_as("SELECT direction FROM votes WHERE user_id = ? AND target_id = ?")
            .bind(&claims.sub)
            .bind(&cid)
            .fetch_optional(&state.pool)
            .await?;
        v.map(|(d,)| d)
    } else { None };
    Ok(Json(Thing { kind: "t1".into(), data: row_to_comment(&r, likes) }))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(post_id): Path<String>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<Json<Thing<Comment>>, ApiError> {
    if req.body.trim().is_empty() {
        return Err(ApiError::BadRequest("comment body is required".into()));
    }
    if req.body.len() > 10000 {
        return Err(ApiError::BadRequest("comment too long (max 10000 chars)".into()));
    }

    let post: Option<(String, String, String)> = sqlx::query_as(
        "SELECT p.id, p.subreddit_id, p.author_id FROM posts p JOIN subreddits s ON s.id = p.subreddit_id WHERE p.id = ? AND p.is_removed = 0 AND p.is_deleted = 0",
    )
    .bind(&post_id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((post_id, sub_id, post_author_id)) = post else {
        return Err(ApiError::NotFound(format!("post t3_{}", post_id)));
    };

    let id = auth::generate_fullname("t1");
    let now = Utc::now();
    let now_str = now.to_rfc3339();

    let (parent_id, depth, path) = if let Some(pid) = &req.parent_id {
        let parent: Option<(Option<String>, i32, String)> =
            sqlx::query_as("SELECT parent_id, depth, path FROM comments WHERE id = ?")
                .bind(pid)
                .fetch_optional(&state.pool)
                .await?;
        let Some((_, pdepth, ppath)) = parent else {
            return Err(ApiError::BadRequest(format!("parent comment not found: t1_{}", pid)));
        };
        let depth = pdepth + 1;
        let path = format!("{}{}/", ppath, id);
        (Some(pid.clone()), depth, path)
    } else {
        let path = format!("/{}/", id);
        (None, 0, path)
    };

    let is_op = if claims.sub == post_author_id { 1 } else { 0 };
    let ups = 1;
    let downs = 0;
    let hot = crate::ranking::hot_score(ups, downs, now);
    let best = crate::ranking::best_score(ups, downs);
    let cont = crate::ranking::controversial_score(ups, downs);
    let ratio = crate::ranking::upvote_ratio(ups, downs);

    sqlx::query(
        "INSERT INTO comments (id, post_id, subreddit_id, author_id, parent_id, path, depth, body,
                                score, ups, downs, upvote_ratio, hot_score, best_score, controversial_score,
                                num_replies, is_sticky, is_locked, is_removed, is_op, is_deleted,
                                created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, 1, 0, ?, ?, ?, 0, 0, 0, 0, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(&post_id)
    .bind(&sub_id)
    .bind(&claims.sub)
    .bind(&parent_id)
    .bind(&path)
    .bind(depth)
    .bind(&req.body)
    .bind(ratio)
    .bind(hot)
    .bind(best)
    .bind(cont)
    .bind(is_op)
    .bind(&now_str)
    .bind(&now_str)
    .execute(&state.pool)
    .await?;

    sqlx::query("UPDATE posts SET num_comments = num_comments + 1 WHERE id = ?")
        .bind(&post_id)
        .execute(&state.pool)
        .await?;
    if let Some(pid) = &parent_id {
        sqlx::query("UPDATE comments SET num_replies = num_replies + 1 WHERE id = ?")
            .bind(pid)
            .execute(&state.pool)
            .await?;
    }
    sqlx::query("UPDATE users SET comment_karma = comment_karma + 1 WHERE id = ?")
        .bind(&claims.sub)
        .execute(&state.pool)
        .await?;

    // Generate notification for parent comment author
    let _ = crate::handlers::notifications::notify_comment_reply(
        &state,
        &post_id,
        parent_id.as_deref(),
        &claims.sub,
        &req.body,
    )
    .await;

    let sql = format!("{} AND c.id = ? LIMIT 1", COMMENT_SELECT);
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(r) = row else {
        return Err(ApiError::Internal("comment vanished after insert".into()));
    };
    Ok(Json(Thing { kind: "t1".into(), data: row_to_comment(&r, Some(1)) }))
}

async fn load_likes(
    state: &AppState,
    maybe_claims: &Option<crate::auth::Claims>,
    ids: &[String],
) -> HashMap<String, Option<i32>> {
    let mut map = HashMap::new();
    if let Some(claims) = maybe_claims {
        for id in ids {
            let v: Option<(i32,)> = sqlx::query_as("SELECT direction FROM votes WHERE user_id = ? AND target_id = ?")
                .bind(&claims.sub)
                .bind(id)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten();
            map.insert(id.clone(), v.map(|(d,)| d));
        }
    }
    map
}

#[derive(Debug, Deserialize)]
pub struct EditCommentRequest {
    pub body: String,
}

pub async fn edit_comment(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
    Json(req): Json<EditCommentRequest>,
) -> Result<Json<Thing<Comment>>, ApiError> {
    if req.body.trim().is_empty() {
        return Err(ApiError::BadRequest("comment body cannot be empty".into()));
    }
    if req.body.len() > 10000 {
        return Err(ApiError::BadRequest("comment too long (max 10000 chars)".into()));
    }
    let row: Option<(String, i64)> = sqlx::query_as("SELECT author_id, is_locked FROM comments WHERE id = ? AND is_deleted = 0")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;
    let Some((author_id, is_locked)) = row else {
        return Err(ApiError::NotFound(format!("comment {}", id)));
    };
    if author_id != claims.sub {
        return Err(ApiError::Forbidden("only the author can edit".into()));
    }
    if is_locked != 0 {
        return Err(ApiError::BadRequest("comment is locked".into()));
    }
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE comments SET body = ?, updated_at = ? WHERE id = ?")
        .bind(&req.body).bind(&now).bind(&id)
        .execute(&state.pool)
        .await?;
    let sql = format!("{} AND c.id = ? LIMIT 1", COMMENT_SELECT);
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(&sql).bind(&id).fetch_optional(&state.pool).await?;
    let Some(r) = row else { return Err(ApiError::Internal("comment vanished after edit".into())); };
    Ok(Json(Thing { kind: "t1".into(), data: row_to_comment(&r, None) }))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT author_id FROM comments WHERE id = ? AND is_deleted = 0")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;
    let Some((author_id,)) = row else {
        return Err(ApiError::NotFound(format!("comment {}", id)));
    };
    if author_id != claims.sub {
        return Err(ApiError::Forbidden("only the author can delete".into()));
    }
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE comments SET is_deleted = 1, body = '[deleted]', updated_at = ? WHERE id = ?")
        .bind(&now).bind(&id)
        .execute(&state.pool)
        .await?;
    Ok(Json(serde_json::json!({ "deleted": true, "comment_id": id })))
}
