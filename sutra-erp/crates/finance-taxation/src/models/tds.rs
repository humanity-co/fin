//! TDS models — section master, deposit challans, returns and certificates.
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

/// Who a TDS section applies to (IT Act deduction schemes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TdsSectionApplicableTo {
    ResidentIndividual,
    ResidentOther,
    NonResident,
    All,
}

impl TdsSectionApplicableTo {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "RESIDENT_INDIVIDUAL" => TdsSectionApplicableTo::ResidentIndividual,
            "RESIDENT_OTHER" => TdsSectionApplicableTo::ResidentOther,
            "NON_RESIDENT" => TdsSectionApplicableTo::NonResident,
            _ => TdsSectionApplicableTo::All,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            TdsSectionApplicableTo::ResidentIndividual => "RESIDENT_INDIVIDUAL",
            TdsSectionApplicableTo::ResidentOther => "RESIDENT_OTHER",
            TdsSectionApplicableTo::NonResident => "NON_RESIDENT",
            TdsSectionApplicableTo::All => "ALL",
        }
    }
}

/// TDS section/rate master row — effective-dated per tenant (IT Rules +
/// annual Finance Act). One active rate per section per period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdsSection {
    pub tds_section_id: EntityId<TdsSection>,
    pub tenant_id: TenantId,
    /// e.g. "194C", "194J", "194I", "194A", "194H", "194Q".
    pub section_code: String,
    pub description: String,
    pub default_rate: Decimal,
    /// Threshold per single payment (paise).
    pub threshold_per_payment: Option<i64>,
    /// Threshold on aggregate payments in the FY (paise).
    pub threshold_aggregate: Option<i64>,
    /// s.194Q-style: TDS applies only on the value exceeding the
    /// FY-aggregate threshold (CBDT Circular 17/2020). `None` per-payment
    /// threshold sections (194A/194I/194Q) have no per-payment relief —
    /// only the FY aggregate gates the deduction.
    pub threshold_excess_only: bool,
    pub applicable_to: TdsSectionApplicableTo,
    pub is_active: bool,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub audit: AuditInfo,
}

/// TDS deposit challan status.
///
/// The deduction's deposit leg (on AP-owned `tds_deductions`) runs
/// PENDING → DEPOSITED → FILED; this mirrors the challan record's own
/// status once DEPOSITED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TdsDepositStatus {
    Deposited,
    Filed,
}

impl TdsDepositStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "FILED" => TdsDepositStatus::Filed,
            _ => TdsDepositStatus::Deposited,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            TdsDepositStatus::Deposited => "DEPOSITED",
            TdsDepositStatus::Filed => "FILED",
        }
    }
}

/// TDS challan deposit record (`tds_deposits`) — one row per deduction
/// covered by an ITNS-281 challan deposit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdsDeposit {
    pub tds_deposit_id: EntityId<TdsDeposit>,
    pub tenant_id: TenantId,
    pub tds_deduction_id: Uuid,
    pub challan_reference: String,
    pub deposit_date: NaiveDate,
    /// Paise.
    pub amount: i64,
    pub bank_account_id: Option<Uuid>,
    /// Posted journal (DR TDS Payable 24.03 / CR Bank).
    pub journal_id: Option<Uuid>,
    pub status: TdsDepositStatus,
    pub recorded_by_id: Uuid,
    pub audit: AuditInfo,
}

/// TDS return types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TdsReturnType {
    #[serde(rename = "FORM_24Q")]
    Form24q,
    #[serde(rename = "FORM_26Q")]
    Form26q,
    #[serde(rename = "FORM_27Q")]
    Form27q,
}

impl TdsReturnType {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "FORM_26Q" => TdsReturnType::Form26q,
            "FORM_27Q" => TdsReturnType::Form27q,
            _ => TdsReturnType::Form24q,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            TdsReturnType::Form24q => "FORM_24Q",
            TdsReturnType::Form26q => "FORM_26Q",
            TdsReturnType::Form27q => "FORM_27Q",
        }
    }
}

/// TDS return lifecycle:
/// DRAFT → GENERATED → FILED → FILED_WITH_ERRORS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TdsReturnStatus {
    Draft,
    Generated,
    Filed,
    FiledWithErrors,
}

impl TdsReturnStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GENERATED" => TdsReturnStatus::Generated,
            "FILED" => TdsReturnStatus::Filed,
            "FILED_WITH_ERRORS" => TdsReturnStatus::FiledWithErrors,
            _ => TdsReturnStatus::Draft,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            TdsReturnStatus::Draft => "DRAFT",
            TdsReturnStatus::Generated => "GENERATED",
            TdsReturnStatus::Filed => "FILED",
            TdsReturnStatus::FiledWithErrors => "FILED_WITH_ERRORS",
        }
    }
}

/// TDS return aggregate root.
///
/// Invariant: unique (entity, return_type, quarter, fiscal_year).
/// 24Q = salary TDS (s.192), 26Q = non-salary (s.194*), 27Q = non-resident.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdsReturn {
    pub tds_return_id: EntityId<TdsReturn>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub return_type: TdsReturnType,
    /// "Q1".."Q4".
    pub quarter: String,
    pub fiscal_year: String,
    pub status: TdsReturnStatus,
    pub due_date: NaiveDate,
    pub filed_date: Option<NaiveDate>,
    pub acknowledgment_no: Option<String>,
    pub total_deductions: i64,
    pub total_deposits: i64,
    /// Full return JSON matching the TRACES schema.
    pub json_data: Option<Value>,
    pub audit: AuditInfo,
}

/// Individual deduction row inside a TDS return.
///
/// 24Q rows require `salary_month`; every row requires PAN and challan
/// details for the deduction-to-deposit trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdsReturnDetail {
    pub tds_return_detail_id: EntityId<TdsReturnDetail>,
    pub tenant_id: TenantId,
    pub tds_return_id: Uuid,
    pub vendor_id: Option<Uuid>,
    pub employee_id: Option<Uuid>,
    pub pan: String,
    pub section: String,
    pub payment_date: NaiveDate,
    pub payment_amount: i64,
    pub tds_rate: Decimal,
    pub tds_amount: i64,
    pub surcharge: i64,
    pub cess: i64,
    pub total_tds: i64,
    pub challan_details: Option<Value>,
    /// 1..12 — required for Form 24Q rows.
    pub salary_month: Option<i32>,
    pub audit: AuditInfo,
}

/// Form 16 / Form 16A certificate type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form16CertificateType {
    #[serde(rename = "FORM_16")]
    Form16,
    #[serde(rename = "FORM_16A")]
    Form16a,
}

impl Form16CertificateType {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "FORM_16A" => Form16CertificateType::Form16a,
            _ => Form16CertificateType::Form16,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            Form16CertificateType::Form16 => "FORM_16",
            Form16CertificateType::Form16a => "FORM_16A",
        }
    }
}

/// Certificate lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Form16CertificateStatus {
    Generated,
    Issued,
    Revoked,
}

impl Form16CertificateStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "ISSUED" => Form16CertificateStatus::Issued,
            "REVOKED" => Form16CertificateStatus::Revoked,
            _ => Form16CertificateStatus::Generated,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            Form16CertificateStatus::Generated => "GENERATED",
            Form16CertificateStatus::Issued => "ISSUED",
            Form16CertificateStatus::Revoked => "REVOKED",
        }
    }
}

/// TDS certificate record — Form 16 (per employee per FY) or Form 16A
/// (per vendor per FY).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Form16Certificate {
    pub form16_certificate_id: EntityId<Form16Certificate>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub certificate_type: Form16CertificateType,
    pub fiscal_year: String,
    /// Form 16 — employee (salary TDS).
    pub employee_id: Option<Uuid>,
    /// Form 16A — vendor (non-salary TDS).
    pub vendor_id: Option<Uuid>,
    pub pan: String,
    pub document_url: String,
    pub status: Form16CertificateStatus,
    pub generated_by_id: Option<Uuid>,
    pub issued_at: Option<DateTime<Utc>>,
    pub audit: AuditInfo,
}
