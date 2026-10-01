pub mod auth;
pub mod db;
pub mod error;
pub mod models;
pub mod routes;
pub mod services;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use sqlx::SqlitePool;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};

pub fn create_app(pool: SqlitePool) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_router = Router::new()
        // Auth
        .route("/auth/register", post(routes::auth::register))
        .route("/auth/login", post(routes::auth::login))
        .route("/auth/me", get(routes::auth::me))
        .route("/auth/profile", patch(routes::auth::update_profile))
        .route("/auth/change-password", post(routes::auth::change_password))
        // Admin
        .route("/admin/users", get(routes::admin::list_users))
        .route("/admin/users/:id/role", patch(routes::admin::update_user_role))
        .route("/admin/users/:id/status", patch(routes::admin::update_user_status))
        .route("/admin/users/:id/reset-password", post(routes::admin::admin_reset_user_password))
        .route("/admin/stats", get(routes::admin::get_admin_stats))
        // Groups
        .route("/groups", post(routes::groups::create_group).get(routes::groups::list_groups))
        .route("/groups/:id", get(routes::groups::get_group))
        .route("/groups/:id/members", post(routes::groups::add_member))
        .route("/groups/:id/members/:user_id", delete(routes::groups::remove_member))
        // Expenses & Balances & Settlements & Activities
        .route("/groups/:id/expenses", post(routes::expenses::create_expense).get(routes::expenses::list_expenses))
        .route(
            "/groups/:id/expenses/:expense_id",
            put(routes::expenses::update_expense)
                .patch(routes::expenses::update_expense)
                .delete(routes::expenses::delete_expense),
        )
        .route("/groups/:id/balances", get(routes::expenses::get_group_balances))
        .route("/groups/:id/settlements", post(routes::expenses::create_settlement).get(routes::expenses::list_settlements))
        .route("/groups/:id/activities", get(routes::expenses::list_activities))
        // Attachments & OCR
        .route("/upload", post(routes::attachments::upload_attachment))
        .route("/attachments/:id", get(routes::attachments::get_attachment_file))
        .route("/attachments/:id/info", get(routes::attachments::get_attachment_info))
        .route("/ocr/scan", post(routes::attachments::scan_ocr_direct))
        // Batch Offline Sync
        .route("/sync/batch", post(routes::sync::batch_sync))
        .route("/sync/changes", get(routes::sync::get_changes))
        .with_state(pool);

    Router::new()
        .nest("/api", api_router)
        .nest_service("/uploads", ServeDir::new("uploads"))
        .nest_service("/", ServeDir::new("static").fallback(ServeDir::new("static/index.html")))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
