//! Refund — aggregate root for student fee refunds.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

/// Refund status states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    Requested,
    Approved,
    Processed,
    Rejected,
}

impl RefundStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            RefundStatus::Requested => "REQUESTED",
            RefundStatus::Approved => "APPROVED",
            RefundStatus::Processed => "PROCESSED",
            RefundStatus::Rejected => "REJECTED",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "REQUESTED" => RefundStatus::Requested,
            "APPROVED" => RefundStatus::Approved,
            "PROCESSED" => RefundStatus::Processed,
            "REJECTED" => RefundStatus::Rejected,
            _ => RefundStatus::Requested,
        }
    }
}

/// Refund payment mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundMode {
    NEFT,
    RTGS,
    IMPS,
    UPI,
    Cheque,
    Cash,
    CreditNote,
}

impl RefundMode {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            RefundMode::NEFT => "NEFT",
            RefundMode::RTGS => "RTGS",
            RefundMode::IMPS => "IMPS",
            RefundMode::UPI => "UPI",
            RefundMode::Cheque => "CHEQUE",
            RefundMode::Cash => "CASH",
            RefundMode::CreditNote => "CREDIT_NOTE",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "NEFT" => RefundMode::NEFT,
            "RTGS" => RefundMode::RTGS,
            "IMPS" => RefundMode::IMPS,
            "UPI" => RefundMode::UPI,
            "CHEQUE" => RefundMode::Cheque,
            "CASH" => RefundMode::Cash,
            "CREDIT_NOTE" => RefundMode::CreditNote,
            _ => RefundMode::NEFT,
        }
    }
}

/// A student fee refund record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refund {
    pub refund_id: EntityId<Refund>,
    pub tenant_id: TenantId,
    pub student_id: Option<Uuid>,
    pub amount: Money,
    pub refund_reason: String,
    pub frc_compliant_pct: Option<Decimal>,
    pub refund_mode: RefundMode,
    pub status: RefundStatus,
    pub linked_payment_id: Option<Uuid>,
    pub reversal_journal_id: Option<Uuid>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub processed_at: Option<DateTime<Utc>>,
    pub bank_transaction_ref: Option<String>,
    pub audit: AuditInfo,
}
