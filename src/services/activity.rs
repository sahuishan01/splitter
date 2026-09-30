use sqlx::{Executor, Sqlite};
use uuid::Uuid;

pub async fn log_activity<'a, E>(
    executor: E,
    group_id: &str,
    user_id: &str,
    action: &str,
    target_id: Option<&str>,
    summary: &str,
    details: Option<&serde_json::Value>,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let details_str = details.map(|d| d.to_string());

    sqlx::query(
        "INSERT INTO activity_logs (id, group_id, user_id, action, target_id, summary, details, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(group_id)
    .bind(user_id)
    .bind(action)
    .bind(target_id)
    .bind(summary)
    .bind(details_str)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(())
}
