//! SutraERP — Fixed Assets & Inventory Module
//!
//! Fixed asset register, depreciation schedule,
//! asset disposal, and inventory management.

pub mod commands;
pub mod errors;
pub mod events;
pub mod models;
pub mod queries;
pub mod repository;

pub use models::FixedAsset;
