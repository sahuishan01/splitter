use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    auth::{hash_password, issue_token, verify_password, AuthUser},
    error::AppError,
    models::{User, UserResponse},
};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

pub async fn register(
    State(pool): State<SqlitePool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = payload.email.trim().to_lowercase();
    let display_name = payload.display_name.trim();

    if email.is_empty() || payload.password.len() < 6 {
        return Err(AppError::BadRequest(
            "Email must not be empty and password must be at least 6 characters".to_string(),
        ));
    }

    let existing: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&pool)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("User with this email already exists".to_string()));
    }

    // Check if this is the first user in the system
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    let is_first_user = count.0 == 0;

    let admin_email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@splitter.local".into());
    let is_admin = is_first_user || email == admin_email.to_lowercase();

    let password_hash = hash_password(&payload.password)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))?;

    let user_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, display_name, is_admin, is_active, created_at)
         VALUES (?, ?, ?, ?, ?, 1, ?)"
    )
    .bind(&user_id)
    .bind(&email)
    .bind(&password_hash)
    .bind(display_name)
    .bind(is_admin)
    .bind(&now)
    .execute(&pool)
    .await?;

    let token = issue_token(&user_id, &email, display_name, is_admin)?;

    let user_res = UserResponse {
        id: user_id,
        email,
        display_name: display_name.to_string(),
        is_admin,
        is_active: true,
        created_at: now,
    };

    Ok(Json(AuthResponse {
        token,
        user: user_res,
    }))
}

pub async fn login(
    State(pool): State<SqlitePool>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = payload.email.trim().to_lowercase();

    let user: User = sqlx::query_as("SELECT * FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

    if !user.is_active {
        return Err(AppError::Forbidden("User account is deactivated. Contact an admin.".to_string()));
    }

    if !verify_password(&payload.password, &user.password_hash) {
        return Err(AppError::Unauthorized("Invalid email or password".to_string()));
    }

    let token = issue_token(&user.id, &user.email, &user.display_name, user.is_admin)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

pub async fn me(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
) -> Result<Json<UserResponse>, AppError> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}
