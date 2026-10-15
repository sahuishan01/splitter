use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    auth::AdminUser,
    error::AppError,
    models::{User, UserResponse},
};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub is_admin: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct AdminStats {
    pub total_users: i64,
    pub total_groups: i64,
    pub total_expenses: i64,
    pub total_volume_cents: i64,
    pub total_settlements: i64,
}

pub async fn list_users(
    State(pool): State<SqlitePool>,
    _admin: AdminUser,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let users: Vec<User> = sqlx::query_as("SELECT * FROM users ORDER BY created_at DESC")
        .fetch_all(&pool)
        .await?;

    let responses: Vec<UserResponse> = users.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

pub async fn update_user_role(
    State(pool): State<SqlitePool>,
    _admin: AdminUser,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateRoleRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&pool)
        .await?;

    let mut user = user.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    sqlx::query("UPDATE users SET is_admin = ? WHERE id = ?")
        .bind(payload.is_admin)
        .bind(&user_id)
        .execute(&pool)
        .await?;

    user.is_admin = payload.is_admin;
    Ok(Json(user.into()))
}

pub async fn update_user_status(
    State(pool): State<SqlitePool>,
    admin: AdminUser,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<Json<UserResponse>, AppError> {
    // Prevent admin from disabling themselves
    if admin.0.sub == user_id && !payload.is_active {
        return Err(AppError::BadRequest("Admin cannot deactivate their own account".to_string()));
    }

    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&pool)
        .await?;

    let mut user = user.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    sqlx::query("UPDATE users SET is_active = ? WHERE id = ?")
        .bind(payload.is_active)
        .bind(&user_id)
        .execute(&pool)
        .await?;

    user.is_active = payload.is_active;
    Ok(Json(user.into()))
}

pub async fn get_admin_stats(
    State(pool): State<SqlitePool>,
    _admin: AdminUser,
) -> Result<Json<AdminStats>, AppError> {
    let (total_users,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;

    let (total_groups,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM groups")
        .fetch_one(&pool)
        .await?;

    let (total_expenses, total_volume_cents): (i64, Option<i64>) =
        sqlx::query_as("SELECT COUNT(*), SUM(amount_cents) FROM expenses")
            .fetch_one(&pool)
            .await?;

    let (total_settlements,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM settlements")
        .fetch_one(&pool)
        .await?;

    Ok(Json(AdminStats {
        total_users,
        total_groups,
        total_expenses,
        total_volume_cents: total_volume_cents.unwrap_or(0),
        total_settlements,
    }))
}

#[derive(Debug, Deserialize)]
pub struct AdminResetPasswordRequest {
    pub new_password: String,
}

pub async fn admin_reset_user_password(
    State(pool): State<SqlitePool>,
    _admin: AdminUser,
    Path(user_id): Path<String>,
    Json(payload): Json<AdminResetPasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if payload.new_password.len() < 6 {
        return Err(AppError::BadRequest("Password must be at least 6 characters".to_string()));
    }

    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&pool)
        .await?;

    let user = user.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let new_hash = crate::auth::hash_password(&payload.new_password)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to hash password: {}", e)))?;

    sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 1 WHERE id = ?")
        .bind(&new_hash)
        .bind(&user.id)
        .execute(&pool)
        .await?;

    Ok(Json(serde_json::json!({
        "message": format!("Password reset successfully for user '{}'. User will be prompted to set a new password upon first login.", user.email)
    })))
}

#[derive(Debug, Deserialize)]
pub struct CreateUserByAdminRequest {
    pub email: String,
    pub display_name: String,
    pub password: Option<String>,
    pub is_admin: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CreateUserByAdminResponse {
    pub user: UserResponse,
    pub temporary_password: String,
}

pub async fn create_user_by_admin(
    State(pool): State<SqlitePool>,
    _admin: AdminUser,
    Json(payload): Json<CreateUserByAdminRequest>,
) -> Result<Json<CreateUserByAdminResponse>, AppError> {
    let email = payload.email.trim().to_lowercase();
    let display_name = payload.display_name.trim();

    if email.is_empty() || display_name.is_empty() {
        return Err(AppError::BadRequest("Email and display name are required".to_string()));
    }

    let existing: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&pool)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("User with this email already exists".to_string()));
    }

    let temp_password = match payload.password {
        Some(p) if p.trim().len() >= 6 => p.trim().to_string(),
        _ => {
            let random_suffix = &Uuid::new_v4().simple().to_string()[..6];
            format!("Splitter#{}", random_suffix)
        }
    };

    let password_hash = crate::auth::hash_password(&temp_password)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))?;

    let user_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let is_admin = payload.is_admin.unwrap_or(false);

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, display_name, is_admin, is_active, must_change_password, created_at)
         VALUES (?, ?, ?, ?, ?, 1, 1, ?)"
    )
    .bind(&user_id)
    .bind(&email)
    .bind(&password_hash)
    .bind(display_name)
    .bind(is_admin)
    .bind(&now)
    .execute(&pool)
    .await?;

    let user_res = UserResponse {
        id: user_id,
        email,
        display_name: display_name.to_string(),
        is_admin,
        is_active: true,
        must_change_password: true,
        created_at: now,
    };

    Ok(Json(CreateUserByAdminResponse {
        user: user_res,
        temporary_password: temp_password,
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemSettingsResponse {
    pub allow_signup: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSystemSettingsRequest {
    pub allow_signup: bool,
}

pub async fn get_system_settings(
    State(pool): State<SqlitePool>,
    _admin: AdminUser,
) -> Result<Json<SystemSettingsResponse>, AppError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM system_settings WHERE key = 'allow_signup'")
        .fetch_optional(&pool)
        .await?;

    let allow_signup = match row {
        Some((val,)) => val == "true" || val == "1",
        None => std::env::var("DISABLE_SIGNUP").map(|v| v != "true" && v != "1").unwrap_or(true),
    };

    Ok(Json(SystemSettingsResponse { allow_signup }))
}

pub async fn update_system_settings(
    State(pool): State<SqlitePool>,
    _admin: AdminUser,
    Json(payload): Json<UpdateSystemSettingsRequest>,
) -> Result<Json<SystemSettingsResponse>, AppError> {
    let value_str = if payload.allow_signup { "true" } else { "false" };

    sqlx::query(
        "INSERT INTO system_settings (key, value, updated_at)
         VALUES ('allow_signup', ?, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"
    )
    .bind(value_str)
    .execute(&pool)
    .await?;

    Ok(Json(SystemSettingsResponse {
        allow_signup: payload.allow_signup,
    }))
}

