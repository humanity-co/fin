//! GST models — registration aggregate root and effective-dated rate master.
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

/// GST registration types (CGST Act).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GstRegistrationType {
    Regular,
    Composition,
    Unregistered,
}

impl GstRegistrationType {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "COMPOSITION" => GstRegistrationType::Composition,
            "UNREGISTERED" => GstRegistrationType::Unregistered,
            _ => GstRegistrationType::Regular,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            GstRegistrationType::Regular => "REGULAR",
            GstRegistrationType::Composition => "COMPOSITION",
            GstRegistrationType::Unregistered => "UNREGISTERED",
        }
    }
}

/// GST filing frequency (normal scheme is monthly; QRMP is quarterly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GstFilingFrequency {
    Monthly,
    Quarterly,
}

impl GstFilingFrequency {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "QUARTERLY" => GstFilingFrequency::Quarterly,
            _ => GstFilingFrequency::Monthly,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            GstFilingFrequency::Monthly => "MONTHLY",
            GstFilingFrequency::Quarterly => "QUARTERLY",
        }
    }
}

/// GST registration aggregate root.
///
/// Invariants: unique (tenant, gstin); unique per entity; GSTIN matches the
/// 15-character regex; `state_code` matches the entity's registered state.
/// Lifecycle: ACTIVE / INACTIVE (`is_active`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GstRegistration {
    pub gst_registration_id: EntityId<GstRegistration>,
    pub tenant_id: TenantId,
    /// Campus/entity the GSTIN belongs to (a multi-campus institution may
    /// hold several GSTINs).
    pub entity_id: Uuid,
    pub gstin: String,
    pub trade_name: String,
    pub legal_name: String,
    pub registration_type: GstRegistrationType,
    pub filing_frequency: GstFilingFrequency,
    pub is_composite: bool,
    /// 2-digit state code (e.g. "27" for Maharashtra).
    pub state_code: String,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub pincode: Option<String>,
    pub is_active: bool,
    pub audit: AuditInfo,
}

/// Supply classification for the rate master.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GstSupplyType {
    Goods,
    Services,
}

impl GstSupplyType {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GOODS" => GstSupplyType::Goods,
            _ => GstSupplyType::Services,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            GstSupplyType::Goods => "GOODS",
            GstSupplyType::Services => "SERVICES",
        }
    }
}

/// A row of the effective-dated GST rate master (`gst_rate_master`).
///
/// Invariants: `rate ∈ {0, 5, 12, 18, 28}`; overlapping effective-date
/// ranges per (tenant, hsn_sac_code, supply_type) are rejected (DB
/// EXCLUDE constraint). `effective_to = None` means open-ended.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GstRate {
    pub gst_rate_id: EntityId<GstRate>,
    pub tenant_id: TenantId,
    /// HSN (goods) or SAC (services) code.
    pub hsn_sac_code: String,
    pub description: Option<String>,
    /// GST rate as a whole percent (0, 5, 12, 18, 28).
    pub rate: i64,
    pub itc_eligible: bool,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub supply_type: GstSupplyType,
    pub is_active: bool,
    pub audit: AuditInfo,
}
