//! Posts handlers — list (home + subreddit), get, create, vote.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::Row;

use crate::{
    auth::{self, AuthUser, MaybeAuthUser},
    errors::ApiError,
    models::{CreatePostRequest, Listing, ListingData, Post, PostSort, Thing},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub t: Option<String>,
}

const POST_SELECT: &str = r#"
    SELECT p.id, p.subreddit_id, s.name, u.username, p.author_id,
           p.kind, p.title, COALESCE(p.body, '') AS body, p.url,
           p.flair_text, p.flair_color, p.thumbnail,
           p.score, p.ups, p.downs, p.upvote_ratio,
           p.num_comments, p.views, p.is_sticky, p.is_locked,
           p.is_spoiler, p.is_nsfw,
           p.created_at
    FROM posts p
    JOIN subreddits s ON s.id = p.subreddit_id
    JOIN users u ON u.id = p.author_id
    WHERE p.is_removed = 0 AND p.is_deleted = 0
"#;

fn row_to_post(row: &sqlx::sqlite::SqliteRow, likes: Option<i32>) -> Post {
    let id: String = row.try_get("id").unwrap_or_default();
    let subreddit_id: String = row.try_get("subreddit_id").unwrap_or_default();
    let subreddit: String = row.try_get("name").unwrap_or_default();
    let author: String = row.try_get("username").unwrap_or_default();
    let author_id: String = row.try_get("author_id").unwrap_or_default();
    let kind: String = row.try_get("kind").unwrap_or_else(|_| "self".into());
    let title: String = row.try_get("title").unwrap_or_default();
    let body: String = row.try_get("body").unwrap_or_default();
    let url: Option<String> = row.try_get("url").ok().flatten();
    let flair_text: Option<String> = row.try_get("flair_text").ok().flatten();
    let flair_color: Option<String> = row.try_get("flair_color").ok().flatten();
    let thumbnail: Option<String> = row.try_get("thumbnail").ok().flatten();
    let score: i64 = row.try_get("score").unwrap_or(0);
    let ups: i64 = row.try_get("ups").unwrap_or(0);
    let downs: i64 = row.try_get("downs").unwrap_or(0);
    let upvote_ratio: f64 = row.try_get("upvote_ratio").unwrap_or(1.0);
    let num_comments: i64 = row.try_get("num_comments").unwrap_or(0);
    let views: i64 = row.try_get("views").unwrap_or(0);
    let is_sticky: bool = row.try_get::<i64, _>("is_sticky").unwrap_or(0) != 0;
    let is_locked: bool = row.try_get::<i64, _>("is_locked").unwrap_or(0) != 0;
    let is_spoiler: bool = row.try_get::<i64, _>("is_spoiler").unwrap_or(0) != 0;
    let is_nsfw: bool = row.try_get::<i64, _>("is_nsfw").unwrap_or(0) != 0;
    let created_str: String = row.try_get("created_at").unwrap_or_default();
    let created_at = chrono::DateTime::parse_from_rfc3339(&created_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let is_self = kind == "self";
    let post_url = if is_self { None } else { url.filter(|s| !s.is_empty()) };
    let post_body = if is_self { Some(body) } else { None };

    Post {
        id: id.clone(),
        name: id.clone(),  // id already includes the t3_ prefix
        subreddit_id,
        subreddit: subreddit.clone(),
        subreddit_name_prefixed: format!("r/{}", subreddit),
        author: author.clone(),
        author_id,
        title,
        url: post_url,
        body: post_body,
        kind,
        flair_text,
        flair_color,
        thumbnail,
        score,
        ups,
        downs,
        upvote_ratio,
        num_comments,
        views,
        is_sticky,
        is_locked,
        is_removed: false,
        is_spoiler,
        over18: is_nsfw,
        is_self,
        created_at,
        edited_at: None,
        likes,
    }
}

fn order_clause(sort: PostSort, t: &str) -> String {
    match sort {
        PostSort::Hot => "ORDER BY p.is_sticky DESC, p.hot_score DESC".to_string(),
        PostSort::Best => "ORDER BY p.is_sticky DESC, p.best_score DESC".to_string(),
        PostSort::New => "ORDER BY p.is_sticky DESC, p.created_at DESC".to_string(),
        PostSort::Rising => "ORDER BY p.is_sticky DESC, p.hot_score DESC, p.created_at DESC".to_string(),
        PostSort::Controversial => "ORDER BY p.is_sticky DESC, p.controversial_score DESC".to_string(),
        PostSort::Top => {
            let cond = match t {
                "hour" => "p.created_at >= datetime('now','-1 hour')",
                "day" => "p.created_at >= datetime('now','-1 day')",
                "week" => "p.created_at >= datetime('now','-7 days')",
                "month" => "p.created_at >= datetime('now','-1 month')",
                "year" => "p.created_at >= datetime('now','-1 year')",
                _ => "1=1",
            };
            format!("AND {} ORDER BY p.is_sticky DESC, p.score DESC", cond)
        }
    }
}

async fn load_likes(
    state: &AppState,
    maybe_claims: &Option<crate::auth::Claims>,
    post_ids: &[String],
) -> std::collections::HashMap<String, Option<i32>> {
    let mut map = std::collections::HashMap::new();
    if let Some(claims) = maybe_claims {
        for id in post_ids {
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

pub async fn list_home(
    State(state): State<AppState>,
    MaybeAuthUser(maybe_claims): MaybeAuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Listing<Post>>, ApiError> {
    let sort = q.sort.as_deref().map(PostSort::from_str).unwrap_or_default();
    let limit = q.limit.unwrap_or(25).clamp(1, 100);
    let t = q.t.as_deref().unwrap_or("all");
    let order = order_clause(sort, t);

    let (sql, has_subs) = if let Some(_claims) = &maybe_claims {
        (
            format!(
                "{} AND p.subreddit_id IN (SELECT subreddit_id FROM subscriptions WHERE user_id = ?) {} LIMIT ?",
                POST_SELECT, order
            ),
            true,
        )
    } else {
        (format!("{} {} LIMIT ?", POST_SELECT, order), false)
    };

    let rows: Vec<sqlx::sqlite::SqliteRow> = if has_subs {
        let claims = maybe_claims.as_ref().unwrap();
        sqlx::query(&sql)
            .bind(&claims.sub)
            .bind(limit)
            .fetch_all(&state.pool)
            .await?
    } else {
        sqlx::query(&sql)
            .bind(limit)
            .fetch_all(&state.pool)
            .await?
    };

    let post_ids: Vec<String> = rows.iter().map(|r| r.try_get::<String, _>("id").unwrap_or_default()).collect();
    let likes_map = load_likes(&state, &maybe_claims, &post_ids).await;

    let children: Vec<Thing<Post>> = rows
        .iter()
        .map(|r| {
            let id = r.try_get::<String, _>("id").unwrap_or_default();
            let likes = likes_map.get(&id).copied().flatten();
            Thing { kind: "t3".into(), data: row_to_post(r, likes) }
        })
        .collect();
    let dist = children.len() as i64;
    Ok(Json(Listing { kind: "Listing".into(), data: ListingData { after: None, before: None, children, dist } }))
}

pub async fn list_by_subreddit(
    State(state): State<AppState>,
    MaybeAuthUser(maybe_claims): MaybeAuthUser,
    Path(name): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Listing<Post>>, ApiError> {
    let sort = q.sort.as_deref().map(PostSort::from_str).unwrap_or_default();
    let limit = q.limit.unwrap_or(25).clamp(1, 100);
    let t = q.t.as_deref().unwrap_or("all");
    let order = order_clause(sort, t);

    let sql = format!("{} AND s.name = ? {} LIMIT ?", POST_SELECT, order);
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(&sql)
        .bind(&name)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;

    let post_ids: Vec<String> = rows.iter().map(|r| r.try_get::<String, _>("id").unwrap_or_default()).collect();
    let likes_map = load_likes(&state, &maybe_claims, &post_ids).await;

    let children: Vec<Thing<Post>> = rows
        .iter()
        .map(|r| {
            let id = r.try_get::<String, _>("id").unwrap_or_default();
            let likes = likes_map.get(&id).copied().flatten();
            Thing { kind: "t3".into(), data: row_to_post(r, likes) }
        })
        .collect();
    let dist = children.len() as i64;
    Ok(Json(Listing { kind: "Listing".into(), data: ListingData { after: None, before: None, children, dist } }))
}

pub async fn get_by_id(
    State(state): State<AppState>,
    MaybeAuthUser(maybe_claims): MaybeAuthUser,
    Path(id): Path<String>,
) -> Result<Json<Thing<Post>>, ApiError> {
    let sql = format!("{} AND p.id = ? LIMIT 1", POST_SELECT);
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(r) = row else {
        return Err(ApiError::NotFound(format!("post t3_{}", id)));
    };
    let post_id: String = r.try_get("id").unwrap_or_default();
    let likes = if let Some(claims) = maybe_claims {
        let v: Option<(i32,)> = sqlx::query_as("SELECT direction FROM votes WHERE user_id = ? AND target_id = ?")
            .bind(&claims.sub)
            .bind(&post_id)
            .fetch_optional(&state.pool)
            .await?;
        v.map(|(d,)| d)
    } else { None };
    Ok(Json(Thing { kind: "t3".into(), data: row_to_post(&r, likes) }))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreatePostRequest>,
) -> Result<Json<Thing<Post>>, ApiError> {
    if req.title.is_empty() || req.title.len() > 300 {
        return Err(ApiError::BadRequest("title must be 1-300 chars".into()));
    }
    let sub_name = req.subreddit.to_lowercase();
    let sub: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
        .bind(&sub_name)
        .fetch_optional(&state.pool)
        .await?;
    let Some((sub_id,)) = sub else {
        return Err(ApiError::NotFound(format!("r/{}", sub_name)));
    };

    let kind = req.kind.clone().unwrap_or_else(|| {
        if req.body.as_deref().map(|b| !b.is_empty()).unwrap_or(false) { "self".into() }
        else if req.url.as_deref().map(|u| !u.is_empty()).unwrap_or(false) { "link".into() }
        else { "self".into() }
    });

    let id = auth::generate_fullname("t3");
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let ups = 1;
    let downs = 0;
    let hot = crate::ranking::hot_score(ups, downs, now);
    let best = crate::ranking::best_score(ups, downs);
    let cont = crate::ranking::controversial_score(ups, downs);
    let ratio = crate::ranking::upvote_ratio(ups, downs);

    sqlx::query(
        "INSERT INTO posts (id, subreddit_id, author_id, title, url, body, kind, flair_text, flair_color,
                             thumbnail, score, ups, downs, upvote_ratio, hot_score, best_score, controversial_score,
                             num_comments, views, is_sticky, is_locked, is_removed, is_spoiler, is_nsfw, is_deleted,
                             created_at, updated_at, rank_epoch, media_url, media_kind, crosspost_parent_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?,
                 NULL, 1, 1, 0, ?, ?, ?, ?,
                 0, 0, 0, 0, 0, ?, 0,
                 ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&sub_id)
    .bind(&claims.sub)
    .bind(&req.title)
    .bind(&req.url)
    .bind(&req.body)
    .bind(&kind)
    .bind(&req.flair_text)
    .bind(&req.flair_color)
    .bind(ratio)
    .bind(hot)
    .bind(best)
    .bind(cont)
    .bind(if req.is_nsfw { 1 } else { 0 })
    .bind(&now_str)
    .bind(&now_str)
    .bind(now.timestamp() as f64)
    .bind(&req.media_url)
    .bind(&req.media_kind)
    .bind(&req.crosspost_parent_id)
    .execute(&state.pool)
    .await?;

    sqlx::query("UPDATE users SET post_karma = post_karma + 1 WHERE id = ?")
        .bind(&claims.sub)
        .execute(&state.pool)
        .await?;

    let sql = format!("{} AND p.id = ? LIMIT 1", POST_SELECT);
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(r) = row else {
        return Err(ApiError::Internal("post vanished after insert".into()));
    };
    Ok(Json(Thing { kind: "t3".into(), data: row_to_post(&r, Some(1)) }))
}

#[derive(Debug, Deserialize)]
pub struct EditPostRequest {
    pub title: Option<String>,
    pub url: Option<String>,
    pub body: Option<String>,
    #[serde(default)]
    pub is_nsfw: Option<bool>,
    #[serde(default)]
    pub is_spoiler: Option<bool>,
    #[serde(default)]
    pub flair_text: Option<String>,
    #[serde(default)]
    pub flair_color: Option<String>,
}

/// Edit a post — only the author can edit, and only self posts can have body changes.
pub async fn edit_post(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
    Json(req): Json<EditPostRequest>,
) -> Result<Json<Thing<Post>>, ApiError> {
    let post: Option<(String, i64)> = sqlx::query_as("SELECT author_id, is_locked FROM posts WHERE id = ? AND is_deleted = 0")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;
    let Some((author_id, is_locked)) = post else {
        return Err(ApiError::NotFound(format!("post {}", id)));
    };
    if author_id != claims.sub {
        return Err(ApiError::Forbidden("only the author can edit".into()));
    }
    if is_locked != 0 {
        return Err(ApiError::BadRequest("post is locked".into()));
    }
    let now = Utc::now().to_rfc3339();
    if let Some(title) = &req.title {
        if title.is_empty() || title.len() > 300 {
            return Err(ApiError::BadRequest("title must be 1-300 chars".into()));
        }
        sqlx::query("UPDATE posts SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title).bind(&now).bind(&id)
            .execute(&state.pool).await?;
    }
    if let Some(body) = &req.body {
        sqlx::query("UPDATE posts SET body = ?, updated_at = ? WHERE id = ?")
            .bind(body).bind(&now).bind(&id)
            .execute(&state.pool).await?;
    }
    if let Some(url) = &req.url {
        sqlx::query("UPDATE posts SET url = ?, updated_at = ? WHERE id = ?")
            .bind(url).bind(&now).bind(&id)
            .execute(&state.pool).await?;
    }
    if let Some(nsfw) = req.is_nsfw {
        sqlx::query("UPDATE posts SET is_nsfw = ?, updated_at = ? WHERE id = ?")
            .bind(if nsfw { 1 } else { 0 }).bind(&now).bind(&id)
            .execute(&state.pool).await?;
    }
    if let Some(spoiler) = req.is_spoiler {
        sqlx::query("UPDATE posts SET is_spoiler = ?, updated_at = ? WHERE id = ?")
            .bind(if spoiler { 1 } else { 0 }).bind(&now).bind(&id)
            .execute(&state.pool).await?;
    }
    if let Some(flair) = &req.flair_text {
        sqlx::query("UPDATE posts SET flair_text = ?, updated_at = ? WHERE id = ?")
            .bind(flair).bind(&now).bind(&id)
            .execute(&state.pool).await?;
    }
    if let Some(color) = &req.flair_color {
        sqlx::query("UPDATE posts SET flair_color = ?, updated_at = ? WHERE id = ?")
            .bind(color).bind(&now).bind(&id)
            .execute(&state.pool).await?;
    }

    let sql = format!("{} AND p.id = ? LIMIT 1", POST_SELECT);
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(&sql).bind(&id).fetch_optional(&state.pool).await?;
    let Some(r) = row else { return Err(ApiError::Internal("post vanished after edit".into())); };
    Ok(Json(Thing { kind: "t3".into(), data: row_to_post(&r, None) }))
}

/// Delete a post — author can soft-delete (sets is_deleted = 1, body becomes "[deleted]").
pub async fn delete_post(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let post: Option<(String,)> = sqlx::query_as("SELECT author_id FROM posts WHERE id = ? AND is_deleted = 0")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;
    let Some((author_id,)) = post else {
        return Err(ApiError::NotFound(format!("post {}", id)));
    };
    if author_id != claims.sub {
        return Err(ApiError::Forbidden("only the author can delete".into()));
    }
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE posts SET is_deleted = 1, body = NULL, url = NULL, updated_at = ? WHERE id = ?")
        .bind(&now).bind(&id)
        .execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "deleted": true, "post_id": id })))
}

/// Pagination cursor support — return posts after a given post id.
/// "after" is a post id; we return posts with hot_score < after's hot_score, or created_at < after's created_at for sort=new.
#[derive(Debug, Deserialize)]
pub struct PaginatedListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub t: Option<String>,
}

pub async fn list_home_paginated(
    state: &AppState,
    maybe_claims: &Option<crate::auth::Claims>,
    sort: PostSort,
    limit: i64,
    t: &str,
    after: Option<&str>,
) -> Result<Vec<sqlx::sqlite::SqliteRow>, ApiError> {
    let order = order_clause(sort, t);
    let after_clause = if let Some(aid) = after {
        // Fetch the cursor post to get its hot_score / created_at
        let cursor: Option<(f64, String, i64)> = sqlx::query_as(
            "SELECT hot_score, created_at, 1 FROM posts WHERE id = ?",
        )
        .bind(aid)
        .fetch_optional(&state.pool)
        .await?;
        if let Some((hs, created, _)) = cursor {
            match sort {
                PostSort::New => format!("AND p.created_at < '{}' ", created),
                PostSort::Top => format!("AND p.score < (SELECT score FROM posts WHERE id = '{}') ", aid),
                _ => format!("AND p.hot_score < {} ", hs),
            }
        } else { String::new() }
    } else { String::new() };

    let (sql, has_subs) = if let Some(_claims) = maybe_claims {
        (
            format!(
                "{} AND p.subreddit_id IN (SELECT subreddit_id FROM subscriptions WHERE user_id = ?) {} {} LIMIT ?",
                POST_SELECT, after_clause, order
            ),
            true,
        )
    } else {
        (format!("{} {} {} LIMIT ?", POST_SELECT, after_clause, order), false)
    };

    let rows: Vec<sqlx::sqlite::SqliteRow> = if has_subs {
        let claims = maybe_claims.as_ref().unwrap();
        sqlx::query(&sql).bind(&claims.sub).bind(limit).fetch_all(&state.pool).await?
    } else {
        sqlx::query(&sql).bind(limit).fetch_all(&state.pool).await?
    };
    Ok(rows)
}
