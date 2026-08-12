//! Treasury models — inter-bank transfers.
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

/// Inter-bank transfer lifecycle (spec state machine):
/// INITIATED → APPROVED → PROCESSED → COMPLETED
/// INITIATED/APPROVED → CANCELLED ; PROCESSED → FAILED (bank rejection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InterBankTransferStatus {
    Initiated,
    Approved,
    Processed,
    Completed,
    Cancelled,
    Failed,
}

impl InterBankTransferStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "APPROVED" => InterBankTransferStatus::Approved,
            "PROCESSED" => InterBankTransferStatus::Processed,
            "COMPLETED" => InterBankTransferStatus::Completed,
            "CANCELLED" => InterBankTransferStatus::Cancelled,
            "FAILED" => InterBankTransferStatus::Failed,
            _ => InterBankTransferStatus::Initiated,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            InterBankTransferStatus::Initiated => "INITIATED",
            InterBankTransferStatus::Approved => "APPROVED",
            InterBankTransferStatus::Processed => "PROCESSED",
            InterBankTransferStatus::Completed => "COMPLETED",
            InterBankTransferStatus::Cancelled => "CANCELLED",
            InterBankTransferStatus::Failed => "FAILED",
        }
    }
}

/// Inter-bank transfer aggregate. Net GL effect is zero: the process step
/// posts a contra journal `DR Bank-B / CR Bank-A` of the same amount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterBankTransfer {
    pub inter_bank_transfer_id: EntityId<InterBankTransfer>,
    pub tenant_id: TenantId,
    pub from_bank_account_id: Uuid,
    pub to_bank_account_id: Uuid,
    pub amount: Money,
    pub transfer_date: NaiveDate,
    pub status: InterBankTransferStatus,
    pub requires_approval: bool,
    pub bank_reference: Option<String>,
    pub journal_id: Option<Uuid>,
    pub initiated_by_id: Uuid,
    pub approved_by_id: Option<Uuid>,
    pub processed_by_id: Option<Uuid>,
    pub failure_reason: Option<String>,
    pub version: i32,
    pub audit: AuditInfo,
}
