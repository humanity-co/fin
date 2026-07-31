//! PurchaseOrder aggregate root with line items.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

/// Purchase Order aggregate root.
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
    pub status: PoStatus,
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
    pub lines: Vec<PurchaseOrderLine>,
    pub audit: AuditInfo,
}

/// A line item within a purchase order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderLine {
    pub po_line_id: Uuid,
    pub purchase_order_id: EntityId<PurchaseOrder>,
    pub line_number: i32,
    pub item_description: String,
    pub hsn_sac_code: Option<String>,
    pub quantity: rust_decimal::Decimal,
    pub unit_price: Money,
    pub discount_percent: Option<rust_decimal::Decimal>,
    pub tax_rate: Option<rust_decimal::Decimal>,
    pub tax_type: Option<TaxType>,
    pub total_amount: Money,
    pub received_quantity: rust_decimal::Decimal,
    pub account_id: Uuid,
    pub cost_center_id: Option<Uuid>,
    pub rcm_applicable: bool,
}

/// Tax type classification for PO/Invoice lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaxType {
    #[serde(rename = "GST_EXEMPT")]
    GstExempt,
    #[serde(rename = "GST_5")]
    Gst5,
    #[serde(rename = "GST_12")]
    Gst12,
    #[serde(rename = "GST_18")]
    Gst18,
    #[serde(rename = "GST_28")]
    Gst28,
    Nil,
}

impl TaxType {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            TaxType::GstExempt => "GST_EXEMPT",
            TaxType::Gst5 => "GST_5",
            TaxType::Gst12 => "GST_12",
            TaxType::Gst18 => "GST_18",
            TaxType::Gst28 => "GST_28",
            TaxType::Nil => "NIL",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "GST_5" => TaxType::Gst5,
            "GST_12" => TaxType::Gst12,
            "GST_18" => TaxType::Gst18,
            "GST_28" => TaxType::Gst28,
            "NIL" => TaxType::Nil,
            _ => TaxType::GstExempt,
        }
    }
}

/// PO lifecycle states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoStatus {
    Draft,
    Issued,
    Acknowledged,
    PartiallyReceived,
    FullyReceived,
    Closed,
    Cancelled,
}

impl PoStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            PoStatus::Draft => "DRAFT",
            PoStatus::Issued => "ISSUED",
            PoStatus::Acknowledged => "ACKNOWLEDGED",
            PoStatus::PartiallyReceived => "PARTIALLY_RECEIVED",
            PoStatus::FullyReceived => "FULLY_RECEIVED",
            PoStatus::Closed => "CLOSED",
            PoStatus::Cancelled => "CANCELLED",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "ISSUED" => PoStatus::Issued,
            "ACKNOWLEDGED" => PoStatus::Acknowledged,
            "PARTIALLY_RECEIVED" => PoStatus::PartiallyReceived,
            "FULLY_RECEIVED" => PoStatus::FullyReceived,
            "CLOSED" => PoStatus::Closed,
            "CANCELLED" => PoStatus::Cancelled,
            _ => PoStatus::Draft,
        }
    }
}
