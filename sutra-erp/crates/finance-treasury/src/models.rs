//! Treasury domain models.
pub mod bank_account;
pub mod gateway;
pub mod petty_cash;
pub mod reconciliation;
pub mod transfer;

pub use bank_account::{BankAccount, BankAccountType, BankSignatory, SignatoryType};
pub use gateway::{
    GatewaySettlement, GatewaySettlementStatus, GatewayType, PaymentGatewayConfig,
};
pub use petty_cash::{
    PettyCashFund, PettyCashFundStatus, PettyCashTransaction, PettyCashTransactionType,
};
pub use reconciliation::{
    BankReconciliation, BankReconciliationStatus, BankStatementLine, BankTransaction,
    MatchStatus,
};
pub use transfer::{InterBankTransfer, InterBankTransferStatus};

/// A treasury policy value read from `system_config` (with code defaults).
///
/// Every business threshold in the module comes from this table — nothing is
/// hardcoded. `tenant_id = NULL` rows are GLOBAL defaults; a tenant row
/// overrides them.
#[derive(Debug, Clone, Default)]
pub struct TreasuryPolicy {
    /// Transfers at/above this amount (paise) require approval. Default ₹10,00,000.
    pub transfer_approval_threshold: i64,
    /// Maximum petty-cash imprest per fund (paise). Default ₹25,000.
    pub petty_cash_imprest_limit: i64,
    /// Cheques at/above this amount (paise) require JOINT signatories. Default ₹1,00,000.
    pub cheque_joint_signature_threshold: i64,
    /// IFSC of the FCRA-mandated bank branch (SBI New Delhi Main Branch).
    pub fcra_bank_ifsc: String,
    pub fcra_bank_name: String,
    pub fcra_branch_name: String,
    /// Block transfers out of endowment-linked accounts unless income-only.
    pub endowment_transfer_blocked: bool,
    /// Auto-match: exact UTR equality.
    pub auto_match_utr_exact: bool,
    /// Auto-match: amount equality + date within ±N days.
    pub auto_match_amount_date_window_days: i64,
    /// Auto-match: amount equality + description similarity.
    pub auto_match_description_similarity: bool,
    /// Candidate COA codes (priority order) under which bank GL leaves are created.
    pub bank_gl_parent_codes: Vec<String>,
    /// Default cash-in-hand GL code for petty-cash journals.
    pub cash_gl_code: String,
}

impl TreasuryPolicy {
    /// Code-level defaults mandated by the spec. Never used when a
    /// `system_config` value exists.
    pub fn defaults() -> Self {
        TreasuryPolicy {
            transfer_approval_threshold: 100_000_000,     // ₹10,00,000
            petty_cash_imprest_limit: 2_500_000,          // ₹25,000
            cheque_joint_signature_threshold: 10_000_000, // ₹1,00,000
            fcra_bank_ifsc: "SBIN0000691".to_string(),
            fcra_bank_name: "State Bank of India".to_string(),
            fcra_branch_name: "New Delhi Main Branch".to_string(),
            endowment_transfer_blocked: true,
            auto_match_utr_exact: true,
            auto_match_amount_date_window_days: 3,
            auto_match_description_similarity: true,
            bank_gl_parent_codes: vec![
                "10.02".to_string(),
                "10.02.01".to_string(),
                "1110".to_string(),
                "1100".to_string(),
            ],
            cash_gl_code: "10.01.01.01".to_string(),
        }
    }
}
