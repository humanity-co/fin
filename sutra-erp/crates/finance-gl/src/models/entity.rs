//! Entity — multi-campus / multi-institute entity aggregate.

use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_id: EntityId<Entity>,
    pub tenant_id: TenantId,
    pub entity_code: String,
    pub entity_name: String,
    pub entity_type: EntityType,
    pub gstin: Option<String>,
    pub pan: Option<String>,
    pub parent_entity_id: Option<EntityId<Entity>>,
    pub consolidation_group: Option<String>,
    pub is_active: bool,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntityType {
    MainCampus,
    SatelliteCampus,
    ResearchCenter,
    SkillCenter,
    Institute,
}
