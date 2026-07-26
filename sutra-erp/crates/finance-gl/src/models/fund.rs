//! Fund — grant/fund aggregate for fund accounting.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fund {
    pub fund_id: EntityId<Fund>,
    pub tenant_id: TenantId,
    pub fund_code: String,
    pub fund_name: String,
    pub fund_type: FundType,
    pub fund_source: FundSource,
    pub grant_scheme: Option<String>,
    pub sanction_order_number: Option<String>,
    pub sanction_date: Option<NaiveDate>,
    pub sanctioned_amount: Money,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub status: FundStatus,
    pub bank_account_id: Option<Uuid>,
    pub fcra_registration_number: Option<String>,
    pub fcra_admin_expense_ratio: Option<rust_decimal::Decimal>,
    pub principal_amount: Option<Money>,
    pub income_only: bool,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FundType {
    Restricted,
    Unrestricted,
    Endowment,
    Fcra,
    Scholarship,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FundSource {
    GovernmentUgc,
    GovernmentState,
    GovernmentOther,
    Private,
    Donation,
    Internal,
    Fcra,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FundStatus {
    Active,
    Completed,
    Terminated,
    Suspended,
}
