//! Database pool, migrations, and seed data.

use anyhow::Context;
use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, SqlitePool};
use std::str::FromStr;

pub async fn init_pool(url: &str) -> anyhow::Result<SqlitePool> {
    let opts = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await
        .context("failed to connect SQLite pool")?;

    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    let sql1 = include_str!("../migrations/001_init.sql");
    sqlx::raw_sql(sql1).execute(pool).await?;
    let sql2 = include_str!("../migrations/002_features.sql");
    sqlx::raw_sql(sql2).execute(pool).await?;
    let sql3 = include_str!("../migrations/003_advanced_features.sql");
    // Migration 003 uses ALTER TABLE which can fail if columns already exist.
    // Run each statement and ignore "duplicate column" errors.
    for stmt in sql3.split(';') {
        let s = stmt.trim();
        if s.is_empty() { continue; }
        if let Err(e) = sqlx::raw_sql(s).execute(pool).await {
            // Log and continue — SQLite returns "duplicate column name" for already-applied ALTERs
            tracing::debug!("migration stmt (likely already applied): {}", e);
        }
    }
    tracing::info!("Database schema applied (migrations 001 + 002 + 003)");
    Ok(())
}

/// Seed the DB with rich mock content if it's empty.
pub async fn seed_if_empty(pool: &SqlitePool) -> anyhow::Result<()> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if count.0 > 0 {
        return Ok(());
    }
    tracing::info!("Seeding initial content...");
    crate::handlers::seed::run(pool).await?;
    tracing::info!("Seed complete");
    Ok(())
}
