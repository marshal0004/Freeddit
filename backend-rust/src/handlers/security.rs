//! User account security: 2FA (TOTP), email verification, password reset.
//! Reddit uses TOTP for 2FA + email verification + password reset via email.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::fs;

use crate::{
    auth::{self, AuthUser},
    errors::ApiError,
    AppState,
};

const EMAIL_VERIF_DIR: &str = "/home/z/my-project/emails";

// ============ 2FA / TOTP ============

#[derive(Debug, Deserialize)]
pub struct EnableTotpRequest {
    #[serde(default)]
    pub code: Option<String>,  // If provided, finalize enable. If not, return secret + QR URL.
}

/// Step 1: Generate a TOTP secret + QR URL.
/// Step 2 (with code): Verify the code and enable TOTP.
pub async fn enable_2fa(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<EnableTotpRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Ensure user_settings row exists
    sqlx::query("INSERT OR IGNORE INTO user_settings (user_id, updated_at) VALUES (?, ?)")
        .bind(&claims.sub).bind(Utc::now().to_rfc3339())
        .execute(&state.pool).await?;

    if let Some(code) = &req.code {
        // Verify code against stored secret
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT totp_secret FROM user_settings WHERE user_id = ?",
        )
        .bind(&claims.sub)
        .fetch_optional(&state.pool)
        .await?;
        let Some((Some(secret),)) = row else {
            return Err(ApiError::BadRequest("no TOTP secret configured — call enable without code first".into()));
        };

        // Verify the code
        let totp = build_totp(&secret, &claims.username)?;
        if !totp.check_current(code).unwrap_or(false) {
            return Err(ApiError::BadRequest("invalid TOTP code".into()));
        }

        // Generate backup codes (10 codes, hashed)
        let mut backup_codes: Vec<String> = Vec::new();
        let mut hashed_codes: Vec<String> = Vec::new();
        for _ in 0..10 {
            let code = crate::auth::generate_fullname("bc").replace("bc_", "").to_uppercase();
            let mut hasher = sha2::Sha256::new();
            hasher.update(code.as_bytes());
            let hashed = hex::encode(hasher.finalize());
            backup_codes.push(code);
            hashed_codes.push(hashed);
        }
        let hashed_json = serde_json::to_string(&hashed_codes).unwrap_or_else(|_| "[]".into());

        sqlx::query("UPDATE user_settings SET totp_enabled = 1, totp_backup_codes = ? WHERE user_id = ?")
            .bind(&hashed_json).bind(&claims.sub)
            .execute(&state.pool).await?;

        return Ok(Json(json!({
            "enabled": true,
            "backup_codes": backup_codes,
            "warning": "Save these backup codes — they will only be shown once.",
        })));
    }

    // Step 1: generate secret + QR URL
    let secret_bytes: Vec<u8> = (0..20).map(|_| {
        let n: u8 = rand::Rng::gen(&mut rand::thread_rng());
        n
    }).collect();
    let totp = totp_rs::TOTP::new_unchecked(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("freeddit".to_string()),
        claims.username.clone(),
    );
    let secret = totp.get_secret_base32();

    // Store the secret (but mark as not-yet-enabled until verified)
    sqlx::query("UPDATE user_settings SET totp_secret = ? WHERE user_id = ?")
        .bind(&secret).bind(&claims.sub)
        .execute(&state.pool).await?;

    // Generate QR URL for authenticator apps
    let qr_url = format!(
        "otpauth://totp/freeddit:{}?secret={}&issuer=freeddit&algorithm=SHA1&digits=6&period=30",
        urlencoding::encode(&claims.username),
        secret
    );

    Ok(Json(json!({
        "enabled": false,
        "secret": secret,
        "qr_url": qr_url,
        "manual_entry": format!("Secret: {} (algorithm: SHA1, digits: 6, period: 30s)", secret),
    })))
}

pub async fn disable_2fa(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let code = body.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code.is_empty() {
        return Err(ApiError::BadRequest("code is required to disable 2FA".into()));
    }
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT totp_secret FROM user_settings WHERE user_id = ?")
        .bind(&claims.sub)
        .fetch_optional(&state.pool)
        .await?;
    let Some((Some(secret),)) = row else {
        return Err(ApiError::BadRequest("2FA is not enabled".into()));
    };
    let totp = build_totp(&secret, &claims.username)?;
    let valid = totp.check_current(code).unwrap_or(false);
    // Also check backup codes
    let mut backup_ok = false;
    if !valid {
        let codes_row: Option<(Option<String>,)> = sqlx::query_as("SELECT totp_backup_codes FROM user_settings WHERE user_id = ?")
            .bind(&claims.sub)
            .fetch_optional(&state.pool).await?;
        if let Some((Some(codes_json),)) = codes_row {
            let codes: Vec<String> = serde_json::from_str(&codes_json).unwrap_or_default();
            let mut hasher = sha2::Sha256::new();
            hasher.update(code.as_bytes());
            let hashed = hex::encode(hasher.finalize());
            if codes.contains(&hashed) {
                backup_ok = true;
                // Remove this backup code from the list
                let new_codes: Vec<String> = codes.into_iter().filter(|c| c != &hashed).collect();
                let new_json = serde_json::to_string(&new_codes).unwrap_or_else(|_| "[]".into());
                sqlx::query("UPDATE user_settings SET totp_backup_codes = ? WHERE user_id = ?")
                    .bind(&new_json).bind(&claims.sub)
                    .execute(&state.pool).await?;
            }
        }
    }
    if !valid && !backup_ok {
        return Err(ApiError::BadRequest("invalid TOTP code or backup code".into()));
    }
    sqlx::query("UPDATE user_settings SET totp_enabled = 0, totp_secret = NULL, totp_backup_codes = NULL WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&state.pool).await?;
    Ok(Json(json!({ "disabled": true })))
}

pub async fn get_2fa_status(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT totp_enabled FROM user_settings WHERE user_id = ?")
        .bind(&claims.sub)
        .fetch_optional(&state.pool)
        .await?;
    let enabled = row.map(|(e,)| e != 0).unwrap_or(false);
    Ok(Json(json!({ "two_factor_enabled": enabled })))
}

fn build_totp(secret: &str, username: &str) -> Result<totp_rs::TOTP, ApiError> {
    let secret_bytes = totp_rs::Secret::Encoded(secret.to_string())
        .to_bytes()
        .map_err(|e| ApiError::Internal(format!("totp secret decode: {}", e)))?;
    Ok(totp_rs::TOTP::new_unchecked(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("freeddit".to_string()),
        username.to_string(),
    ))
}

// ============ Email verification ============

#[derive(Debug, Deserialize)]
pub struct SendVerificationRequest {
    #[serde(default)]
    pub email: Option<String>,
}

/// Generate a verification token and (mock) send a verification email.
/// In production this would SMTP-send; in dev we write to /home/z/my-project/emails/{user}.txt
pub async fn send_email_verification(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<SendVerificationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get user's email
    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT email FROM users WHERE id = ?",
    )
    .bind(&claims.sub)
    .fetch_optional(&state.pool)
    .await?;
    let email_opt = row.and_then(|(e,)| e);
    let email = req.email.or(email_opt).unwrap_or_default();
    if email.is_empty() || !email.contains('@') {
        return Err(ApiError::BadRequest("valid email is required".into()));
    }

    // Update user's email if changed
    sqlx::query("UPDATE users SET email = ? WHERE id = ?")
        .bind(&email).bind(&claims.sub)
        .execute(&state.pool).await?;

    // Generate token (32 chars)
    let token = crate::auth::generate_fullname("evt").replace("evt_", "");
    let expires = (Utc::now() + Duration::hours(24)).to_rfc3339();

    sqlx::query("INSERT OR REPLACE INTO user_settings (user_id, email_verification_token, email_verification_expires, updated_at) VALUES (?, ?, ?, ?)")
        .bind(&claims.sub).bind(&token).bind(&expires).bind(Utc::now().to_rfc3339())
        .execute(&state.pool).await?;

    // Mock send email
    let _ = fs::create_dir_all(EMAIL_VERIF_DIR);
    let verification_url = format!("/api/verify-email?token={}&XTransformPort=3001", token);
    let email_body = format!(
        r#"To: {}
From: noreply@freeddit.local
Subject: Verify your freeddit account

Hi u/{},

Please verify your email by visiting:
{}

This link expires in 24 hours.

— freeddit"#,
        email, claims.username, verification_url
    );
    let _ = fs::write(
        format!("{}/verify-{}.txt", EMAIL_VERIF_DIR, claims.username),
        email_body,
    );

    Ok(Json(json!({
        "sent": true,
        "email": email,
        "expires_at": expires,
        "dev_note": "Email written to /home/z/my-project/emails/ — production would SMTP-send.",
    })))
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    pub token: String,
}

pub async fn verify_email(
    State(state): State<AppState>,
    Query(q): Query<VerifyEmailQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT user_id, email_verification_expires FROM user_settings WHERE email_verification_token = ?",
    )
    .bind(&q.token)
    .fetch_optional(&state.pool)
    .await?;
    let Some((user_id, expires_opt)) = row else {
        return Err(ApiError::BadRequest("invalid or expired verification token".into()));
    };
    if let Some(exp) = expires_opt {
        if let Ok(expires_dt) = chrono::DateTime::parse_from_rfc3339(&exp) {
            if Utc::now() > expires_dt.with_timezone(&Utc) {
                return Err(ApiError::BadRequest("verification token expired".into()));
            }
        }
    }
    sqlx::query("UPDATE user_settings SET email_verified = 1, email_verification_token = NULL, email_verification_expires = NULL WHERE user_id = ?")
        .bind(&user_id)
        .execute(&state.pool).await?;
    Ok(Json(json!({
        "verified": true,
        "user_id": user_id,
        "message": "Your email has been verified. Thank you!",
    })))
}

// ============ Password reset ============

#[derive(Debug, Deserialize)]
pub struct PasswordResetRequestRequest {
    pub username_or_email: String,
}

/// Step 1: user requests password reset.
/// Generates a token + (mock) sends email with reset link.
pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetRequestRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let username_or_email = req.username_or_email.trim().to_lowercase();
    let row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, username, email FROM users WHERE LOWER(username) = ? OR LOWER(COALESCE(email, '')) = ? LIMIT 1",
    )
    .bind(&username_or_email).bind(&username_or_email)
    .fetch_optional(&state.pool).await?;
    let Some((user_id, username, email_opt)) = row else {
        // Don't leak existence — return success even if user not found
        return Ok(Json(json!({ "sent": true, "note": "If the account exists, a reset link was sent." })));
    };

    let token = crate::auth::generate_fullname("prt").replace("prt_", "");
    let expires = (Utc::now() + Duration::hours(1)).to_rfc3339();

    sqlx::query("INSERT OR REPLACE INTO user_settings (user_id, password_reset_token, password_reset_expires, updated_at) VALUES (?, ?, ?, ?)")
        .bind(&user_id).bind(&token).bind(&expires).bind(Utc::now().to_rfc3339())
        .execute(&state.pool).await?;

    // Mock send email
    let _ = fs::create_dir_all(EMAIL_VERIF_DIR);
    let reset_url = format!("/#/reset-password?token={}", token);
    let email_body = format!(
        r#"To: {}
From: noreply@freeddit.local
Subject: Reset your freeddit password

Hi u/{},

We received a request to reset your password. Visit:
{}

This link expires in 1 hour. If you didn't request this, ignore this email.

— freeddit"#,
        email_opt.as_deref().unwrap_or("(no email on file)"),
        username,
        reset_url,
    );
    let _ = fs::write(
        format!("{}/reset-{}.txt", EMAIL_VERIF_DIR, username),
        email_body,
    );

    Ok(Json(json!({
        "sent": true,
        "note": "If the account exists, a reset link was sent.",
        "token": token,  // Returned in dev only — production would NOT include this.
        "dev_note": "Email written to /home/z/my-project/emails/ — production would SMTP-send.",
    })))
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetConfirmRequest {
    pub token: String,
    pub new_password: String,
}

pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetConfirmRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.new_password.len() < 6 {
        return Err(ApiError::BadRequest("password must be at least 6 chars".into()));
    }
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT user_id, password_reset_expires FROM user_settings WHERE password_reset_token = ?",
    )
    .bind(&req.token)
    .fetch_optional(&state.pool)
    .await?;
    let Some((user_id, expires_opt)) = row else {
        return Err(ApiError::BadRequest("invalid or expired reset token".into()));
    };
    if let Some(exp) = expires_opt {
        if let Ok(expires_dt) = chrono::DateTime::parse_from_rfc3339(&exp) {
            if Utc::now() > expires_dt.with_timezone(&Utc) {
                return Err(ApiError::BadRequest("reset token expired".into()));
            }
        }
    }
    let hash = auth::hash_password(&req.new_password)?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(&hash).bind(&user_id)
        .execute(&state.pool).await?;
    sqlx::query("UPDATE user_settings SET password_reset_token = NULL, password_reset_expires = NULL WHERE user_id = ?")
        .bind(&user_id)
        .execute(&state.pool).await?;
    Ok(Json(json!({
        "reset": true,
        "message": "Password reset successful. You can now log in with your new password.",
    })))
}

// ============ User settings ============

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub content_visibility: Option<String>,
    #[serde(default)]
    pub default_sort: Option<String>,
    #[serde(default)]
    pub default_comment_sort: Option<String>,
    #[serde(default)]
    pub email_on_pm: Option<bool>,
    #[serde(default)]
    pub email_on_mention: Option<bool>,
    #[serde(default)]
    pub email_on_reply: Option<bool>,
    #[serde(default)]
    pub show_online_status: Option<bool>,
    #[serde(default)]
    pub show_post_history: Option<bool>,
    #[serde(default)]
    pub show_comment_history: Option<bool>,
}

pub async fn get_settings(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT theme, content_visibility, default_sort, default_comment_sort,
                email_on_pm, email_on_mention, email_on_reply,
                show_online_status, show_post_history, show_comment_history,
                totp_enabled, email_verified
         FROM user_settings WHERE user_id = ?",
    )
    .bind(&claims.sub)
    .fetch_optional(&state.pool)
    .await?;
    let r = match row {
        Some(r) => r,
        None => {
            // Default settings
            return Ok(Json(json!({
                "theme": "system",
                "content_visibility": "safe",
                "default_sort": "hot",
                "default_comment_sort": "best",
                "email_on_pm": true,
                "email_on_mention": false,
                "email_on_reply": true,
                "show_online_status": true,
                "show_post_history": true,
                "show_comment_history": true,
                "two_factor_enabled": false,
                "email_verified": false,
            })));
        }
    };

    fn g(r: &sqlx::sqlite::SqliteRow, k: &str) -> String {
        r.try_get(k).unwrap_or_default()
    }
    fn gi(r: &sqlx::sqlite::SqliteRow, k: &str) -> bool {
        r.try_get::<i64, _>(k).unwrap_or(0) != 0
    }
    Ok(Json(json!({
        "theme": g(&r, "theme"),
        "content_visibility": g(&r, "content_visibility"),
        "default_sort": g(&r, "default_sort"),
        "default_comment_sort": g(&r, "default_comment_sort"),
        "email_on_pm": gi(&r, "email_on_pm"),
        "email_on_mention": gi(&r, "email_on_mention"),
        "email_on_reply": gi(&r, "email_on_reply"),
        "show_online_status": gi(&r, "show_online_status"),
        "show_post_history": gi(&r, "show_post_history"),
        "show_comment_history": gi(&r, "show_comment_history"),
        "two_factor_enabled": gi(&r, "totp_enabled"),
        "email_verified": gi(&r, "email_verified"),
    })))
}

pub async fn update_settings(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("INSERT OR IGNORE INTO user_settings (user_id, updated_at) VALUES (?, ?)")
        .bind(&claims.sub).bind(Utc::now().to_rfc3339())
        .execute(&state.pool).await?;

    let now = Utc::now().to_rfc3339();
    if let Some(theme) = &req.theme {
        let _ = sqlx::query("UPDATE user_settings SET theme = ?, updated_at = ? WHERE user_id = ?")
            .bind(theme).bind(&now).bind(&claims.sub).execute(&state.pool).await;
    }
    if let Some(vis) = &req.content_visibility {
        let _ = sqlx::query("UPDATE user_settings SET content_visibility = ?, updated_at = ? WHERE user_id = ?")
            .bind(vis).bind(&now).bind(&claims.sub).execute(&state.pool).await;
    }
    if let Some(s) = &req.default_sort {
        let _ = sqlx::query("UPDATE user_settings SET default_sort = ?, updated_at = ? WHERE user_id = ?")
            .bind(s).bind(&now).bind(&claims.sub).execute(&state.pool).await;
    }
    if let Some(s) = &req.default_comment_sort {
        let _ = sqlx::query("UPDATE user_settings SET default_comment_sort = ?, updated_at = ? WHERE user_id = ?")
            .bind(s).bind(&now).bind(&claims.sub).execute(&state.pool).await;
    }
    for (field, val) in [
        ("email_on_pm", req.email_on_pm),
        ("email_on_mention", req.email_on_mention),
        ("email_on_reply", req.email_on_reply),
        ("show_online_status", req.show_online_status),
        ("show_post_history", req.show_post_history),
        ("show_comment_history", req.show_comment_history),
    ] {
        if let Some(v) = val {
            let sql = format!("UPDATE user_settings SET {} = ?, updated_at = ? WHERE user_id = ?", field);
            let _ = sqlx::query(&sql)
                .bind(if v { 1 } else { 0 }).bind(&now).bind(&claims.sub).execute(&state.pool).await;
        }
    }
    Ok(Json(json!({ "updated": true, "updated_at": now })))
}

// ============ Account: update profile + delete account ============

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub banner_color: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

pub async fn update_profile(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let now = Utc::now().to_rfc3339();
    if let Some(a) = &req.avatar {
        sqlx::query("UPDATE users SET avatar = ?, updated_at = ? WHERE id = ?")
            .bind(a).bind(&now).bind(&claims.sub).execute(&state.pool).await?;
    }
    if let Some(b) = &req.banner_color {
        sqlx::query("UPDATE users SET banner_color = ?, updated_at = ? WHERE id = ?")
            .bind(b).bind(&now).bind(&claims.sub).execute(&state.pool).await?;
    }
    if let Some(bio) = &req.bio {
        if bio.len() > 200 {
            return Err(ApiError::BadRequest("bio too long (max 200 chars)".into()));
        }
        sqlx::query("UPDATE users SET bio = ?, updated_at = ? WHERE id = ?")
            .bind(bio).bind(&now).bind(&claims.sub).execute(&state.pool).await?;
    }
    if let Some(email) = &req.email {
        if !email.is_empty() && !email.contains('@') {
            return Err(ApiError::BadRequest("invalid email".into()));
        }
        sqlx::query("UPDATE users SET email = ?, updated_at = ? WHERE id = ?")
            .bind(email).bind(&now).bind(&claims.sub).execute(&state.pool).await?;
        // Mark email as unverified (will need to re-verify)
        sqlx::query("INSERT OR IGNORE INTO user_settings (user_id, updated_at) VALUES (?, ?)")
            .bind(&claims.sub).bind(&now).execute(&state.pool).await?;
        sqlx::query("UPDATE user_settings SET email_verified = 0, updated_at = ? WHERE user_id = ?")
            .bind(&now).bind(&claims.sub).execute(&state.pool).await?;
    }
    Ok(Json(json!({ "updated": true, "updated_at": now })))
}

#[derive(Debug, Deserialize)]
pub struct DeleteAccountRequest {
    pub password: String,
}

pub async fn delete_account(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<DeleteAccountRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT password_hash FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_optional(&state.pool).await?;
    let Some((hash,)) = row else {
        return Err(ApiError::NotFound("user".into()));
    };
    if !auth::verify_password(&req.password, &hash)? {
        return Err(ApiError::Unauthorized("incorrect password".into()));
    }
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE users SET is_deleted = 1, password_hash = '', email = NULL, bio = NULL, avatar = NULL, banner_color = NULL, updated_at = ? WHERE id = ?")
        .bind(&now).bind(&claims.sub)
        .execute(&state.pool).await?;
    Ok(Json(json!({
        "deleted": true,
        "message": "Account soft-deleted. Posts/comments remain but show as [deleted]."
    })))
}
