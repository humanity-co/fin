//! Treasury & Banking module errors.
use thiserror::Error;

/// Errors raised by the treasury module.
///
/// Every variant maps to a stable, machine-readable code so the API layer
/// can return consistent error envelopes without string parsing.
#[derive(Debug, Error)]
pub enum TreasuryError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("journal engine error: {0}")]
    Gl(String),

    #[error("bank account {0} not found")]
    BankAccountNotFound(String),

    #[error("bank account number already exists for this tenant")]
    DuplicateBankAccount,

    #[error("invalid IFSC code: {0}")]
    InvalidIfsc(String),

    #[error("invalid bank account type: {0}")]
    InvalidAccountType(String),

    #[error("FCRA accounts must be held at SBI New Delhi Main Branch (FCRA 2010 s.17) — expected IFSC {expected_ifsc}, got {actual_ifsc}")]
    FcraBankMismatch { expected_ifsc: String, actual_ifsc: String },

    #[error("GRANT_SPECIFIC bank accounts require a fund_id (UGC Grant-in-aid rules)")]
    GrantFundRequired,

    #[error("chart of accounts parent for bank leaves (10.02) not found — seed the default COA first")]
    BankGlParentMissing,

    #[error("GL account {0} not found in chart of accounts")]
    GlAccountNotFound(String),

    #[error("signatory already exists for this bank account")]
    DuplicateSignatory,

    #[error("no active signatory for bank account {0} — at least one active signatory is required before transfers")]
    NoActiveSignatory(String),

    #[error("bank account {0} is not active")]
    BankAccountInactive(String),

    #[error("bank account {0} has a non-zero unreconciled balance — cannot deactivate")]
    BankAccountHasBalance(String),

    #[error("an open reconciliation already exists for bank account {0} and period {1}")]
    OpenReconciliationExists(String, String),

    #[error("bank reconciliation {0} not found")]
    ReconciliationNotFound(String),

    #[error("bank reconciliation is not in the required state (current: {0}, expected: {1})")]
    InvalidReconciliationState(String, String),

    #[error("statement opening/closing balance does not match statement totals (opening {opening}, closing {closing}, computed closing {computed})")]
    StatementBalanceMismatch { opening: i64, closing: i64, computed: i64 },

    #[error("unsupported statement format: {0} (supported: CSV, OFX)")]
    UnsupportedStatementFormat(String),

    #[error("statement parse error: {0}")]
    StatementParse(String),

    #[error("statement line {0} not found")]
    StatementLineNotFound(String),

    #[error("statement line {0} is already matched")]
    LineAlreadyMatched(String),

    #[error("statement line {0} is not matched — cannot unmatch")]
    LineNotMatched(String),

    #[error("manual match requires amount equality (statement {statement_amount} vs transaction {transaction_amount})")]
    AmountMismatch { statement_amount: i64, transaction_amount: i64 },

    #[error("transaction {0} not found for matching")]
    MatchTransactionNotFound(String),

    #[error("transfer {0} not found")]
    TransferNotFound(String),

    #[error("invalid inter-bank transfer: {0}")]
    InvalidTransfer(String),

    #[error("transfer {0} is not in the required state (current: {1}, expected: {2})")]
    InvalidTransferState(String, String, String),

    #[error("creator cannot approve their own transfer (segregation of duties)")]
    CreatorApproverConflict,

    #[error("insufficient available balance for transfer (available {available}, required {required})")]
    InsufficientBalance { available: i64, required: i64 },

    #[error("endowment-linked account transfers are blocked unless the fund is income-only (Maharashtra Self-Financed Universities Act 2013)")]
    EndowmentTransferBlocked,

    #[error("no open accounting period available for posting")]
    NoOpenPeriod,

    #[error("petty cash fund {0} not found")]
    PettyCashFundNotFound(String),

    #[error("petty cash fund {0} is not ACTIVE")]
    PettyCashFundInactive(String),

    #[error("an ACTIVE petty cash fund already exists for entity {0}")]
    ActivePettyCashFundExists(String),

    #[error("imprest amount {0} exceeds the configured petty-cash imprest limit {1}")]
    ImprestLimitExceeded(i64, i64),

    #[error("petty cash expense {0} exceeds the available fund balance {1}")]
    InsufficientPettyCashBalance(i64, i64),

    #[error("petty cash expense requires an expense account and a voucher number")]
    ExpenseVoucherRequired,

    #[error("payment gateway config {0} not found")]
    GatewayConfigNotFound(String),

    #[error("payment gateway config already exists for entity {0} and gateway {1}")]
    DuplicateGatewayConfig(String, String),

    #[error("gateway settlement not found for entity {0}, gateway {1}, date {2}")]
    GatewaySettlementNotFound(String, String, String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("event publish error: {0}")]
    EventPublish(String),

    #[error("internal error: {0}")]
    Internal(String),
}
