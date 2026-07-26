//! FeeStructure — aggregate root for student fee definitions.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructure {
    pub fee_structure_id: EntityId<FeeStructure>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub program_id: Option<Uuid>,
    pub academic_year: String,
    pub semester_term: String,
    pub student_category: Option<String>,
    pub fee_structure_name: String,
    pub frc_approval_order_number: Option<String>,
    pub frc_approval_date: Option<NaiveDate>,
    pub status: FeeStructureStatus,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub lines: Vec<FeeStructureLine>,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeHead {
    pub fee_head_id: EntityId<FeeHead>,
    pub tenant_id: TenantId,
    pub fee_head_code: String,
    pub fee_head_name: String,
    pub fee_type: String,
    pub gst_classification: Option<String>,
    pub hsn_sac_code: Option<String>,
    pub is_optional: bool,
    pub is_refundable: bool,
    pub is_mandatory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructureLine {
    pub fee_structure_line_id: Uuid,
    pub fee_head_id: EntityId<FeeHead>,
    pub amount: Money,
    pub is_optional: bool,
    pub installment_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallmentPlan {
    pub installment_plan_id: EntityId<InstallmentPlan>,
    pub tenant_id: TenantId,
    pub fee_structure_id: EntityId<FeeStructure>,
    pub plan_name: String,
    pub number_of_installments: i32,
    pub installment_distribution: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeeStructureStatus {
    Draft,
    Active,
    Archived,
}
