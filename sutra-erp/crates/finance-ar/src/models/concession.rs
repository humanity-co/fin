//! Concession — aggregate root for student fee concessions.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concession {
    pub concession_id: EntityId<Concession>,
    pub tenant_id: TenantId,
    pub student_id: Uuid,
    pub student_fee_account_id: Uuid,
    pub concession_type: String,
    pub concession_percent: rust_decimal::Decimal,
    pub concession_amount: Option<Money>,
    pub approved_by_id: Option<Uuid>,
    pub approval_date: Option<DateTime<Utc>>,
    pub sanction_order_number: Option<String>,
    pub valid_from: NaiveDate,
    pub valid_to: Option<NaiveDate>,
    pub status: String,
    pub remarks: Option<String>,
    pub audit: AuditInfo,
}
