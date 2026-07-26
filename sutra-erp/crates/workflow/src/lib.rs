//! SutraERP — Workflow & Approval Engine
//!
//! Multi-level approval workflows for financial transactions:
//! purchase orders, invoices, payments, refunds, concessions, budgets.

pub mod commands;
pub mod errors;
pub mod events;
pub mod models;
pub mod queries;
pub mod repository;

pub use models::{ApprovalRequest, ApprovalWorkflow};
