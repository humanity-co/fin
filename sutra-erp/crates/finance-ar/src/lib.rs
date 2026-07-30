//! SutraERP — Accounts Receivable Module
//!
//! Student fee management, fee collection, concessions,
//! scholarships, refunds, and security deposits.

pub mod commands;
pub mod errors;
pub mod events;
pub mod maha_dbt;
pub mod models;
pub mod queries;
pub mod repository;

pub use commands::{
    ApplyScholarshipCmd, ArCommandHandler, AssessStudentFeesCmd, GrantConcessionCmd,
    InitiateRefundCmd, ProcessRefundCmd, RecordFeePaymentCmd,
    RecordScholarshipDisbursementCmd, VerifyScholarshipCmd,
};
pub use errors::ArError;
pub use maha_dbt::{MahaDbtClient, MahaDbtDisbursementStatus, MahaDbtVerificationResult};
pub use models::concession::{Concession, ConcessionStatus, ConcessionType};
pub use models::fee_head::{FeeHead, FeeType, GstClassification};
pub use models::fee_structure::{FeeStructure, FeeStructureLine, FeeStructureStatus};
pub use models::installment_plan::{InstallmentPlan, InstallmentSlot};
pub use models::payment_receipt::{PaymentMode, PaymentReceipt, ReceiptStatus};
pub use models::refund::{Refund, RefundMode, RefundStatus};
pub use models::scholarship::{
    FundingSource, ScholarshipCategory, ScholarshipScheme, ScholarshipStatus, StudentScholarship,
};
pub use models::security_deposit::SecurityDeposit;
pub use models::student_fee::{
    FeeAccountStatus, FeeInstallment, FeeTransaction, FeeTransactionType, InstallmentStatus,
    StudentFeeAccount,
};
