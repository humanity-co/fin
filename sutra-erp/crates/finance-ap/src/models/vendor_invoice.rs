//! VendorInvoice aggregate root with line items and 3-way matching.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

/// Vendor Invoice aggregate root.
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
    pub matching_status: MatchingStatus,
    pub status: InvoiceStatus,
    pub payment_status: PaymentStatus,
    pub due_date: NaiveDate,
    pub posted_journal_id: Option<Uuid>,
    pub approved_by_id: Option<Uuid>,
    pub lines: Vec<InvoiceLine>,
    pub audit: AuditInfo,
}

/// A line item within a vendor invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub invoice_line_id: Uuid,
    pub vendor_invoice_id: EntityId<VendorInvoice>,
    pub po_line_id: Option<Uuid>,
    pub line_number: i32,
    pub item_description: String,
    pub quantity: rust_decimal::Decimal,
    pub unit_price: Money,
    pub tax_rate: Option<rust_decimal::Decimal>,
    pub tax_amount: Option<Money>,
    pub total_amount: Money,
    pub account_id: Uuid,
    pub cost_center_id: Option<Uuid>,
}

/// 3-way matching status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchingStatus {
    Pending,
    Matched,
    Mismatch,
    Overridden,
}

impl MatchingStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            MatchingStatus::Pending => "PENDING",
            MatchingStatus::Matched => "MATCHED",
            MatchingStatus::Mismatch => "MISMATCHED",
            MatchingStatus::Overridden => "OVERRIDDEN",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "MATCHED" => MatchingStatus::Matched,
            "MISMATCHED" => MatchingStatus::Mismatch,
            "OVERRIDDEN" => MatchingStatus::Overridden,
            _ => MatchingStatus::Pending,
        }
    }
}

/// Invoice lifecycle states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Draft,
    PendingApproval,
    Approved,
    Posted,
    Paid,
    Cancelled,
}

impl InvoiceStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "DRAFT",
            InvoiceStatus::PendingApproval => "MATCHED",   // after match, before approval
            InvoiceStatus::Approved => "APPROVED",
            InvoiceStatus::Posted => "POSTED",
            InvoiceStatus::Paid => "PAID",
            InvoiceStatus::Cancelled => "CANCELLED",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "APPROVED" => InvoiceStatus::Approved,
            "POSTED" => InvoiceStatus::Posted,
            "PAID" => InvoiceStatus::Paid,
            "CANCELLED" => InvoiceStatus::Cancelled,
            "MATCHED" => InvoiceStatus::PendingApproval,
            "MISMATCHED" => InvoiceStatus::Draft,
            _ => InvoiceStatus::Draft,
        }
    }
}

/// Payment status of an invoice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Unpaid,
    PartiallyPaid,
    Paid,
}

impl PaymentStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            PaymentStatus::Unpaid => "UNPAID",
            PaymentStatus::PartiallyPaid => "PARTIALLY_PAID",
            PaymentStatus::Paid => "PAID",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "PARTIALLY_PAID" => PaymentStatus::PartiallyPaid,
            "PAID" => PaymentStatus::Paid,
            _ => PaymentStatus::Unpaid,
        }
    }
}
