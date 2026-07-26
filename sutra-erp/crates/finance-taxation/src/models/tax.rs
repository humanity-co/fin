//! Taxation models — GST Registration aggregate root.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GstRegistration {
    pub gst_registration_id: EntityId<GstRegistration>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub gstin: String,
    pub trade_name: String,
    pub legal_name: String,
    pub registration_type: String,
    pub filing_frequency: String,
    pub is_composite: bool,
    pub state_code: String,
    pub is_active: bool,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdsSection {
    pub tds_section_id: EntityId<TdsSection>,
    pub section_code: String,
    pub description: String,
    pub default_rate: rust_decimal::Decimal,
    pub threshold_per_payment: Option<i64>,
    pub applicable_to: String,
    pub is_active: bool,
}
