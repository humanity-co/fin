//! General Ledger error types.

use sutra_core::Money;
use thiserror::Error;

/// Errors specific to the General Ledger module.
#[derive(Debug, Error)]
pub enum GlError {
    #[error("journal not balanced: debits={0}, credits={1}")]
    UnbalancedJournal(Money, Money),

    #[error("journal contains negative amounts")]
    NegativeAmount,

    #[error("accounting period {0} is closed")]
    PeriodClosed(String),

    #[error("account {0} is not active")]
    AccountInactive(String),

    #[error("account {0} is not a leaf node — only leaf accounts can receive journal entries")]
    AccountNotLeaf(String),

    #[error("journal {0} is not in draft status")]
    JournalNotDraft(String),

    #[error("journal {0} is already posted")]
    JournalAlreadyPosted(String),

    #[error("journal {0} not found")]
    JournalNotFound(String),

    #[error("account {0} not found")]
    AccountNotFound(String),

    #[error("accounting period {0} not found")]
    PeriodNotFound(String),

    #[error("journal has fewer than 2 lines")]
    TooFewLines,

    #[error("journal has more than 500 lines")]
    TooManyLines,

    #[error("database error: {0}")]
    Database(String),

    #[error("event publishing error: {0}")]
    EventPublish(String),
}

impl From<sqlx::Error> for GlError {
    fn from(e: sqlx::Error) -> Self {
        GlError::Database(e.to_string())
    }
}
