//! PurchaseOrder aggregate root.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub purchase_order_id: EntityId<PurchaseOrder>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub po_number: String,
    pub vendor_id: Uuid,
    pub purchase_requisition_id: Option<Uuid>,
    pub order_date: NaiveDate,
    pub delivery_date: Option<NaiveDate>,
    pub payment_terms: Option<String>,
    pub status: String,
    pub total_amount: Money,
    pub tax_amount: Money,
    pub net_amount: Money,
    pub is_rcm_applicable: bool,
    pub tds_section: Option<String>,
    pub tds_rate: Option<rust_decimal::Decimal>,
    pub fund_id: Option<Uuid>,
    pub budget_head_id: Option<Uuid>,
    pub issued_by_id: Option<Uuid>,
    pub approved_by_id: Option<Uuid>,
    pub audit: AuditInfo,
}
