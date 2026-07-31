//! GoodsReceiptNote aggregate root.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

/// Goods Receipt Note — records receipt of goods/services against a purchase order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodsReceiptNote {
    pub goods_receipt_note_id: EntityId<GoodsReceiptNote>,
    pub tenant_id: TenantId,
    pub grn_number: String,
    pub purchase_order_id: Uuid,
    pub received_date: NaiveDate,
    pub received_by_id: Option<Uuid>,
    pub status: GrnStatus,
    pub remarks: Option<String>,
    pub lines: Vec<GoodsReceiptNoteLine>,
    pub audit: AuditInfo,
}

/// A line item within a GRN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodsReceiptNoteLine {
    pub grn_line_id: Uuid,
    pub goods_receipt_note_id: EntityId<GoodsReceiptNote>,
    pub po_line_id: Uuid,
    pub received_quantity: rust_decimal::Decimal,
    pub accepted_quantity: rust_decimal::Decimal,
    pub rejected_quantity: rust_decimal::Decimal,
    pub rejection_reason: Option<String>,
}

/// GRN lifecycle states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrnStatus {
    Draft,
    Completed,
    Cancelled,
}

impl GrnStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            GrnStatus::Draft => "DRAFT",
            GrnStatus::Completed => "COMPLETED",
            GrnStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "COMPLETED" => GrnStatus::Completed,
            "CANCELLED" => GrnStatus::Cancelled,
            _ => GrnStatus::Draft,
        }
    }
}
