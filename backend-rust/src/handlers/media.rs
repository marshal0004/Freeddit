//! Media uploads handler — image/video upload via multipart form.
//! Files are stored locally under /home/z/my-project/uploads/{user_id}/{filename}.
//! Production would use S3 / GCS / Cloudflare R2.

use axum::{
    extract::{Multipart, State},
    Json,
};
use chrono::Utc;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::{auth::AuthUser, errors::ApiError, AppState};

const UPLOAD_DIR: &str = "/home/z/my-project/uploads";
const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50 MB

pub async fn upload_media(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut filename: Option<String> = None;
    let mut mime_type: Option<String> = None;
    let mut data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            mime_type = field.content_type().map(|s| s.to_string());
            filename = field.file_name().map(|s| s.to_string());
            let bytes = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("read error: {}", e)))?;
            if bytes.len() > MAX_FILE_SIZE {
                return Err(ApiError::BadRequest(format!(
                    "file too large (max {} bytes)",
                    MAX_FILE_SIZE
                )));
            }
            data = Some(bytes.to_vec());
        }
    }

    let Some(bytes) = data else {
        return Err(ApiError::BadRequest("no file uploaded".into()));
    };
    let mime = mime_type.unwrap_or_else(|| "application/octet-stream".into());
    let orig_name = filename.unwrap_or_else(|| "upload".into());

    // Determine kind from mime
    let kind = if mime.starts_with("image/") {
        "image"
    } else if mime.starts_with("video/") {
        "video"
    } else if mime == "image/gif" {
        "gif"
    } else {
        "other"
    };
    if kind == "other" {
        return Err(ApiError::BadRequest(format!(
            "unsupported mime type: {} (only image/* and video/* allowed)",
            mime
        )));
    }

    // Compute sha256
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha = hex::encode(hasher.finalize());

    // Create user-specific directory
    let user_dir = PathBuf::from(UPLOAD_DIR).join(&claims.sub);
    if let Err(e) = std::fs::create_dir_all(&user_dir) {
        return Err(ApiError::Internal(format!("create_dir_all failed: {}", e)));
    }

    // Generate a unique filename: sha256[:12] + original extension
    let ext = orig_name
        .rsplit('.')
        .next()
        .filter(|ext| !ext.contains('/') && ext.len() <= 8)
        .unwrap_or("bin");
    let safe_name = format!("{}.{}", &sha[..12], ext);
    let storage_path = user_dir.join(&safe_name);
    let url = format!("/uploads/{}/{}", &claims.sub, &safe_name);

    // Write to disk
    if let Err(e) = std::fs::write(&storage_path, &bytes) {
        return Err(ApiError::Internal(format!("write failed: {}", e)));
    }

    // Insert into DB
    let id = crate::auth::generate_fullname("med");
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO media_uploads (id, user_id, filename, original_name, mime_type, size_bytes, kind, storage_path, url, sha256, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&safe_name)
    .bind(&orig_name)
    .bind(&mime)
    .bind(bytes.len() as i64)
    .bind(kind)
    .bind(storage_path.to_string_lossy().to_string())
    .bind(&url)
    .bind(&sha)
    .bind(&now)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("db insert failed: {}", e)))?;

    Ok(Json(json!({
        "id": id,
        "url": url,
        "kind": kind,
        "mime_type": mime,
        "size": bytes.len(),
        "sha256": sha,
        "filename": safe_name,
        "created_at": now,
    })))
}

pub async fn list_my_media(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows: Vec<(String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, url, kind, mime_type, size_bytes, created_at FROM media_uploads WHERE user_id = ? ORDER BY created_at DESC LIMIT 100",
    )
    .bind(&claims.sub)
    .fetch_all(&state.pool)
    .await?;
    let media: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0,
                "url": r.1,
                "kind": r.2,
                "mime_type": r.3,
                "size": r.4,
                "created_at": r.5,
            })
        })
        .collect();
    Ok(Json(json!({ "media": media })))
}
