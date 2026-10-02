use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    models::{Group, GroupMemberRow, User},
    services::activity::log_activity,
};

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
    pub default_currency: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub email: Option<String>,
    pub user_id: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_currency: String,
    pub created_by: String,
    pub created_at: String,
    pub member_count: i64,
    pub user_role: String,
    #[serde(default)]
    pub user_spent_cents: i64,
}

#[derive(Debug, Serialize)]
pub struct GroupDetail {
    pub group: Group,
    pub members: Vec<GroupMemberDetail>,
}

#[derive(Debug, Serialize)]
pub struct GroupMemberDetail {
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub joined_at: String,
    pub display_name: String,
    pub email: String,
}

pub async fn check_group_membership(
    pool: &SqlitePool,
    group_id: &str,
    user_id: &str,
) -> Result<String, AppError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT role FROM group_members WHERE group_id = ? AND user_id = ?"
    )
    .bind(group_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((role,)) => Ok(role),
        None => Err(AppError::Forbidden("You are not a member of this group".to_string())),
    }
}

pub async fn create_group(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Json(payload): Json<CreateGroupRequest>,
) -> Result<Json<Group>, AppError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Group name cannot be empty".to_string()));
    }

    let group_id = Uuid::new_v4().to_string();
    let desc = payload.description.unwrap_or_default();
    let currency = payload.default_currency.unwrap_or_else(|| "USD".to_string()).to_uppercase();
    let now = chrono::Utc::now().to_rfc3339();

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO groups (id, name, description, default_currency, created_by, created_at)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&group_id)
    .bind(name)
    .bind(&desc)
    .bind(&currency)
    .bind(&auth.0.sub)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO group_members (id, group_id, user_id, role, joined_at)
         VALUES (?, ?, ?, 'admin', ?)"
    )
    .bind(&member_id)
    .bind(&group_id)
    .bind(&auth.0.sub)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    let actor_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&mut *tx)
        .await?;

    let summary = format!("{} created group '{}'", actor_name.0, name);
    let details = serde_json::json!({
        "group_id": group_id,
        "default_currency": currency
    });
    let _ = log_activity(
        &mut *tx,
        &group_id,
        &auth.0.sub,
        "CREATE_GROUP",
        Some(&group_id),
        &summary,
        Some(&details),
    )
    .await;

    tx.commit().await?;

    let group = Group {
        id: group_id,
        name: name.to_string(),
        description: desc,
        default_currency: currency,
        created_by: auth.0.sub,
        created_at: now,
    };

    Ok(Json(group))
}

pub async fn list_groups(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
) -> Result<Json<Vec<GroupSummary>>, AppError> {
    let rows: Vec<(String, String, String, String, String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT g.id, g.name, g.description, g.default_currency, g.created_by, g.created_at, gm.role,
                (SELECT COUNT(*) FROM group_members WHERE group_id = g.id) as member_count,
                (SELECT COALESCE(SUM(es.amount_cents), 0)
                 FROM expense_splits es
                 JOIN expenses e ON es.expense_id = e.id
                 WHERE e.group_id = g.id AND es.user_id = ?) as user_spent_cents
         FROM groups g
         JOIN group_members gm ON g.id = gm.group_id
         WHERE gm.user_id = ?
         ORDER BY g.created_at DESC"
    )
    .bind(&auth.0.sub)
    .bind(&auth.0.sub)
    .fetch_all(&pool)
    .await?;

    let summaries = rows
        .into_iter()
        .map(|(id, name, desc, curr, created_by, created_at, role, member_count, user_spent_cents)| GroupSummary {
            id,
            name,
            description: desc,
            default_currency: curr,
            created_by,
            created_at,
            member_count,
            user_role: role,
            user_spent_cents,
        })
        .collect();

    Ok(Json(summaries))
}

pub async fn get_group(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path(group_id): Path<String>,
) -> Result<Json<GroupDetail>, AppError> {
    check_group_membership(&pool, &group_id, &auth.0.sub).await?;

    let group: Group = sqlx::query_as("SELECT * FROM groups WHERE id = ?")
        .bind(&group_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    let member_rows: Vec<GroupMemberRow> = sqlx::query_as(
        "SELECT gm.id, gm.group_id, gm.user_id, gm.role, gm.joined_at, u.email, u.display_name
         FROM group_members gm
         JOIN users u ON gm.user_id = u.id
         WHERE gm.group_id = ?
         ORDER BY gm.joined_at ASC"
    )
    .bind(&group_id)
    .fetch_all(&pool)
    .await?;

    let members = member_rows
        .into_iter()
        .map(|m| GroupMemberDetail {
            id: m.id,
            user_id: m.user_id,
            role: m.role,
            joined_at: m.joined_at,
            display_name: m.display_name.unwrap_or_default(),
            email: m.email.unwrap_or_default(),
        })
        .collect();

    Ok(Json(GroupDetail { group, members }))
}

pub async fn add_member(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path(group_id): Path<String>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<GroupMemberDetail>, AppError> {
    check_group_membership(&pool, &group_id, &auth.0.sub).await?;

    let target_user: User = if let Some(email) = payload.email {
        let clean_email = email.trim().to_lowercase();
        sqlx::query_as("SELECT * FROM users WHERE email = ?")
            .bind(&clean_email)
            .fetch_optional(&pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User with email '{}' not found", clean_email)))?
    } else if let Some(uid) = payload.user_id {
        sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(&uid)
            .fetch_optional(&pool)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?
    } else {
        return Err(AppError::BadRequest("Must provide email or user_id".to_string()));
    };

    // Check if already in group
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM group_members WHERE group_id = ? AND user_id = ?"
    )
    .bind(&group_id)
    .bind(&target_user.id)
    .fetch_optional(&pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("User is already a member of this group".to_string()));
    }

    let member_id = Uuid::new_v4().to_string();
    let role = payload.role.unwrap_or_else(|| "member".to_string());
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO group_members (id, group_id, user_id, role, joined_at)
         VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&member_id)
    .bind(&group_id)
    .bind(&target_user.id)
    .bind(&role)
    .bind(&now)
    .execute(&pool)
    .await?;

    let actor_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&pool)
        .await?;

    let summary = format!("{} added {} ({}) as {}", actor_name.0, target_user.display_name, target_user.email, role);
    let details = serde_json::json!({
        "member_id": member_id,
        "user_id": target_user.id,
        "user_name": target_user.display_name,
        "role": role,
    });
    let _ = log_activity(
        &pool,
        &group_id,
        &auth.0.sub,
        "ADD_MEMBER",
        Some(&target_user.id),
        &summary,
        Some(&details),
    )
    .await;

    Ok(Json(GroupMemberDetail {
        id: member_id,
        user_id: target_user.id,
        role,
        joined_at: now,
        display_name: target_user.display_name,
        email: target_user.email,
    }))
}

pub async fn remove_member(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path((group_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let caller_role = check_group_membership(&pool, &group_id, &auth.0.sub).await?;

    // Only group admin or the user themselves can remove
    if caller_role != "admin" && auth.0.sub != user_id {
        return Err(AppError::Forbidden("Only group admins can remove other members".to_string()));
    }

    let removed_user_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&pool)
        .await?
        .unwrap_or(("Member".to_string(),));

    let res = sqlx::query("DELETE FROM group_members WHERE group_id = ? AND user_id = ?")
        .bind(&group_id)
        .bind(&user_id)
        .execute(&pool)
        .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Member not found in group".to_string()));
    }

    let actor_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&pool)
        .await?;

    let summary = format!("{} removed {} from the group", actor_name.0, removed_user_name.0);
    let details = serde_json::json!({ "user_id": user_id });
    let _ = log_activity(
        &pool,
        &group_id,
        &auth.0.sub,
        "REMOVE_MEMBER",
        Some(&user_id),
        &summary,
        Some(&details),
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Member removed successfully" })))
}
