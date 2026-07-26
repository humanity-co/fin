//! Budgeting models — Budget aggregate root.

use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub budget_id: EntityId<Budget>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub fiscal_year_id: Uuid,
    pub budget_type: String,
    pub budget_name: String,
    pub status: String,
    pub total_amount: Money,
    pub revised_amount: Option<Money>,
    pub fund_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub approved_by_id: Option<Uuid>,
    pub audit: AuditInfo,
}
