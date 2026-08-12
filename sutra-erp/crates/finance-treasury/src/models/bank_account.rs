//! Treasury models — BankAccount aggregate root.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

/// Bank account types recognised by the register.
///
/// `GRANT_SPECIFIC` accounts hold grant money separately (UGC Grant-in-aid
/// rules); `FCRA` accounts must be at SBI New Delhi Main Branch (FCRA 2010
/// s.17) and are flagged with `is_fcra_account = true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BankAccountType {
    Current,
    Savings,
    Fcra,
    GrantSpecific,
    Deposit,
    CashCredit,
}

impl BankAccountType {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "SAVINGS" => BankAccountType::Savings,
            "FCRA" => BankAccountType::Fcra,
            "GRANT_SPECIFIC" => BankAccountType::GrantSpecific,
            "DEPOSIT" => BankAccountType::Deposit,
            "CASH_CREDIT" => BankAccountType::CashCredit,
            _ => BankAccountType::Current,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            BankAccountType::Current => "CURRENT",
            BankAccountType::Savings => "SAVINGS",
            BankAccountType::Fcra => "FCRA",
            BankAccountType::GrantSpecific => "GRANT_SPECIFIC",
            BankAccountType::Deposit => "DEPOSIT",
            BankAccountType::CashCredit => "CASH_CREDIT",
        }
    }
}

/// Bank account register entry — the money plumbing of the institution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    pub bank_account_id: EntityId<BankAccount>,
    pub tenant_id: TenantId,
    /// Campus/entity the account belongs to; `None` = shared/central.
    pub entity_id: Option<Uuid>,
    /// Encrypted at rest at the application layer.
    pub account_number: String,
    pub account_name: String,
    pub bank_name: String,
    pub branch_name: Option<String>,
    pub ifsc_code: String,
    pub account_type: BankAccountType,
    /// Required for GRANT_SPECIFIC accounts (UGC separate-account rule).
    pub fund_id: Option<Uuid>,
    pub is_fcra_account: bool,
    pub minimum_balance: Money,
    pub is_active: bool,
    /// Dedicated GL leaf account under 10.02, auto-created on creation.
    pub gl_account_id: Option<Uuid>,
    pub last_reconciled_at: Option<DateTime<Utc>>,
    pub audit: AuditInfo,
}

/// A signatory on a bank account. `JOINT` signatories are required for
/// cheques at/above the `cheque_joint_signature_threshold` policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankSignatory {
    pub bank_signatory_id: EntityId<BankSignatory>,
    pub tenant_id: TenantId,
    pub bank_account_id: Uuid,
    pub user_id: Uuid,
    pub signatory_type: SignatoryType,
    pub is_active: bool,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignatoryType {
    Individual,
    Joint,
}

impl SignatoryType {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "JOINT" => SignatoryType::Joint,
            _ => SignatoryType::Individual,
        }
    }
}
