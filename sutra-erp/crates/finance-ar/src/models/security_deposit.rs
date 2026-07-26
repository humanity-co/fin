//! SecurityDeposit — aggregate root for student security deposits.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityDeposit {
    pub security_deposit_id: EntityId<SecurityDeposit>,
    pub tenant_id: TenantId,
    pub student_id: Uuid,
    pub deposit_type: String,
    pub amount: Money,
    pub collection_date: NaiveDate,
    pub receipt_id: Option<Uuid>,
    pub interest_rate: Option<rust_decimal::Decimal>,
    pub status: String,
    pub refund_date: Option<NaiveDate>,
    pub refund_amount: Option<Money>,
    pub deduction_amount: Option<Money>,
    pub deduction_reason: Option<String>,
    pub audit: AuditInfo,
}
