//! Treasury models — BankAccount aggregate root.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    pub bank_account_id: EntityId<BankAccount>,
    pub tenant_id: TenantId,
    pub entity_id: Option<Uuid>,
    pub account_number: String,
    pub account_name: String,
    pub bank_name: String,
    pub branch_name: Option<String>,
    pub ifsc_code: String,
    pub account_type: String,
    pub fund_id: Option<Uuid>,
    pub is_fcra_account: bool,
    pub minimum_balance: Money,
    pub is_active: bool,
    pub last_reconciled_at: Option<DateTime<Utc>>,
    pub audit: AuditInfo,
}
