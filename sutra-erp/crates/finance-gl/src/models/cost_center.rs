//! CostCenter — cost center aggregate for financial dimensioning.

use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostCenter {
    pub cost_center_id: EntityId<CostCenter>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub cost_center_code: String,
    pub cost_center_name: String,
    pub cost_center_type: CostCenterType,
    pub parent_id: Option<EntityId<CostCenter>>,
    pub manager_id: Option<Uuid>,
    pub budget_amount: Money,
    pub budget_period: Option<BudgetPeriod>,
    pub is_active: bool,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CostCenterType {
    Department,
    Campus,
    Project,
    Activity,
    Program,
    Course,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BudgetPeriod {
    Monthly,
    Quarterly,
    Annual,
}
