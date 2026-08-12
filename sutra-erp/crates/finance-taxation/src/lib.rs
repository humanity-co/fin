//! SutraERP — Taxation Module (Tax Engine)
//!
//! Computes, tracks, and reports every statutory tax the institution owes:
//! GST (liability from journal/transaction data, ITC with Rule 42/43
//! reversal, RCM, GSTR-1/3B/9 return-ready files, configurable rate
//! master), TDS (section/rate/threshold engine used by AP at payment time,
//! deposit tracking, Form 16/16A, 24Q/26Q/27Q data extraction, TDS payable
//! ledger), and income-tax compliance for the trust/society itself
//! (12A/12AB/10(23C) registration validity, 85% application rule,
//! Section 11(5) investments, FCRA caps, audit-deadline tracking).
//!
//! ## Architecture
//!
//! - **Commands** — CQRS write side (`TaxCommandHandler`, Phase 2)
//! - **Queries** — CQRS read side (`TaxQueryHandler`, Phase 2)
//! - **Repository** — data access (SQLx/PostgreSQL, tenant-scoped, Phase 2)
//! - **Events** — outbox event payloads (`TaxationEventData`)
//! - **Models** — aggregates & value objects
//! - **Errors** — module error types (`TaxError`)
//!
//! ## Non-negotiable rules
//!
//! - Money is i64 paise — never floats.
//! - Every row is tenant-scoped.
//! - Every GL posting goes through the GL module's create/post commands and
//!   carries `reference_type` / `reference_id` for traceability
//!   (RCM, ITC_REVERSAL, TDS-deposit journals).
//! - Every mutation leaves an audit trail and writes an outbox event.
//! - Business thresholds come from `system_config` (see `004_tax.sql`
//!   policy seeds), never from code.
//! - Boundary: TDS deduction at payment time stays in AP (it owns
//!   `tds_deductions`); this module owns rates, deposits, returns and
//!   certificates. The RCM leg is posted by this module (single owner).
pub mod commands;
pub mod errors;
pub mod events;
pub mod models;
pub mod queries;
pub mod repository;

pub use errors::TaxError;
pub use events::{write_outbox, TaxationEventData};
pub use models::{
    Form16Certificate, Form16CertificateStatus, Form16CertificateType, FcraRegistration,
    FcraStatus, GstFilingFrequency, GstRate, GstRegistration, GstRegistrationType, GstReturn,
    GstReturnLine, GstReturnStatus, GstReturnType, GstSupplyType, IncomeApplication,
    IncomeApplicationCategory, IncomeApplicationLine, IncomeApplicationStatus, ItcEligibility,
    ItcRegister, ItcRegisterLine, ItcRegisterStatus, TdsDeposit, TdsDepositStatus, TdsReturn,
    TdsReturnDetail, TdsReturnStatus, TdsReturnType, TdsSection, TdsSectionApplicableTo,
    TrustExemption, TrustExemptionSection, TrustExemptionStatus,
};
