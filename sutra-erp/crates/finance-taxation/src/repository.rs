//! Tax Engine data access layer (SQLx / PostgreSQL).
//!
//! Every query is tenant-scoped; money is BIGINT paise; masters soft-delete
//! (`deleted_at IS NULL`), fact tables (ITC register lines, GST return
//! lines, TDS deposits, income-application lines) are INSERT-only.
//!
//! Policy values (`itc_reversal_deminimis_paise`, `income_application_threshold`,
//! `accumulation_years`, `fcra_admin_expense_ratio_limit`, ...) come from
//! `system_config` (see `004_tax.sql` seeds) — tenant rows override GLOBAL
//! defaults, mirroring `TreasuryRepository::load_policy`.
//!
//! GL posting is NOT done here — commands post journals through the GL
//! module (`GlCommandHandler`) and only record the resulting journal_id.
//! GL account lookups by code prefix (24.03 TDS Payable, 24.02 RCM Payable,
//! 11.01 ITC Recoverable) live here so both commands and queries share one
//! definition of the statutory account map.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use sutra_core::{AuditInfo, EntityId, TenantId};

use crate::errors::TaxError;
use crate::models::gst::{GstFilingFrequency, GstRate, GstRegistration, GstRegistrationType, GstSupplyType};
use crate::models::gst_return::{GstReturn, GstReturnLine, GstReturnStatus, GstReturnType};
use crate::models::income::{
    FcraRegistration, FcraStatus, IncomeApplication, IncomeApplicationCategory,
    IncomeApplicationLine, IncomeApplicationStatus, TrustExemption, TrustExemptionSection,
    TrustExemptionStatus,
};
use crate::models::itc::{ItcEligibility, ItcRegister, ItcRegisterLine, ItcRegisterStatus};
use crate::models::tds::{
    Form16Certificate, Form16CertificateStatus, Form16CertificateType, TdsDeposit, TdsDepositStatus,
    TdsReturn, TdsReturnDetail, TdsReturnStatus, TdsReturnType, TdsSection, TdsSectionApplicableTo,
};

/// Nil tenant (GLOBAL defaults) — rows migrated from the base DDL map here.
pub const GLOBAL_TENANT: &str = "00000000-0000-0000-0000-000000000000";

/// Tax policy snapshot resolved from `system_config` (never hardcoded).
///
/// Defaults mirror the `004_tax.sql` seeds; a tenant row overrides the
/// GLOBAL default (tenant-first ordering, newest `valid_from` wins).
#[derive(Debug, Clone)]
pub struct TaxPolicy {
    /// Rule 42 de-minimis: no reversal when the computed reversal ≤ this
    /// fixed paise amount per tax period (proviso to CGST Rules r.42(1) —
    /// ₹5,000 default; CA review DE-MINIMIS).
    pub itc_reversal_deminimis_paise: i64,
    /// Minimum % of income applied to educational purposes (IT Act s.11(1)(a)).
    pub income_application_threshold: i64,
    /// Max period (years) unapplied income may be accumulated (s.11(2)).
    pub accumulation_years: i64,
    /// FCRA s.17 — admin expenses must be ≤ this % of FCRA receipts.
    pub fcra_admin_expense_ratio_limit: i64,
    /// TDS deposits due by the Nth of the following month (IT Rules r.30).
    pub tds_deposit_due_day: i64,
    /// Comma-separated s.11(5) approved investment categories.
    pub section115_securities_list: Vec<String>,
    /// State name → 2-digit code map ("Maharashtra:27,Karnataka:29") used to
    /// validate that a GSTIN's state code matches the entity's registered
    /// state (spec `RegisterGstin` invariant). Entities whose state has no
    /// mapping are not checked.
    pub state_code_map: Vec<(String, String)>,
    /// Section code used for salary TDS (Form 24Q / Form 16). IT Act s.192
    /// — "192" by default; configurable per tenant.
    pub salary_tds_section: String,
}

impl Default for TaxPolicy {
    fn default() -> Self {
        Self {
            // ₹5,000 per tax period — proviso to CGST Rules r.42(1)
            // (conservative; configurable per tenant).
            itc_reversal_deminimis_paise: 500_000,
            income_application_threshold: 85,
            accumulation_years: 5,
            fcra_admin_expense_ratio_limit: 20,
            tds_deposit_due_day: 7,
            section115_securities_list: vec![],
            state_code_map: vec![],
            salary_tds_section: "192".to_string(),
        }
    }
}

// ─── Row structs (mirror the base DDL + 004_tax.sql) ─────────────────

#[derive(sqlx::FromRow)]
pub(crate) struct GstRegistrationRow {
    gst_registration_id: Uuid,
    tenant_id: Uuid,
    entity_id: Uuid,
    gstin: String,
    trade_name: String,
    legal_name: String,
    registration_type: String,
    filing_frequency: String,
    is_composite: bool,
    state_code: String,
    address_line1: Option<String>,
    address_line2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    pincode: Option<String>,
    is_active: bool,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl GstRegistrationRow {
    pub(crate) fn into_model(self) -> GstRegistration {
        GstRegistration {
            gst_registration_id: EntityId::from_uuid(self.gst_registration_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            entity_id: self.entity_id,
            gstin: self.gstin,
            trade_name: self.trade_name,
            legal_name: self.legal_name,
            registration_type: GstRegistrationType::from_db_str(&self.registration_type),
            filing_frequency: GstFilingFrequency::from_db_str(&self.filing_frequency),
            is_composite: self.is_composite,
            state_code: self.state_code,
            address_line1: self.address_line1,
            address_line2: self.address_line2,
            city: self.city,
            state: self.state,
            pincode: self.pincode,
            is_active: self.is_active,
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct GstReturnRow {
    gst_return_id: Uuid,
    tenant_id: Uuid,
    gst_registration_id: Uuid,
    return_type: String,
    period: String,
    fiscal_year: String,
    status: String,
    due_date: NaiveDate,
    filed_date: Option<NaiveDate>,
    filed_by_id: Option<Uuid>,
    acknowledgment_no: Option<String>,
    json_data: Option<Value>,
    tax_liability: Option<i64>,
    itc_claimed: Option<i64>,
    net_tax_payable: Option<i64>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl GstReturnRow {
    pub(crate) fn into_model(self) -> GstReturn {
        GstReturn {
            gst_return_id: EntityId::from_uuid(self.gst_return_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            gst_registration_id: self.gst_registration_id,
            return_type: GstReturnType::from_db_str(&self.return_type),
            period: self.period,
            fiscal_year: self.fiscal_year,
            status: GstReturnStatus::from_db_str(&self.status),
            due_date: self.due_date,
            filed_date: self.filed_date,
            filed_by_id: self.filed_by_id,
            acknowledgment_no: self.acknowledgment_no,
            json_data: self.json_data,
            tax_liability: self.tax_liability.unwrap_or(0),
            itc_claimed: self.itc_claimed.unwrap_or(0),
            net_tax_payable: self.net_tax_payable.unwrap_or(0),
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct GstReturnLineRow {
    gst_return_line_id: Uuid,
    tenant_id: Uuid,
    gst_return_id: Uuid,
    section: String,
    description: Option<String>,
    taxable_value: Option<i64>,
    igst_amount: Option<i64>,
    cgst_amount: Option<i64>,
    sgst_amount: Option<i64>,
    cess_amount: Option<i64>,
    created_at: Option<DateTime<Utc>>,
}

impl GstReturnLineRow {
    pub(crate) fn into_model(self) -> GstReturnLine {
        GstReturnLine {
            gst_return_line_id: EntityId::from_uuid(self.gst_return_line_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            gst_return_id: self.gst_return_id,
            section: self.section,
            description: self.description,
            taxable_value: self.taxable_value.unwrap_or(0),
            igst_amount: self.igst_amount.unwrap_or(0),
            cgst_amount: self.cgst_amount.unwrap_or(0),
            sgst_amount: self.sgst_amount.unwrap_or(0),
            cess_amount: self.cess_amount.unwrap_or(0),
            audit: AuditInfo {
                created_by: Uuid::nil(),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: Uuid::nil(),
                updated_at: self.created_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct ItcRegisterRow {
    itc_register_id: Uuid,
    tenant_id: Uuid,
    gst_registration_id: Uuid,
    period: String,
    status: String,
    total_itc: Option<i64>,
    itc_on_inputs: Option<i64>,
    itc_on_capital_goods: Option<i64>,
    itc_reversal_rule_42: Option<i64>,
    itc_reversal_rule_43: Option<i64>,
    net_itc_eligible: Option<i64>,
    exempt_turnover: Option<i64>,
    total_turnover: Option<i64>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl ItcRegisterRow {
    pub(crate) fn into_model(self) -> ItcRegister {
        ItcRegister {
            itc_register_id: EntityId::from_uuid(self.itc_register_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            gst_registration_id: self.gst_registration_id,
            period: self.period,
            status: ItcRegisterStatus::from_db_str(&self.status),
            total_itc: self.total_itc.unwrap_or(0),
            itc_on_inputs: self.itc_on_inputs.unwrap_or(0),
            itc_on_capital_goods: self.itc_on_capital_goods.unwrap_or(0),
            itc_reversal_rule_42: self.itc_reversal_rule_42.unwrap_or(0),
            itc_reversal_rule_43: self.itc_reversal_rule_43.unwrap_or(0),
            net_itc_eligible: self.net_itc_eligible.unwrap_or(0),
            exempt_turnover: self.exempt_turnover.unwrap_or(0),
            total_turnover: self.total_turnover.unwrap_or(0),
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct ItcRegisterLineRow {
    itc_register_line_id: Uuid,
    tenant_id: Uuid,
    itc_register_id: Uuid,
    invoice_id: Uuid,
    invoice_number: String,
    invoice_date: NaiveDate,
    vendor_gstin: Option<String>,
    taxable_value: Option<i64>,
    igst: Option<i64>,
    cgst: Option<i64>,
    sgst: Option<i64>,
    total_tax: Option<i64>,
    itc_eligibility: String,
    acquisition_period: Option<String>,
    reversal_percent: Option<Decimal>,
    reversal_amount: Option<i64>,
    is_reversed: bool,
    created_at: Option<DateTime<Utc>>,
}

impl ItcRegisterLineRow {
    pub(crate) fn into_model(self) -> ItcRegisterLine {
        ItcRegisterLine {
            itc_register_line_id: EntityId::from_uuid(self.itc_register_line_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            itc_register_id: self.itc_register_id,
            invoice_id: self.invoice_id,
            invoice_number: self.invoice_number,
            invoice_date: self.invoice_date,
            vendor_gstin: self.vendor_gstin,
            taxable_value: self.taxable_value.unwrap_or(0),
            igst: self.igst.unwrap_or(0),
            cgst: self.cgst.unwrap_or(0),
            sgst: self.sgst.unwrap_or(0),
            total_tax: self.total_tax.unwrap_or(0),
            itc_eligibility: ItcEligibility::from_db_str(&self.itc_eligibility),
            acquisition_period: self.acquisition_period,
            reversal_percent: self.reversal_percent,
            reversal_amount: self.reversal_amount,
            is_reversed: self.is_reversed,
            audit: AuditInfo {
                created_by: Uuid::nil(),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: Uuid::nil(),
                updated_at: self.created_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct TdsSectionRow {
    tds_section_id: Uuid,
    tenant_id: Uuid,
    section_code: String,
    description: String,
    default_rate: Decimal,
    threshold_per_payment: Option<i64>,
    threshold_aggregate: Option<i64>,
    threshold_excess_only: bool,
    applicable_to: String,
    is_active: bool,
    effective_from: NaiveDate,
    effective_to: Option<NaiveDate>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

impl TdsSectionRow {
    pub(crate) fn into_model(self) -> TdsSection {
        TdsSection {
            tds_section_id: EntityId::from_uuid(self.tds_section_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            section_code: self.section_code,
            description: self.description,
            default_rate: self.default_rate,
            threshold_per_payment: self.threshold_per_payment,
            threshold_aggregate: self.threshold_aggregate,
            threshold_excess_only: self.threshold_excess_only,
            applicable_to: TdsSectionApplicableTo::from_db_str(&self.applicable_to),
            is_active: self.is_active,
            effective_from: self.effective_from,
            effective_to: self.effective_to,
            audit: AuditInfo {
                created_by: Uuid::nil(),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: Uuid::nil(),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

/// A TDS deduction row — owned by AP (`tds_deductions`), read by tax for
/// the deposit leg (`DepositTdsToGovt`), register and pending-deposits.
#[derive(sqlx::FromRow, Debug, Clone, serde::Serialize)]
pub struct TdsDeductionRow {
    pub tds_deduction_id: Uuid,
    pub tenant_id: Uuid,
    pub payment_id: Uuid,
    pub tds_section: String,
    pub tds_rate: Decimal,
    pub tds_amount: i64,
    pub pan_of_deductee: String,
    pub section_197_cert_id: Option<Uuid>,
    pub tds_deposit_status: String,
    pub tds_deposit_date: Option<NaiveDate>,
    pub tds_return_filed_date: Option<NaiveDate>,
    pub tds_journal_id: Option<Uuid>,
    pub challan_reference: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

/// One deduction covered by a TDS return, joined with its payment and
/// deposit-challan context (used by `GenerateTdsReturn` / `FileTdsReturn` /
/// Form 16 / Form 16A generation). `entity_id` comes from the vendor
/// payment, `deposit_date`/`challan_reference` from `tds_deposits`.
#[derive(sqlx::FromRow, Debug, Clone)]
pub(crate) struct TdsReturnDeductionRow {
    pub tds_deduction_id: Uuid,
    pub entity_id: Uuid,
    pub vendor_id: Option<Uuid>,
    pub pan: String,
    pub section: String,
    pub payment_date: NaiveDate,
    pub payment_amount: i64,
    pub tds_rate: Decimal,
    pub tds_amount: i64,
    pub challan_reference: Option<String>,
    pub deposit_date: Option<NaiveDate>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct TdsDepositRow {
    tds_deposit_id: Uuid,
    tenant_id: Uuid,
    tds_deduction_id: Uuid,
    challan_reference: String,
    deposit_date: NaiveDate,
    amount: Option<i64>,
    bank_account_id: Option<Uuid>,
    journal_id: Option<Uuid>,
    status: String,
    recorded_by_id: Uuid,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl TdsDepositRow {
    pub(crate) fn into_model(self) -> TdsDeposit {
        TdsDeposit {
            tds_deposit_id: EntityId::from_uuid(self.tds_deposit_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            tds_deduction_id: self.tds_deduction_id,
            challan_reference: self.challan_reference,
            deposit_date: self.deposit_date,
            amount: self.amount.unwrap_or(0),
            bank_account_id: self.bank_account_id,
            journal_id: self.journal_id,
            status: TdsDepositStatus::from_db_str(&self.status),
            recorded_by_id: self.recorded_by_id,
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

/// A deduction row inside a TDS return (mirrors `tds_return_details`).
#[derive(sqlx::FromRow)]
pub(crate) struct TdsReturnDetailRow {
    tds_return_detail_id: Uuid,
    tenant_id: Uuid,
    tds_return_id: Uuid,
    vendor_id: Option<Uuid>,
    employee_id: Option<Uuid>,
    pan: String,
    section: String,
    payment_date: NaiveDate,
    payment_amount: i64,
    tds_rate: Decimal,
    tds_amount: i64,
    surcharge: Option<i64>,
    cess: Option<i64>,
    total_tds: i64,
    challan_details: Option<Value>,
    salary_month: Option<i32>,
    created_at: Option<DateTime<Utc>>,
}

impl TdsReturnDetailRow {
    pub(crate) fn into_model(self) -> TdsReturnDetail {
        TdsReturnDetail {
            tds_return_detail_id: EntityId::from_uuid(self.tds_return_detail_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            tds_return_id: self.tds_return_id,
            vendor_id: self.vendor_id,
            employee_id: self.employee_id,
            pan: self.pan,
            section: self.section,
            payment_date: self.payment_date,
            payment_amount: self.payment_amount,
            tds_rate: self.tds_rate,
            tds_amount: self.tds_amount,
            surcharge: self.surcharge.unwrap_or(0),
            cess: self.cess.unwrap_or(0),
            total_tds: self.total_tds,
            challan_details: self.challan_details,
            salary_month: self.salary_month,
            audit: AuditInfo {
                created_by: Uuid::nil(),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: Uuid::nil(),
                updated_at: Utc::now(),
            },
        }
    }
}

/// A TDS return row (mirrors `tds_returns`).
#[derive(sqlx::FromRow)]
pub(crate) struct TdsReturnRow {
    tds_return_id: Uuid,
    tenant_id: Uuid,
    entity_id: Uuid,
    return_type: String,
    quarter: String,
    fiscal_year: String,
    status: String,
    due_date: NaiveDate,
    filed_date: Option<NaiveDate>,
    acknowledgment_no: Option<String>,
    total_deductions: Option<i64>,
    total_deposits: Option<i64>,
    json_data: Option<Value>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl TdsReturnRow {
    pub(crate) fn into_model(self) -> TdsReturn {
        TdsReturn {
            tds_return_id: EntityId::from_uuid(self.tds_return_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            entity_id: self.entity_id,
            return_type: TdsReturnType::from_db_str(&self.return_type),
            quarter: self.quarter,
            fiscal_year: self.fiscal_year,
            status: TdsReturnStatus::from_db_str(&self.status),
            due_date: self.due_date,
            filed_date: self.filed_date,
            acknowledgment_no: self.acknowledgment_no,
            total_deductions: self.total_deductions.unwrap_or(0),
            total_deposits: self.total_deposits.unwrap_or(0),
            json_data: self.json_data,
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

/// A Form 16 / Form 16A certificate row (mirrors `form16_certificates`).
#[derive(sqlx::FromRow)]
pub(crate) struct Form16CertificateRow {
    form16_certificate_id: Uuid,
    tenant_id: Uuid,
    entity_id: Uuid,
    certificate_type: String,
    fiscal_year: String,
    employee_id: Option<Uuid>,
    vendor_id: Option<Uuid>,
    pan: String,
    document_url: String,
    status: String,
    generated_by_id: Option<Uuid>,
    issued_at: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl Form16CertificateRow {
    pub(crate) fn into_model(self) -> Form16Certificate {
        Form16Certificate {
            form16_certificate_id: EntityId::from_uuid(self.form16_certificate_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            entity_id: self.entity_id,
            certificate_type: Form16CertificateType::from_db_str(&self.certificate_type),
            fiscal_year: self.fiscal_year,
            employee_id: self.employee_id,
            vendor_id: self.vendor_id,
            pan: self.pan,
            document_url: self.document_url,
            status: Form16CertificateStatus::from_db_str(&self.status),
            generated_by_id: self.generated_by_id,
            issued_at: self.issued_at,
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct TrustExemptionRow {
    trust_exemption_id: Uuid,
    tenant_id: Uuid,
    entity_id: Uuid,
    exemption_section: String,
    registration_no: String,
    registration_date: NaiveDate,
    valid_from: NaiveDate,
    valid_to: NaiveDate,
    status: String,
    approving_authority: Option<String>,
    is_trust: bool,
    trust_name: Option<String>,
    trust_pan: Option<String>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl TrustExemptionRow {
    pub(crate) fn into_model(self) -> TrustExemption {
        TrustExemption {
            trust_exemption_id: EntityId::from_uuid(self.trust_exemption_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            entity_id: self.entity_id,
            exemption_section: TrustExemptionSection::from_db_str(&self.exemption_section),
            registration_no: self.registration_no,
            registration_date: self.registration_date,
            valid_from: self.valid_from,
            valid_to: self.valid_to,
            status: TrustExemptionStatus::from_db_str(&self.status),
            approving_authority: self.approving_authority,
            is_trust: self.is_trust,
            trust_name: self.trust_name,
            trust_pan: self.trust_pan,
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct IncomeApplicationRow {
    income_application_id: Uuid,
    tenant_id: Uuid,
    fiscal_year_id: Uuid,
    entity_id: Uuid,
    total_income: Option<i64>,
    amount_applied: Option<i64>,
    application_percent: Option<Decimal>,
    accumulated_amount: Option<i64>,
    accumulation_year: Option<i32>,
    accumulation_purpose: Option<String>,
    status: String,
    last_computed_at: DateTime<Utc>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl IncomeApplicationRow {
    pub(crate) fn into_model(self) -> IncomeApplication {
        IncomeApplication {
            income_application_id: EntityId::from_uuid(self.income_application_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            fiscal_year_id: self.fiscal_year_id,
            entity_id: self.entity_id,
            total_income: self.total_income.unwrap_or(0),
            amount_applied: self.amount_applied.unwrap_or(0),
            application_percent: self.application_percent,
            accumulated_amount: self.accumulated_amount.unwrap_or(0),
            accumulation_year: self.accumulation_year,
            accumulation_purpose: self.accumulation_purpose,
            status: IncomeApplicationStatus::from_db_str(&self.status),
            last_computed_at: self.last_computed_at,
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct IncomeApplicationLineRow {
    income_app_line_id: Uuid,
    tenant_id: Uuid,
    income_application_id: Uuid,
    category: String,
    amount: Option<i64>,
    account_id: Uuid,
    description: Option<String>,
    created_at: Option<DateTime<Utc>>,
}

impl IncomeApplicationLineRow {
    pub(crate) fn into_model(self) -> IncomeApplicationLine {
        IncomeApplicationLine {
            income_app_line_id: EntityId::from_uuid(self.income_app_line_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            income_application_id: self.income_application_id,
            category: IncomeApplicationCategory::from_db_str(&self.category),
            amount: self.amount.unwrap_or(0),
            account_id: self.account_id,
            description: self.description,
            audit: AuditInfo {
                created_by: Uuid::nil(),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: Uuid::nil(),
                updated_at: self.created_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct FcraRegistrationRow {
    fcra_registration_id: Uuid,
    tenant_id: Uuid,
    entity_id: Uuid,
    registration_no: String,
    valid_from: NaiveDate,
    valid_to: NaiveDate,
    bank_account_id: Uuid,
    status: String,
    total_receipts: Option<i64>,
    admin_expenses: Option<i64>,
    admin_expense_ratio: Option<Decimal>,
    fc4_return_filed_date: Option<NaiveDate>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl FcraRegistrationRow {
    pub(crate) fn into_model(self) -> FcraRegistration {
        FcraRegistration {
            fcra_registration_id: EntityId::from_uuid(self.fcra_registration_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            entity_id: self.entity_id,
            registration_no: self.registration_no,
            valid_from: self.valid_from,
            valid_to: self.valid_to,
            bank_account_id: self.bank_account_id,
            status: FcraStatus::from_db_str(&self.status),
            total_receipts: self.total_receipts.unwrap_or(0),
            admin_expenses: self.admin_expenses.unwrap_or(0),
            admin_expense_ratio: self.admin_expense_ratio,
            fc4_return_filed_date: self.fc4_return_filed_date,
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

/// A posted vendor invoice aggregated with its GST split for the ITC
/// register (ComputeItc) and RCM (CreateRcmEntry).
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct VendorInvoiceTaxRow {
    pub vendor_invoice_id: Uuid,
    pub entity_id: Uuid,
    pub vendor_id: Uuid,
    pub invoice_number: String,
    pub invoice_date: NaiveDate,
    pub invoice_amount: i64,
    pub tax_amount: i64,
    pub net_amount: i64,
    pub is_rcm: bool,
    pub rcm_payable_amount: Option<i64>,
    pub vendor_gstin: Option<String>,
    pub status: String,
}

/// A single vendor invoice line with the GL account's ITC eligibility.
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct VendorInvoiceLineTaxRow {
    pub invoice_line_id: Uuid,
    pub vendor_invoice_id: Uuid,
    pub line_number: i32,
    pub total_amount: i64,
    pub tax_rate: Option<Decimal>,
    pub tax_amount: Option<i64>,
    pub account_id: Uuid,
    pub itc_eligibility: Option<String>,
}

/// Aggregated outward-supply row for GSTR-1/3B generation (from posted
/// journal lines whose account carries a GST classification).
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct GstJournalAggregateRow {
    pub gst_classification: String,
    pub tax_rate: Option<Decimal>,
    pub net_amount: i64,
    pub tax_amount: Option<i64>,
}

// ─── Repository ───────────────────────────────────────────────────────

/// Data access for the Tax Engine. All reads/writes are tenant-scoped.
pub struct TaxRepository {
    pool: PgPool,
}

impl TaxRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ── Policy ─────────────────────────────────────────────────────

    /// Resolve tax policy from `system_config` (tenant overrides GLOBAL).
    pub async fn load_policy(&self, tenant_id: Uuid) -> Result<TaxPolicy, TaxError> {
        let mut policy = TaxPolicy::default();
        let rows: Vec<(String, Value)> = sqlx::query_as(
            r#"
            SELECT config_key, config_value FROM system_config
            WHERE is_active = TRUE
              AND (tenant_id = $1 OR tenant_id IS NULL)
              AND config_key LIKE 'tax.%'
              AND (valid_to IS NULL OR valid_to > now())
            ORDER BY (tenant_id = $1) DESC, valid_from DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        for (key, value) in rows {
            let text = match &value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                other => other.to_string(),
            };
            match key.as_str() {
                "tax.itc_reversal_deminimis_paise" => {
                    policy.itc_reversal_deminimis_paise =
                        text.parse().unwrap_or(policy.itc_reversal_deminimis_paise);
                }
                "tax.income_application_threshold" => {
                    policy.income_application_threshold =
                        text.parse().unwrap_or(policy.income_application_threshold);
                }
                "tax.accumulation_years" => {
                    policy.accumulation_years = text.parse().unwrap_or(policy.accumulation_years);
                }
                "tax.fcra_admin_expense_ratio_limit" => {
                    policy.fcra_admin_expense_ratio_limit =
                        text.parse().unwrap_or(policy.fcra_admin_expense_ratio_limit);
                }
                "tax.tds_deposit_due_day" => {
                    policy.tds_deposit_due_day = text.parse().unwrap_or(policy.tds_deposit_due_day);
                }
                "tax.section115_securities_list" => {
                    policy.section115_securities_list = text
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                "tax.state_code_map" => {
                    policy.state_code_map = text
                        .split(',')
                        .filter_map(|pair| {
                            let (k, v) = pair.split_once(':')?;
                            Some((k.trim().to_string(), v.trim().to_string()))
                        })
                        .collect();
                }
                "tax.salary_tds_section" => {
                    let s = text.trim();
                    if !s.is_empty() {
                        policy.salary_tds_section = s.to_string();
                    }
                }
                _ => {}
            }
        }
        Ok(policy)
    }

    // ── GST registration / rate master ─────────────────────────────

    pub async fn find_gst_registration(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
    ) -> Result<Option<GstRegistration>, TaxError> {
        let row = sqlx::query_as::<_, GstRegistrationRow>(
            r#"
            SELECT gst_registration_id, tenant_id, entity_id, gstin, trade_name, legal_name,
                   registration_type, filing_frequency, is_composite, state_code,
                   address_line1, address_line2, city, state, pincode, is_active,
                   created_at, created_by, updated_at, updated_by
            FROM gst_registrations
            WHERE tenant_id = $1 AND gst_registration_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(registration_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(GstRegistrationRow::into_model))
    }

    pub async fn find_gst_rate(
        &self,
        tenant_id: Uuid,
        hsn_sac_code: &str,
        supply_type: GstSupplyType,
        as_of: NaiveDate,
    ) -> Result<Option<GstRate>, TaxError> {
        let row = sqlx::query_as::<_, GstRateRow>(
            r#"
            SELECT gst_rate_id, tenant_id, hsn_sac_code, description, rate, itc_eligible,
                   effective_from, effective_to, supply_type, is_active,
                   created_at, created_by, updated_at, updated_by
            FROM gst_rate_master
            WHERE tenant_id = $1 AND hsn_sac_code = $2 AND supply_type = $3
              AND is_active = TRUE AND deleted_at IS NULL
              AND effective_from <= $4 AND (effective_to IS NULL OR effective_to >= $4)
            ORDER BY effective_from DESC LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(hsn_sac_code)
        .bind(supply_type.to_db_str())
        .bind(as_of)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(GstRateRow::into_model))
    }

    /// List GST registrations, optionally scoped to one entity (spec
    /// `GetGstRegistration(s)` — list registrations by entity).
    pub async fn list_gst_registrations(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
    ) -> Result<Vec<GstRegistration>, TaxError> {
        let rows = match entity_id {
            Some(eid) => {
                sqlx::query_as::<_, GstRegistrationRow>(
                    r#"
                    SELECT gst_registration_id, tenant_id, entity_id, gstin, trade_name, legal_name,
                           registration_type, filing_frequency, is_composite, state_code,
                           address_line1, address_line2, city, state, pincode, is_active,
                           created_at, created_by, updated_at, updated_by
                    FROM gst_registrations
                    WHERE tenant_id = $1 AND entity_id = $2 AND deleted_at IS NULL
                    ORDER BY created_at
                    "#,
                )
                .bind(tenant_id)
                .bind(eid)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, GstRegistrationRow>(
                    r#"
                    SELECT gst_registration_id, tenant_id, entity_id, gstin, trade_name, legal_name,
                           registration_type, filing_frequency, is_composite, state_code,
                           address_line1, address_line2, city, state, pincode, is_active,
                           created_at, created_by, updated_at, updated_by
                    FROM gst_registrations
                    WHERE tenant_id = $1 AND deleted_at IS NULL
                    ORDER BY created_at
                    "#,
                )
                .bind(tenant_id)
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(rows.into_iter().map(GstRegistrationRow::into_model).collect())
    }

    /// Find any non-deleted GST registration of an entity (the base DDL
    /// constrains one registration per entity).
    pub async fn find_gst_registration_by_entity(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Option<GstRegistration>, TaxError> {
        let row = sqlx::query_as::<_, GstRegistrationRow>(
            r#"
            SELECT gst_registration_id, tenant_id, entity_id, gstin, trade_name, legal_name,
                   registration_type, filing_frequency, is_composite, state_code,
                   address_line1, address_line2, city, state, pincode, is_active,
                   created_at, created_by, updated_at, updated_by
            FROM gst_registrations
            WHERE tenant_id = $1 AND entity_id = $2 AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(GstRegistrationRow::into_model))
    }

    /// Insert a GST registration (spec `RegisterGstin`). Uniqueness is
    /// enforced by the DB (tenant_id, gstin) and (entity_id) constraints.
    pub async fn insert_gst_registration(&self, r: &GstRegistration) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO gst_registrations (
                gst_registration_id, tenant_id, entity_id, gstin, trade_name, legal_name,
                registration_type, filing_frequency, is_composite, state_code,
                address_line1, address_line2, city, state, pincode, is_active,
                created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
            "#,
        )
        .bind(r.gst_registration_id.as_uuid())
        .bind(r.tenant_id.as_uuid())
        .bind(r.entity_id)
        .bind(&r.gstin)
        .bind(&r.trade_name)
        .bind(&r.legal_name)
        .bind(r.registration_type.to_db_str())
        .bind(r.filing_frequency.to_db_str())
        .bind(r.is_composite)
        .bind(&r.state_code)
        .bind(&r.address_line1)
        .bind(&r.address_line2)
        .bind(&r.city)
        .bind(&r.state)
        .bind(&r.pincode)
        .bind(r.is_active)
        .bind(r.audit.created_by)
        .bind(r.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// All effective-dated rate-master rows for an HSN/SAC code (optionally
    /// filtered by supply type) — the spec `GetGstRates(hsn_sac)` master.
    pub async fn list_gst_rates(
        &self,
        tenant_id: Uuid,
        hsn_sac_code: &str,
        supply_type: Option<GstSupplyType>,
    ) -> Result<Vec<GstRate>, TaxError> {
        let rows = match supply_type {
            Some(st) => {
                sqlx::query_as::<_, GstRateRow>(
                    r#"
                    SELECT gst_rate_id, tenant_id, hsn_sac_code, description, rate, itc_eligible,
                           effective_from, effective_to, supply_type, is_active,
                           created_at, created_by, updated_at, updated_by
                    FROM gst_rate_master
                    WHERE tenant_id = $1 AND hsn_sac_code = $2 AND supply_type = $3
                      AND deleted_at IS NULL
                    ORDER BY effective_from
                    "#,
                )
                .bind(tenant_id)
                .bind(hsn_sac_code)
                .bind(st.to_db_str())
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, GstRateRow>(
                    r#"
                    SELECT gst_rate_id, tenant_id, hsn_sac_code, description, rate, itc_eligible,
                           effective_from, effective_to, supply_type, is_active,
                           created_at, created_by, updated_at, updated_by
                    FROM gst_rate_master
                    WHERE tenant_id = $1 AND hsn_sac_code = $2 AND deleted_at IS NULL
                    ORDER BY effective_from
                    "#,
                )
                .bind(tenant_id)
                .bind(hsn_sac_code)
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(rows.into_iter().map(GstRateRow::into_model).collect())
    }

    /// Insert a rate-master row (spec `UpsertGstRate`). The DB rejects
    /// overlapping effective ranges per (tenant, hsn_sac, supply_type) via
    /// the EXCLUDE constraint (migration 004) — callers pre-check overlaps
    /// and map the unique violation here.
    pub async fn insert_gst_rate(&self, r: &GstRate) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO gst_rate_master (
                gst_rate_id, tenant_id, hsn_sac_code, description, rate, itc_eligible,
                effective_from, effective_to, supply_type, is_active, created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(r.gst_rate_id.as_uuid())
        .bind(r.tenant_id.as_uuid())
        .bind(&r.hsn_sac_code)
        .bind(&r.description)
        .bind(r.rate)
        .bind(r.itc_eligible)
        .bind(r.effective_from)
        .bind(r.effective_to)
        .bind(r.supply_type.to_db_str())
        .bind(r.is_active)
        .bind(r.audit.created_by)
        .bind(r.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update an existing rate-master row in place (same effective_from —
    /// the PUT/upsert path of `UpsertGstRate`).
    pub async fn update_gst_rate(
        &self,
        tenant_id: Uuid,
        gst_rate_id: Uuid,
        description: Option<&str>,
        rate: i64,
        itc_eligible: bool,
        effective_to: Option<NaiveDate>,
        is_active: bool,
        updated_by: Uuid,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE gst_rate_master
            SET description = $3, rate = $4, itc_eligible = $5, effective_to = $6,
                is_active = $7, updated_at = now(), updated_by = $8, version = version + 1
            WHERE tenant_id = $1 AND gst_rate_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(gst_rate_id)
        .bind(description)
        .bind(rate)
        .bind(itc_eligible)
        .bind(effective_to)
        .bind(is_active)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── GST returns ─────────────────────────────────────────────────

    pub async fn find_gst_return(
        &self,
        tenant_id: Uuid,
        return_id: Uuid,
    ) -> Result<Option<GstReturn>, TaxError> {
        let row = sqlx::query_as::<_, GstReturnRow>(
            r#"
            SELECT gst_return_id, tenant_id, gst_registration_id, return_type, period,
                   fiscal_year, status, due_date, filed_date, filed_by_id, acknowledgment_no,
                   json_data, tax_liability, itc_claimed, net_tax_payable,
                   created_at, created_by, updated_at, updated_by
            FROM gst_returns
            WHERE tenant_id = $1 AND gst_return_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(return_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(GstReturnRow::into_model))
    }

    pub async fn find_gst_return_for_period(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
        return_type: GstReturnType,
        period: &str,
    ) -> Result<Option<GstReturn>, TaxError> {
        let row = sqlx::query_as::<_, GstReturnRow>(
            r#"
            SELECT gst_return_id, tenant_id, gst_registration_id, return_type, period,
                   fiscal_year, status, due_date, filed_date, filed_by_id, acknowledgment_no,
                   json_data, tax_liability, itc_claimed, net_tax_payable,
                   created_at, created_by, updated_at, updated_by
            FROM gst_returns
            WHERE tenant_id = $1 AND gst_registration_id = $2 AND return_type = $3 AND period = $4
              AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(registration_id)
        .bind(return_type.to_db_str())
        .bind(period)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(GstReturnRow::into_model))
    }

    pub async fn list_gst_returns(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
        fiscal_year: Option<&str>,
    ) -> Result<Vec<GstReturn>, TaxError> {
        let rows = match fiscal_year {
            Some(fy) => {
                sqlx::query_as::<_, GstReturnRow>(
                    r#"
                    SELECT gst_return_id, tenant_id, gst_registration_id, return_type, period,
                           fiscal_year, status, due_date, filed_date, filed_by_id, acknowledgment_no,
                           json_data, tax_liability, itc_claimed, net_tax_payable,
                           created_at, created_by, updated_at, updated_by
                    FROM gst_returns
                    WHERE tenant_id = $1 AND gst_registration_id = $2 AND fiscal_year = $3
                      AND deleted_at IS NULL
                    ORDER BY period DESC
                    "#,
                )
                .bind(tenant_id)
                .bind(registration_id)
                .bind(fy)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, GstReturnRow>(
                    r#"
                    SELECT gst_return_id, tenant_id, gst_registration_id, return_type, period,
                           fiscal_year, status, due_date, filed_date, filed_by_id, acknowledgment_no,
                           json_data, tax_liability, itc_claimed, net_tax_payable,
                           created_at, created_by, updated_at, updated_by
                    FROM gst_returns
                    WHERE tenant_id = $1 AND gst_registration_id = $2 AND deleted_at IS NULL
                    ORDER BY period DESC
                    "#,
                )
                .bind(tenant_id)
                .bind(registration_id)
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(rows.into_iter().map(GstReturnRow::into_model).collect())
    }

    pub async fn insert_gst_return(
        &self,
        r: &GstReturn,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO gst_returns (
                gst_return_id, tenant_id, gst_registration_id, return_type, period, fiscal_year,
                status, due_date, filed_date, filed_by_id, acknowledgment_no, json_data,
                tax_liability, itc_claimed, net_tax_payable, created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)
            "#,
        )
        .bind(r.gst_return_id.as_uuid())
        .bind(r.tenant_id.as_uuid())
        .bind(r.gst_registration_id)
        .bind(r.return_type.to_db_str())
        .bind(&r.period)
        .bind(&r.fiscal_year)
        .bind(r.status.to_db_str())
        .bind(r.due_date)
        .bind(r.filed_date)
        .bind(r.filed_by_id)
        .bind(&r.acknowledgment_no)
        .bind(&r.json_data)
        .bind(r.tax_liability)
        .bind(r.itc_claimed)
        .bind(r.net_tax_payable)
        .bind(r.audit.created_by)
        .bind(r.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Record a filing: GENERATED → FILED (state transition owned by the
    /// command handler; this only persists the transition).
    pub async fn record_gst_filing(
        &self,
        tenant_id: Uuid,
        return_id: Uuid,
        acknowledgment_no: &str,
        filed_date: NaiveDate,
        filed_by: Uuid,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE gst_returns
            SET status = 'FILED', acknowledgment_no = $3, filed_date = $4, filed_by_id = $5,
                updated_at = now(), updated_by = $5, entity_version = entity_version + 1
            WHERE tenant_id = $1 AND gst_return_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(return_id)
        .bind(acknowledgment_no)
        .bind(filed_date)
        .bind(filed_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_gst_return_line(
        &self,
        line: &GstReturnLine,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO gst_return_lines (
                gst_return_line_id, tenant_id, gst_return_id, section, description,
                taxable_value, igst_amount, cgst_amount, sgst_amount, cess_amount
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(line.gst_return_line_id.as_uuid())
        .bind(line.tenant_id.as_uuid())
        .bind(line.gst_return_id)
        .bind(&line.section)
        .bind(&line.description)
        .bind(line.taxable_value)
        .bind(line.igst_amount)
        .bind(line.cgst_amount)
        .bind(line.sgst_amount)
        .bind(line.cess_amount)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_gst_return_lines(
        &self,
        tenant_id: Uuid,
        return_id: Uuid,
    ) -> Result<Vec<GstReturnLine>, TaxError> {
        let rows = sqlx::query_as::<_, GstReturnLineRow>(
            r#"
            SELECT gst_return_line_id, tenant_id, gst_return_id, section, description,
                   taxable_value, igst_amount, cgst_amount, sgst_amount, cess_amount, created_at
            FROM gst_return_lines
            WHERE tenant_id = $1 AND gst_return_id = $2
            ORDER BY section
            "#,
        )
        .bind(tenant_id)
        .bind(return_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(GstReturnLineRow::into_model).collect())
    }

    /// Aggregate outward taxable supplies for GSTR-1/3B: posted journal
    /// lines on accounts carrying a GST classification, grouped by
    /// classification and rate. `net_amount` = credit − debit (supplies
    /// are credit-side); `tax_amount` = the line's GST.
    pub async fn gst_journal_aggregates(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<GstJournalAggregateRow>, TaxError> {
        let rows = sqlx::query_as::<_, GstJournalAggregateRow>(
            r#"
            SELECT a.gst_classification AS gst_classification,
                   l.tax_rate AS tax_rate,
                   (COALESCE(l.credit_amount, 0) - COALESCE(l.debit_amount, 0)) AS net_amount,
                   l.tax_amount AS tax_amount
            FROM journal_entry_lines l
            JOIN journal_entries je ON je.journal_id = l.journal_id
            JOIN chart_of_accounts a ON a.account_id = l.account_id
            WHERE je.status = 'POSTED'
              AND je.tenant_id = $1
              AND je.entity_id = $2
              AND je.posting_date BETWEEN $3 AND $4
              AND a.tenant_id = $1
              AND a.gst_classification IS NOT NULL
              AND a.deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ── ITC register ────────────────────────────────────────────────

    pub async fn find_itc_register(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
        period: &str,
    ) -> Result<Option<ItcRegister>, TaxError> {
        let row = sqlx::query_as::<_, ItcRegisterRow>(
            r#"
            SELECT itc_register_id, tenant_id, gst_registration_id, period, status,
                   total_itc, itc_on_inputs, itc_on_capital_goods, itc_reversal_rule_42,
                   itc_reversal_rule_43, net_itc_eligible, exempt_turnover, total_turnover,
                   created_at, created_by, updated_at, updated_by
            FROM itc_register
            WHERE tenant_id = $1 AND gst_registration_id = $2 AND period = $3
            "#,
        )
        .bind(tenant_id)
        .bind(registration_id)
        .bind(period)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(ItcRegisterRow::into_model))
    }

    pub async fn find_itc_register_by_id(
        &self,
        tenant_id: Uuid,
        register_id: Uuid,
    ) -> Result<Option<ItcRegister>, TaxError> {
        let row = sqlx::query_as::<_, ItcRegisterRow>(
            r#"
            SELECT itc_register_id, tenant_id, gst_registration_id, period, status,
                   total_itc, itc_on_inputs, itc_on_capital_goods, itc_reversal_rule_42,
                   itc_reversal_rule_43, net_itc_eligible, exempt_turnover, total_turnover,
                   created_at, created_by, updated_at, updated_by
            FROM itc_register
            WHERE tenant_id = $1 AND itc_register_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(register_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(ItcRegisterRow::into_model))
    }

    pub async fn list_itc_registers(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
    ) -> Result<Vec<ItcRegister>, TaxError> {
        let rows = sqlx::query_as::<_, ItcRegisterRow>(
            r#"
            SELECT itc_register_id, tenant_id, gst_registration_id, period, status,
                   total_itc, itc_on_inputs, itc_on_capital_goods, itc_reversal_rule_42,
                   itc_reversal_rule_43, net_itc_eligible, exempt_turnover, total_turnover,
                   created_at, created_by, updated_at, updated_by
            FROM itc_register
            WHERE tenant_id = $1 AND gst_registration_id = $2
            ORDER BY period DESC
            "#,
        )
        .bind(tenant_id)
        .bind(registration_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(ItcRegisterRow::into_model).collect())
    }

    pub async fn insert_itc_register(&self, r: &ItcRegister) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO itc_register (
                itc_register_id, tenant_id, gst_registration_id, period, status,
                total_itc, itc_on_inputs, itc_on_capital_goods, itc_reversal_rule_42,
                itc_reversal_rule_43, net_itc_eligible, exempt_turnover, total_turnover,
                created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            "#,
        )
        .bind(r.itc_register_id.as_uuid())
        .bind(r.tenant_id.as_uuid())
        .bind(r.gst_registration_id)
        .bind(&r.period)
        .bind(r.status.to_db_str())
        .bind(r.total_itc)
        .bind(r.itc_on_inputs)
        .bind(r.itc_on_capital_goods)
        .bind(r.itc_reversal_rule_42)
        .bind(r.itc_reversal_rule_43)
        .bind(r.net_itc_eligible)
        .bind(r.exempt_turnover)
        .bind(r.total_turnover)
        .bind(r.audit.created_by)
        .bind(r.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update reversal/status fields of an ITC register.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_itc_register_totals(
        &self,
        tenant_id: Uuid,
        register_id: Uuid,
        rule42: i64,
        rule43: i64,
        net_itc_eligible: i64,
        exempt_turnover: i64,
        total_turnover: i64,
        status: ItcRegisterStatus,
        updated_by: Uuid,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE itc_register
            SET itc_reversal_rule_42 = $3, itc_reversal_rule_43 = $4, net_itc_eligible = $5,
                exempt_turnover = $6, total_turnover = $7, status = $8,
                updated_at = now(), updated_by = $9, entity_version = entity_version + 1
            WHERE tenant_id = $1 AND itc_register_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(register_id)
        .bind(rule42)
        .bind(rule43)
        .bind(net_itc_eligible)
        .bind(exempt_turnover)
        .bind(total_turnover)
        .bind(status.to_db_str())
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_itc_register_reversed(
        &self,
        tenant_id: Uuid,
        register_id: Uuid,
        updated_by: Uuid,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE itc_register
            SET status = 'REVERSED', updated_at = now(), updated_by = $3,
                entity_version = entity_version + 1
            WHERE tenant_id = $1 AND itc_register_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(register_id)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_itc_register_line(
        &self,
        tenant_id: Uuid,
        line_id: Uuid,
    ) -> Result<Option<ItcRegisterLine>, TaxError> {
        let row = sqlx::query_as::<_, ItcRegisterLineRow>(
            r#"
            SELECT itc_register_line_id, tenant_id, itc_register_id, invoice_id, invoice_number,
                   invoice_date, vendor_gstin, taxable_value, igst, cgst, sgst, total_tax,
                   itc_eligibility, acquisition_period, reversal_percent, reversal_amount, is_reversed, created_at
            FROM itc_register_lines
            WHERE tenant_id = $1 AND itc_register_line_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(line_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(ItcRegisterLineRow::into_model))
    }

    pub async fn list_itc_register_lines(
        &self,
        tenant_id: Uuid,
        register_id: Uuid,
    ) -> Result<Vec<ItcRegisterLine>, TaxError> {
        let rows = sqlx::query_as::<_, ItcRegisterLineRow>(
            r#"
            SELECT itc_register_line_id, tenant_id, itc_register_id, invoice_id, invoice_number,
                   invoice_date, vendor_gstin, taxable_value, igst, cgst, sgst, total_tax,
                   itc_eligibility, acquisition_period, reversal_percent, reversal_amount, is_reversed, created_at
            FROM itc_register_lines
            WHERE tenant_id = $1 AND itc_register_id = $2
            ORDER BY invoice_date, invoice_number
            "#,
        )
        .bind(tenant_id)
        .bind(register_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(ItcRegisterLineRow::into_model).collect())
    }

    pub async fn insert_itc_register_line(&self, l: &ItcRegisterLine) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO itc_register_lines (
                itc_register_line_id, tenant_id, itc_register_id, invoice_id, invoice_number,
                invoice_date, vendor_gstin, taxable_value, igst, cgst, sgst, total_tax,
                itc_eligibility, acquisition_period, reversal_percent, reversal_amount, is_reversed
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)
            "#,
        )
        .bind(l.itc_register_line_id.as_uuid())
        .bind(l.tenant_id.as_uuid())
        .bind(l.itc_register_id)
        .bind(l.invoice_id)
        .bind(&l.invoice_number)
        .bind(l.invoice_date)
        .bind(&l.vendor_gstin)
        .bind(l.taxable_value)
        .bind(l.igst)
        .bind(l.cgst)
        .bind(l.sgst)
        .bind(l.total_tax)
        .bind(l.itc_eligibility.to_db_str())
        .bind(&l.acquisition_period)
        .bind(l.reversal_percent)
        .bind(l.reversal_amount)
        .bind(l.is_reversed)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_itc_register_line_reversal(
        &self,
        tenant_id: Uuid,
        line_id: Uuid,
        reversal_percent: Option<Decimal>,
        reversal_amount: Option<i64>,
        is_reversed: bool,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE itc_register_lines
            SET reversal_percent = $3, reversal_amount = $4, is_reversed = $5
            WHERE tenant_id = $1 AND itc_register_line_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(line_id)
        .bind(reversal_percent)
        .bind(reversal_amount)
        .bind(is_reversed)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Posted vendor invoices for an entity in a date range (ITC register
    /// source). `is_rcm` invoices carry the RCM flag for the RCM leg.
    pub async fn posted_vendor_invoices(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<VendorInvoiceTaxRow>, TaxError> {
        let rows = sqlx::query_as::<_, VendorInvoiceTaxRow>(
            r#"
            SELECT vi.vendor_invoice_id, vi.entity_id, vi.vendor_id, vi.invoice_number,
                   vi.invoice_date, vi.invoice_amount, vi.tax_amount, vi.net_amount,
                   vi.is_rcm, vi.rcm_payable_amount, v.gstin AS vendor_gstin, vi.status
            FROM vendor_invoices vi
            LEFT JOIN vendors v ON v.vendor_id = vi.vendor_id AND v.tenant_id = vi.tenant_id
            WHERE vi.tenant_id = $1 AND vi.entity_id = $2
              AND vi.status = 'POSTED'
              AND vi.invoice_date BETWEEN $3 AND $4
              AND vi.deleted_at IS NULL
            ORDER BY vi.invoice_date, vi.invoice_number
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Vendor invoice (any status) by id — used by CreateRcmEntry.
    pub async fn vendor_invoice_by_id(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
    ) -> Result<Option<VendorInvoiceTaxRow>, TaxError> {
        let row = sqlx::query_as::<_, VendorInvoiceTaxRow>(
            r#"
            SELECT vi.vendor_invoice_id, vi.entity_id, vi.vendor_id, vi.invoice_number,
                   vi.invoice_date, vi.invoice_amount, vi.tax_amount, vi.net_amount,
                   vi.is_rcm, vi.rcm_payable_amount, v.gstin AS vendor_gstin, vi.status
            FROM vendor_invoices vi
            LEFT JOIN vendors v ON v.vendor_id = vi.vendor_id AND v.tenant_id = vi.tenant_id
            WHERE vi.tenant_id = $1 AND vi.vendor_invoice_id = $2 AND vi.deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Lines of an invoice joined with the GL account's ITC eligibility.
    pub async fn vendor_invoice_lines(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
    ) -> Result<Vec<VendorInvoiceLineTaxRow>, TaxError> {
        let rows = sqlx::query_as::<_, VendorInvoiceLineTaxRow>(
            r#"
            SELECT vil.invoice_line_id, vil.vendor_invoice_id, vil.line_number,
                   vil.total_amount, vil.tax_rate, vil.tax_amount, vil.account_id,
                   a.itc_eligibility AS itc_eligibility
            FROM vendor_invoice_lines vil
            LEFT JOIN chart_of_accounts a ON a.account_id = vil.account_id AND a.tenant_id = vil.tenant_id
            WHERE vil.tenant_id = $1 AND vil.vendor_invoice_id = $2
            ORDER BY vil.line_number
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// The first expense account of an invoice (used as the DR side of the
    /// no-ITC RCM entry and the ITC-reversal expense leg).
    pub async fn first_invoice_expense_account(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
    ) -> Result<Option<Uuid>, TaxError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT vil.account_id
            FROM vendor_invoice_lines vil
            WHERE vil.tenant_id = $1 AND vil.vendor_invoice_id = $2
            ORDER BY vil.line_number LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    /// Idempotency probe for RCM: has a journal already been posted for
    /// this invoice (reference_type = 'RCM')?
    pub async fn rcm_journal_exists(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
    ) -> Result<bool, TaxError> {
        let row: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT 1
            FROM journal_entry_lines l
            JOIN journal_entries je ON je.journal_id = l.journal_id
            WHERE je.status = 'POSTED'
              AND l.tenant_id = $1
              AND l.reference_type = 'RCM'
              AND l.reference_id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }

    // ── TDS ─────────────────────────────────────────────────────────

    /// Effective-dated TDS section lookup — tenant row preferred, GLOBAL
    /// (nil-tenant) row as fallback (mirrors the 004_tax.sql migration).
    pub async fn find_tds_section(
        &self,
        tenant_id: Uuid,
        section_code: &str,
        as_of: NaiveDate,
    ) -> Result<Option<TdsSection>, TaxError> {
        let row = sqlx::query_as::<_, TdsSectionRow>(
            r#"
            SELECT tds_section_id, tenant_id, section_code, description, default_rate,
                   threshold_per_payment, threshold_aggregate, threshold_excess_only,
                   applicable_to, is_active,
                   effective_from, effective_to, created_at, updated_at
            FROM tds_sections
            WHERE tenant_id IN ($1, $2)
              AND section_code = $3
              AND is_active = TRUE
              AND effective_from <= $4 AND (effective_to IS NULL OR effective_to >= $4)
            ORDER BY (tenant_id = $1) DESC, effective_from DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(Uuid::parse_str(GLOBAL_TENANT).expect("static GLOBAL_TENANT uuid"))
        .bind(section_code)
        .bind(as_of)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(TdsSectionRow::into_model))
    }

    pub async fn list_tds_sections(
        &self,
        tenant_id: Uuid,
        as_of: NaiveDate,
    ) -> Result<Vec<TdsSection>, TaxError> {
        let rows = sqlx::query_as::<_, TdsSectionRow>(
            r#"
            SELECT tds_section_id, tenant_id, section_code, description, default_rate,
                   threshold_per_payment, threshold_aggregate, threshold_excess_only,
                   applicable_to, is_active,
                   effective_from, effective_to, created_at, updated_at
            FROM tds_sections
            WHERE tenant_id IN ($1, $2)
              AND is_active = TRUE
              AND effective_from <= $3 AND (effective_to IS NULL OR effective_to >= $3)
            ORDER BY (tenant_id = $1) DESC, section_code, effective_from DESC
            "#,
        )
        .bind(tenant_id)
        .bind(Uuid::parse_str(GLOBAL_TENANT).expect("static GLOBAL_TENANT uuid"))
        .bind(as_of)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(TdsSectionRow::into_model).collect())
    }

    /// Insert a tenant-specific TDS section configuration row
    /// (spec `ConfigureTdsSection` — effective-dated per the model).
    pub async fn insert_tds_section(&self, s: &TdsSection) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO tds_sections (
                tds_section_id, tenant_id, section_code, description, default_rate,
                threshold_per_payment, threshold_aggregate, applicable_to, is_active,
                effective_from, effective_to, threshold_excess_only, created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            "#,
        )
        .bind(s.tds_section_id.as_uuid())
        .bind(s.tenant_id.as_uuid())
        .bind(&s.section_code)
        .bind(&s.description)
        .bind(s.default_rate)
        .bind(s.threshold_per_payment)
        .bind(s.threshold_aggregate)
        .bind(s.applicable_to.to_db_str())
        .bind(s.is_active)
        .bind(s.effective_from)
        .bind(s.effective_to)
        .bind(s.threshold_excess_only)
        .bind(s.audit.created_by)
        .bind(s.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update the currently-effective TDS section row in place
    /// (PUT-style `ConfigureTdsSection` on a tenant row).
    pub async fn update_tds_section(
        &self,
        tenant_id: Uuid,
        tds_section_id: Uuid,
        default_rate: Decimal,
        threshold_per_payment: Option<i64>,
        threshold_aggregate: Option<i64>,
        threshold_excess_only: bool,
        applicable_to: TdsSectionApplicableTo,
        updated_by: Uuid,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE tds_sections
            SET default_rate = $3, threshold_per_payment = $4, threshold_aggregate = $5,
                threshold_excess_only = $6, applicable_to = $7,
                updated_at = now(), updated_by = $8, entity_version = entity_version + 1
            WHERE tenant_id = $1 AND tds_section_id = $2 AND is_active = TRUE
            "#,
        )
        .bind(tenant_id)
        .bind(tds_section_id)
        .bind(default_rate)
        .bind(threshold_per_payment)
        .bind(threshold_aggregate)
        .bind(threshold_excess_only)
        .bind(applicable_to.to_db_str())
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// A valid Section 197 certificate for the vendor/section as of a date
    /// (lower/nil-deduction override — IT Act s.197).
    pub async fn find_section197_certificate(
        &self,
        tenant_id: Uuid,
        vendor_id: Uuid,
        section_code: &str,
        as_of: NaiveDate,
    ) -> Result<Option<Decimal>, TaxError> {
        let row: Option<(Decimal,)> = sqlx::query_as(
            r#"
            SELECT specified_rate
            FROM section_197_certificates
            WHERE tenant_id = $1 AND vendor_id = $2 AND section = $3
              AND is_active = TRUE AND deleted_at IS NULL
              AND valid_from <= $4 AND valid_to >= $4
            ORDER BY valid_from DESC LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(vendor_id)
        .bind(section_code)
        .bind(as_of)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn find_tds_deduction(
        &self,
        tenant_id: Uuid,
        deduction_id: Uuid,
    ) -> Result<Option<TdsDeductionRow>, TaxError> {
        let row = sqlx::query_as::<_, TdsDeductionRow>(
            r#"
            SELECT tds_deduction_id, tenant_id, payment_id, tds_section, tds_rate, tds_amount,
                   pan_of_deductee, section_197_cert_id, tds_deposit_status, tds_deposit_date,
                   tds_return_filed_date, tds_journal_id, challan_reference, created_at
            FROM tds_deductions
            WHERE tenant_id = $1 AND tds_deduction_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(deduction_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_tds_deductions(
        &self,
        tenant_id: Uuid,
        deduction_ids: &[Uuid],
    ) -> Result<Vec<TdsDeductionRow>, TaxError> {
        if deduction_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows = sqlx::query_as::<_, TdsDeductionRow>(
            r#"
            SELECT tds_deduction_id, tenant_id, payment_id, tds_section, tds_rate, tds_amount,
                   pan_of_deductee, section_197_cert_id, tds_deposit_status, tds_deposit_date,
                   tds_return_filed_date, tds_journal_id, challan_reference, created_at
            FROM tds_deductions
            WHERE tenant_id = $1 AND tds_deduction_id = ANY($2)
            "#,
        )
        .bind(tenant_id)
        .bind(deduction_ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Pending TDS deposits (deposit_status = PENDING) for an entity —
    /// joins vendor_payments for the entity scoping.
    pub async fn pending_tds_deposits(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
    ) -> Result<Vec<TdsDeductionRow>, TaxError> {
        let rows = match entity_id {
            Some(eid) => {
                sqlx::query_as::<_, TdsDeductionRow>(
                    r#"
                    SELECT td.tds_deduction_id, td.tenant_id, td.payment_id, td.tds_section,
                           td.tds_rate, td.tds_amount, td.pan_of_deductee,
                           td.section_197_cert_id, td.tds_deposit_status, td.tds_deposit_date,
                           td.tds_return_filed_date, td.tds_journal_id, td.challan_reference,
                           td.created_at
                    FROM tds_deductions td
                    JOIN vendor_payments vp ON vp.payment_id = td.payment_id AND vp.tenant_id = td.tenant_id
                    WHERE td.tenant_id = $1 AND td.tds_deposit_status = 'PENDING'
                      AND vp.entity_id = $2
                    ORDER BY td.created_at
                    "#,
                )
                .bind(tenant_id)
                .bind(eid)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, TdsDeductionRow>(
                    r#"
                    SELECT tds_deduction_id, tenant_id, payment_id, tds_section, tds_rate,
                           tds_amount, pan_of_deductee, section_197_cert_id, tds_deposit_status,
                           tds_deposit_date, tds_return_filed_date, tds_journal_id,
                           challan_reference, created_at
                    FROM tds_deductions
                    WHERE tenant_id = $1 AND tds_deposit_status = 'PENDING'
                    ORDER BY created_at
                    "#,
                )
                .bind(tenant_id)
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(rows)
    }

    /// TDS register — GL balance of the TDS Payable accounts (24.03) by
    /// GL account within a period. `balance = credits − debits` because a
    /// deduction credits TDS Payable and a deposit debits it.
    pub async fn tds_register_balances(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<TdsRegisterBalanceRow>, TaxError> {
        self.tds_register_balances_for_prefix(tenant_id, entity_id, from, to, "24.03")
            .await
    }

    /// GL balances of accounts whose code starts with `prefix` (24.03 TDS
    /// Payable, 24.02 RCM Payable) within a period.
    pub async fn tds_register_balances_for_prefix(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
        from: NaiveDate,
        to: NaiveDate,
        prefix: &str,
    ) -> Result<Vec<TdsRegisterBalanceRow>, TaxError> {
        let rows = match entity_id {
            Some(eid) => {
                sqlx::query_as::<_, TdsRegisterBalanceRow>(
                    r#"
                    SELECT a.account_id, a.account_code, a.account_name,
                           SUM(COALESCE(l.credit_amount, 0)) - SUM(COALESCE(l.debit_amount, 0)) AS balance_paise
                    FROM journal_entry_lines l
                    JOIN journal_entries je ON je.journal_id = l.journal_id
                    JOIN chart_of_accounts a ON a.account_id = l.account_id
                    WHERE je.status = 'POSTED'
                      AND je.tenant_id = $1
                      AND je.entity_id = $2
                      AND je.posting_date BETWEEN $3 AND $4
                      AND a.tenant_id = $1
                      AND a.account_code LIKE $5
                      AND a.deleted_at IS NULL
                    GROUP BY a.account_id, a.account_code, a.account_name
                    ORDER BY a.account_code
                    "#,
                )
                .bind(tenant_id)
                .bind(eid)
                .bind(from)
                .bind(to)
                .bind(format!("{prefix}%"))
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, TdsRegisterBalanceRow>(
                    r#"
                    SELECT a.account_id, a.account_code, a.account_name,
                           SUM(COALESCE(l.credit_amount, 0)) - SUM(COALESCE(l.debit_amount, 0)) AS balance_paise
                    FROM journal_entry_lines l
                    JOIN journal_entries je ON je.journal_id = l.journal_id
                    JOIN chart_of_accounts a ON a.account_id = l.account_id
                    WHERE je.status = 'POSTED'
                      AND je.tenant_id = $1
                      AND je.posting_date BETWEEN $2 AND $3
                      AND a.tenant_id = $1
                      AND a.account_code LIKE $4
                      AND a.deleted_at IS NULL
                    GROUP BY a.account_id, a.account_code, a.account_name
                    ORDER BY a.account_code
                    "#,
                )
                .bind(tenant_id)
                .bind(from)
                .bind(to)
                .bind(format!("{prefix}%"))
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(rows)
    }

    pub async fn insert_tds_deposit(&self, d: &TdsDeposit) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO tds_deposits (
                tds_deposit_id, tenant_id, tds_deduction_id, challan_reference, deposit_date,
                amount, bank_account_id, journal_id, status, recorded_by_id, created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(d.tds_deposit_id.as_uuid())
        .bind(d.tenant_id.as_uuid())
        .bind(d.tds_deduction_id)
        .bind(&d.challan_reference)
        .bind(d.deposit_date)
        .bind(d.amount)
        .bind(d.bank_account_id)
        .bind(d.journal_id)
        .bind(d.status.to_db_str())
        .bind(d.recorded_by_id)
        .bind(d.audit.created_by)
        .bind(d.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Transition a deduction's deposit leg PENDING → DEPOSITED.
    pub async fn mark_deduction_deposited(
        &self,
        tenant_id: Uuid,
        deduction_id: Uuid,
        deposit_date: NaiveDate,
        challan_reference: &str,
        journal_id: Uuid,
        updated_by: Uuid,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE tds_deductions
            SET tds_deposit_status = 'DEPOSITED', tds_deposit_date = $3,
                challan_reference = $4, tds_journal_id = $5,
                updated_at = now(), updated_by = $6, entity_version = entity_version + 1
            WHERE tenant_id = $1 AND tds_deduction_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(deduction_id)
        .bind(deposit_date)
        .bind(challan_reference)
        .bind(journal_id)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_tds_deposits(
        &self,
        tenant_id: Uuid,
        deduction_id: Uuid,
    ) -> Result<Vec<TdsDeposit>, TaxError> {
        let rows = sqlx::query_as::<_, TdsDepositRow>(
            r#"
            SELECT tds_deposit_id, tenant_id, tds_deduction_id, challan_reference, deposit_date,
                   amount, bank_account_id, journal_id, status, recorded_by_id,
                   created_at, created_by, updated_at, updated_by
            FROM tds_deposits
            WHERE tenant_id = $1 AND tds_deduction_id = $2
            ORDER BY deposit_date
            "#,
        )
        .bind(tenant_id)
        .bind(deduction_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(TdsDepositRow::into_model).collect())
    }

    // ── TDS returns & certificates (Phase 3a) ─────────────────────

    /// Deductions with DEPOSITED deposit status whose challan deposit falls
    /// in [quarter_start, quarter_end] for an entity, filtered by the
    /// return type's section set (24Q = salary section, 27Q = sections
    /// flagged NON_RESIDENT, 26Q = everything else). One row per deduction
    /// (DISTINCT ON — a deduction may have several challans). `entity_id`
    /// comes from the vendor payment; challan context from `tds_deposits`.
    pub async fn tds_deductions_for_return(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        return_type: TdsReturnType,
        salary_section: &str,
        quarter_start: NaiveDate,
        quarter_end: NaiveDate,
    ) -> Result<Vec<TdsReturnDeductionRow>, TaxError> {
        let global = Uuid::parse_str(GLOBAL_TENANT).expect("static GLOBAL_TENANT uuid");
        let base = r#"
            SELECT DISTINCT ON (td.tds_deduction_id)
                   td.tds_deduction_id, vp.entity_id, vp.vendor_id,
                   td.pan_of_deductee AS pan, td.tds_section AS section,
                   vp.payment_date, vp.amount AS payment_amount,
                   td.tds_rate, td.tds_amount, dep.challan_reference, dep.deposit_date
            FROM tds_deductions td
            JOIN vendor_payments vp ON vp.payment_id = td.payment_id AND vp.tenant_id = td.tenant_id
            JOIN tds_deposits dep ON dep.tds_deduction_id = td.tds_deduction_id AND dep.tenant_id = td.tenant_id
            WHERE td.tenant_id = $1 AND vp.entity_id = $2
              AND td.tds_deposit_status = 'DEPOSITED'
              AND dep.deposit_date BETWEEN $3 AND $4
        "#;
        let rows: Vec<TdsReturnDeductionRow> = match return_type {
            TdsReturnType::Form24q => sqlx::query_as::<_, TdsReturnDeductionRow>(
                &format!("{base} AND td.tds_section = $5 ORDER BY td.tds_deduction_id, dep.deposit_date"),
            )
            .bind(tenant_id)
            .bind(entity_id)
            .bind(quarter_start)
            .bind(quarter_end)
            .bind(salary_section)
            .fetch_all(&self.pool)
            .await?,
            TdsReturnType::Form27q => sqlx::query_as::<_, TdsReturnDeductionRow>(
                &format!(
                    "{base} AND td.tds_section IN (SELECT section_code FROM tds_sections \
                     WHERE tenant_id IN ($5, $6) AND applicable_to = 'NON_RESIDENT' AND is_active = TRUE) \
                     ORDER BY td.tds_deduction_id, dep.deposit_date"
                ),
            )
            .bind(tenant_id)
            .bind(entity_id)
            .bind(quarter_start)
            .bind(quarter_end)
            .bind(tenant_id)
            .bind(global)
            .fetch_all(&self.pool)
            .await?,
            TdsReturnType::Form26q => sqlx::query_as::<_, TdsReturnDeductionRow>(
                &format!(
                    "{base} AND td.tds_section <> $5 AND td.tds_section NOT IN \
                     (SELECT section_code FROM tds_sections \
                     WHERE tenant_id IN ($6, $7) AND applicable_to = 'NON_RESIDENT' AND is_active = TRUE) \
                     ORDER BY td.tds_deduction_id, dep.deposit_date"
                ),
            )
            .bind(tenant_id)
            .bind(entity_id)
            .bind(quarter_start)
            .bind(quarter_end)
            .bind(salary_section)
            .bind(tenant_id)
            .bind(global)
            .fetch_all(&self.pool)
            .await?,
        };
        Ok(rows)
    }

    /// Deductions for one PAN within a date range, optionally restricted to
    /// a section fragment (Form 16: salary section; Form 16A: non-salary).
    /// `section_filter` is a static SQL fragment built from enum branches
    /// only (never user input).
    pub async fn tds_deductions_for_pan(
        &self,
        tenant_id: Uuid,
        pan: &str,
        from: NaiveDate,
        to: NaiveDate,
        section_filter: Option<&str>,
    ) -> Result<Vec<TdsReturnDeductionRow>, TaxError> {
        let base = r#"
            SELECT DISTINCT ON (td.tds_deduction_id)
                   td.tds_deduction_id, vp.entity_id, vp.vendor_id,
                   td.pan_of_deductee AS pan, td.tds_section AS section,
                   vp.payment_date, vp.amount AS payment_amount,
                   td.tds_rate, td.tds_amount, dep.challan_reference, dep.deposit_date
            FROM tds_deductions td
            JOIN vendor_payments vp ON vp.payment_id = td.payment_id AND vp.tenant_id = td.tenant_id
            LEFT JOIN tds_deposits dep ON dep.tds_deduction_id = td.tds_deduction_id AND dep.tenant_id = td.tenant_id
            WHERE td.tenant_id = $1 AND td.pan_of_deductee = $2
              AND vp.payment_date BETWEEN $3 AND $4
              AND td.tds_deposit_status IN ('DEPOSITED', 'FILED')
        "#;
        let sql = match section_filter {
            Some(f) => format!("{base} AND {f} ORDER BY td.tds_deduction_id, dep.deposit_date"),
            None => format!("{base} ORDER BY td.tds_deduction_id, dep.deposit_date"),
        };
        let rows = sqlx::query_as::<_, TdsReturnDeductionRow>(&sql)
            .bind(tenant_id)
            .bind(pan)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    /// Insert a TDS return (spec `GenerateTdsReturn`); the (entity, type,
    /// quarter, fy) uniqueness constraint is the idempotency key.
    pub async fn insert_tds_return(&self, r: &TdsReturn) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO tds_returns (
                tds_return_id, tenant_id, entity_id, return_type, quarter, fiscal_year,
                status, due_date, filed_date, acknowledgment_no,
                total_deductions, total_deposits, json_data, created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            "#,
        )
        .bind(r.tds_return_id.as_uuid())
        .bind(r.tenant_id.as_uuid())
        .bind(r.entity_id)
        .bind(r.return_type.to_db_str())
        .bind(&r.quarter)
        .bind(&r.fiscal_year)
        .bind(r.status.to_db_str())
        .bind(r.due_date)
        .bind(r.filed_date)
        .bind(&r.acknowledgment_no)
        .bind(r.total_deductions)
        .bind(r.total_deposits)
        .bind(&r.json_data)
        .bind(r.audit.created_by)
        .bind(r.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Insert a deduction row inside a TDS return (immutable fact row).
    pub async fn insert_tds_return_detail(&self, d: &TdsReturnDetail) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO tds_return_details (
                tds_return_detail_id, tenant_id, tds_return_id, vendor_id, employee_id, pan,
                section, payment_date, payment_amount, tds_rate, tds_amount,
                surcharge, cess, total_tds, challan_details, salary_month
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
            "#,
        )
        .bind(d.tds_return_detail_id.as_uuid())
        .bind(d.tenant_id.as_uuid())
        .bind(d.tds_return_id)
        .bind(d.vendor_id)
        .bind(d.employee_id)
        .bind(&d.pan)
        .bind(&d.section)
        .bind(d.payment_date)
        .bind(d.payment_amount)
        .bind(d.tds_rate)
        .bind(d.tds_amount)
        .bind(d.surcharge)
        .bind(d.cess)
        .bind(d.total_tds)
        .bind(&d.challan_details)
        .bind(d.salary_month)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_tds_return(
        &self,
        tenant_id: Uuid,
        return_id: Uuid,
    ) -> Result<Option<TdsReturn>, TaxError> {
        let row = sqlx::query_as::<_, TdsReturnRow>(
            r#"
            SELECT tds_return_id, tenant_id, entity_id, return_type, quarter, fiscal_year,
                   status, due_date, filed_date, acknowledgment_no,
                   total_deductions, total_deposits, json_data,
                   created_at, created_by, updated_at, updated_by
            FROM tds_returns
            WHERE tenant_id = $1 AND tds_return_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(return_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(TdsReturnRow::into_model))
    }

    /// Find an existing return by its natural key (the idempotency key of
    /// `GenerateTdsReturn`).
    pub async fn find_tds_return_by_key(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        return_type: TdsReturnType,
        quarter: &str,
        fiscal_year: &str,
    ) -> Result<Option<TdsReturn>, TaxError> {
        let row = sqlx::query_as::<_, TdsReturnRow>(
            r#"
            SELECT tds_return_id, tenant_id, entity_id, return_type, quarter, fiscal_year,
                   status, due_date, filed_date, acknowledgment_no,
                   total_deductions, total_deposits, json_data,
                   created_at, created_by, updated_at, updated_by
            FROM tds_returns
            WHERE tenant_id = $1 AND entity_id = $2 AND return_type = $3
              AND quarter = $4 AND fiscal_year = $5
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(return_type.to_db_str())
        .bind(quarter)
        .bind(fiscal_year)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(TdsReturnRow::into_model))
    }

    /// List TDS returns for an entity, optionally by fiscal year (spec
    /// `GetTdsReturns(entity, fy)`).
    pub async fn list_tds_returns(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
        fiscal_year: Option<&str>,
    ) -> Result<Vec<TdsReturn>, TaxError> {
        let rows = match (entity_id, fiscal_year) {
            (Some(eid), Some(fy)) => {
                sqlx::query_as::<_, TdsReturnRow>(
                    r#"
                    SELECT tds_return_id, tenant_id, entity_id, return_type, quarter, fiscal_year,
                           status, due_date, filed_date, acknowledgment_no,
                           total_deductions, total_deposits, json_data,
                           created_at, created_by, updated_at, updated_by
                    FROM tds_returns
                    WHERE tenant_id = $1 AND entity_id = $2 AND fiscal_year = $3
                    ORDER BY quarter, return_type
                    "#,
                )
                .bind(tenant_id)
                .bind(eid)
                .bind(fy)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(eid), None) => {
                sqlx::query_as::<_, TdsReturnRow>(
                    r#"
                    SELECT tds_return_id, tenant_id, entity_id, return_type, quarter, fiscal_year,
                           status, due_date, filed_date, acknowledgment_no,
                           total_deductions, total_deposits, json_data,
                           created_at, created_by, updated_at, updated_by
                    FROM tds_returns
                    WHERE tenant_id = $1 AND entity_id = $2
                    ORDER BY fiscal_year DESC, quarter, return_type
                    "#,
                )
                .bind(tenant_id)
                .bind(eid)
                .fetch_all(&self.pool)
                .await?
            }
            (None, _) => {
                sqlx::query_as::<_, TdsReturnRow>(
                    r#"
                    SELECT tds_return_id, tenant_id, entity_id, return_type, quarter, fiscal_year,
                           status, due_date, filed_date, acknowledgment_no,
                           total_deductions, total_deposits, json_data,
                           created_at, created_by, updated_at, updated_by
                    FROM tds_returns
                    WHERE tenant_id = $1
                    ORDER BY fiscal_year DESC, quarter, return_type
                    "#,
                )
                .bind(tenant_id)
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(rows.into_iter().map(TdsReturnRow::into_model).collect())
    }

    pub async fn list_tds_return_details(
        &self,
        tenant_id: Uuid,
        return_id: Uuid,
    ) -> Result<Vec<TdsReturnDetail>, TaxError> {
        let rows = sqlx::query_as::<_, TdsReturnDetailRow>(
            r#"
            SELECT tds_return_detail_id, tenant_id, tds_return_id, vendor_id, employee_id, pan,
                   section, payment_date, payment_amount, tds_rate, tds_amount,
                   surcharge, cess, total_tds, challan_details, salary_month, created_at
            FROM tds_return_details
            WHERE tenant_id = $1 AND tds_return_id = $2
            ORDER BY payment_date
            "#,
        )
        .bind(tenant_id)
        .bind(return_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(TdsReturnDetailRow::into_model).collect())
    }

    /// Persist a filing transition: GENERATED → FILED / FILED_WITH_ERRORS
    /// with the acknowledgment number (state check owned by the command).
    pub async fn update_tds_return_filed(
        &self,
        tenant_id: Uuid,
        return_id: Uuid,
        status: TdsReturnStatus,
        acknowledgment_no: &str,
        filed_date: NaiveDate,
        filed_by: Uuid,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE tds_returns
            SET status = $3, acknowledgment_no = $4, filed_date = $5,
                updated_at = now(), updated_by = $6, entity_version = entity_version + 1
            WHERE tenant_id = $1 AND tds_return_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(return_id)
        .bind(status.to_db_str())
        .bind(acknowledgment_no)
        .bind(filed_date)
        .bind(filed_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Advance the deduction deposit leg DEPOSITED → FILED for the
    /// deductions covered by a filed return, and mirror the state on their
    /// `tds_deposits` rows (deduction state machine PENDING → DEPOSITED →
    /// FILED per spec).
    pub async fn mark_deductions_filed(
        &self,
        tenant_id: Uuid,
        deduction_ids: &[Uuid],
        filed_date: NaiveDate,
        filed_by: Uuid,
    ) -> Result<(), TaxError> {
        for id in deduction_ids {
            sqlx::query(
                r#"
                UPDATE tds_deductions
                SET tds_deposit_status = 'FILED', tds_return_filed_date = $3,
                    updated_at = now(), updated_by = $4, entity_version = entity_version + 1
                WHERE tenant_id = $1 AND tds_deduction_id = $2 AND tds_deposit_status = 'DEPOSITED'
                "#,
            )
            .bind(tenant_id)
            .bind(id)
            .bind(filed_date)
            .bind(filed_by)
            .execute(&self.pool)
            .await?;
            sqlx::query(
                r#"
                UPDATE tds_deposits
                SET status = 'FILED', updated_at = now(), updated_by = $3
                WHERE tenant_id = $1 AND tds_deduction_id = $2 AND status = 'DEPOSITED'
                "#,
            )
            .bind(tenant_id)
            .bind(id)
            .bind(filed_by)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Find a Form 16 / Form 16A certificate (unique per tenant, FY,
    /// employee/vendor).
    pub async fn find_form16_certificate(
        &self,
        tenant_id: Uuid,
        certificate_type: Form16CertificateType,
        employee_id: Option<Uuid>,
        vendor_id: Option<Uuid>,
        fiscal_year: &str,
    ) -> Result<Option<Form16Certificate>, TaxError> {
        let row = sqlx::query_as::<_, Form16CertificateRow>(
            r#"
            SELECT form16_certificate_id, tenant_id, entity_id, certificate_type, fiscal_year,
                   employee_id, vendor_id, pan, document_url, status, generated_by_id, issued_at,
                   created_at, created_by, updated_at, updated_by
            FROM form16_certificates
            WHERE tenant_id = $1 AND certificate_type = $2 AND fiscal_year = $3
              AND ($4::uuid IS NULL OR employee_id = $4)
              AND ($5::uuid IS NULL OR vendor_id = $5)
            "#,
        )
        .bind(tenant_id)
        .bind(certificate_type.to_db_str())
        .bind(fiscal_year)
        .bind(employee_id)
        .bind(vendor_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Form16CertificateRow::into_model))
    }

    /// Insert a Form 16 / Form 16A certificate record.
    pub async fn insert_form16_certificate(&self, c: &Form16Certificate) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO form16_certificates (
                form16_certificate_id, tenant_id, entity_id, certificate_type, fiscal_year,
                employee_id, vendor_id, pan, document_url, status, generated_by_id,
                created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            "#,
        )
        .bind(c.form16_certificate_id.as_uuid())
        .bind(c.tenant_id.as_uuid())
        .bind(c.entity_id)
        .bind(c.certificate_type.to_db_str())
        .bind(&c.fiscal_year)
        .bind(c.employee_id)
        .bind(c.vendor_id)
        .bind(&c.pan)
        .bind(&c.document_url)
        .bind(c.status.to_db_str())
        .bind(c.generated_by_id)
        .bind(c.audit.created_by)
        .bind(c.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The entity's registered state (name) — used for the GSTIN state-code
    /// match in `RegisterGstin` (mapped to a 2-digit code via
    /// `tax.state_code_map`).
    pub async fn entity_state(&self, tenant_id: Uuid, entity_id: Uuid) -> Result<Option<String>, TaxError> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            r#"
            SELECT state FROM entities
            WHERE tenant_id = $1 AND entity_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|r| r.0))
    }

    /// A vendor's PAN (Form 16/16A subjects are vendor-master rows — salary
    /// employees are modelled as vendors with a PAN).
    pub async fn vendor_pan(&self, tenant_id: Uuid, vendor_id: Uuid) -> Result<Option<String>, TaxError> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            r#"
            SELECT pan FROM vendors
            WHERE tenant_id = $1 AND vendor_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(vendor_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|r| r.0))
    }

    /// Annual turnover band (paise) from posted GL aggregates over a fiscal
    /// year — the GSTR-9 / GSTR-9C applicability input (> ₹2 Cr / > ₹5 Cr).
    pub async fn annual_turnover(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<i64, TaxError> {
        let row: Option<(Option<i64>,)> = sqlx::query_as(
            r#"
            SELECT SUM(GREATEST(COALESCE(l.credit_amount, 0) - COALESCE(l.debit_amount, 0), 0))
            FROM journal_entry_lines l
            JOIN journal_entries je ON je.journal_id = l.journal_id
            JOIN chart_of_accounts a ON a.account_id = l.account_id
            WHERE je.status = 'POSTED'
              AND je.tenant_id = $1
              AND je.entity_id = $2
              AND je.posting_date BETWEEN $3 AND $4
              AND a.tenant_id = $1
              AND a.gst_classification IS NOT NULL
              AND a.deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(from)
        .bind(to)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|r| r.0).unwrap_or(0))
    }

    // ── Trust exemption & income tax compliance ─────────────────────


    pub async fn find_trust_exemption(
        &self,
        tenant_id: Uuid,
        exemption_id: Uuid,
    ) -> Result<Option<TrustExemption>, TaxError> {
        let row = sqlx::query_as::<_, TrustExemptionRow>(
            r#"
            SELECT trust_exemption_id, tenant_id, entity_id, exemption_section, registration_no,
                   registration_date, valid_from, valid_to, status, approving_authority,
                   is_trust, trust_name, trust_pan, created_at, created_by, updated_at, updated_by
            FROM trust_exemptions
            WHERE tenant_id = $1 AND trust_exemption_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(exemption_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(TrustExemptionRow::into_model))
    }

    pub async fn find_trust_exemption_by_section(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        section: TrustExemptionSection,
    ) -> Result<Option<TrustExemption>, TaxError> {
        let row = sqlx::query_as::<_, TrustExemptionRow>(
            r#"
            SELECT trust_exemption_id, tenant_id, entity_id, exemption_section, registration_no,
                   registration_date, valid_from, valid_to, status, approving_authority,
                   is_trust, trust_name, trust_pan, created_at, created_by, updated_at, updated_by
            FROM trust_exemptions
            WHERE tenant_id = $1 AND entity_id = $2 AND exemption_section = $3
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(section.to_db_str())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(TrustExemptionRow::into_model))
    }

    pub async fn list_trust_exemptions(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<TrustExemption>, TaxError> {
        let rows = sqlx::query_as::<_, TrustExemptionRow>(
            r#"
            SELECT trust_exemption_id, tenant_id, entity_id, exemption_section, registration_no,
                   registration_date, valid_from, valid_to, status, approving_authority,
                   is_trust, trust_name, trust_pan, created_at, created_by, updated_at, updated_by
            FROM trust_exemptions
            WHERE tenant_id = $1 AND entity_id = $2
            ORDER BY valid_to DESC
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(TrustExemptionRow::into_model).collect())
    }

    pub async fn insert_trust_exemption(&self, e: &TrustExemption) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO trust_exemptions (
                trust_exemption_id, tenant_id, entity_id, exemption_section, registration_no,
                registration_date, valid_from, valid_to, status, approving_authority,
                is_trust, trust_name, trust_pan, created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            "#,
        )
        .bind(e.trust_exemption_id.as_uuid())
        .bind(e.tenant_id.as_uuid())
        .bind(e.entity_id)
        .bind(e.exemption_section.to_db_str())
        .bind(&e.registration_no)
        .bind(e.registration_date)
        .bind(e.valid_from)
        .bind(e.valid_to)
        .bind(e.status.to_db_str())
        .bind(&e.approving_authority)
        .bind(e.is_trust)
        .bind(&e.trust_name)
        .bind(&e.trust_pan)
        .bind(e.audit.created_by)
        .bind(e.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn renew_trust_exemption(
        &self,
        tenant_id: Uuid,
        exemption_id: Uuid,
        valid_from: NaiveDate,
        valid_to: NaiveDate,
        updated_by: Uuid,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE trust_exemptions
            SET valid_from = $3, valid_to = $4, status = 'ACTIVE',
                updated_at = now(), updated_by = $5, entity_version = entity_version + 1
            WHERE tenant_id = $1 AND trust_exemption_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(exemption_id)
        .bind(valid_from)
        .bind(valid_to)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_income_application(
        &self,
        tenant_id: Uuid,
        fiscal_year_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Option<IncomeApplication>, TaxError> {
        let row = sqlx::query_as::<_, IncomeApplicationRow>(
            r#"
            SELECT income_application_id, tenant_id, fiscal_year_id, entity_id, total_income,
                   amount_applied, application_percent, accumulated_amount, accumulation_year,
                   accumulation_purpose, status, last_computed_at,
                   created_at, created_by, updated_at, updated_by
            FROM income_applications
            WHERE tenant_id = $1 AND fiscal_year_id = $2 AND entity_id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(fiscal_year_id)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(IncomeApplicationRow::into_model))
    }

    pub async fn insert_income_application(&self, a: &IncomeApplication) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO income_applications (
                income_application_id, tenant_id, fiscal_year_id, entity_id, total_income,
                amount_applied, application_percent, accumulated_amount, accumulation_year,
                accumulation_purpose, status, last_computed_at, created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,now(),$12,$13)
            "#,
        )
        .bind(a.income_application_id.as_uuid())
        .bind(a.tenant_id.as_uuid())
        .bind(a.fiscal_year_id)
        .bind(a.entity_id)
        .bind(a.total_income)
        .bind(a.amount_applied)
        .bind(a.application_percent)
        .bind(a.accumulated_amount)
        .bind(a.accumulation_year)
        .bind(&a.accumulation_purpose)
        .bind(a.status.to_db_str())
        .bind(a.audit.created_by)
        .bind(a.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_income_application(&self, a: &IncomeApplication) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE income_applications
            SET total_income = $4, amount_applied = $5, application_percent = $6,
                accumulated_amount = $7, accumulation_year = $8, accumulation_purpose = $9,
                status = $10, last_computed_at = now(),
                updated_at = now(), updated_by = $11, entity_version = entity_version + 1
            WHERE tenant_id = $1 AND income_application_id = $2 AND entity_id = $3
            "#,
        )
        .bind(a.tenant_id.as_uuid())
        .bind(a.income_application_id.as_uuid())
        .bind(a.entity_id)
        .bind(a.total_income)
        .bind(a.amount_applied)
        .bind(a.application_percent)
        .bind(a.accumulated_amount)
        .bind(a.accumulation_year)
        .bind(&a.accumulation_purpose)
        .bind(a.status.to_db_str())
        .bind(a.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_income_application_line(&self, l: &IncomeApplicationLine) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO income_application_lines (
                income_app_line_id, tenant_id, income_application_id, category, amount,
                account_id, description
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(l.income_app_line_id.as_uuid())
        .bind(l.tenant_id.as_uuid())
        .bind(l.income_application_id)
        .bind(l.category.to_db_str())
        .bind(l.amount)
        .bind(l.account_id)
        .bind(&l.description)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_income_application_lines(
        &self,
        tenant_id: Uuid,
        application_id: Uuid,
    ) -> Result<Vec<IncomeApplicationLine>, TaxError> {
        let rows = sqlx::query_as::<_, IncomeApplicationLineRow>(
            r#"
            SELECT income_app_line_id, tenant_id, income_application_id, category, amount,
                   account_id, description, created_at
            FROM income_application_lines
            WHERE tenant_id = $1 AND income_application_id = $2
            ORDER BY category
            "#,
        )
        .bind(tenant_id)
        .bind(application_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(IncomeApplicationLineRow::into_model).collect())
    }

    /// Total accumulated (unapplied) income across FYs for an entity —
    /// the s.11(2) 5-year accumulation pool.
    pub async fn accumulated_income(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
    ) -> Result<i64, TaxError> {
        let row: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(accumulated_amount), 0)
            FROM income_applications
            WHERE tenant_id = $1 AND entity_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0).unwrap_or(0))
    }

    pub async fn find_fcra_registration(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
    ) -> Result<Option<FcraRegistration>, TaxError> {
        let row = sqlx::query_as::<_, FcraRegistrationRow>(
            r#"
            SELECT fcra_registration_id, tenant_id, entity_id, registration_no, valid_from,
                   valid_to, bank_account_id, status, total_receipts, admin_expenses,
                   admin_expense_ratio, fc4_return_filed_date,
                   created_at, created_by, updated_at, updated_by
            FROM fcra_registrations
            WHERE tenant_id = $1 AND fcra_registration_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(registration_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(FcraRegistrationRow::into_model))
    }

    pub async fn list_fcra_registrations(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<FcraRegistration>, TaxError> {
        let rows = sqlx::query_as::<_, FcraRegistrationRow>(
            r#"
            SELECT fcra_registration_id, tenant_id, entity_id, registration_no, valid_from,
                   valid_to, bank_account_id, status, total_receipts, admin_expenses,
                   admin_expense_ratio, fc4_return_filed_date,
                   created_at, created_by, updated_at, updated_by
            FROM fcra_registrations
            WHERE tenant_id = $1 AND entity_id = $2
            ORDER BY valid_to DESC
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(FcraRegistrationRow::into_model).collect())
    }

    pub async fn insert_fcra_registration(&self, r: &FcraRegistration) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            INSERT INTO fcra_registrations (
                fcra_registration_id, tenant_id, entity_id, registration_no, valid_from,
                valid_to, bank_account_id, status, total_receipts, admin_expenses,
                admin_expense_ratio, fc4_return_filed_date, created_by, updated_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            "#,
        )
        .bind(r.fcra_registration_id.as_uuid())
        .bind(r.tenant_id.as_uuid())
        .bind(r.entity_id)
        .bind(&r.registration_no)
        .bind(r.valid_from)
        .bind(r.valid_to)
        .bind(r.bank_account_id)
        .bind(r.status.to_db_str())
        .bind(r.total_receipts)
        .bind(r.admin_expenses)
        .bind(r.admin_expense_ratio)
        .bind(r.fc4_return_filed_date)
        .bind(r.audit.created_by)
        .bind(r.audit.updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_fcra_compliance(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
        total_receipts: i64,
        admin_expenses: i64,
        admin_expense_ratio: Option<Decimal>,
        updated_by: Uuid,
    ) -> Result<(), TaxError> {
        sqlx::query(
            r#"
            UPDATE fcra_registrations
            SET total_receipts = $3, admin_expenses = $4, admin_expense_ratio = $5,
                updated_at = now(), updated_by = $6, entity_version = entity_version + 1
            WHERE tenant_id = $1 AND fcra_registration_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(registration_id)
        .bind(total_receipts)
        .bind(admin_expenses)
        .bind(admin_expense_ratio)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── GL account map & bank accounts ──────────────────────────────

    /// Find a GL account by exact code, then by code prefix (first leaf).
    /// Used for the statutory accounts: 24.03 TDS Payable, 24.02 RCM
    /// Payable, 11.01 ITC Recoverable.
    pub async fn find_account_by_code_prefix(
        &self,
        tenant_id: Uuid,
        prefix: &str,
    ) -> Result<Option<(Uuid, String)>, TaxError> {
        let exact: Option<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT account_id, account_code FROM chart_of_accounts
            WHERE tenant_id = $1 AND account_code = $2 AND deleted_at IS NULL AND is_active = TRUE
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(prefix)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(e) = exact {
            return Ok(Some(e));
        }
        let prefixed: Option<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT account_id, account_code FROM chart_of_accounts
            WHERE tenant_id = $1 AND account_code LIKE $2 AND deleted_at IS NULL AND is_active = TRUE
            ORDER BY length(account_code) LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(format!("{prefix}%"))
        .fetch_optional(&self.pool)
        .await?;
        Ok(prefixed)
    }

    /// Resolve the TDS Payable GL leaf per section (`24.03.<section>`) for
    /// a challan deposit (CA review S7). Resolution order per section:
    /// exact leaf `24.03.<section>` → deeper leaf `24.03.<section>.*` →
    /// the `24.03` parent (charts that only keep the parent). Sections with
    /// no resolvable account are simply absent from the result and the
    /// command errors.
    pub async fn find_tds_payable_accounts(
        &self,
        tenant_id: Uuid,
        sections: &[String],
    ) -> Result<std::collections::BTreeMap<String, Uuid>, TaxError> {
        let mut resolved = std::collections::BTreeMap::new();
        if sections.is_empty() {
            return Ok(resolved);
        }
        let rows: Vec<(String, Uuid)> = sqlx::query_as(
            r#"
            SELECT account_code, account_id FROM chart_of_accounts
            WHERE tenant_id = $1 AND account_code LIKE '24.03%'
              AND deleted_at IS NULL AND is_active = TRUE
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        for section in sections {
            let exact = format!("24.03.{section}");
            let deeper = format!("{exact}.");
            let account_id = rows
                .iter()
                .find(|(c, _)| c == &exact)
                .map(|(_, id)| *id)
                .or_else(|| rows.iter().find(|(c, _)| c.starts_with(&deeper)).map(|(_, id)| *id))
                .or_else(|| rows.iter().find(|(c, _)| c == "24.03").map(|(_, id)| *id));
            if let Some(id) = account_id {
                resolved.insert(section.clone(), id);
            }
        }
        Ok(resolved)
    }

    /// The ITC Recoverable account that RCM ITC posts to: prefer an 11.01
    /// leaf whose name mentions RCM, else the first 11.01 leaf.
    pub async fn find_rcm_itc_account(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<(Uuid, String)>, TaxError> {
        let named: Option<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT account_id, account_code FROM chart_of_accounts
            WHERE tenant_id = $1 AND account_code LIKE '11.01%' AND deleted_at IS NULL
              AND is_active = TRUE AND account_name ILIKE '%RCM%'
            ORDER BY length(account_code) LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(n) = named {
            return Ok(Some(n));
        }
        self.find_account_by_code_prefix(tenant_id, "11.01").await
    }

    /// GL account backing a bank account (for CR Bank legs).
    /// Returns (gl_account_id, account_type, account_name); gl_account_id
    /// is None when the account has no linked GL leaf yet.
    pub async fn bank_gl_account(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<Option<(Option<Uuid>, String, String)>, TaxError> {
        let row: Option<(Option<Uuid>, String, String)> = sqlx::query_as(
            r#"
            SELECT ba.gl_account_id, ba.account_type, ba.account_name
            FROM bank_accounts ba
            WHERE ba.tenant_id = $1 AND ba.bank_account_id = $2 AND ba.deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Verify a bank account is FCRA-designated (FCRA 2010 s.17).
    pub async fn is_fcra_bank_account(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<bool, TaxError> {
        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT account_type FROM bank_accounts
            WHERE tenant_id = $1 AND bank_account_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(matches!(row, Some((t,)) if t == "FCRA"))
    }

    /// GL balance of a single account (credits − debits) within a period,
    /// posted journals only.
    pub async fn gl_account_balance(
        &self,
        tenant_id: Uuid,
        account_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<i64, TaxError> {
        let row: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT SUM(COALESCE(l.credit_amount, 0)) - SUM(COALESCE(l.debit_amount, 0))
            FROM journal_entry_lines l
            JOIN journal_entries je ON je.journal_id = l.journal_id
            WHERE je.status = 'POSTED'
              AND je.tenant_id = $1
              AND l.account_id = $2
              AND je.posting_date BETWEEN $3 AND $4
            "#,
        )
        .bind(tenant_id)
        .bind(account_id)
        .bind(from)
        .bind(to)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0).unwrap_or(0))
    }

    /// Latest `Section115BreachDetected` event payload for the tenant
    /// (breaches are evented, not stored in a fact table).
    pub async fn latest_section115_breach(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<Value>, TaxError> {
        let row: Option<(Value,)> = sqlx::query_as(
            r#"
            SELECT event_payload FROM event_outbox
            WHERE tenant_id = $1 AND aggregate_type = 'Taxation'
              AND event_type = 'Section115BreachDetected'
            ORDER BY created_at DESC LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }
}

/// TDS register balance row — one per TDS Payable GL account (24.03.x).
#[derive(sqlx::FromRow, Debug, Clone, serde::Serialize)]
pub struct TdsRegisterBalanceRow {
    pub account_id: Uuid,
    pub account_code: String,
    pub account_name: String,
    pub balance_paise: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct GstRateRow {
    gst_rate_id: Uuid,
    tenant_id: Uuid,
    hsn_sac_code: String,
    description: Option<String>,
    rate: i64,
    itc_eligible: bool,
    effective_from: NaiveDate,
    effective_to: Option<NaiveDate>,
    supply_type: String,
    is_active: bool,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl GstRateRow {
    fn into_model(self) -> GstRate {
        GstRate {
            gst_rate_id: EntityId::from_uuid(self.gst_rate_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            hsn_sac_code: self.hsn_sac_code,
            description: self.description,
            rate: self.rate,
            itc_eligible: self.itc_eligible,
            effective_from: self.effective_from,
            effective_to: self.effective_to,
            supply_type: GstSupplyType::from_db_str(&self.supply_type),
            is_active: self.is_active,
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

// Note: TdsReturn/TdsReturnDetail/Form16Certificate model types are
// re-exported by models.rs and consumed by queries.rs / commands.rs —
// the repository does not need dedicated row types for them in Phase 2.
