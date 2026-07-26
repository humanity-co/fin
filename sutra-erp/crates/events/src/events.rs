//! Domain event definitions per bounded context.
//!
//! Each module's events are organized into sub-modules with
//! a corresponding event enum. These are serialized into the
//! outbox and published to Redis Streams.

use chrono::{DateTime, Utc};
use serde::Serialize;

// ─── General Ledger Events ────────────────────────────────────────

/// General Ledger domain events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum GlEvent {
    JournalCreated {
        journal_id: String,
        journal_number: String,
        journal_type: String,
        status: String,
        created_by: String,
        occurred_at: DateTime<Utc>,
    },
    JournalPosted {
        journal_id: String,
        journal_number: String,
        total_debit: i64,
        total_credit: i64,
        period_id: String,
        posted_by: String,
        occurred_at: DateTime<Utc>,
    },
    JournalReversed {
        original_journal_id: String,
        reversing_journal_id: String,
        reason: String,
        reversed_by: String,
        occurred_at: DateTime<Utc>,
    },
    JournalCancelled {
        journal_id: String,
        reason: String,
        cancelled_by: String,
        occurred_at: DateTime<Utc>,
    },
    AccountCreated {
        account_id: String,
        account_code: String,
        account_name: String,
        account_type: String,
        occurred_at: DateTime<Utc>,
    },
}

// ─── Accounts Receivable Events ──────────────────────────────────

/// Accounts Receivable domain events (fee collection, scholarships, refunds).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ArEvent {
    PaymentReceiptCreated {
        receipt_id: String,
        receipt_number: String,
        student_id: String,
        amount: i64,
        payment_mode: String,
        occurred_at: DateTime<Utc>,
    },
    PaymentAllocated {
        receipt_id: String,
        occurred_at: DateTime<Utc>,
    },
    FeeStructureActivated {
        fee_structure_id: String,
        effective_from: String,
        occurred_at: DateTime<Utc>,
    },
    ScholarshipDisbursed {
        scholarship_id: String,
        dbt_amount: i64,
        dbt_date: DateTime<Utc>,
        transaction_ref: String,
        occurred_at: DateTime<Utc>,
    },
    RefundInitiated {
        refund_id: String,
        refund_number: String,
        amount: i64,
        refund_type: String,
        occurred_at: DateTime<Utc>,
    },
}

// ─── Accounts Payable Events ─────────────────────────────────────

/// Accounts Payable domain events (vendors, procurement, payments).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ApEvent {
    VendorCreated {
        vendor_id: String,
        vendor_code: String,
        vendor_name: String,
        pan: String,
        occurred_at: DateTime<Utc>,
    },
    PurchaseOrderIssued {
        po_id: String,
        po_number: String,
        vendor_id: String,
        total_amount: i64,
        is_rcm_applicable: bool,
        occurred_at: DateTime<Utc>,
    },
    VendorPaymentProcessed {
        payment_id: String,
        payment_number: String,
        tds_amount: i64,
        net_amount: i64,
        occurred_at: DateTime<Utc>,
    },
    InvoiceMatched {
        invoice_id: String,
        invoice_number: String,
        po_id: String,
        occurred_at: DateTime<Utc>,
    },
}

// ─── Treasury Events ─────────────────────────────────────────────

/// Treasury & Banking domain events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum TreasuryEvent {
    BankReconciliationCompleted {
        reconciliation_id: String,
        bank_account_id: String,
        period_id: String,
        completed_by: String,
        occurred_at: DateTime<Utc>,
    },
    FundAmountReceived {
        fund_id: String,
        amount: i64,
        received_date: String,
        reference: String,
        occurred_at: DateTime<Utc>,
    },
}

// ─── Taxation Events ─────────────────────────────────────────────

/// Taxation domain events (GST, TDS).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum TaxationEvent {
    GstReturnGenerated {
        gst_return_id: String,
        gstin: String,
        return_type: String,
        period: String,
        tax_liability: i64,
        occurred_at: DateTime<Utc>,
    },
    TdsDeducted {
        tds_deduction_id: String,
        payment_id: String,
        tds_section: String,
        tds_amount: i64,
        occurred_at: DateTime<Utc>,
    },
    TdsReturnGenerated {
        tds_return_id: String,
        return_type: String,
        quarter: String,
        fiscal_year: String,
        total_deductions: i64,
        occurred_at: DateTime<Utc>,
    },
}

// ─── Budgeting Events ────────────────────────────────────────────

/// Budget & Planning domain events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum BudgetEvent {
    BudgetApproved {
        budget_id: String,
        budget_name: String,
        fiscal_year_id: String,
        total_amount: i64,
        approved_by: String,
        occurred_at: DateTime<Utc>,
    },
    EncumbranceCreated {
        encumbrance_id: String,
        budget_line_id: String,
        reference_type: String,
        reference_id: String,
        amount: i64,
        occurred_at: DateTime<Utc>,
    },
    BudgetExceeded {
        budget_line_id: String,
        budgeted_amount: i64,
        actual_amount: i64,
        occurred_at: DateTime<Utc>,
    },
}

// ─── Fixed Assets Events ─────────────────────────────────────────

/// Fixed Assets & Inventory domain events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum AssetEvent {
    AssetCapitalized {
        fixed_asset_id: String,
        asset_code: String,
        asset_name: String,
        purchase_cost: i64,
        department_id: String,
        occurred_at: DateTime<Utc>,
    },
    DepreciationPosted {
        fixed_asset_id: String,
        period_number: i32,
        depreciation_amount: i64,
        occurred_at: DateTime<Utc>,
    },
    AssetDisposed {
        fixed_asset_id: String,
        disposal_type: String,
        disposal_date: String,
        occurred_at: DateTime<Utc>,
    },
}

// ─── Compliance Events ───────────────────────────────────────────

/// Compliance & Reporting domain events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ComplianceEvent {
    ComplianceDeadlineApproaching {
        event_id: String,
        event_type: String,
        due_date: String,
        days_remaining: i32,
        occurred_at: DateTime<Utc>,
    },
    StatutoryReportFiled {
        report_id: String,
        report_type: String,
        period: String,
        filed_by: String,
        acknowledgment_no: String,
        occurred_at: DateTime<Utc>,
    },
}

// ─── Workflow Events ─────────────────────────────────────────────

/// Workflow & Approval domain events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WorkflowEvent {
    ApprovalRequested {
        approval_request_id: String,
        transaction_type: String,
        transaction_id: String,
        amount: i64,
        requested_by: String,
        occurred_at: DateTime<Utc>,
    },
    ApprovalGranted {
        approval_request_id: String,
        level: i32,
        approver_id: String,
        decision: String,
        occurred_at: DateTime<Utc>,
    },
}
