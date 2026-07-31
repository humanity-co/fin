//! AP query types and read-side projections.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Query filter for listing vendors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorFilter {
    pub search: Option<String>,
    pub vendor_type: Option<String>,
    pub is_active: Option<bool>,
    pub is_blacklisted: Option<bool>,
    pub has_gstin: Option<bool>,
    pub pan: Option<String>,
}

/// Query filter for listing purchase orders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoFilter {
    pub vendor_id: Option<Uuid>,
    pub status: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub po_number: Option<String>,
}

/// Query filter for listing invoices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceFilter {
    pub vendor_id: Option<Uuid>,
    pub purchase_order_id: Option<Uuid>,
    pub status: Option<String>,
    pub payment_status: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

/// Query filter for TDS deductions register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdsDeductionFilter {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub section: Option<String>,
    pub vendor_id: Option<Uuid>,
    pub deposit_status: Option<String>,
}
