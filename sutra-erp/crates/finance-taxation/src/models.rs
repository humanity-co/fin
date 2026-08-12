//! Taxation domain models.
//!
//! One module per aggregate cluster, mirroring the base DDL (PART 8
//! "Taxation") and `migrations/004_tax.sql`. Money is i64 paise — never
//! floats. Every model carries `tenant_id` and audit columns.

pub mod gst;
pub mod gst_return;
pub mod income;
pub mod itc;
pub mod tds;

pub use gst::{GstFilingFrequency, GstRate, GstRegistration, GstRegistrationType, GstSupplyType};
pub use gst_return::{GstReturn, GstReturnLine, GstReturnStatus, GstReturnType};
pub use income::{
    FcraRegistration, FcraStatus, IncomeApplication, IncomeApplicationCategory,
    IncomeApplicationLine, IncomeApplicationStatus, TrustExemption, TrustExemptionSection,
    TrustExemptionStatus,
};
pub use itc::{ItcEligibility, ItcRegister, ItcRegisterLine, ItcRegisterStatus};
pub use tds::{
    Form16Certificate, Form16CertificateStatus, Form16CertificateType, TdsDeposit,
    TdsDepositStatus, TdsReturn, TdsReturnDetail, TdsReturnStatus, TdsReturnType, TdsSection,
    TdsSectionApplicableTo,
};
