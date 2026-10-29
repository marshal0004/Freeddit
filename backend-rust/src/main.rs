//! Freeddit backend — Rust reimplementation of Reddit's core services.
//!
//! Architecture (matches Reddit's published system design):
//!   - Web layer: axum (Reddit uses hyper + custom routing; axum is closest modern eq.)
//!   - Service layer: handlers modules mirroring Reddit's microservices
//!       AccountService -> handlers/users.rs
//!       SubredditService -> handlers/subreddits.rs
//!       LinkService (posts) -> handlers/posts.rs
//!       CommentService -> handlers/comments.rs
//!       VoteService -> handlers/votes.rs
//!       SearchService -> handlers/search.rs
//!   - Data layer: sqlx (production: Postgres for relational, Cassandra for votes/time-series)
//!   - Cache: moka in-memory (production: Redis/Memcached)
//!   - Ranking: Reddit's published Hot/Best/Top/New/Controversial/Rising algorithms (ranking.rs)
//!
//! Routes (REST + JSON, matches Reddit's oauth.reddit.com API shape):
//!   POST   /api/register                /api/login
//!   GET    /api/subreddits              POST /api/subreddits
//!   GET    /api/subreddits/:name        POST /api/subreddits/:name/subscribe
//!   GET    /api/posts                   (home feed, ?sort=hot|best|new|top|controversial|rising)
//!   GET    /api/subreddits/:name/posts
//!   POST   /api/posts
//!   GET    /api/posts/:id
//!   GET    /api/posts/:id/comments
//!   POST   /api/posts/:id/comments
//!   POST   /api/vote                    { target_id, direction: 1|0|-1 }
//!   GET    /api/users/:name
//!   GET    /api/users/:name/posts
//!   GET    /api/users/:name/comments
//!   GET    /api/search?q=...

use std::sync::Arc;
use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer, compression::CompressionLayer};
use moka::future::Cache;
use sqlx::SqlitePool;

mod db;
mod models;
mod auth;
mod ranking;
mod errors;
mod handlers;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub cache: Cache<String, String>, // simple JSON cache (production: Redis)
    pub jwt_secret: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "freeddit_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .init();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:///home/z/my-project/freeddit.db?mode=rwc".to_string());
    tracing::info!("Connecting to DB: {}", db_url);

    let pool = db::init_pool(&db_url).await?;
    db::run_migrations(&pool).await?;
    db::seed_if_empty(&pool).await?;

    let cache = Cache::builder()
        .max_capacity(10_000)
        .time_to_live(std::time::Duration::from_secs(60))
        .build();

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "freeddit-dev-secret-change-in-production".to_string());

    let state = AppState { pool, cache, jwt_secret };

    let app = Router::new()
        .merge(handlers::routes())
        .layer(CorsLayer::very_permissive())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok().and_then(|s| s.parse().ok())
        .unwrap_or(3001);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Freeddit backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
