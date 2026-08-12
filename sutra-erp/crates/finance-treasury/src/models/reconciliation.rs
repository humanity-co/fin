//! Treasury models — bank reconciliation aggregate.
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{EntityId, Money, TenantId};
use uuid::Uuid;

/// Bank reconciliation lifecycle (spec state machine):
/// IN_PROGRESS → COMPLETED → VERIFIED.
/// Completing requires no UNMATCHED lines OR CFO-certified open items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BankReconciliationStatus {
    InProgress,
    Completed,
    Verified,
}

impl BankReconciliationStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "COMPLETED" => BankReconciliationStatus::Completed,
            "VERIFIED" => BankReconciliationStatus::Verified,
            _ => BankReconciliationStatus::InProgress,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            BankReconciliationStatus::InProgress => "IN_PROGRESS",
            BankReconciliationStatus::Completed => "COMPLETED",
            BankReconciliationStatus::Verified => "VERIFIED",
        }
    }
}

/// BankReconciliation aggregate root. One open reconciliation per account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankReconciliation {
    pub bank_reconciliation_id: EntityId<BankReconciliation>,
    pub tenant_id: TenantId,
    pub bank_account_id: Uuid,
    pub period_id: Uuid,
    pub statement_date: NaiveDate,
    pub opening_balance: Money,
    pub closing_balance: Money,
    pub status: BankReconciliationStatus,
    pub verified_by_id: Option<Uuid>,
    pub completed_at: Option<DateTime<Utc>>,
    pub version: i32,
    pub audit: sutra_core::AuditInfo,
}

/// Match status of a single statement line. PARTIAL_MATCH is never
/// auto-completed (spec rule 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatchStatus {
    Unmatched,
    Matched,
    PartialMatch,
    ManualMatch,
}

impl MatchStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "MATCHED" => MatchStatus::Matched,
            "PARTIAL_MATCH" => MatchStatus::PartialMatch,
            "MANUAL_MATCH" => MatchStatus::ManualMatch,
            _ => MatchStatus::Unmatched,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            MatchStatus::Unmatched => "UNMATCHED",
            MatchStatus::Matched => "MATCHED",
            MatchStatus::PartialMatch => "PARTIAL_MATCH",
            MatchStatus::ManualMatch => "MANUAL_MATCH",
        }
    }
}

/// A single line of an uploaded bank statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankStatementLine {
    pub bank_statement_line_id: EntityId<BankStatementLine>,
    pub tenant_id: TenantId,
    pub bank_reconciliation_id: Uuid,
    pub transaction_date: NaiveDate,
    /// UTR / transaction reference (required for auto-match candidates).
    pub transaction_ref: Option<String>,
    pub description: Option<String>,
    /// Exactly one of debit_amount / credit_amount is set (XOR).
    pub debit_amount: Option<Money>,
    pub credit_amount: Option<Money>,
    pub match_status: MatchStatus,
    pub matched_transaction_id: Option<Uuid>,
    pub matched_transaction_type: Option<String>,
    pub audit: sutra_core::AuditInfo,
}

impl BankStatementLine {
    /// The absolute amount of the line (debit XOR credit).
    pub fn amount(&self) -> Money {
        self.debit_amount
            .or(self.credit_amount)
            .unwrap_or(Money::ZERO)
    }
}

/// Immutable bank transaction register (INSERT-only). Running balance must
/// equal the prior balance ± amount. `is_reconciled` flips on match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransaction {
    pub bank_transaction_id: EntityId<BankTransaction>,
    pub tenant_id: TenantId,
    pub bank_account_id: Uuid,
    pub transaction_date: NaiveDate,
    pub value_date: Option<NaiveDate>,
    pub transaction_ref: Option<String>,
    pub description: Option<String>,
    pub debit_amount: Option<Money>,
    pub credit_amount: Option<Money>,
    pub balance: Money,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub is_reconciled: bool,
    pub reconciled_at: Option<DateTime<Utc>>,
    pub version: i32,
    pub audit: sutra_core::AuditInfo,
}

impl BankTransaction {
    pub fn amount(&self) -> Money {
        self.debit_amount
            .or(self.credit_amount)
            .unwrap_or(Money::ZERO)
    }
}
