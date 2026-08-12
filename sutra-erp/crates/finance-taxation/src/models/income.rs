//! Income-tax compliance models — trust exemptions, 85% application,
//! s.11(5) investments monitor and FCRA register.
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

/// Exemption section under the Income Tax Act.
///
/// `SECTION_10_23C` and `SECTION_10_23C_VI` cover the s.10(23C) variants;
/// `SECTION_11_12A` covers a s.11 trust registered under s.12A;
/// `SECTION_12AB` covers the post-2021 s.12AB regime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustExemptionSection {
    #[serde(rename = "SECTION_10_23C")]
    Section10_23c,
    #[serde(rename = "SECTION_10_23C_VI")]
    Section10_23cVi,
    #[serde(rename = "SECTION_11_12A")]
    Section11_12a,
    #[serde(rename = "SECTION_12AB")]
    Section12ab,
}

impl TrustExemptionSection {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "SECTION_10_23C_VI" => TrustExemptionSection::Section10_23cVi,
            "SECTION_11_12A" => TrustExemptionSection::Section11_12a,
            "SECTION_12AB" => TrustExemptionSection::Section12ab,
            _ => TrustExemptionSection::Section10_23c,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            TrustExemptionSection::Section10_23c => "SECTION_10_23C",
            TrustExemptionSection::Section10_23cVi => "SECTION_10_23C_VI",
            TrustExemptionSection::Section11_12a => "SECTION_11_12A",
            TrustExemptionSection::Section12ab => "SECTION_12AB",
        }
    }
}

/// Exemption lifecycle:
/// ACTIVE → (≤180 days left) → RENEWAL_PENDING → Renew → ACTIVE;
/// (past valid_to) → EXPIRED; CANCELLED from ACTIVE only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrustExemptionStatus {
    Active,
    Expired,
    RenewalPending,
    Cancelled,
}

impl TrustExemptionStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "EXPIRED" => TrustExemptionStatus::Expired,
            "RENEWAL_PENDING" => TrustExemptionStatus::RenewalPending,
            "CANCELLED" => TrustExemptionStatus::Cancelled,
            _ => TrustExemptionStatus::Active,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            TrustExemptionStatus::Active => "ACTIVE",
            TrustExemptionStatus::Expired => "EXPIRED",
            TrustExemptionStatus::RenewalPending => "RENEWAL_PENDING",
            TrustExemptionStatus::Cancelled => "CANCELLED",
        }
    }
}

/// Trust/society exemption registration (12A/12AB/10(23C)).
///
/// Invariants: unique (entity, exemption_section); provisional 12AB /
/// 10(23C) registration has 3-year validity; renewal must happen before
/// expiry (auto reminder at 180 days).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustExemption {
    pub trust_exemption_id: EntityId<TrustExemption>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub exemption_section: TrustExemptionSection,
    pub registration_no: String,
    pub registration_date: NaiveDate,
    pub valid_from: NaiveDate,
    pub valid_to: NaiveDate,
    pub status: TrustExemptionStatus,
    pub approving_authority: Option<String>,
    pub is_trust: bool,
    pub trust_name: Option<String>,
    pub trust_pan: Option<String>,
    pub audit: AuditInfo,
}

/// Income-application status (85% rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IncomeApplicationStatus {
    Compliant,
    NonCompliant,
    UnderReview,
}

impl IncomeApplicationStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "NON_COMPLIANT" => IncomeApplicationStatus::NonCompliant,
            "UNDER_REVIEW" => IncomeApplicationStatus::UnderReview,
            _ => IncomeApplicationStatus::Compliant,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            IncomeApplicationStatus::Compliant => "COMPLIANT",
            IncomeApplicationStatus::NonCompliant => "NON_COMPLIANT",
            IncomeApplicationStatus::UnderReview => "UNDER_REVIEW",
        }
    }
}

/// Income-application category (qualifying "application" heads under
/// s.11(1)(a)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IncomeApplicationCategory {
    Salaries,
    Infrastructure,
    Scholarships,
    Research,
    Maintenance,
    OtherEducational,
}

impl IncomeApplicationCategory {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "INFRASTRUCTURE" => IncomeApplicationCategory::Infrastructure,
            "SCHOLARSHIPS" => IncomeApplicationCategory::Scholarships,
            "RESEARCH" => IncomeApplicationCategory::Research,
            "MAINTENANCE" => IncomeApplicationCategory::Maintenance,
            "OTHER_EDUCATIONAL" => IncomeApplicationCategory::OtherEducational,
            _ => IncomeApplicationCategory::Salaries,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            IncomeApplicationCategory::Salaries => "SALARIES",
            IncomeApplicationCategory::Infrastructure => "INFRASTRUCTURE",
            IncomeApplicationCategory::Scholarships => "SCHOLARSHIPS",
            IncomeApplicationCategory::Research => "RESEARCH",
            IncomeApplicationCategory::Maintenance => "MAINTENANCE",
            IncomeApplicationCategory::OtherEducational => "OTHER_EDUCATIONAL",
        }
    }
}

/// 85%-application aggregate root.
///
/// Invariants: unique (fiscal_year, entity);
/// `application_percent = amount_applied / total_income × 100` — must be
/// ≥ `tax.income_application_threshold` (default 85).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeApplication {
    pub income_application_id: EntityId<IncomeApplication>,
    pub tenant_id: TenantId,
    pub fiscal_year_id: Uuid,
    pub entity_id: Uuid,
    pub total_income: i64,
    pub amount_applied: i64,
    pub application_percent: Option<Decimal>,
    pub accumulated_amount: i64,
    /// Year the accumulation started (5-year s.11(2) window).
    pub accumulation_year: Option<i32>,
    pub accumulation_purpose: Option<String>,
    pub status: IncomeApplicationStatus,
    pub last_computed_at: DateTime<Utc>,
    pub audit: AuditInfo,
}

/// Line item of the income-application computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeApplicationLine {
    pub income_app_line_id: EntityId<IncomeApplicationLine>,
    pub tenant_id: TenantId,
    pub income_application_id: Uuid,
    pub category: IncomeApplicationCategory,
    /// Paise; must be > 0.
    pub amount: i64,
    pub account_id: Uuid,
    pub description: Option<String>,
    pub audit: AuditInfo,
}

/// FCRA registration status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FcraStatus {
    Active,
    Expired,
    RenewalPending,
    Cancelled,
}

impl FcraStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "EXPIRED" => FcraStatus::Expired,
            "RENEWAL_PENDING" => FcraStatus::RenewalPending,
            "CANCELLED" => FcraStatus::Cancelled,
            _ => FcraStatus::Active,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            FcraStatus::Active => "ACTIVE",
            FcraStatus::Expired => "EXPIRED",
            FcraStatus::RenewalPending => "RENEWAL_PENDING",
            FcraStatus::Cancelled => "CANCELLED",
        }
    }
}

/// FCRA (Foreign Contribution Regulation Act 2010) registration.
///
/// Invariants: `bank_account_id` must be an FCRA-designated account;
/// `admin_expense_ratio ≤ 20%` of receipts (FCRA s.17, policy
/// `tax.fcra_admin_expense_ratio_limit`); FC-4 return by 31 Dec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcraRegistration {
    pub fcra_registration_id: EntityId<FcraRegistration>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub registration_no: String,
    pub valid_from: NaiveDate,
    pub valid_to: NaiveDate,
    pub bank_account_id: Uuid,
    pub status: FcraStatus,
    pub total_receipts: i64,
    pub admin_expenses: i64,
    pub admin_expense_ratio: Option<Decimal>,
    pub fc4_return_filed_date: Option<NaiveDate>,
    pub audit: AuditInfo,
}
