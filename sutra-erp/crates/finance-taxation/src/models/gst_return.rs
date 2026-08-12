//! GST return models — GstReturn aggregate root and GstReturnLine entity.
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

/// GST return types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GstReturnType {
    Gstr1,
    Gstr3b,
    Gstr9,
    Gstr9c,
}

impl GstReturnType {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GSTR3B" => GstReturnType::Gstr3b,
            "GSTR9" => GstReturnType::Gstr9,
            "GSTR9C" => GstReturnType::Gstr9c,
            _ => GstReturnType::Gstr1,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            GstReturnType::Gstr1 => "GSTR1",
            GstReturnType::Gstr3b => "GSTR3B",
            GstReturnType::Gstr9 => "GSTR9",
            GstReturnType::Gstr9c => "GSTR9C",
        }
    }
}

/// GST return lifecycle:
/// DRAFT → GENERATED → FILED → FILED_WITH_ERRORS → ADJUSTED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GstReturnStatus {
    Draft,
    Generated,
    Filed,
    FiledWithErrors,
    Adjusted,
}

impl GstReturnStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GENERATED" => GstReturnStatus::Generated,
            "FILED" => GstReturnStatus::Filed,
            "FILED_WITH_ERRORS" => GstReturnStatus::FiledWithErrors,
            "ADJUSTED" => GstReturnStatus::Adjusted,
            _ => GstReturnStatus::Draft,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            GstReturnStatus::Draft => "DRAFT",
            GstReturnStatus::Generated => "GENERATED",
            GstReturnStatus::Filed => "FILED",
            GstReturnStatus::FiledWithErrors => "FILED_WITH_ERRORS",
            GstReturnStatus::Adjusted => "ADJUSTED",
        }
    }
}

/// GST return aggregate root.
///
/// Invariant: unique (gst_registration, return_type, period). GSTR-3B
/// consistency: 3.1(d) RCM output must equal 4(B)(2) RCM ITC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GstReturn {
    pub gst_return_id: EntityId<GstReturn>,
    pub tenant_id: TenantId,
    pub gst_registration_id: Uuid,
    pub return_type: GstReturnType,
    /// e.g. "072026" for July 2026.
    pub period: String,
    pub fiscal_year: String,
    pub status: GstReturnStatus,
    pub due_date: NaiveDate,
    pub filed_date: Option<NaiveDate>,
    pub filed_by_id: Option<Uuid>,
    pub acknowledgment_no: Option<String>,
    /// Full return JSON matching the GSTN schema.
    pub json_data: Option<Value>,
    pub tax_liability: i64,
    pub itc_claimed: i64,
    pub net_tax_payable: i64,
    pub audit: AuditInfo,
}

/// A single section line of a GST return (GSTR-1 4A/4B/4C/6/7;
/// GSTR-3B 3.1/4/5). `section` must be valid for the return type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GstReturnLine {
    pub gst_return_line_id: EntityId<GstReturnLine>,
    pub tenant_id: TenantId,
    pub gst_return_id: Uuid,
    pub section: String,
    pub description: Option<String>,
    pub taxable_value: i64,
    pub igst_amount: i64,
    pub cgst_amount: i64,
    pub sgst_amount: i64,
    pub cess_amount: i64,
    pub audit: AuditInfo,
}
