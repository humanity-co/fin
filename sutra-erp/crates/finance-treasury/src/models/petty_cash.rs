//! Treasury models — petty cash.
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PettyCashFundStatus {
    Active,
    Closed,
}

impl PettyCashFundStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "CLOSED" => PettyCashFundStatus::Closed,
            _ => PettyCashFundStatus::Active,
        }
    }
}

/// Petty-cash fund (imprest system). Per-entity single ACTIVE fund
/// (spec ASSUMPTION), imprest capped by `petty_cash_imprest_limit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PettyCashFund {
    pub petty_cash_fund_id: EntityId<PettyCashFund>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub fund_name: String,
    pub cash_gl_account_id: Uuid,
    pub imprest_amount: Money,
    pub balance: Money,
    pub custodian_id: Uuid,
    pub status: PettyCashFundStatus,
    pub version: i32,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PettyCashTransactionType {
    TopUp,
    Recoupment,
    Expense,
    Adjustment,
}

impl PettyCashTransactionType {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            PettyCashTransactionType::TopUp => "TOP_UP",
            PettyCashTransactionType::Recoupment => "RECOUPMENT",
            PettyCashTransactionType::Expense => "EXPENSE",
            PettyCashTransactionType::Adjustment => "ADJUSTMENT",
        }
    }
}

/// Petty-cash transaction — immutable fact row (INSERT-only). TOP_UP /
/// RECOUPMENT are funding inflows and may exceed the current balance;
/// EXPENSE is capped at the current balance and always carries an expense
/// account + voucher (internal-control rule).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PettyCashTransaction {
    pub petty_cash_txn_id: EntityId<PettyCashTransaction>,
    pub tenant_id: TenantId,
    pub petty_cash_fund_id: Uuid,
    pub transaction_type: PettyCashTransactionType,
    pub amount: Money,
    pub account_id: Option<Uuid>,
    pub voucher_no: Option<String>,
    pub narration: Option<String>,
    pub journal_id: Option<Uuid>,
    pub transaction_date: NaiveDate,
    pub recorded_by_id: Uuid,
    pub version: i32,
    pub audit: AuditInfo,
}
