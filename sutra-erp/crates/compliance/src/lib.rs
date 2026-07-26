//! SutraERP — Compliance & Reporting Module
//!
//! Compliance calendar, statutory reports (GST, TDS, PT),
//! regulatory reports (NAAC, AISHE, UGC UC), and audit management.

pub mod commands;
pub mod errors;
pub mod events;
pub mod models;
pub mod queries;
pub mod repository;

pub use models::ComplianceEvent;
