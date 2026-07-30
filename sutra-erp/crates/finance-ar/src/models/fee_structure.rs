//! FeeStructure — aggregate root for student fee definitions.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

use super::fee_head::FeeHead;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructure {
    pub fee_structure_id: EntityId<FeeStructure>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub name: String,
    pub program_id: Option<Uuid>,
    pub batch: Option<String>,
    pub academic_year: String,
    pub semester: Option<String>,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub status: FeeStructureStatus,
    pub frc_approval_number: Option<String>,
    pub frc_approved_amount: Option<Money>,
    pub lines: Vec<FeeStructureLine>,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructureLine {
    pub fee_structure_line_id: Uuid,
    pub fee_head_id: EntityId<FeeHead>,
    pub amount: Money,
    pub installment_plan_id: Option<EntityId<super::installment_plan::InstallmentPlan>>,
    pub is_mandatory: bool,
    pub gst_rate: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeeStructureStatus {
    Draft,
    Active,
    Inactive,
}
