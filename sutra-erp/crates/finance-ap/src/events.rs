//! AP domain event data types.

use serde::Serialize;
use chrono::{DateTime, Utc};

/// AP-specific event payload for outbox serialization.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ApEventData {
    VendorCreated {
        vendor_id: String,
        vendor_code: String,
        vendor_name: String,
        pan: Option<String>,
        occurred_at: DateTime<Utc>,
    },
    PurchaseOrderCreated {
        po_id: String,
        po_number: String,
        vendor_id: String,
        total_amount: i64,
        occurred_at: DateTime<Utc>,
    },
    PurchaseOrderIssued {
        po_id: String,
        po_number: String,
        vendor_id: String,
        occurred_at: DateTime<Utc>,
    },
    GoodsReceiptNoteCompleted {
        grn_id: String,
        grn_number: String,
        po_id: String,
        po_status: String,
        occurred_at: DateTime<Utc>,
    },
    PurchaseInvoiceCreated {
        invoice_id: String,
        invoice_number: String,
        vendor_id: String,
        amount: i64,
        occurred_at: DateTime<Utc>,
    },
    InvoiceMatched {
        invoice_id: String,
        status: String,
        mismatches: Vec<String>,
        occurred_at: DateTime<Utc>,
    },
    InvoicePosted {
        invoice_id: String,
        journal_id: String,
        occurred_at: DateTime<Utc>,
    },
    PaymentInitiated {
        payment_id: String,
        payment_number: String,
        vendor_id: String,
        amount: i64,
        tds_amount: i64,
        occurred_at: DateTime<Utc>,
    },
    PaymentProcessed {
        payment_id: String,
        payment_journal_id: String,
        tds_journal_id: Option<String>,
        bank_reference: Option<String>,
        processed_by: String,
        occurred_at: DateTime<Utc>,
    },
    TdsDeducted {
        payment_id: String,
        section: String,
        tds_amount: i64,
        pan: Option<String>,
        occurred_at: DateTime<Utc>,
    },
}
