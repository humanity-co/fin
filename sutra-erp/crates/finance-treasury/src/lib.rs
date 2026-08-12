//! SutraERP — Treasury & Banking Module
//!
//! Owns the institution's money plumbing: the bank account register
//! (multi-campus, per-entity, fund-linked), bank reconciliation
//! (statement import → auto/manual match → BRS), payment-gateway
//! configuration and settlement reconciliation, inter-bank transfers,
//! petty cash, and the Tally-style Bank Book / cash-position views.
//!
//! ## Architecture
//!
//! - **Commands** — CQRS write side (`TreasuryCommandHandler`)
//! - **Queries** — CQRS read side (`TreasuryQueryHandler`)
//! - **Repository** — data access (SQLx/PostgreSQL, tenant-scoped)
//! - **Events** — outbox event payloads (`TreasuryEventData`)
//! - **Models** — aggregates & value objects
//! - **Errors** — module error types
//!
//! ## Non-negotiable rules
//!
//! - Money is i64 paise — never floats.
//! - Every row is tenant-scoped.
//! - Every GL posting goes through the GL module's create/post commands and
//!   carries `reference_type` / `reference_id` for traceability.
//! - Every mutation leaves an audit trail and writes an outbox event.
//! - Business thresholds come from `system_config` (see [`TreasuryPolicy`]),
//!   never from code.
pub mod commands;
pub mod errors;
pub mod events;
pub mod models;
pub mod queries;
pub mod repository;

pub use commands::{
    AddSignatoryCmd, CompleteReconciliationCmd, ConfigureGatewayCmd, CreateBankAccountCmd,
    InitiateInterBankTransferCmd, ManualMatchCmd, RecordPettyCashExpenseCmd,
    ReconcileGatewaySettlementCmd, StartReconciliationCmd, SyncBankBalanceCmd,
    TopUpPettyCashCmd, TreasuryCommandHandler, UpdateBankAccountCmd,
    UploadBankStatementCmd,
};
pub use errors::TreasuryError;
pub use models::{
    BankAccount, BankAccountType, BankReconciliation, BankReconciliationStatus,
    BankSignatory, BankStatementLine, BankTransaction, GatewaySettlement,
    GatewaySettlementStatus, GatewayType, InterBankTransfer,
    InterBankTransferStatus, MatchStatus, PaymentGatewayConfig, PettyCashFund,
    PettyCashFundStatus, PettyCashTransaction, PettyCashTransactionType,
    SignatoryType, TreasuryPolicy,
};
pub use queries::{CashPositionRow, ReconciliationSummaryRow, TreasuryQueryHandler};
pub use repository::{
    decrypt_secret, encrypt_secret, BankBookRow, MatchCandidate, TreasuryRepository,
};
