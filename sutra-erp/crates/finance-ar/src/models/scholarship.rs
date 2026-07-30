//! Scholarship — aggregate root for student scholarship grants.
//! ScholarshipScheme — configuration entity for scholarship programs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

/// Categories for scholarship schemes (caste/merit/sports-based, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScholarshipCategory {
    SC,
    ST,
    OBC,
    VJNT,
    EBC,
    Minority,
    Merit,
    Sports,
    PhysicallyDisabled,
    Other,
}

impl ScholarshipCategory {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            ScholarshipCategory::SC => "SC",
            ScholarshipCategory::ST => "ST",
            ScholarshipCategory::OBC => "OBC",
            ScholarshipCategory::VJNT => "VJNT",
            ScholarshipCategory::EBC => "EBC",
            ScholarshipCategory::Minority => "MINORITY",
            ScholarshipCategory::Merit => "MERIT",
            ScholarshipCategory::Sports => "SPORTS",
            ScholarshipCategory::PhysicallyDisabled => "PHYSICALLY_DISABLED",
            ScholarshipCategory::Other => "OTHER",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "SC" => ScholarshipCategory::SC,
            "ST" => ScholarshipCategory::ST,
            "OBC" => ScholarshipCategory::OBC,
            "VJNT" => ScholarshipCategory::VJNT,
            "EBC" => ScholarshipCategory::EBC,
            "MINORITY" => ScholarshipCategory::Minority,
            "MERIT" => ScholarshipCategory::Merit,
            "SPORTS" => ScholarshipCategory::Sports,
            "PHYSICALLY_DISABLED" => ScholarshipCategory::PhysicallyDisabled,
            _ => ScholarshipCategory::Other,
        }
    }
}

/// Funding source for a scholarship scheme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FundingSource {
    CentralGovt,
    StateGovt,
    Institution,
    Private,
}

impl FundingSource {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            FundingSource::CentralGovt => "CENTRAL_GOVT",
            FundingSource::StateGovt => "STATE_GOVT",
            FundingSource::Institution => "INSTITUTION",
            FundingSource::Private => "PRIVATE",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "CENTRAL_GOVT" => FundingSource::CentralGovt,
            "STATE_GOVT" => FundingSource::StateGovt,
            "INSTITUTION" => FundingSource::Institution,
            _ => FundingSource::Private,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScholarshipStatus {
    Applied,
    Verified,
    Sanctioned,
    Disbursed,
    Rejected,
    Reconciled,
}

impl ScholarshipStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            ScholarshipStatus::Applied => "APPLIED",
            ScholarshipStatus::Verified => "VERIFIED",
            ScholarshipStatus::Sanctioned => "SANCTIONED",
            ScholarshipStatus::Disbursed => "DISBURSED",
            ScholarshipStatus::Rejected => "REJECTED",
            ScholarshipStatus::Reconciled => "RECONCILED",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "APPLIED" => ScholarshipStatus::Applied,
            "VERIFIED" => ScholarshipStatus::Verified,
            "SANCTIONED" => ScholarshipStatus::Sanctioned,
            "DISBURSED" => ScholarshipStatus::Disbursed,
            "REJECTED" => ScholarshipStatus::Rejected,
            "RECONCILED" => ScholarshipStatus::Reconciled,
            _ => ScholarshipStatus::Applied,
        }
    }
}

/// A scholarship scheme definition (e.g., PMS for SC, Rajarshi Shahu).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScholarshipScheme {
    pub scholarship_scheme_id: EntityId<ScholarshipScheme>,
    pub tenant_id: TenantId,
    pub name: String,
    pub code: String,
    pub category: ScholarshipCategory,
    pub funding_source: FundingSource,
    pub maha_dbt_scheme_code: Option<String>,
    pub max_amount: Money,
    pub eligibility_criteria: Option<serde_json::Value>,
    pub is_active: bool,
    pub requires_aadhaar: bool,
    pub requires_bank_account: bool,
    pub requires_income_cert: bool,
    pub requires_caste_cert: bool,
    pub audit: AuditInfo,
}

/// A student's scholarship application and its lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentScholarship {
    pub scholarship_id: EntityId<StudentScholarship>,
    pub tenant_id: TenantId,
    pub student_id: Uuid,
    pub scheme_id: EntityId<ScholarshipScheme>,
    pub student_fee_account_id: Option<EntityId<super::student_fee::StudentFeeAccount>>,
    pub academic_year: String,
    pub expected_amount: Money,
    pub sanctioned_amount: Option<Money>,
    pub disbursed_amount: Option<Money>,
    pub status: ScholarshipStatus,
    pub maha_dbt_application_id: Option<String>,
    pub dbt_transaction_id: Option<String>,
    pub dbt_date: Option<DateTime<Utc>>,
    pub verified_by: Option<Uuid>,
    pub verified_at: Option<DateTime<Utc>>,
    pub sanctioned_by: Option<Uuid>,
    pub sanctioned_at: Option<DateTime<Utc>>,
    pub remarks: Option<String>,
    pub audit: AuditInfo,
}
