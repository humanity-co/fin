//! Vendor aggregate root.

use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vendor {
    pub vendor_id: EntityId<Vendor>,
    pub tenant_id: TenantId,
    pub entity_id: Option<Uuid>,
    pub vendor_code: String,
    pub vendor_name: String,
    pub vendor_type: String,
    pub pan: Option<String>,
    pub pan_status: String,
    pub gstin: Option<String>,
    pub gstin_status: String,
    pub gst_composition_scheme: bool,
    pub registration_type: String,
    pub contact_person: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub payment_terms: i32,
    pub default_tds_section: Option<String>,
    pub tds_applicable: bool,
    pub tax_applicable: bool,
    pub is_active: bool,
    pub is_blacklisted: bool,
    pub msme_reg_number: Option<String>,
    pub msme_type: Option<String>,
    pub audit: AuditInfo,
}
