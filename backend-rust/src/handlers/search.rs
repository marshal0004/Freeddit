//! Search handler — searches across posts, subreddits, and users.
//! Reddit uses Lucene/Solr; we use SQLite LIKE queries (production: Meilisearch or Elasticsearch).

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::{auth::MaybeAuthUser, errors::ApiError, AppState};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub r#type: Option<String>,
}

pub async fn search(
    State(state): State<AppState>,
    MaybeAuthUser(_): MaybeAuthUser,
    Query(q): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = q.limit.unwrap_or(25).clamp(1, 100);
    let pattern = format!("%{}%", q.q.replace('%', "\\%"));
    let mut results: Vec<serde_json::Value> = Vec::new();

    if matches!(q.r#type.as_deref(), None | Some("link")) {
        let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
            "SELECT p.id, s.name, u.username, p.kind, p.title, COALESCE(p.body, '') AS body,
                    p.url, p.thumbnail, p.score, p.ups, p.downs, p.num_comments,
                    p.created_at, p.flair_text, p.is_sticky, p.is_nsfw
             FROM posts p
             JOIN subreddits s ON s.id = p.subreddit_id
             JOIN users u ON u.id = p.author_id
             WHERE p.is_removed = 0 AND p.is_deleted = 0
               AND (p.title LIKE ? ESCAPE '\\' OR COALESCE(p.body, '') LIKE ? ESCAPE '\\')
             ORDER BY p.score DESC LIMIT ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;

        for r in rows {
            let id: String = r.try_get("id").unwrap_or_default();
            let subreddit: String = r.try_get("name").unwrap_or_default();
            let author: String = r.try_get("username").unwrap_or_default();
            let kind: String = r.try_get("kind").unwrap_or_else(|_| "self".into());
            let title: String = r.try_get("title").unwrap_or_default();
            let body: String = r.try_get("body").unwrap_or_default();
            let url: Option<String> = r.try_get("url").ok().flatten();
            let thumbnail: Option<String> = r.try_get("thumbnail").ok().flatten();
            let score: i64 = r.try_get("score").unwrap_or(0);
            let ups: i64 = r.try_get("ups").unwrap_or(0);
            let downs: i64 = r.try_get("downs").unwrap_or(0);
            let num_comments: i64 = r.try_get("num_comments").unwrap_or(0);
            let flair_text: Option<String> = r.try_get("flair_text").ok().flatten();
            let is_sticky: bool = r.try_get::<i64, _>("is_sticky").unwrap_or(0) != 0;
            let is_nsfw: bool = r.try_get::<i64, _>("is_nsfw").unwrap_or(0) != 0;
            let created_str: String = r.try_get("created_at").unwrap_or_default();
            let created = chrono::DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let is_self = kind == "self";

            results.push(json!({
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
                    "thumbnail": thumbnail,
                    "score": score,
                    "ups": ups,
                    "downs": downs,
                    "num_comments": num_comments,
                    "created_at": created,
                    "flair_text": flair_text,
                    "is_sticky": is_sticky,
                    "over18": is_nsfw,
                    "is_self": is_self,
                }
            }));
        }
    }

    if matches!(q.r#type.as_deref(), None | Some("sr")) {
        let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
            "SELECT id, name, display_name, public_description, subscribers, active_users
             FROM subreddits
             WHERE name LIKE ? ESCAPE '\\'
                OR display_name LIKE ? ESCAPE '\\'
                OR description LIKE ? ESCAPE '\\'
             ORDER BY subscribers DESC LIMIT ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;

        for r in rows {
            let id: String = r.try_get("id").unwrap_or_default();
            let name: String = r.try_get("name").unwrap_or_default();
            let display: String = r.try_get("display_name").unwrap_or_default();
            let public: Option<String> = r.try_get("public_description").ok().flatten();
            let subs: i64 = r.try_get("subscribers").unwrap_or(0);
            let active: i64 = r.try_get("active_users").unwrap_or(0);

            results.push(json!({
                "kind": "t5",
                "data": {
                    "id": id,
                    "display_name": name,
                    "display_name_prefixed": format!("r/{}", name),
                    "title": display,
                    "public_description": public,
                    "subscribers": subs,
                    "active_user_count": active,
                }
            }));
        }
    }

    if matches!(q.r#type.as_deref(), None | Some("user")) {
        let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
            "SELECT id, username, post_karma, comment_karma, created_at
             FROM users WHERE is_deleted = 0 AND username LIKE ? ESCAPE '\\'
             ORDER BY (post_karma + comment_karma) DESC LIMIT ?",
        )
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;

        for r in rows {
            let id: String = r.try_get("id").unwrap_or_default();
            let username: String = r.try_get("username").unwrap_or_default();
            let pk: i64 = r.try_get("post_karma").unwrap_or(0);
            let ck: i64 = r.try_get("comment_karma").unwrap_or(0);
            let created_str: String = r.try_get("created_at").unwrap_or_default();
            let created = chrono::DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(json!({
                "kind": "t2",
                "data": {
                    "id": id,
                    "name": username,
                    "display_name": username,
                    "post_karma": pk,
                    "comment_karma": ck,
                    "total_karma": pk + ck,
                    "created_at": created,
                }
            }));
        }
    }

    let dist = results.len() as i64;
    Ok(Json(json!({
        "kind": "Listing",
        "data": { "after": null, "before": null, "children": results, "dist": dist }
    })))
}
