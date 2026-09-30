use std::net::SocketAddr;
use sqlx::SqlitePool;

async fn bootstrap_admin_if_configured(pool: &SqlitePool) -> anyhow::Result<()> {
    let admin_email = match std::env::var("ADMIN_EMAIL") {
        Ok(email) if !email.trim().is_empty() => email.trim().to_lowercase(),
        _ => return Ok(()),
    };

    let admin_password = match std::env::var("ADMIN_PASSWORD") {
        Ok(pwd) if !pwd.trim().is_empty() => pwd,
        _ => return Ok(()),
    };

    let existing: Option<(String, bool)> = sqlx::query_as(
        "SELECT id, is_admin FROM users WHERE email = ?"
    )
    .bind(&admin_email)
    .fetch_optional(pool)
    .await?;

    match existing {
        Some((id, is_admin)) => {
            if !is_admin {
                sqlx::query("UPDATE users SET is_admin = 1 WHERE id = ?")
                    .bind(&id)
                    .execute(pool)
                    .await?;
                tracing::info!("Existing user '{}' promoted to admin", admin_email);
            }
        }
        None => {
            let password_hash = splitter::auth::hash_password(&admin_password)?;
            let user_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            sqlx::query(
                "INSERT INTO users (id, email, password_hash, display_name, is_admin, is_active, created_at)
                 VALUES (?, ?, ?, 'System Admin', 1, 1, ?)"
            )
            .bind(&user_id)
            .bind(&admin_email)
            .bind(&password_hash)
            .bind(&now)
            .execute(pool)
            .await?;

            tracing::info!("Bootstrap admin created: {}", admin_email);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://splitter.db".into());
    let pool = splitter::db::init_pool(&database_url).await?;
    tracing::info!("Database initialized and migrations applied");

    bootstrap_admin_if_configured(&pool).await?;

    let app = splitter::create_app(pool);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8088);

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    tracing::info!("Splitter server running on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
