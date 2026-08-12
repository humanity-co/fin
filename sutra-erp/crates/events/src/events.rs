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
    // ── Treasury & Banking (spec §Treasury) ──────────────────────────
    BankAccountCreated {
        bank_account_id: String,
        account_name: String,
        bank_name: String,
        account_type: String,
        entity_id: String,
        gl_account_id: String,
        occurred_at: DateTime<Utc>,
    },
    BankAccountUpdated {
        bank_account_id: String,
        occurred_at: DateTime<Utc>,
    },
    BankAccountDeactivated {
        bank_account_id: String,
        occurred_at: DateTime<Utc>,
    },
    SignatoryAdded {
        bank_account_id: String,
        user_id: String,
        occurred_at: DateTime<Utc>,
    },
    SignatoryRemoved {
        bank_account_id: String,
        user_id: String,
        occurred_at: DateTime<Utc>,
    },
    BankBalanceSynced {
        bank_account_id: String,
        balance: i64,
        synced_at: DateTime<Utc>,
        occurred_at: DateTime<Utc>,
    },
    MinimumBalanceAlert {
        bank_account_id: String,
        current_balance: i64,
        minimum_balance: i64,
        occurred_at: DateTime<Utc>,
    },
    ReconciliationStarted {
        reconciliation_id: String,
        bank_account_id: String,
        period_id: String,
        occurred_at: DateTime<Utc>,
    },
    BankStatementUploaded {
        reconciliation_id: String,
        line_count: i64,
        occurred_at: DateTime<Utc>,
    },
    AutoReconciliationCompleted {
        reconciliation_id: String,
        matched_count: i64,
        unmatched_count: i64,
        occurred_at: DateTime<Utc>,
    },
    LineMatched {
        reconciliation_id: String,
        statement_line_id: String,
        transaction_id: String,
        transaction_type: String,
        occurred_at: DateTime<Utc>,
    },
    LineUnmatched {
        reconciliation_id: String,
        statement_line_id: String,
        occurred_at: DateTime<Utc>,
    },
    ReconciliationVerified {
        reconciliation_id: String,
        verified_by: String,
        occurred_at: DateTime<Utc>,
    },
    BrsGenerated {
        reconciliation_id: String,
        document_url: String,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferInitiated {
        transfer_id: String,
        from_account: String,
        to_account: String,
        amount: i64,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferApproved {
        transfer_id: String,
        approved_by: String,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferProcessed {
        transfer_id: String,
        journal_id: String,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferCompleted {
        transfer_id: String,
        from_account: String,
        to_account: String,
        amount: i64,
        bank_reference: String,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferCancelled {
        transfer_id: String,
        reason: String,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferFailed {
        transfer_id: String,
        reason: String,
        occurred_at: DateTime<Utc>,
    },
    PettyCashFundCreated {
        fund_id: String,
        entity_id: String,
        imprest_amount: i64,
        occurred_at: DateTime<Utc>,
    },
    PettyCashTopUp {
        fund_id: String,
        amount: i64,
        occurred_at: DateTime<Utc>,
    },
    PettyCashExpenseRecorded {
        fund_id: String,
        voucher_no: String,
        amount: i64,
        account_id: String,
        occurred_at: DateTime<Utc>,
    },
    GatewayConfigured {
        entity_id: String,
        gateway_type: String,
        occurred_at: DateTime<Utc>,
    },
    GatewaySettlementReconciled {
        entity_id: String,
        gateway_type: String,
        settlement_date: String,
        settled_amount: i64,
        fee_amount: i64,
        occurred_at: DateTime<Utc>,
    },
}

// ─── Taxation Events ─────────────────────────────────────────────

/// Taxation domain events (GST, ITC, RCM, TDS, income-tax compliance).
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
    // ── GST (spec §Tax Engine events) ────────────────────────────────
    GstinRegistered {
        reg_id: String,
        entity_id: String,
        gstin: String,
        occurred_at: DateTime<Utc>,
    },
    GstRateUpdated {
        hsn_sac_code: String,
        rate: i64,
        effective_from: String,
        occurred_at: DateTime<Utc>,
    },
    Gstr1Generated {
        return_id: String,
        period: String,
        tax_liability: i64,
        occurred_at: DateTime<Utc>,
    },
    Gstr3bGenerated {
        return_id: String,
        period: String,
        tax_liability: i64,
        itc_claimed: i64,
        occurred_at: DateTime<Utc>,
    },
    GstReturnFiled {
        return_id: String,
        period: String,
        acknowledgment_no: String,
        occurred_at: DateTime<Utc>,
    },
    RcmEntryCreated {
        invoice_id: String,
        rcm_payable_amount: i64,
        journal_id: String,
        occurred_at: DateTime<Utc>,
    },
    // ── ITC register ────────────────────────────────────────────────
    ItcComputed {
        reg_id: String,
        period: String,
        total_itc: i64,
        net_itc_eligible: i64,
        occurred_at: DateTime<Utc>,
    },
    Rule42ReversalComputed {
        reg_id: String,
        period: String,
        reversal_amount: i64,
        exempt_turnover: i64,
        total_turnover: i64,
        occurred_at: DateTime<Utc>,
    },
    Rule43ReversalComputed {
        reg_id: String,
        period: String,
        reversal_amount: i64,
        capital_goods_itc: i64,
        occurred_at: DateTime<Utc>,
    },
    ItcReversalPosted {
        register_line_id: String,
        reversal_amount: i64,
        journal_id: String,
        occurred_at: DateTime<Utc>,
    },
    // ── TDS ─────────────────────────────────────────────────────────
    TdsSectionUpdated {
        section_code: String,
        rate: String,
        threshold: Option<i64>,
        occurred_at: DateTime<Utc>,
    },
    TdsDeposited {
        tds_deduction_id: String,
        challan_reference: String,
        deposit_date: String,
        amount: i64,
        occurred_at: DateTime<Utc>,
    },
    TdsReturnFiled {
        return_id: String,
        acknowledgment_no: String,
        occurred_at: DateTime<Utc>,
    },
    Form16Generated {
        employee_id: String,
        fiscal_year: String,
        document_url: String,
        occurred_at: DateTime<Utc>,
    },
    Form16AGenerated {
        vendor_id: String,
        fiscal_year: String,
        document_url: String,
        occurred_at: DateTime<Utc>,
    },
    // ── Income-tax compliance (trust/society) ───────────────────────
    TrustExemptionRegistered {
        reg_id: String,
        section: String,
        registration_no: String,
        valid_to: String,
        occurred_at: DateTime<Utc>,
    },
    ExemptionRenewed {
        reg_id: String,
        section: String,
        valid_to: String,
        occurred_at: DateTime<Utc>,
    },
    ExemptionExpiring {
        reg_id: String,
        section: String,
        days_remaining: i64,
        occurred_at: DateTime<Utc>,
    },
    IncomeApplicationComputed {
        fiscal_year_id: String,
        total_income: i64,
        applied_percent: String,
        is_compliant: bool,
        occurred_at: DateTime<Utc>,
    },
    IncomeApplicationThresholdMissed {
        fiscal_year_id: String,
        applied_percent: String,
        threshold: i64,
        occurred_at: DateTime<Utc>,
    },
    Section115BreachDetected {
        fiscal_year_id: String,
        details: String,
        occurred_at: DateTime<Utc>,
    },
    FcraRegistered {
        reg_id: String,
        registration_no: String,
        valid_to: String,
        occurred_at: DateTime<Utc>,
    },
    FcraAdminExpenseExceeded {
        fiscal_year_id: String,
        expense_ratio: String,
        max_allowed: i64,
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
