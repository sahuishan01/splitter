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

#[derive(Debug, Serialize)]
pub struct AuthConfigResponse {
    pub allow_signup: bool,
    pub first_run: bool,
}

pub async fn get_auth_config(
    State(pool): State<SqlitePool>,
) -> Result<Json<AuthConfigResponse>, AppError> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    let first_run = count == 0;

    let allow_signup = if first_run {
        true
    } else {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM system_settings WHERE key = 'allow_signup'")
            .fetch_optional(&pool)
            .await?;
        match row {
            Some((val,)) => val == "true" || val == "1",
            None => std::env::var("DISABLE_SIGNUP").map(|v| v != "true" && v != "1").unwrap_or(true),
        }
    };

    Ok(Json(AuthConfigResponse {
        allow_signup,
        first_run,
    }))
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

    if !is_first_user {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM system_settings WHERE key = 'allow_signup'")
            .fetch_optional(&pool)
            .await?;
        let allow_signup = match row {
            Some((val,)) => val == "true" || val == "1",
            None => std::env::var("DISABLE_SIGNUP").map(|v| v != "true" && v != "1").unwrap_or(true),
        };

        if !allow_signup {
            return Err(AppError::Forbidden(
                "Public registration is disabled. Please contact an administrator to create your account.".to_string(),
            ));
        }
    }

    let admin_email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@splitter.local".into());
    let is_admin = is_first_user || email == admin_email.to_lowercase();

    let password_hash = hash_password(&payload.password)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))?;

    let user_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, display_name, is_admin, is_active, must_change_password, created_at)
         VALUES (?, ?, ?, ?, ?, 1, 0, ?)"
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
        must_change_password: false,
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

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn update_profile(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let name = payload.display_name.trim();
    if name.is_empty() || name.len() > 60 {
        return Err(AppError::BadRequest("Display name must be between 1 and 60 characters".to_string()));
    }

    sqlx::query("UPDATE users SET display_name = ? WHERE id = ?")
        .bind(name)
        .bind(&auth.0.sub)
        .execute(&pool)
        .await?;

    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&pool)
        .await?;

    let token = issue_token(&user.id, &user.email, &user.display_name, user.is_admin)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

pub async fn change_password(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if payload.new_password.len() < 6 {
        return Err(AppError::BadRequest("New password must be at least 6 characters".to_string()));
    }

    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if !verify_password(&payload.current_password, &user.password_hash) {
        return Err(AppError::Unauthorized("Current password is incorrect".to_string()));
    }

    let new_hash = hash_password(&payload.new_password)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))?;

    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(&new_hash)
        .bind(&user.id)
        .execute(&pool)
        .await?;

    Ok(Json(serde_json::json!({
        "message": "Password updated successfully"
    })))
}

#[derive(Debug, Deserialize)]
pub struct SetInitialPasswordRequest {
    pub new_password: String,
}

pub async fn set_initial_password(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Json(payload): Json<SetInitialPasswordRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if payload.new_password.len() < 6 {
        return Err(AppError::BadRequest("New password must be at least 6 characters".to_string()));
    }

    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let new_hash = hash_password(&payload.new_password)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))?;

    sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 0 WHERE id = ?")
        .bind(&new_hash)
        .bind(&user.id)
        .execute(&pool)
        .await?;

    let mut updated_user = user;
    updated_user.must_change_password = false;

    let token = issue_token(
        &updated_user.id,
        &updated_user.email,
        &updated_user.display_name,
        updated_user.is_admin,
    )?;

    Ok(Json(AuthResponse {
        token,
        user: updated_user.into(),
    }))
}

