//! VendorPayment aggregate root.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorPayment {
    pub payment_id: EntityId<VendorPayment>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub payment_number: String,
    pub vendor_id: Uuid,
    pub payment_type: String,
    pub payment_mode: String,
    pub payment_date: NaiveDate,
    pub amount: Money,
    pub tds_amount: Money,
    pub net_amount: Money,
    pub status: String,
    pub bank_account_id: Option<Uuid>,
    pub bank_transaction_ref: Option<String>,
    pub cheque_number: Option<String>,
    pub cheque_date: Option<NaiveDate>,
    pub approved_by_id: Option<Uuid>,
    pub processed_by_id: Option<Uuid>,
    pub payment_journal_id: Option<Uuid>,
    pub audit: AuditInfo,
}
