//! Users handlers — get profile, posts by user, comments by user.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::{auth::MaybeAuthUser, errors::ApiError, AppState};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

pub async fn get_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, username, email, avatar, banner_color, bio, post_karma, comment_karma, is_admin, created_at
         FROM users WHERE username = ? AND is_deleted = 0",
    )
    .bind(&name)
    .fetch_optional(&state.pool)
    .await?;
    let Some(r) = row else {
        return Err(ApiError::NotFound(format!("u/{}", name)));
    };
    let id: String = r.try_get("id").unwrap_or_default();
    let username: String = r.try_get("username").unwrap_or_default();
    let email: Option<String> = r.try_get("email").ok().flatten();
    let avatar: Option<String> = r.try_get("avatar").ok().flatten();
    let banner_color: Option<String> = r.try_get("banner_color").ok().flatten();
    let bio: Option<String> = r.try_get("bio").ok().flatten();
    let pk: i64 = r.try_get("post_karma").unwrap_or(0);
    let ck: i64 = r.try_get("comment_karma").unwrap_or(0);
    let is_admin: bool = r.try_get::<i64, _>("is_admin").unwrap_or(0) != 0;
    let created_str: String = r.try_get("created_at").unwrap_or_default();
    let created = chrono::DateTime::parse_from_rfc3339(&created_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(Json(json!({
        "kind": "t2",
        "data": {
            "id": id,
            "name": username,
            "display_name": username,
            "email": email,
            "avatar": avatar,
            "banner_color": banner_color,
            "bio": bio,
            "post_karma": pk,
            "comment_karma": ck,
            "total_karma": pk + ck,
            "is_admin": is_admin,
            "created_at": created,
        }
    })))
}

pub async fn posts_by_user(
    State(state): State<AppState>,
    MaybeAuthUser(_claims): MaybeAuthUser,
    Path(name): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = q.limit.unwrap_or(25).clamp(1, 100);
    let order = match q.sort.as_deref().unwrap_or("new") {
        "top" => "p.score DESC",
        "controversial" => "p.controversial_score DESC",
        "hot" => "p.hot_score DESC",
        _ => "p.created_at DESC",
    };
    let sql = format!(
        "SELECT p.id, s.name, u.username, p.kind, p.title, COALESCE(p.body, '') AS body, p.url,
                p.flair_text, p.thumbnail, p.score, p.ups, p.downs, p.upvote_ratio,
                p.num_comments, p.views, p.is_sticky, p.is_locked, p.is_spoiler, p.is_nsfw,
                p.created_at
         FROM posts p
         JOIN subreddits s ON s.id = p.subreddit_id
         JOIN users u ON u.id = p.author_id
         WHERE u.username = ? AND p.is_removed = 0 AND p.is_deleted = 0
         ORDER BY {} LIMIT ?",
        order
    );

    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(&sql)
        .bind(&name)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;

    let children: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let id: String = r.try_get("id").unwrap_or_default();
            let subreddit: String = r.try_get("name").unwrap_or_default();
            let author: String = r.try_get("username").unwrap_or_default();
            let kind: String = r.try_get("kind").unwrap_or_else(|_| "self".into());
            let title: String = r.try_get("title").unwrap_or_default();
            let body: String = r.try_get("body").unwrap_or_default();
            let url: Option<String> = r.try_get("url").ok().flatten();
            let flair: Option<String> = r.try_get("flair_text").ok().flatten();
            let thumbnail: Option<String> = r.try_get("thumbnail").ok().flatten();
            let score: i64 = r.try_get("score").unwrap_or(0);
            let ups: i64 = r.try_get("ups").unwrap_or(0);
            let downs: i64 = r.try_get("downs").unwrap_or(0);
            let ratio: f64 = r.try_get("upvote_ratio").unwrap_or(1.0);
            let num_comments: i64 = r.try_get("num_comments").unwrap_or(0);
            let views: i64 = r.try_get("views").unwrap_or(0);
            let is_sticky: bool = r.try_get::<i64, _>("is_sticky").unwrap_or(0) != 0;
            let is_locked: bool = r.try_get::<i64, _>("is_locked").unwrap_or(0) != 0;
            let is_spoiler: bool = r.try_get::<i64, _>("is_spoiler").unwrap_or(0) != 0;
            let is_nsfw: bool = r.try_get::<i64, _>("is_nsfw").unwrap_or(0) != 0;
            let created_str: String = r.try_get("created_at").unwrap_or_default();
            let created = chrono::DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let is_self = kind == "self";

            json!({
                "kind": "t3",
                "data": {
                    "id": id,
                    "name": id,
                    "subreddit": subreddit,
                    "subreddit_name_prefixed": format!("r/{}", subreddit),
                    "author": author,
                    "kind": kind,
                    "title": title,
                    "url": if is_self { None } else { url.filter(|s| !s.is_empty()) },
                    "selftext": if is_self { Some(body) } else { None },
                    "flair_text": flair,
                    "thumbnail": thumbnail,
                    "score": score,
                    "ups": ups,
                    "downs": downs,
                    "upvote_ratio": ratio,
                    "num_comments": num_comments,
                    "views": views,
                    "is_sticky": is_sticky,
                    "is_locked": is_locked,
                    "is_spoiler": is_spoiler,
                    "over18": is_nsfw,
                    "is_self": is_self,
                    "created_at": created,
                    "likes": null,
                }
            })
        })
        .collect();
    let dist = children.len() as i64;
    Ok(Json(json!({
        "kind": "Listing",
        "data": { "after": null, "before": null, "children": children, "dist": dist }
    })))
}

pub async fn comments_by_user(
    State(state): State<AppState>,
    MaybeAuthUser(_claims): MaybeAuthUser,
    Path(name): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = q.limit.unwrap_or(25).clamp(1, 100);
    let order = match q.sort.as_deref().unwrap_or("new") {
        "top" => "c.score DESC",
        "controversial" => "c.controversial_score DESC",
        _ => "c.created_at DESC",
    };
    let sql = format!(
        "SELECT c.id, c.post_id, c.subreddit_id, s.name, u.username, c.author_id,
                c.parent_id, c.depth, c.body, c.score, c.ups, c.downs, c.upvote_ratio,
                c.num_replies, c.is_sticky, c.is_locked, c.is_op,
                c.is_removed, c.is_deleted, c.created_at
         FROM comments c
         JOIN subreddits s ON s.id = c.subreddit_id
         JOIN users u ON u.id = c.author_id
         WHERE u.username = ? AND c.is_removed = 0 AND c.is_deleted = 0
         ORDER BY {} LIMIT ?",
        order
    );

    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(&sql)
        .bind(&name)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;

    let children: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let id: String = r.try_get("id").unwrap_or_default();
            let post_id: String = r.try_get("post_id").unwrap_or_default();
            let subreddit: String = r.try_get("name").unwrap_or_default();
            let author: String = r.try_get("username").unwrap_or_default();
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
            let created_str: String = r.try_get("created_at").unwrap_or_default();
            let created = chrono::DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            json!({
                "kind": "t1",
                "data": {
                    "id": id,
                    "name": id,
                    "post_id": post_id,
                    "subreddit": subreddit,
                    "author": author,
                    "parent_id": parent_id,
                    "depth": depth,
                    "body": body,
                    "score": score,
                    "ups": ups,
                    "downs": downs,
                    "upvote_ratio": ratio,
                    "num_replies": num_replies,
                    "is_sticky": is_sticky,
                    "is_locked": is_locked,
                    "is_op": is_op,
                    "created_at": created,
                    "likes": null,
                }
            })
        })
        .collect();
    let dist = children.len() as i64;
    Ok(Json(json!({
        "kind": "Listing",
        "data": { "after": null, "before": null, "children": children, "dist": dist }
    })))
}
