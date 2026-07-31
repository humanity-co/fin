//! Vendor aggregate root with Section 197 certificates and bank accounts.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};
use uuid::Uuid;

/// Vendor aggregate root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vendor {
    pub vendor_id: EntityId<Vendor>,
    pub tenant_id: TenantId,
    pub entity_id: Option<Uuid>,
    pub vendor_code: String,
    pub vendor_name: String,
    pub vendor_type: VendorType,
    pub pan: Option<String>,
    pub pan_status: PanStatus,
    pub gstin: Option<String>,
    pub gstin_status: GstinStatus,
    pub gst_composition_scheme: bool,
    pub registration_type: RegistrationType,
    pub contact_person: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub pincode: Option<String>,
    pub payment_terms: i32,
    pub default_tds_section: Option<String>,
    pub tds_applicable: bool,
    pub tax_applicable: bool,
    pub is_active: bool,
    pub is_blacklisted: bool,
    pub blacklist_reason: Option<String>,
    pub msme_reg_number: Option<String>,
    pub msme_type: Option<MsmeType>,
    pub section_197_certificates: Vec<Section197Certificate>,
    pub bank_accounts: Vec<VendorBankAccount>,
    pub audit: AuditInfo,
}

/// Vendor type / legal constitution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VendorType {
    Individual,
    Proprietorship,
    Partnership,
    #[serde(rename = "LLP")]
    Llp,
    #[serde(rename = "PrivateLimited")]
    PrivateLimited,
    #[serde(rename = "PublicLimited")]
    PublicLimited,
    Government,
    Trust,
    Society,
    #[serde(rename = "HUF")]
    Huf,
    Other,
}

impl VendorType {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            VendorType::Individual => "INDIVIDUAL",
            VendorType::Proprietorship => "PROPRIETORSHIP",
            VendorType::Partnership => "PARTNERSHIP",
            VendorType::Llp => "LLP",
            VendorType::PrivateLimited => "PVT_LTD",
            VendorType::PublicLimited => "PUB_LTD",
            VendorType::Government => "GOVERNMENT",
            VendorType::Trust => "TRUST",
            VendorType::Society => "SOCIETY",
            VendorType::Huf => "HUF",
            VendorType::Other => "OTHER",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "INDIVIDUAL" => VendorType::Individual,
            "PROPRIETORSHIP" => VendorType::Proprietorship,
            "PARTNERSHIP" => VendorType::Partnership,
            "LLP" => VendorType::Llp,
            "PVT_LTD" => VendorType::PrivateLimited,
            "PUB_LTD" => VendorType::PublicLimited,
            "GOVERNMENT" => VendorType::Government,
            "TRUST" => VendorType::Trust,
            "SOCIETY" => VendorType::Society,
            "HUF" => VendorType::Huf,
            _ => VendorType::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanStatus {
    Verified,
    Unverified,
    Invalid,
}

impl PanStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            PanStatus::Verified => "VERIFIED",
            PanStatus::Unverified => "UNVERIFIED",
            PanStatus::Invalid => "INVALID",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "VERIFIED" => PanStatus::Verified,
            "INVALID" => PanStatus::Invalid,
            _ => PanStatus::Unverified,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GstinStatus {
    Verified,
    Unverified,
    Invalid,
    NotRegistered,
}

impl GstinStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            GstinStatus::Verified => "VERIFIED",
            GstinStatus::Unverified => "UNVERIFIED",
            GstinStatus::Invalid => "INVALID",
            GstinStatus::NotRegistered => "NOT_REGISTERED",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "VERIFIED" => GstinStatus::Verified,
            "INVALID" => GstinStatus::Invalid,
            "NOT_REGISTERED" => GstinStatus::NotRegistered,
            _ => GstinStatus::Unverified,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationType {
    Regular,
    Composition,
    Unregistered,
    NonResident,
}

impl RegistrationType {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            RegistrationType::Regular => "REGULAR",
            RegistrationType::Composition => "COMPOSITION",
            RegistrationType::Unregistered => "UNREGISTERED",
            RegistrationType::NonResident => "NON_RESIDENT",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "COMPOSITION" => RegistrationType::Composition,
            "UNREGISTERED" => RegistrationType::Unregistered,
            "NON_RESIDENT" => RegistrationType::NonResident,
            _ => RegistrationType::Regular,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MsmeType {
    Micro,
    Small,
    Medium,
}

impl MsmeType {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            MsmeType::Micro => "MICRO",
            MsmeType::Small => "SMALL",
            MsmeType::Medium => "MEDIUM",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "SMALL" => MsmeType::Small,
            "MEDIUM" => MsmeType::Medium,
            _ => MsmeType::Micro,
        }
    }
}

/// Section 197 lower/nil deduction certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section197Certificate {
    pub certificate_id: EntityId<Section197Certificate>,
    pub tenant_id: TenantId,
    pub vendor_id: EntityId<Vendor>,
    pub certificate_no: String,
    pub section: String,
    pub specified_rate: rust_decimal::Decimal,
    pub issued_by: Option<String>,
    pub valid_from: NaiveDate,
    pub valid_to: NaiveDate,
    pub is_active: bool,
    pub document_url: Option<String>,
    pub audit: AuditInfo,
}

/// Vendor bank account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorBankAccount {
    pub vendor_bank_account_id: EntityId<VendorBankAccount>,
    pub tenant_id: TenantId,
    pub vendor_id: EntityId<Vendor>,
    pub account_number: String,
    pub ifsc_code: String,
    pub bank_name: String,
    pub branch_name: Option<String>,
    pub account_type: BankAccountType,
    pub is_primary: bool,
    pub validation_status: BankValidationStatus,
    pub penny_drop_amount: Option<sutra_core::Money>,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BankAccountType {
    Savings,
    Current,
    CashCredit,
}

impl BankAccountType {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            BankAccountType::Savings => "SAVINGS",
            BankAccountType::Current => "CURRENT",
            BankAccountType::CashCredit => "CASH_CREDIT",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "SAVINGS" => BankAccountType::Savings,
            "CASH_CREDIT" => BankAccountType::CashCredit,
            _ => BankAccountType::Current,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BankValidationStatus {
    Unverified,
    Verified,
    Failed,
}

impl BankValidationStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            BankValidationStatus::Unverified => "UNVERIFIED",
            BankValidationStatus::Verified => "VERIFIED",
            BankValidationStatus::Failed => "FAILED",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "VERIFIED" => BankValidationStatus::Verified,
            "FAILED" => BankValidationStatus::Failed,
            _ => BankValidationStatus::Unverified,
        }
    }
}
