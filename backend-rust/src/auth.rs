//! JWT auth — Reddit uses OAuth2 access + refresh tokens. We use a single
//! JWT-signed access token + a stored refresh token (same shape as Reddit).

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // user id
    pub username: String,
    pub is_admin: bool,
    pub exp: i64,               // unix epoch seconds
    pub iat: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("missing or malformed Authorization header")]
    MissingHeader,
    #[error("invalid token: {0}")]
    InvalidToken(String),
    #[error("token expired")]
    Expired,
}

impl From<AuthError> for crate::errors::ApiError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::MissingHeader | AuthError::InvalidToken(_) | AuthError::Expired => {
                crate::errors::ApiError::Unauthorized(e.to_string())
            }
        }
    }
}

pub fn issue_access_token(user_id: &str, username: &str, is_admin: bool, secret: &str) -> anyhow::Result<String> {
    let now = Utc::now();
    let exp = now + Duration::minutes(15); // 15-min access token (Reddit: 1h)
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        is_admin,
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

pub fn verify_access_token(token: &str, secret: &str) -> Result<Claims, AuthError> {
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        if matches!(e.kind(), jsonwebtoken::errors::ErrorKind::ExpiredSignature) {
            AuthError::Expired
        } else {
            AuthError::InvalidToken(e.to_string())
        }
    })?;
    Ok(data.claims)
}

/// Hash a password using Argon2id (Reddit uses bcrypt; we use Argon2id — stronger).
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    use argon2::{password_hash::{SaltString, rand_core::OsRng}, Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    let hash = argon
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("argon2 hash error: {}", e))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let parsed = PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("argon2 parse error: {}", e))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Generate a base36 fullname — Reddit's "t1_", "t3_", "t5_" prefix + base36 ID.
/// We use a random UUID v4 and base36-encode its first 8 bytes.
pub fn generate_fullname(prefix: &str) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 8] = rng.gen();
    let n = u64::from_be_bytes(bytes);
    let base36 = to_base36(n);
    format!("{}_{}", prefix, base36)
}

fn to_base36(mut n: u64) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_else(|_| "0".to_string())
}

/// Extractor: pulls user claims from the Authorization header.
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = crate::errors::ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let auth = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AuthError::MissingHeader)?;

        let token = auth
            .strip_prefix("Bearer ")
            .or_else(|| auth.strip_prefix("bearer "))
            .ok_or_else(|| AuthError::MissingHeader)?;

        let claims = verify_access_token(token, &app_state.jwt_secret)
            .map_err(crate::errors::ApiError::from)?;
        Ok(AuthUser(claims))
    }
}

/// Optional extractor: returns None if no token (used for public endpoints that personalize for logged-in users).
#[derive(Debug, Clone)]
pub struct MaybeAuthUser(pub Option<Claims>);

impl<S> FromRequestParts<S> for MaybeAuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = crate::errors::ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let Some(auth) = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
        else {
            return Ok(MaybeAuthUser(None));
        };
        let Some(token) = auth
            .strip_prefix("Bearer ")
            .or_else(|| auth.strip_prefix("bearer "))
        else {
            return Ok(MaybeAuthUser(None));
        };
        match verify_access_token(token, &app_state.jwt_secret) {
            Ok(claims) => Ok(MaybeAuthUser(Some(claims))),
            Err(_) => Ok(MaybeAuthUser(None)),
        }
    }
}
