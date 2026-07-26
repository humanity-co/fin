//! VendorInvoice aggregate root.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorInvoice {
    pub vendor_invoice_id: EntityId<VendorInvoice>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub invoice_number: String,
    pub invoice_date: NaiveDate,
    pub purchase_order_id: Option<Uuid>,
    pub goods_receipt_note_id: Option<Uuid>,
    pub vendor_id: Uuid,
    pub invoice_amount: Money,
    pub tax_amount: Money,
    pub net_amount: Money,
    pub tds_amount: Money,
    pub is_rcm: bool,
    pub rcm_payable_amount: Option<Money>,
    pub status: String,
    pub payment_status: String,
    pub due_date: NaiveDate,
    pub posted_journal_id: Option<Uuid>,
    pub approved_by_id: Option<Uuid>,
    pub audit: AuditInfo,
}
