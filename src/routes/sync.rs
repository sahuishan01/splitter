use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    auth::AuthUser,
    error::AppError,
    models::{ExpenseDetail, IdempotencyRecord, SettlementDetail},
    routes::{
        expenses::{
            create_expense, create_settlement, list_expenses, list_settlements,
            CreateExpenseRequest, CreateSettlementRequest,
        },
        groups::check_group_membership,
    },
};

#[derive(Debug, Deserialize)]
pub struct QueuedMutation {
    pub idempotency_key: String,
    pub action: String, // "create_expense" or "record_settlement"
    pub group_id: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct BatchSyncRequest {
    pub mutations: Vec<QueuedMutation>,
}

#[derive(Debug, Serialize)]
pub struct MutationResult {
    pub idempotency_key: String,
    pub status: String, // "applied", "already_processed", "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchSyncResponse {
    pub results: Vec<MutationResult>,
}

#[derive(Debug, Deserialize)]
pub struct SyncChangesQuery {
    pub group_id: String,
    pub since: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GroupChangesResponse {
    pub group_id: String,
    pub expenses: Vec<ExpenseDetail>,
    pub settlements: Vec<SettlementDetail>,
    pub synced_at: String,
}

pub async fn batch_sync(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Json(payload): Json<BatchSyncRequest>,
) -> Result<Json<BatchSyncResponse>, AppError> {
    let mut results = Vec::new();

    for mutation in payload.mutations {
        let ikey = mutation.idempotency_key.clone();

        // 1. Check if already processed
        let cached: Option<IdempotencyRecord> = sqlx::query_as(
            "SELECT * FROM idempotency_records WHERE idempotency_key = ?"
        )
        .bind(&ikey)
        .fetch_optional(&pool)
        .await?;

        if let Some(record) = cached {
            let data: Option<serde_json::Value> = serde_json::from_str(&record.response_body).ok();
            results.push(MutationResult {
                idempotency_key: ikey,
                status: "already_processed".to_string(),
                data,
                error: None,
            });
            continue;
        }

        // 2. Process based on action
        match mutation.action.as_str() {
            "create_expense" => {
                match serde_json::from_value::<CreateExpenseRequest>(mutation.payload) {
                    Ok(mut req) => {
                        req.idempotency_key = Some(ikey.clone());
                        match create_expense(
                            State(pool.clone()),
                            auth.clone(),
                            axum::extract::Path(mutation.group_id.clone()),
                            Json(req),
                        )
                        .await
                        {
                            Ok(Json(detail)) => {
                                results.push(MutationResult {
                                    idempotency_key: ikey,
                                    status: "applied".to_string(),
                                    data: serde_json::to_value(detail).ok(),
                                    error: None,
                                });
                            }
                            Err(e) => {
                                results.push(MutationResult {
                                    idempotency_key: ikey,
                                    status: "error".to_string(),
                                    data: None,
                                    error: Some(e.to_string()),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        results.push(MutationResult {
                            idempotency_key: ikey,
                            status: "error".to_string(),
                            data: None,
                            error: Some(format!("Invalid create_expense payload: {}", e)),
                        });
                    }
                }
            }
            "record_settlement" => {
                match serde_json::from_value::<CreateSettlementRequest>(mutation.payload) {
                    Ok(mut req) => {
                        req.idempotency_key = Some(ikey.clone());
                        match create_settlement(
                            State(pool.clone()),
                            auth.clone(),
                            axum::extract::Path(mutation.group_id.clone()),
                            Json(req),
                        )
                        .await
                        {
                            Ok(Json(detail)) => {
                                results.push(MutationResult {
                                    idempotency_key: ikey,
                                    status: "applied".to_string(),
                                    data: serde_json::to_value(detail).ok(),
                                    error: None,
                                });
                            }
                            Err(e) => {
                                results.push(MutationResult {
                                    idempotency_key: ikey,
                                    status: "error".to_string(),
                                    data: None,
                                    error: Some(e.to_string()),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        results.push(MutationResult {
                            idempotency_key: ikey,
                            status: "error".to_string(),
                            data: None,
                            error: Some(format!("Invalid record_settlement payload: {}", e)),
                        });
                    }
                }
            }
            unknown => {
                results.push(MutationResult {
                    idempotency_key: ikey,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("Unsupported sync action: {}", unknown)),
                });
            }
        }
    }

    Ok(Json(BatchSyncResponse { results }))
}

pub async fn get_changes(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Query(query): Query<SyncChangesQuery>,
) -> Result<Json<GroupChangesResponse>, AppError> {
    check_group_membership(&pool, &query.group_id, &auth.0.sub).await?;

    let expenses = list_expenses(
        State(pool.clone()),
        auth.clone(),
        axum::extract::Path(query.group_id.clone()),
    )
    .await?
    .0;

    let settlements = list_settlements(
        State(pool.clone()),
        auth.clone(),
        axum::extract::Path(query.group_id.clone()),
    )
    .await?
    .0;

    // Filter by `since` if provided
    let filtered_expenses = if let Some(ref since_ts) = query.since {
        expenses
            .into_iter()
            .filter(|e| e.created_at >= *since_ts)
            .collect()
    } else {
        expenses
    };

    let filtered_settlements = if let Some(ref since_ts) = query.since {
        settlements
            .into_iter()
            .filter(|s| s.created_at >= *since_ts)
            .collect()
    } else {
        settlements
    };

    let now = chrono::Utc::now().to_rfc3339();

    Ok(Json(GroupChangesResponse {
        group_id: query.group_id,
        expenses: filtered_expenses,
        settlements: filtered_settlements,
        synced_at: now,
    }))
}
