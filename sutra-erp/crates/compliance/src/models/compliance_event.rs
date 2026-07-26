//! Compliance models — ComplianceEvent aggregate root.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceEvent {
    pub compliance_event_id: EntityId<ComplianceEvent>,
    pub tenant_id: TenantId,
    pub entity_id: Option<Uuid>,
    pub event_type: String,
    pub event_title: String,
    pub due_date: NaiveDate,
    pub reminder_days: Vec<i32>,
    pub status: String,
    pub completed_date: Option<NaiveDate>,
    pub completed_by_id: Option<Uuid>,
    pub reference_id: Option<Uuid>,
    pub audit: AuditInfo,
}
