//! Journal — the core aggregate root for double-entry accounting.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

/// Journal aggregate root.
///
/// A journal entry records a financial transaction with at least
/// two lines (one debit, one credit). Once posted, a journal is
/// immutable — corrections require reversing entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Journal {
    pub journal_id: EntityId<Journal>,
    pub tenant_id: TenantId,
    pub journal_number: String,
    pub journal_type: JournalType,
    pub accounting_period_id: Uuid,
    pub entity_id: Uuid,
    pub fund_id: Option<Uuid>,
    pub cost_center_id: Option<Uuid>,
    pub posting_date: NaiveDate,
    pub description: String,
    pub status: JournalStatus,
    pub total_debit: Money,
    pub total_credit: Money,
    pub lines: Vec<JournalLine>,
    pub posted_at: Option<DateTime<Utc>>,
    pub posted_by: Option<Uuid>,
    pub reversed_by_id: Option<EntityId<Journal>>,
    pub attachment_ids: Vec<Uuid>,
    pub version: i32,
    pub audit: AuditInfo,
}

/// A single debit or credit line within a journal entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalLine {
    pub journal_line_id: Uuid,
    pub journal_id: EntityId<Journal>,
    pub line_number: i32,
    pub account_id: Uuid,
    pub debit_amount: Option<Money>,
    pub credit_amount: Option<Money>,
    pub description: Option<String>,
    pub cost_center_id: Option<Uuid>,
    pub fund_id: Option<Uuid>,
    pub reference_id: Option<String>,
    pub reference_type: Option<String>,
    pub tax_rate: Option<rust_decimal::Decimal>,
    pub tax_amount: Option<Money>,
    pub is_itc_claimed: bool,
    pub itc_reversal_percent: Option<rust_decimal::Decimal>,
    pub version: i32,
}

/// Types of journal entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JournalType {
    Standard,
    Reversing,
    Adjustment,
    Opening,
    Closing,
    Rcm,
    ItcReversal,
    Tds,
    Accrual,
    Prepayment,
}

/// Journal lifecycle states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JournalStatus {
    Draft,
    Posted,
    Reversed,
    Cancelled,
}
