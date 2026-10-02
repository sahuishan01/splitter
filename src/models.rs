use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub is_admin: bool,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub is_admin: bool,
    pub is_active: bool,
    pub created_at: String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            display_name: u.display_name,
            is_admin: u.is_admin,
            is_active: u.is_active,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_currency: String,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct GroupMemberRow {
    pub id: String,
    pub group_id: String,
    pub user_id: String,
    pub role: String,
    pub joined_at: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Expense {
    pub id: String,
    pub group_id: String,
    pub paid_by: String,
    pub created_by: Option<String>,
    pub description: String,
    pub amount_cents: i64,
    pub currency: String,
    pub split_type: String,
    pub category: String,
    pub expense_date: String,
    pub created_at: String,
    pub idempotency_key: Option<String>,
    pub attachment_url: Option<String>,
    pub attachment_name: Option<String>,
    pub attachment_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
#[allow(dead_code)]
pub struct ExpenseSplit {
    pub id: String,
    pub expense_id: String,
    pub user_id: String,
    pub amount_cents: i64,
    pub share_percentage: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExpenseDetail {
    pub id: String,
    pub group_id: String,
    pub paid_by: String,
    pub paid_by_name: String,
    pub created_by: String,
    pub created_by_name: String,
    pub description: String,
    pub amount_cents: i64,
    pub currency: String,
    pub split_type: String,
    pub category: String,
    pub expense_date: String,
    pub created_at: String,
    pub splits: Vec<ExpenseSplitDetail>,
    pub attachment_url: Option<String>,
    pub attachment_name: Option<String>,
    pub attachment_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Attachment {
    pub id: String,
    pub expense_id: Option<String>,
    pub group_id: Option<String>,
    pub uploaded_by: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub content_type: String,
    pub ocr_text: Option<String>,
    pub detected_amount_cents: Option<i64>,
    pub detected_merchant: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttachmentResponse {
    pub id: String,
    pub file_url: String,
    pub file_name: String,
    pub file_size: i64,
    pub content_type: String,
    pub ocr_text: Option<String>,
    pub detected_amount: Option<f64>,
    pub detected_amount_cents: Option<i64>,
    pub detected_merchant: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExpenseSplitDetail {
    pub user_id: String,
    pub display_name: String,
    pub amount_cents: i64,
    pub share_percentage: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Settlement {
    pub id: String,
    pub group_id: String,
    pub payer_id: String,
    pub payee_id: String,
    pub amount_cents: i64,
    pub currency: String,
    pub settled_at: String,
    pub created_at: String,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettlementDetail {
    pub id: String,
    pub group_id: String,
    pub payer_id: String,
    pub payer_name: String,
    pub payee_id: String,
    pub payee_name: String,
    pub amount_cents: i64,
    pub currency: String,
    pub settled_at: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemberBalance {
    pub user_id: String,
    pub display_name: String,
    #[serde(default)]
    pub total_paid_cents: i64,
    #[serde(default)]
    pub total_owed_cents: i64,
    pub net_balance_cents: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimplifiedDebt {
    pub from_user_id: String,
    pub from_user_name: String,
    pub to_user_id: String,
    pub to_user_name: String,
    pub amount_cents: i64,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupBalanceSummary {
    pub group_id: String,
    pub currency: String,
    pub balances: Vec<MemberBalance>,
    #[serde(default)]
    pub direct_debts: Vec<SimplifiedDebt>,
    pub simplified_debts: Vec<SimplifiedDebt>,
    #[serde(default)]
    pub user_spent_cents: i64,
    #[serde(default)]
    pub total_expense_cents: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct IdempotencyRecord {
    pub idempotency_key: String,
    pub user_id: String,
    pub action: String,
    pub status_code: i64,
    pub response_body: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct ActivityLog {
    pub id: String,
    pub group_id: String,
    pub user_id: String,
    pub action: String,
    pub target_id: Option<String>,
    pub summary: String,
    pub details: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivityLogDetail {
    pub id: String,
    pub group_id: String,
    pub user_id: String,
    pub user_name: String,
    pub action: String,
    pub target_id: Option<String>,
    pub summary: String,
    pub details: Option<String>,
    pub created_at: String,
}
