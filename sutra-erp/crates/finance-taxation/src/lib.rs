//! SutraERP — Taxation Module
//!
//! GST engine, TDS engine, income tax compliance,
//! ITC register, and statutory return generation.

pub mod commands;
pub mod errors;
pub mod events;
pub mod models;
pub mod queries;
pub mod repository;

pub use models::{GstRegistration, TdsSection};
