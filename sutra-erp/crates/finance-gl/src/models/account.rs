//! Account — Chart of Accounts aggregate.

use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};

/// Account aggregate root (Chart of Accounts entry).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub account_id: EntityId<Account>,
    pub tenant_id: TenantId,
    pub account_code: String,
    pub account_name: String,
    pub account_type: AccountType,
    pub parent_account_id: Option<EntityId<Account>>,
    pub level: i32,
    pub gst_classification: Option<GstClassification>,
    pub hsn_sac_code: Option<String>,
    pub itc_eligibility: Option<ItcEligibility>,
    pub aishe_head_code: Option<String>,
    pub naac_metric_key: Option<String>,
    pub is_active: bool,
    pub is_system: bool,
    pub opening_balance: Money,
    pub current_balance: Money,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GstClassification {
    Exempt,
    Nil,
    Taxable5,
    Taxable12,
    Taxable18,
    Taxable28,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ItcEligibility {
    Full,
    Blocked,
    Reversal4243,
    CapitalGoods,
}
