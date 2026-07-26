//! Refund — aggregate root for student fee refunds.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refund {
    pub refund_id: EntityId<Refund>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub refund_number: String,
    pub student_id: Option<Uuid>,
    pub source_receipt_id: Option<Uuid>,
    pub refund_type: String,
    pub refund_mode: String,
    pub amount: Money,
    pub reason: String,
    pub frc_refund_percent: Option<rust_decimal::Decimal>,
    pub status: String,
    pub approved_by_id: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub bank_transaction_ref: Option<String>,
    pub refund_journal_id: Option<Uuid>,
    pub audit: AuditInfo,
}
