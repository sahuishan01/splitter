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
        ActivityLogDetail, Expense, ExpenseDetail, ExpenseSplitDetail, Group, GroupBalanceSummary,
        GroupMemberRow, IdempotencyRecord, MemberBalance, Settlement, SettlementDetail,
    },
    routes::groups::check_group_membership,
    services::{activity::log_activity, settlement::simplify_debts},
};

#[derive(Debug, Deserialize, Clone)]
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
    pub attachment_id: Option<String>,
    pub attachment_url: Option<String>,
    pub attachment_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExpenseRequest {
    pub description: String,
    pub amount_cents: i64,
    pub paid_by: Option<String>,
    pub currency: Option<String>,
    pub split_type: Option<String>, // EQUAL, EXACT, PERCENT
    pub category: Option<String>,
    pub expense_date: Option<String>,
    pub participants: Option<Vec<String>>,
    pub splits: Option<Vec<SplitInput>>,
    pub attachment_id: Option<String>,
    pub attachment_url: Option<String>,
    pub attachment_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSettlementRequest {
    pub payer_id: String,
    pub payee_id: String,
    pub amount_cents: i64,
    pub currency: Option<String>,
    pub idempotency_key: Option<String>,
}

fn format_money_str(cents: i64, currency: &str) -> String {
    let sym = match currency {
        "USD" => "$",
        "INR" => "₹",
        "EUR" => "€",
        "GBP" => "£",
        _ => "",
    };
    if sym.is_empty() {
        format!("{:.2} {}", (cents as f64) / 100.0, currency)
    } else {
        format!("{}{:.2}", sym, (cents as f64) / 100.0)
    }
}

async fn compute_splits(
    pool: &SqlitePool,
    group_id: &str,
    split_type: &str,
    amount_cents: i64,
    participants: Option<Vec<String>>,
    splits: Option<Vec<SplitInput>>,
) -> Result<Vec<(String, i64, Option<f64>)>, AppError> {
    let mut calculated_splits: Vec<(String, i64, Option<f64>)> = Vec::new();

    match split_type {
        "EQUAL" => {
            let parts = if let Some(p) = participants {
                if p.is_empty() {
                    return Err(AppError::BadRequest("Participants list cannot be empty for EQUAL split".to_string()));
                }
                p
            } else {
                let members: Vec<(String,)> = sqlx::query_as(
                    "SELECT user_id FROM group_members WHERE group_id = ?"
                )
                .bind(group_id)
                .fetch_all(pool)
                .await?;
                members.into_iter().map(|m| m.0).collect()
            };

            let count = parts.len() as i64;
            let base_amount = amount_cents / count;
            let mut remainder = amount_cents % count;

            for user_id in parts {
                check_group_membership(pool, group_id, &user_id).await?;
                let mut split_amt = base_amount;
                if remainder > 0 {
                    split_amt += 1;
                    remainder -= 1;
                }
                calculated_splits.push((user_id, split_amt, None));
            }
        }
        "EXACT" => {
            let split_list = splits
                .ok_or_else(|| AppError::BadRequest("Splits array is required for EXACT split".to_string()))?;

            if split_list.is_empty() {
                return Err(AppError::BadRequest("Splits list cannot be empty for EXACT split".to_string()));
            }

            let mut total_split = 0i64;
            for split in split_list {
                let amt = split
                    .amount_cents
                    .ok_or_else(|| AppError::BadRequest("amount_cents is required for each participant in EXACT split".to_string()))?;
                if amt <= 0 {
                    return Err(AppError::BadRequest("Split amount must be greater than zero".to_string()));
                }
                check_group_membership(pool, group_id, &split.user_id).await?;
                total_split += amt;
                calculated_splits.push((split.user_id, amt, None));
            }

            if total_split != amount_cents {
                return Err(AppError::BadRequest(format!(
                    "Sum of exact splits ({} cents) does not match total amount ({} cents)",
                    total_split, amount_cents
                )));
            }
        }
        "PERCENT" => {
            let split_list = splits
                .ok_or_else(|| AppError::BadRequest("Splits array is required for PERCENT split".to_string()))?;

            if split_list.is_empty() {
                return Err(AppError::BadRequest("Splits list cannot be empty for PERCENT split".to_string()));
            }

            let mut total_pct = 0.0f64;
            let mut allocated_cents = 0i64;

            for split in &split_list {
                let pct = split
                    .percentage
                    .ok_or_else(|| AppError::BadRequest("percentage is required for each participant in PERCENT split".to_string()))?;
                if pct <= 0.0 {
                    return Err(AppError::BadRequest("Split percentage must be greater than zero".to_string()));
                }
                check_group_membership(pool, group_id, &split.user_id).await?;
                total_pct += pct;

                let amt = ((amount_cents as f64) * (pct / 100.0)).round() as i64;
                allocated_cents += amt;
                calculated_splits.push((split.user_id.clone(), amt, Some(pct)));
            }

            if (total_pct - 100.0).abs() > 0.05 {
                return Err(AppError::BadRequest(format!(
                    "Sum of percentages ({:.2}%) must equal 100%",
                    total_pct
                )));
            }

            // Adjust rounding discrepancy on the first split
            let diff = amount_cents - allocated_cents;
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

    Ok(calculated_splits)
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
    let calculated_splits = compute_splits(
        &pool,
        &group_id,
        &split_type,
        payload.amount_cents,
        payload.participants,
        payload.splits,
    )
    .await?;

    let expense_id = Uuid::new_v4().to_string();

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO expenses (id, group_id, paid_by, created_by, description, amount_cents, currency, split_type, category, expense_date, created_at, idempotency_key, attachment_url, attachment_name, attachment_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&expense_id)
    .bind(&group_id)
    .bind(&paid_by)
    .bind(&auth.0.sub)
    .bind(&payload.description)
    .bind(payload.amount_cents)
    .bind(&currency)
    .bind(&split_type)
    .bind(&category)
    .bind(&expense_date)
    .bind(&now)
    .bind(&payload.idempotency_key)
    .bind(&payload.attachment_url)
    .bind(&payload.attachment_name)
    .bind(&payload.attachment_id)
    .execute(&mut *tx)
    .await?;

    if let Some(ref att_id) = payload.attachment_id {
        sqlx::query("UPDATE attachments SET expense_id = ?, group_id = ? WHERE id = ?")
            .bind(&expense_id)
            .bind(&group_id)
            .bind(att_id)
            .execute(&mut *tx)
            .await?;
    }

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

    let creator_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&mut *tx)
        .await?;

    // Log Activity
    let summary = format!(
        "{} added expense '{}' ({})",
        creator_name.0,
        payload.description,
        format_money_str(payload.amount_cents, &currency)
    );
    let details = serde_json::json!({
        "expense_id": expense_id,
        "amount_cents": payload.amount_cents,
        "currency": currency,
        "paid_by": paid_by,
        "paid_by_name": payer_name.0,
        "split_type": split_type
    });
    log_activity(
        &mut *tx,
        &group_id,
        &auth.0.sub,
        "CREATE_EXPENSE",
        Some(&expense_id),
        &summary,
        Some(&details),
    )
    .await?;

    let detail = ExpenseDetail {
        id: expense_id,
        group_id,
        paid_by,
        paid_by_name: payer_name.0,
        created_by: auth.0.sub.clone(),
        created_by_name: creator_name.0,
        description: payload.description,
        amount_cents: payload.amount_cents,
        currency,
        split_type,
        category,
        expense_date,
        created_at: now.clone(),
        splits: split_details,
        attachment_url: payload.attachment_url,
        attachment_name: payload.attachment_name,
        attachment_id: payload.attachment_id,
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

pub async fn update_expense(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path((group_id, expense_id)): Path<(String, String)>,
    Json(payload): Json<UpdateExpenseRequest>,
) -> Result<Json<ExpenseDetail>, AppError> {
    let role = check_group_membership(&pool, &group_id, &auth.0.sub).await?;

    let existing: Expense = sqlx::query_as(
        "SELECT * FROM expenses WHERE id = ? AND group_id = ?"
    )
    .bind(&expense_id)
    .bind(&group_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Expense not found".to_string()))?;

    // Authorization: Creator of the expense OR group admin OR system admin
    let creator_id = existing.created_by.clone().unwrap_or_else(|| existing.paid_by.clone());
    let can_edit = role == "admin" || auth.0.is_admin || creator_id == auth.0.sub;
    if !can_edit {
        return Err(AppError::Forbidden(
            "Only the person who added this expense or a group admin can edit it".to_string(),
        ));
    }

    if payload.description.trim().is_empty() {
        return Err(AppError::BadRequest("Expense description cannot be empty".to_string()));
    }
    if payload.amount_cents <= 0 {
        return Err(AppError::BadRequest("Expense amount must be greater than zero".to_string()));
    }

    let paid_by = payload.paid_by.unwrap_or(existing.paid_by);
    check_group_membership(&pool, &group_id, &paid_by).await?;

    let currency = payload.currency.unwrap_or_else(|| existing.currency.clone());
    let split_type = payload
        .split_type
        .unwrap_or_else(|| existing.split_type.clone())
        .to_uppercase();
    let category = payload.category.unwrap_or_else(|| existing.category.clone());
    let expense_date = payload.expense_date.unwrap_or_else(|| existing.expense_date.clone());

    let calculated_splits = compute_splits(
        &pool,
        &group_id,
        &split_type,
        payload.amount_cents,
        payload.participants,
        payload.splits,
    )
    .await?;

    let mut tx = pool.begin().await?;

    let att_url = payload.attachment_url.or(existing.attachment_url);
    let att_name = payload.attachment_name.or(existing.attachment_name);
    let att_id = payload.attachment_id.or(existing.attachment_id);

    sqlx::query(
        "UPDATE expenses
         SET paid_by = ?, description = ?, amount_cents = ?, currency = ?, split_type = ?, category = ?, expense_date = ?, attachment_url = ?, attachment_name = ?, attachment_id = ?
         WHERE id = ?"
    )
    .bind(&paid_by)
    .bind(&payload.description)
    .bind(payload.amount_cents)
    .bind(&currency)
    .bind(&split_type)
    .bind(&category)
    .bind(&expense_date)
    .bind(&att_url)
    .bind(&att_name)
    .bind(&att_id)
    .bind(&expense_id)
    .execute(&mut *tx)
    .await?;

    if let Some(ref aid) = att_id {
        sqlx::query("UPDATE attachments SET expense_id = ?, group_id = ? WHERE id = ?")
            .bind(&expense_id)
            .bind(&group_id)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query("DELETE FROM expense_splits WHERE expense_id = ?")
        .bind(&expense_id)
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

    let creator_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&creator_id)
        .fetch_one(&mut *tx)
        .await?;

    let actor_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&mut *tx)
        .await?;

    // Log Activity
    let summary = format!(
        "{} edited expense '{}' (now {})",
        actor_name.0,
        payload.description,
        format_money_str(payload.amount_cents, &currency)
    );
    let details = serde_json::json!({
        "expense_id": expense_id,
        "old_description": existing.description,
        "new_description": payload.description,
        "old_amount_cents": existing.amount_cents,
        "new_amount_cents": payload.amount_cents,
        "old_split_type": existing.split_type,
        "new_split_type": split_type,
        "currency": currency,
        "paid_by": paid_by
    });
    log_activity(
        &mut *tx,
        &group_id,
        &auth.0.sub,
        "UPDATE_EXPENSE",
        Some(&expense_id),
        &summary,
        Some(&details),
    )
    .await?;

    tx.commit().await?;

    let detail = ExpenseDetail {
        id: expense_id,
        group_id,
        paid_by,
        paid_by_name: payer_name.0,
        created_by: creator_id,
        created_by_name: creator_name.0,
        description: payload.description,
        amount_cents: payload.amount_cents,
        currency,
        split_type,
        category,
        expense_date,
        created_at: existing.created_at,
        splits: split_details,
        attachment_url: att_url,
        attachment_name: att_name,
        attachment_id: att_id,
    };

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

        let creator_id = exp.created_by.clone().unwrap_or_else(|| exp.paid_by.clone());
        let creator_name: (String,) = sqlx::query_as(
            "SELECT display_name FROM users WHERE id = ?"
        )
        .bind(&creator_id)
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
            created_by: creator_id,
            created_by_name: creator_name.0,
            description: exp.description,
            amount_cents: exp.amount_cents,
            currency: exp.currency,
            split_type: exp.split_type,
            category: exp.category,
            expense_date: exp.expense_date,
            created_at: exp.created_at,
            splits: split_details,
            attachment_url: exp.attachment_url,
            attachment_name: exp.attachment_name,
            attachment_id: exp.attachment_id,
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

    // Allow deleting if group admin, system admin, person who added it, or payer
    let creator_id = expense.created_by.clone().unwrap_or_else(|| expense.paid_by.clone());
    let can_delete = role == "admin" || auth.0.is_admin || creator_id == auth.0.sub || expense.paid_by == auth.0.sub;
    if !can_delete {
        return Err(AppError::Forbidden("Only the creator, payer, or group admin can delete an expense".to_string()));
    }

    let actor_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&pool)
        .await?;

    // Log Activity
    let summary = format!(
        "{} deleted expense '{}' ({})",
        actor_name.0,
        expense.description,
        format_money_str(expense.amount_cents, &expense.currency)
    );
    let details = serde_json::json!({
        "expense_id": expense_id,
        "description": expense.description,
        "amount_cents": expense.amount_cents,
        "currency": expense.currency,
        "paid_by": expense.paid_by
    });
    let _ = log_activity(
        &pool,
        &group_id,
        &auth.0.sub,
        "DELETE_EXPENSE",
        Some(&expense_id),
        &summary,
        Some(&details),
    )
    .await;

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

    let actor_name: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&mut *tx)
        .await?;

    // Log Activity
    let summary = format!(
        "{} recorded settlement: {} paid {} ({})",
        actor_name.0,
        payer_name.0,
        payee_name.0,
        format_money_str(payload.amount_cents, &currency)
    );
    let details = serde_json::json!({
        "settlement_id": settlement_id,
        "payer_id": payload.payer_id,
        "payer_name": payer_name.0,
        "payee_id": payload.payee_id,
        "payee_name": payee_name.0,
        "amount_cents": payload.amount_cents,
        "currency": currency
    });
    log_activity(
        &mut *tx,
        &group_id,
        &auth.0.sub,
        "CREATE_SETTLEMENT",
        Some(&settlement_id),
        &summary,
        Some(&details),
    )
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

pub async fn list_activities(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    Path(group_id): Path<String>,
) -> Result<Json<Vec<ActivityLogDetail>>, AppError> {
    let role = check_group_membership(&pool, &group_id, &auth.0.sub).await?;
    if role != "admin" && !auth.0.is_admin {
        return Err(AppError::Forbidden(
            "Only group admins can view the activity log".to_string(),
        ));
    }

    let rows: Vec<(String, String, String, String, String, Option<String>, String, Option<String>, String)> = sqlx::query_as(
        "SELECT a.id, a.group_id, a.user_id, u.display_name as user_name, a.action, a.target_id, a.summary, a.details, a.created_at
         FROM activity_logs a
         JOIN users u ON a.user_id = u.id
         WHERE a.group_id = ?
         ORDER BY a.created_at DESC
         LIMIT 200"
    )
    .bind(&group_id)
    .fetch_all(&pool)
    .await?;

    let logs = rows
        .into_iter()
        .map(|(id, group_id, user_id, user_name, action, target_id, summary, details, created_at)| ActivityLogDetail {
            id,
            group_id,
            user_id,
            user_name,
            action,
            target_id,
            summary,
            details,
            created_at,
        })
        .collect();

    Ok(Json(logs))
}
