use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    models::{
        Expense, ExpenseDetail, ExpenseSplitDetail, Group, GroupBalanceSummary,
        GroupMemberRow, IdempotencyRecord, MemberBalance, Settlement, SettlementDetail,
    },
    routes::groups::check_group_membership,
    services::settlement::simplify_debts,
};

#[derive(Debug, Deserialize)]
pub struct SplitInput {
    pub user_id: String,
    pub amount_cents: Option<i64>,
    pub percentage: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateExpenseRequest {
    pub description: String,
    pub amount_cents: i64,
    pub paid_by: Option<String>,
    pub currency: Option<String>,
    pub split_type: Option<String>, // EQUAL, EXACT, PERCENT
    pub category: Option<String>,
    pub expense_date: Option<String>,
    pub participants: Option<Vec<String>>,
    pub splits: Option<Vec<SplitInput>>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSettlementRequest {
    pub payer_id: String,
    pub payee_id: String,
    pub amount_cents: i64,
    pub currency: Option<String>,
    pub idempotency_key: Option<String>,
}

pub async fn create_expense(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path(group_id): Path<String>,
    Json(payload): Json<CreateExpenseRequest>,
) -> Result<Json<ExpenseDetail>, AppError> {
    check_group_membership(&pool, &group_id, &auth.0.sub).await?;

    // Check Idempotency Key
    if let Some(ref ikey) = payload.idempotency_key {
        let cached: Option<IdempotencyRecord> = sqlx::query_as(
            "SELECT * FROM idempotency_records WHERE idempotency_key = ?"
        )
        .bind(ikey)
        .fetch_optional(&pool)
        .await?;

        if let Some(record) = cached {
            if let Ok(detail) = serde_json::from_str::<ExpenseDetail>(&record.response_body) {
                return Ok(Json(detail));
            }
        }
    }

    if payload.description.trim().is_empty() {
        return Err(AppError::BadRequest("Expense description cannot be empty".to_string()));
    }
    if payload.amount_cents <= 0 {
        return Err(AppError::BadRequest("Expense amount must be greater than zero".to_string()));
    }

    let group: Group = sqlx::query_as("SELECT * FROM groups WHERE id = ?")
        .bind(&group_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    let paid_by = payload.paid_by.unwrap_or_else(|| auth.0.sub.clone());
    check_group_membership(&pool, &group_id, &paid_by).await?;

    let currency = payload.currency.unwrap_or(group.default_currency);
    let split_type = payload.split_type.unwrap_or_else(|| "EQUAL".to_string()).to_uppercase();
    let category = payload.category.unwrap_or_else(|| "General".to_string());
    let expense_date = payload
        .expense_date
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let now = chrono::Utc::now().to_rfc3339();

    // Calculate splits
    let mut calculated_splits: Vec<(String, i64, Option<f64>)> = Vec::new();

    match split_type.as_str() {
        "EQUAL" => {
            let participants = if let Some(parts) = payload.participants {
                if parts.is_empty() {
                    return Err(AppError::BadRequest("Participants list cannot be empty for EQUAL split".to_string()));
                }
                parts
            } else {
                // If not provided, default to all members of the group
                let members: Vec<(String,)> = sqlx::query_as(
                    "SELECT user_id FROM group_members WHERE group_id = ?"
                )
                .bind(&group_id)
                .fetch_all(&pool)
                .await?;
                members.into_iter().map(|m| m.0).collect()
            };

            let count = participants.len() as i64;
            let base_amount = payload.amount_cents / count;
            let mut remainder = payload.amount_cents % count;

            for user_id in participants {
                check_group_membership(&pool, &group_id, &user_id).await?;
                let mut split_amt = base_amount;
                if remainder > 0 {
                    split_amt += 1;
                    remainder -= 1;
                }
                calculated_splits.push((user_id, split_amt, None));
            }
        }
        "EXACT" => {
            let splits = payload
                .splits
                .ok_or_else(|| AppError::BadRequest("Splits array is required for EXACT split".to_string()))?;

            let mut total_split = 0i64;
            for split in splits {
                let amt = split
                    .amount_cents
                    .ok_or_else(|| AppError::BadRequest("amount_cents is required for each participant in EXACT split".to_string()))?;
                if amt <= 0 {
                    return Err(AppError::BadRequest("Split amount must be greater than zero".to_string()));
                }
                check_group_membership(&pool, &group_id, &split.user_id).await?;
                total_split += amt;
                calculated_splits.push((split.user_id, amt, None));
            }

            if total_split != payload.amount_cents {
                return Err(AppError::BadRequest(format!(
                    "Sum of splits ({} cents) does not match total amount ({} cents)",
                    total_split, payload.amount_cents
                )));
            }
        }
        "PERCENT" => {
            let splits = payload
                .splits
                .ok_or_else(|| AppError::BadRequest("Splits array is required for PERCENT split".to_string()))?;

            let mut total_pct = 0.0f64;
            let mut allocated_cents = 0i64;

            for split in &splits {
                let pct = split
                    .percentage
                    .ok_or_else(|| AppError::BadRequest("percentage is required for each participant in PERCENT split".to_string()))?;
                if pct <= 0.0 {
                    return Err(AppError::BadRequest("Split percentage must be greater than zero".to_string()));
                }
                check_group_membership(&pool, &group_id, &split.user_id).await?;
                total_pct += pct;

                let amt = ((payload.amount_cents as f64) * (pct / 100.0)).round() as i64;
                allocated_cents += amt;
                calculated_splits.push((split.user_id.clone(), amt, Some(pct)));
            }

            if (total_pct - 100.0).abs() > 0.01 {
                return Err(AppError::BadRequest(format!(
                    "Sum of percentages ({:.2}%) must equal 100%",
                    total_pct
                )));
            }

            // Adjust rounding discrepancy on the first split
            let diff = payload.amount_cents - allocated_cents;
            if diff != 0 && !calculated_splits.is_empty() {
                calculated_splits[0].1 += diff;
            }
        }
        _ => {
            return Err(AppError::BadRequest(
                "Invalid split_type. Allowed: EQUAL, EXACT, PERCENT".to_string(),
            ));
        }
    }

    let expense_id = Uuid::new_v4().to_string();

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO expenses (id, group_id, paid_by, description, amount_cents, currency, split_type, category, expense_date, created_at, idempotency_key)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&expense_id)
    .bind(&group_id)
    .bind(&paid_by)
    .bind(&payload.description)
    .bind(payload.amount_cents)
    .bind(&currency)
    .bind(&split_type)
    .bind(&category)
    .bind(&expense_date)
    .bind(&now)
    .bind(&payload.idempotency_key)
    .execute(&mut *tx)
    .await?;

    let mut split_details = Vec::new();

    for (uid, amt, pct) in calculated_splits {
        let split_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO expense_splits (id, expense_id, user_id, amount_cents, share_percentage)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&split_id)
        .bind(&expense_id)
        .bind(&uid)
        .bind(amt)
        .bind(pct)
        .execute(&mut *tx)
        .await?;

        let user_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
            .bind(&uid)
            .fetch_one(&mut *tx)
            .await?;

        split_details.push(ExpenseSplitDetail {
            user_id: uid,
            display_name: user_name.0,
            amount_cents: amt,
            share_percentage: pct,
        });
    }

    let payer_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&paid_by)
        .fetch_one(&mut *tx)
        .await?;

    let detail = ExpenseDetail {
        id: expense_id,
        group_id,
        paid_by,
        paid_by_name: payer_name.0,
        description: payload.description,
        amount_cents: payload.amount_cents,
        currency,
        split_type,
        category,
        expense_date,
        created_at: now.clone(),
        splits: split_details,
    };

    // Save idempotency record if key provided
    if let Some(ref ikey) = payload.idempotency_key {
        let body_json = serde_json::to_string(&detail).unwrap_or_default();
        sqlx::query(
            "INSERT INTO idempotency_records (idempotency_key, user_id, action, status_code, response_body, created_at)
             VALUES (?, ?, 'create_expense', 200, ?, ?)"
        )
        .bind(ikey)
        .bind(&auth.0.sub)
        .bind(&body_json)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(detail))
}

pub async fn list_expenses(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path(group_id): Path<String>,
) -> Result<Json<Vec<ExpenseDetail>>, AppError> {
    check_group_membership(&pool, &group_id, &auth.0.sub).await?;

    let expenses: Vec<Expense> = sqlx::query_as(
        "SELECT * FROM expenses WHERE group_id = ? ORDER BY expense_date DESC, created_at DESC"
    )
    .bind(&group_id)
    .fetch_all(&pool)
    .await?;

    let mut result = Vec::new();

    for exp in expenses {
        let payer_name: (String,) = sqlx::query_as(
            "SELECT display_name FROM users WHERE id = ?"
        )
        .bind(&exp.paid_by)
        .fetch_one(&pool)
        .await?;

        let splits: Vec<(String, i64, Option<f64>, String)> = sqlx::query_as(
            "SELECT s.user_id, s.amount_cents, s.share_percentage, u.display_name
             FROM expense_splits s
             JOIN users u ON s.user_id = u.id
             WHERE s.expense_id = ?"
        )
        .bind(&exp.id)
        .fetch_all(&pool)
        .await?;

        let split_details = splits
            .into_iter()
            .map(|(uid, amt, pct, name)| ExpenseSplitDetail {
                user_id: uid,
                display_name: name,
                amount_cents: amt,
                share_percentage: pct,
            })
            .collect();

        result.push(ExpenseDetail {
            id: exp.id,
            group_id: exp.group_id,
            paid_by: exp.paid_by,
            paid_by_name: payer_name.0,
            description: exp.description,
            amount_cents: exp.amount_cents,
            currency: exp.currency,
            split_type: exp.split_type,
            category: exp.category,
            expense_date: exp.expense_date,
            created_at: exp.created_at,
            splits: split_details,
        });
    }

    Ok(Json(result))
}

pub async fn delete_expense(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path((group_id, expense_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = check_group_membership(&pool, &group_id, &auth.0.sub).await?;

    let expense: Expense = sqlx::query_as("SELECT * FROM expenses WHERE id = ? AND group_id = ?")
        .bind(&expense_id)
        .bind(&group_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Expense not found".to_string()))?;

    // Allow deleting if group admin or the person who paid
    if role != "admin" && expense.paid_by != auth.0.sub {
        return Err(AppError::Forbidden("Only the payer or group admin can delete an expense".to_string()));
    }

    sqlx::query("DELETE FROM expenses WHERE id = ?")
        .bind(&expense_id)
        .execute(&pool)
        .await?;

    Ok(Json(serde_json::json!({ "message": "Expense deleted successfully" })))
}

pub async fn get_group_balances(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path(group_id): Path<String>,
) -> Result<Json<GroupBalanceSummary>, AppError> {
    check_group_membership(&pool, &group_id, &auth.0.sub).await?;

    let group: Group = sqlx::query_as("SELECT * FROM groups WHERE id = ?")
        .bind(&group_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    let members: Vec<GroupMemberRow> = sqlx::query_as(
        "SELECT gm.id, gm.group_id, gm.user_id, gm.role, gm.joined_at, u.email, u.display_name
         FROM group_members gm
         JOIN users u ON gm.user_id = u.id
         WHERE gm.group_id = ?"
    )
    .bind(&group_id)
    .fetch_all(&pool)
    .await?;

    let mut balances = Vec::new();

    for member in &members {
        // Total amount paid by this member in group expenses
        let (total_paid,): (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(amount_cents) FROM expenses WHERE group_id = ? AND paid_by = ?"
        )
        .bind(&group_id)
        .bind(&member.user_id)
        .fetch_one(&pool)
        .await?;

        // Total amount owed by this member from expense splits in this group
        let (total_owed,): (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(es.amount_cents)
             FROM expense_splits es
             JOIN expenses e ON es.expense_id = e.id
             WHERE e.group_id = ? AND es.user_id = ?"
        )
        .bind(&group_id)
        .bind(&member.user_id)
        .fetch_one(&pool)
        .await?;

        // Total settlements paid by this member
        let (settled_paid,): (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(amount_cents) FROM settlements WHERE group_id = ? AND payer_id = ?"
        )
        .bind(&group_id)
        .bind(&member.user_id)
        .fetch_one(&pool)
        .await?;

        // Total settlements received by this member
        let (settled_received,): (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(amount_cents) FROM settlements WHERE group_id = ? AND payee_id = ?"
        )
        .bind(&group_id)
        .bind(&member.user_id)
        .fetch_one(&pool)
        .await?;

        let paid = total_paid.unwrap_or(0);
        let owed = total_owed.unwrap_or(0);
        let s_paid = settled_paid.unwrap_or(0);
        let s_rec = settled_received.unwrap_or(0);

        // Net balance: positive means member is owed money, negative means member owes money
        let net = (paid - owed) + (s_paid - s_rec);

        balances.push(MemberBalance {
            user_id: member.user_id.clone(),
            display_name: member.display_name.clone().unwrap_or_else(|| "User".into()),
            net_balance_cents: net,
        });
    }

    let simplified = simplify_debts(&balances, &group.default_currency);

    Ok(Json(GroupBalanceSummary {
        group_id,
        currency: group.default_currency,
        balances,
        simplified_debts: simplified,
    }))
}

pub async fn create_settlement(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path(group_id): Path<String>,
    Json(payload): Json<CreateSettlementRequest>,
) -> Result<Json<SettlementDetail>, AppError> {
    check_group_membership(&pool, &group_id, &auth.0.sub).await?;
    check_group_membership(&pool, &group_id, &payload.payer_id).await?;
    check_group_membership(&pool, &group_id, &payload.payee_id).await?;

    if payload.payer_id == payload.payee_id {
        return Err(AppError::BadRequest("Payer and payee cannot be the same user".to_string()));
    }
    if payload.amount_cents <= 0 {
        return Err(AppError::BadRequest("Settlement amount must be greater than zero".to_string()));
    }

    // Check Idempotency Key
    if let Some(ref ikey) = payload.idempotency_key {
        let cached: Option<IdempotencyRecord> = sqlx::query_as(
            "SELECT * FROM idempotency_records WHERE idempotency_key = ?"
        )
        .bind(ikey)
        .fetch_optional(&pool)
        .await?;

        if let Some(record) = cached {
            if let Ok(detail) = serde_json::from_str::<SettlementDetail>(&record.response_body) {
                return Ok(Json(detail));
            }
        }
    }

    let group: Group = sqlx::query_as("SELECT * FROM groups WHERE id = ?")
        .bind(&group_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    let settlement_id = Uuid::new_v4().to_string();
    let currency = payload.currency.unwrap_or(group.default_currency);
    let now = chrono::Utc::now().to_rfc3339();

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO settlements (id, group_id, payer_id, payee_id, amount_cents, currency, settled_at, created_at, idempotency_key)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&settlement_id)
    .bind(&group_id)
    .bind(&payload.payer_id)
    .bind(&payload.payee_id)
    .bind(payload.amount_cents)
    .bind(&currency)
    .bind(&now)
    .bind(&now)
    .bind(&payload.idempotency_key)
    .execute(&mut *tx)
    .await?;

    let payer_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&payload.payer_id)
        .fetch_one(&mut *tx)
        .await?;

    let payee_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&payload.payee_id)
        .fetch_one(&mut *tx)
        .await?;

    let detail = SettlementDetail {
        id: settlement_id,
        group_id,
        payer_id: payload.payer_id,
        payer_name: payer_name.0,
        payee_id: payload.payee_id,
        payee_name: payee_name.0,
        amount_cents: payload.amount_cents,
        currency,
        settled_at: now.clone(),
        created_at: now.clone(),
    };

    if let Some(ref ikey) = payload.idempotency_key {
        let body_json = serde_json::to_string(&detail).unwrap_or_default();
        sqlx::query(
            "INSERT INTO idempotency_records (idempotency_key, user_id, action, status_code, response_body, created_at)
             VALUES (?, ?, 'record_settlement', 200, ?, ?)"
        )
        .bind(ikey)
        .bind(&auth.0.sub)
        .bind(&body_json)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(detail))
}

pub async fn list_settlements(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path(group_id): Path<String>,
) -> Result<Json<Vec<SettlementDetail>>, AppError> {
    check_group_membership(&pool, &group_id, &auth.0.sub).await?;

    let settlements: Vec<Settlement> = sqlx::query_as(
        "SELECT * FROM settlements WHERE group_id = ? ORDER BY settled_at DESC"
    )
    .bind(&group_id)
    .fetch_all(&pool)
    .await?;

    let mut result = Vec::new();

    for s in settlements {
        let payer_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
            .bind(&s.payer_id)
            .fetch_one(&pool)
            .await?;

        let payee_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
            .bind(&s.payee_id)
            .fetch_one(&pool)
            .await?;

        result.push(SettlementDetail {
            id: s.id,
            group_id: s.group_id,
            payer_id: s.payer_id,
            payer_name: payer_name.0,
            payee_id: s.payee_id,
            payee_name: payee_name.0,
            amount_cents: s.amount_cents,
            currency: s.currency,
            settled_at: s.settled_at,
            created_at: s.created_at,
        });
    }

    Ok(Json(result))
}
