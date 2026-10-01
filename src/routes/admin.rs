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

    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(&new_hash)
        .bind(&user.id)
        .execute(&pool)
        .await?;

    Ok(Json(serde_json::json!({
        "message": format!("Password reset successfully for user '{}'", user.email)
    })))
}

