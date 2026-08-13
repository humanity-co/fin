//! ITC models — ItcRegister aggregate root and invoice-level lines.
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

/// ITC register lifecycle:
/// OPEN → COMPUTED → (reversal applied) → REVERSED → (period close) → CLOSED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ItcRegisterStatus {
    Open,
    Computed,
    Reversed,
    Closed,
}

impl ItcRegisterStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "COMPUTED" => ItcRegisterStatus::Computed,
            "REVERSED" => ItcRegisterStatus::Reversed,
            "CLOSED" => ItcRegisterStatus::Closed,
            _ => ItcRegisterStatus::Open,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            ItcRegisterStatus::Open => "OPEN",
            ItcRegisterStatus::Computed => "COMPUTED",
            ItcRegisterStatus::Reversed => "REVERSED",
            ItcRegisterStatus::Closed => "CLOSED",
        }
    }
}

/// ITC eligibility of an invoice line (CGST Act s.17(2)/s.17(5), Rules
/// 42/43). Must match the account's `itc_eligibility` at posting time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItcEligibility {
    #[serde(rename = "FULL")]
    Full,
    #[serde(rename = "BLOCKED")]
    Blocked,
    #[serde(rename = "REVERSAL_42")]
    Reversal42,
    #[serde(rename = "REVERSAL_43")]
    Reversal43,
    #[serde(rename = "CAPITAL_GOODS")]
    CapitalGoods,
}

impl ItcEligibility {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "BLOCKED" => ItcEligibility::Blocked,
            "REVERSAL_42" => ItcEligibility::Reversal42,
            "REVERSAL_43" => ItcEligibility::Reversal43,
            "CAPITAL_GOODS" => ItcEligibility::CapitalGoods,
            _ => ItcEligibility::Full,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            ItcEligibility::Full => "FULL",
            ItcEligibility::Blocked => "BLOCKED",
            ItcEligibility::Reversal42 => "REVERSAL_42",
            ItcEligibility::Reversal43 => "REVERSAL_43",
            ItcEligibility::CapitalGoods => "CAPITAL_GOODS",
        }
    }
}

/// ITC register aggregate root — per registration per period.
///
/// Invariant: unique (gst_registration_id, period);
/// `net_itc_eligible = total_itc − rule_42_reversal − rule_43_reversal`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItcRegister {
    pub itc_register_id: EntityId<ItcRegister>,
    pub tenant_id: TenantId,
    pub gst_registration_id: Uuid,
    /// e.g. "072026".
    pub period: String,
    pub status: ItcRegisterStatus,
    pub total_itc: i64,
    pub itc_on_inputs: i64,
    pub itc_on_capital_goods: i64,
    pub itc_reversal_rule_42: i64,
    pub itc_reversal_rule_43: i64,
    pub net_itc_eligible: i64,
    pub exempt_turnover: i64,
    pub total_turnover: i64,
    pub audit: AuditInfo,
}

/// Invoice-level ITC tracking line.
///
/// `itc_eligibility` must match the account's `itc_eligibility` at posting;
/// Rule 42/43 lines carry `reversal_percent` and `reversal_amount` and are
/// flagged `is_reversed` once the reversal journal is posted. Capital-goods
/// lines (Rule 43) carry `acquisition_period` ("MMYYYY") so the 60-month
/// reversal horizon is measured from the actual acquisition month.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItcRegisterLine {
    pub itc_register_line_id: EntityId<ItcRegisterLine>,
    pub tenant_id: TenantId,
    pub itc_register_id: Uuid,
    pub invoice_id: Uuid,
    pub invoice_number: String,
    pub invoice_date: NaiveDate,
    pub vendor_gstin: Option<String>,
    pub taxable_value: i64,
    pub igst: i64,
    pub cgst: i64,
    pub sgst: i64,
    pub total_tax: i64,
    pub itc_eligibility: ItcEligibility,
    /// Capital-goods acquisition month "MMYYYY" (Rule 43 horizon).
    pub acquisition_period: Option<String>,
    pub reversal_percent: Option<Decimal>,
    pub reversal_amount: Option<i64>,
    pub is_reversed: bool,
    pub audit: AuditInfo,
}
