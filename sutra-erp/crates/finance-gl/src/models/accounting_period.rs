//! AccountingPeriod — period within a fiscal year.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

/// An accounting period within a fiscal year (monthly, 1-13).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountingPeriod {
    pub accounting_period_id: EntityId<AccountingPeriod>,
    pub tenant_id: TenantId,
    pub fiscal_year_id: Uuid,
    pub period_number: i32,
    pub period_name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: PeriodStatus,
    pub gst_filing_deadline: Option<NaiveDate>,
    pub tds_filing_deadline: Option<NaiveDate>,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PeriodStatus {
    Open,
    Closing,
    Closed,
}
