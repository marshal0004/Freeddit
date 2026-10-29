//! Subreddit handlers.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;

use crate::{
    auth::{AuthUser, MaybeAuthUser},
    errors::ApiError,
    models::{CreateSubredditRequest, Listing, ListingData, Subreddit, Thing},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub after: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    MaybeAuthUser(maybe_claims): MaybeAuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Listing<Subreddit>>, ApiError> {
    let limit = q.limit.unwrap_or(25).clamp(1, 100);

    let rows: Vec<(String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, i64, i64, String, String, i64, String)> =
        if let Some(claims) = maybe_claims {
            sqlx::query_as(
                "SELECT s.id, s.name, s.display_name, s.description, s.public_description,
                        s.sidebar, s.icon, s.banner, s.subscribers, s.active_users,
                        s.type, s.primary_color, s.over18, s.created_at
                 FROM subreddits s
                 LEFT JOIN subscriptions sub ON sub.subreddit_id = s.id AND sub.user_id = ?
                 ORDER BY (CASE WHEN sub.user_id IS NULL THEN 1 ELSE 0 END), s.subscribers DESC
                 LIMIT ?",
            )
            .bind(&claims.sub)
            .bind(limit)
            .fetch_all(&state.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT id, name, display_name, description, public_description,
                        sidebar, icon, banner, subscribers, active_users,
                        type, primary_color, over18, created_at
                 FROM subreddits ORDER BY subscribers DESC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&state.pool)
            .await?
        };

    let children: Vec<Thing<Subreddit>> = rows
        .into_iter()
        .map(|r| Thing {
            kind: "t5".into(),
            data: Subreddit {
                id: r.0,
                display_name: r.1.clone(),
                display_name_prefixed: format!("r/{}", r.1),
                title: r.2,
                description: r.3,
                public_description: r.4,
                sidebar: r.5,
                icon: r.6,
                banner: r.7,
                primary_color: Some(r.11),
                subscribers: r.8,
                active_user_count: r.9,
                sub_type: r.10,
                over18: r.12 != 0,
                created_at: chrono::DateTime::parse_from_rfc3339(&r.13).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
            },
        })
        .collect();
    let dist = children.len() as i64;
    Ok(Json(Listing {
        kind: "Listing".into(),
        data: ListingData { after: None, before: None, children, dist },
    }))
}

pub async fn popular(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Listing<Subreddit>>, ApiError> {
    let limit = q.limit.unwrap_or(25).clamp(1, 100);
    let rows: Vec<(String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, i64, i64, String, String, i64, String)> =
        sqlx::query_as(
            "SELECT id, name, display_name, description, public_description,
                    sidebar, icon, banner, subscribers, active_users,
                    type, primary_color, over18, created_at
             FROM subreddits ORDER BY subscribers DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;
    let children = rows.into_iter().map(|r| Thing { kind: "t5".into(), data: Subreddit {
        id: r.0, display_name: r.1.clone(), display_name_prefixed: format!("r/{}", r.1),
        title: r.2, description: r.3, public_description: r.4, sidebar: r.5, icon: r.6, banner: r.7,
        primary_color: Some(r.11), subscribers: r.8, active_user_count: r.9, sub_type: r.10,
        over18: r.12 != 0,
        created_at: chrono::DateTime::parse_from_rfc3339(&r.13).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
    }}).collect();
    Ok(Json(Listing { kind: "Listing".into(), data: ListingData { after: None, before: None, children, dist: limit } }))
}

pub async fn get_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Thing<Subreddit>>, ApiError> {
    let row: Option<(String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, i64, i64, String, String, i64, String)> =
        sqlx::query_as(
            "SELECT id, name, display_name, description, public_description,
                    sidebar, icon, banner, subscribers, active_users,
                    type, primary_color, over18, created_at
             FROM subreddits WHERE name = ?",
        )
        .bind(&name)
        .fetch_optional(&state.pool)
        .await?;

    let Some(r) = row else {
        return Err(ApiError::NotFound(format!("r/{}", name)));
    };
    Ok(Json(Thing {
        kind: "t5".into(),
        data: Subreddit {
            id: r.0,
            display_name: r.1.clone(),
            display_name_prefixed: format!("r/{}", r.1),
            title: r.2,
            description: r.3,
            public_description: r.4,
            sidebar: r.5,
            icon: r.6,
            banner: r.7,
            primary_color: Some(r.11),
            subscribers: r.8,
            active_user_count: r.9,
            sub_type: r.10,
            over18: r.12 != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&r.13).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
        },
    }))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateSubredditRequest>,
) -> Result<Json<Thing<Subreddit>>, ApiError> {
    let name_lower = req.name.to_lowercase();
    if name_lower.len() < 3 || name_lower.len() > 21 {
        return Err(ApiError::BadRequest("subreddit name must be 3-21 chars".into()));
    }
    if !name_lower.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ApiError::BadRequest("subreddit name may only contain letters, digits, or _".into()));
    }

    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
        .bind(&name_lower)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_some() {
        return Err(ApiError::Conflict("subreddit already exists".into()));
    }

    let id = crate::auth::generate_fullname("t5");
    let now = Utc::now().to_rfc3339();
    let display = if req.display_name.is_empty() { name_lower.clone() } else { req.display_name };

    sqlx::query(
        "INSERT INTO subreddits (id, name, display_name, description, sidebar, public_description, primary_color, subscribers, active_users, type, over18, created_at, updated_at, creator_id)
         VALUES (?, ?, ?, ?, ?, NULL, '#0079D3', 1, 1, 'public', 0, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name_lower)
    .bind(&display)
    .bind(&req.description)
    .bind(&req.sidebar)
    .bind(&now)
    .bind(&now)
    .bind(&claims.sub)
    .execute(&state.pool)
    .await?;

    // Creator is auto-subscribed as moderator
    sqlx::query("INSERT INTO subscriptions (user_id, subreddit_id, role, created_at) VALUES (?, ?, 'moderator', ?)")
        .bind(&claims.sub)
        .bind(&id)
        .bind(&now)
        .execute(&state.pool)
        .await?;

    Ok(Json(Thing {
        kind: "t5".into(),
        data: Subreddit {
            id,
            display_name: name_lower.clone(),
            display_name_prefixed: format!("r/{}", name_lower),
            title: display,
            description: req.description,
            public_description: None,
            sidebar: req.sidebar,
            icon: None,
            banner: None,
            primary_color: Some("#0079D3".into()),
            subscribers: 1,
            active_user_count: 1,
            sub_type: "public".into(),
            over18: false,
            created_at: chrono::DateTime::parse_from_rfc3339(&now).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
        },
    }))
}

pub async fn subscribe(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let action = body.get("action").and_then(|v| v.as_str()).unwrap_or("sub");
    let sub: Option<(String,)> = sqlx::query_as("SELECT id FROM subreddits WHERE name = ?")
        .bind(&name)
        .fetch_optional(&state.pool)
        .await?;
    let Some((sub_id,)) = sub else {
        return Err(ApiError::NotFound(format!("r/{}", name)));
    };
    let now = Utc::now().to_rfc3339();
    if action == "unsub" {
        sqlx::query("DELETE FROM subscriptions WHERE user_id = ? AND subreddit_id = ?")
            .bind(&claims.sub)
            .bind(&sub_id)
            .execute(&state.pool)
            .await?;
        sqlx::query("UPDATE subreddits SET subscribers = MAX(0, subscribers - 1) WHERE id = ?")
            .bind(&sub_id)
            .execute(&state.pool)
            .await?;
    } else {
        sqlx::query("INSERT OR IGNORE INTO subscriptions (user_id, subreddit_id, role, created_at) VALUES (?, ?, 'subscriber', ?)")
            .bind(&claims.sub)
            .bind(&sub_id)
            .bind(&now)
            .execute(&state.pool)
            .await?;
        sqlx::query("UPDATE subreddits SET subscribers = subscribers + 1 WHERE id = ?")
            .bind(&sub_id)
            .execute(&state.pool)
            .await?;
    }
    Ok(Json(json!({ "subscribed": action != "unsub", "subreddit": name })))
}
