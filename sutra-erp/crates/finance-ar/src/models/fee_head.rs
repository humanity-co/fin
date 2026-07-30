//! FeeHead — a specific chargeable fee item in the institution's fee catalog.

use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};

/// The type/category of a fee head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeeType {
    Tuition,
    Exam,
    Library,
    Lab,
    Hostel,
    Transport,
    Mess,
    Sports,
    Development,
    Other,
}

impl FeeType {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            FeeType::Tuition => "TUITION",
            FeeType::Exam => "EXAM",
            FeeType::Library => "LIBRARY",
            FeeType::Lab => "LAB",
            FeeType::Hostel => "HOSTEL",
            FeeType::Transport => "TRANSPORT",
            FeeType::Mess => "MESS",
            FeeType::Sports => "SPORTS",
            FeeType::Development => "DEVELOPMENT",
            FeeType::Other => "OTHER",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "TUITION" => FeeType::Tuition,
            "EXAM" => FeeType::Exam,
            "LIBRARY" => FeeType::Library,
            "LAB" => FeeType::Lab,
            "HOSTEL" => FeeType::Hostel,
            "TRANSPORT" => FeeType::Transport,
            "MESS" => FeeType::Mess,
            "SPORTS" => FeeType::Sports,
            "DEVELOPMENT" => FeeType::Development,
            _ => FeeType::Other,
        }
    }
}

/// GST classification for a fee head — determines whether GST is charged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GstClassification {
    Exempt,
    Nil,
    Taxable5,
    Taxable12,
    Taxable18,
    Taxable28,
}

impl GstClassification {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            GstClassification::Exempt => "EXEMPT",
            GstClassification::Nil => "NIL",
            GstClassification::Taxable5 => "TAXABLE_5",
            GstClassification::Taxable12 => "TAXABLE_12",
            GstClassification::Taxable18 => "TAXABLE_18",
            GstClassification::Taxable28 => "TAXABLE_28",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "EXEMPT" => GstClassification::Exempt,
            "NIL" => GstClassification::Nil,
            "TAXABLE_5" => GstClassification::Taxable5,
            "TAXABLE_12" => GstClassification::Taxable12,
            "TAXABLE_18" => GstClassification::Taxable18,
            "TAXABLE_28" => GstClassification::Taxable28,
            _ => GstClassification::Exempt,
        }
    }

    /// Get the GST rate as a decimal (0.0 for exempt).
    pub fn rate(&self) -> rust_decimal::Decimal {
        use rust_decimal::Decimal;
        match self {
            GstClassification::Exempt | GstClassification::Nil => Decimal::ZERO,
            GstClassification::Taxable5 => Decimal::new(5, 0),
            GstClassification::Taxable12 => Decimal::new(12, 0),
            GstClassification::Taxable18 => Decimal::new(18, 0),
            GstClassification::Taxable28 => Decimal::new(28, 0),
        }
    }
}

/// A fee head defines a chargeable line item in the fee catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeHead {
    pub fee_head_id: EntityId<FeeHead>,
    pub tenant_id: TenantId,
    pub code: String,
    pub name: String,
    pub fee_type: FeeType,
    pub gst_classification: GstClassification,
    pub sac_code: Option<String>,
    pub is_refundable: bool,
    pub audit: AuditInfo,
}
