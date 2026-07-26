//! Workflow models — ApprovalRequest aggregate root.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub approval_request_id: EntityId<ApprovalRequest>,
    pub tenant_id: TenantId,
    pub workflow_id: Uuid,
    pub transaction_type: String,
    pub transaction_id: Uuid,
    pub transaction_number: Option<String>,
    pub amount: Money,
    pub current_level: i32,
    pub status: String,
    pub requested_by_id: Uuid,
    pub requested_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalWorkflow {
    pub approval_workflow_id: EntityId<ApprovalWorkflow>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub transaction_type: String,
    pub workflow_name: String,
    pub is_active: bool,
    pub levels: i32,
    pub config: serde_json::Value,
    pub audit: AuditInfo,
}
