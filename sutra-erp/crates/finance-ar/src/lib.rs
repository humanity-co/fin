//! SutraERP — Accounts Receivable Module
//!
//! Student fee management, fee collection, concessions,
//! scholarships, refunds, and security deposits.

pub mod commands;
pub mod errors;
pub mod events;
pub mod models;
pub mod queries;
pub mod repository;

pub use models::concession::Concession;
pub use models::fee_structure::{FeeStructure, FeeHead, InstallmentPlan};
pub use models::payment_receipt::PaymentReceipt;
pub use models::refund::Refund;
pub use models::scholarship::{Scholarship, ScholarshipScheme};
pub use models::security_deposit::SecurityDeposit;
