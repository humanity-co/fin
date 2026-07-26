//! SutraERP — General Ledger Module
//!
//! Core double-entry accounting engine: Chart of Accounts,
//! journal entries, accounting periods, cost centers, funds,
//! and multi-entity support.
//!
//! ## Architecture
//!
//! - **Commands** — CQRS write side, validated mutations with outbox events
//! - **Queries** — CQRS read side, direct DB queries returning projections
//! - **Repository** — Data access traits + SQLx PostgreSQL implementations
//! - **Events** — Domain event definitions for outbox publishing
//! - **Errors** — Module-specific error types
//! - **Models** — Domain entities and value objects

pub mod commands;
pub mod errors;
pub mod events;
pub mod models;
pub mod queries;
pub mod repository;

// Re-export key model types
pub use models::account::{Account, AccountType, GstClassification, ItcEligibility};
pub use models::accounting_period::{AccountingPeriod, PeriodStatus};
pub use models::cost_center::{BudgetPeriod, CostCenter, CostCenterType};
pub use models::entity::{Entity, EntityType};
pub use models::fund::{Fund, FundSource, FundStatus, FundType};
pub use models::journal::{Journal, JournalLine, JournalStatus, JournalType};

// Re-export command types
pub use commands::{
    CreateAccountCmd, CreateJournalCmd, CreateJournalLineCmd, GlCommandHandler,
    PostJournalCmd, ReverseJournalCmd,
};

// Re-export query types
pub use queries::{
    CoaTreeNode, GlQueryHandler, JournalFilter, JournalListResponse, LedgerEntry,
    TrialBalanceQuery, TrialBalanceRow,
};

// Re-export error types
pub use errors::GlError;
