use anyhow::{Result, bail};
use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{PasswordHash, SaltString, rand_core::OsRng},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx_core::query::query;
use sqlx_core::row::Row;
use sqlx_postgres::{PgPool, PgRow, Postgres};
use uuid::Uuid;

const JWT_EXPIRY_SECONDS: usize = 7 * 24 * 60 * 60; // 7 days
const MIN_PASSWORD_LENGTH: usize = 8;
const API_KEY_PREFIX_LENGTH: usize = 12;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub account_id: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub key: String,
    pub prefix: String,
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {e}"))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid password hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn create_jwt(account_id: &Uuid, secret: &str) -> Result<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as usize;

    let claims = Claims {
        sub: account_id.to_string(),
        iat: now,
        exp: now + JWT_EXPIRY_SECONDS,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub async fn register(pool: &PgPool, email: &str, password: &str) -> Result<Uuid> {
    if password.len() < MIN_PASSWORD_LENGTH {
        bail!(
            "Password must be at least {} characters",
            MIN_PASSWORD_LENGTH
        );
    }

    let password_hash = hash_password(password)?;

    let row: PgRow = query::<Postgres>(
        "INSERT INTO accounts (email, password_hash) VALUES ($1, $2) RETURNING id",
    )
    .bind(email)
    .bind(&password_hash)
    .fetch_one(pool)
    .await?;

    let id: Uuid = row.get("id");
    Ok(id)
}

pub async fn login(
    pool: &PgPool,
    email: &str,
    password: &str,
    jwt_secret: &str,
) -> Result<AuthResponse> {
    let row: Option<PgRow> =
        query::<Postgres>("SELECT id, password_hash FROM accounts WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await?;

    let Some(row) = row else {
        bail!("Invalid email or password");
    };

    let account_id: Uuid = row.get("id");
    let stored_hash: String = row.get("password_hash");

    if !verify_password(password, &stored_hash)? {
        bail!("Invalid email or password");
    }

    let token = create_jwt(&account_id, jwt_secret)?;

    Ok(AuthResponse {
        token,
        account_id: account_id.to_string(),
    })
}

pub async fn generate_api_key(pool: &PgPool, account_id: &Uuid) -> Result<ApiKeyResponse> {
    let raw_key = format!("ctx_{}", Uuid::new_v4().as_simple());
    let prefix = raw_key[..API_KEY_PREFIX_LENGTH].to_string();
    let key_hash = hash_password(&raw_key)?;

    query::<Postgres>("INSERT INTO api_keys (account_id, key_hash, prefix) VALUES ($1, $2, $3)")
        .bind(account_id)
        .bind(&key_hash)
        .bind(&prefix)
        .execute(pool)
        .await?;

    Ok(ApiKeyResponse {
        key: raw_key,
        prefix,
    })
}

pub async fn verify_api_key(pool: &PgPool, key: &str) -> Result<Uuid> {
    if key.len() < API_KEY_PREFIX_LENGTH {
        bail!("Invalid API key format");
    }

    let prefix = &key[..API_KEY_PREFIX_LENGTH];

    let rows: Vec<PgRow> = query::<Postgres>(
        "SELECT account_id, key_hash FROM api_keys WHERE prefix = $1 AND revoked_at IS NULL",
    )
    .bind(prefix)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let stored_hash: String = row.get("key_hash");
        if verify_password(key, &stored_hash)? {
            let account_id: Uuid = row.get("account_id");
            return Ok(account_id);
        }
    }

    bail!("Invalid API key")
}
