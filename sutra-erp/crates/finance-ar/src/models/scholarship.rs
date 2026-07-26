//! Scholarship — aggregate root for student scholarship grants.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scholarship {
    pub scholarship_id: EntityId<Scholarship>,
    pub tenant_id: TenantId,
    pub student_id: Uuid,
    pub scheme_id: Uuid,
    pub student_fee_account_id: Option<Uuid>,
    pub application_reference: Option<String>,
    pub expected_amount: Money,
    pub sanctioned_amount: Option<Money>,
    pub disbursed_amount: Option<Money>,
    pub dbt_date: Option<DateTime<Utc>>,
    pub dbt_transaction_ref: Option<String>,
    pub status: String,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScholarshipScheme {
    pub scholarship_scheme_id: EntityId<ScholarshipScheme>,
    pub tenant_id: TenantId,
    pub scheme_code: String,
    pub scheme_name: String,
    pub provider: String,
    pub state: Option<String>,
    pub scheme_type: String,
    pub max_amount: Money,
    pub eligibility_criteria: Option<serde_json::Value>,
    pub is_active: bool,
    pub requires_aadhaar: bool,
    pub requires_bank_account: bool,
    pub requires_income_cert: bool,
    pub requires_caste_cert: bool,
    pub audit: AuditInfo,
}
