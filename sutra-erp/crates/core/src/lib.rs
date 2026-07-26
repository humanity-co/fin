//! SutraERP — Shared Kernel
//!
//! This crate contains the fundamental domain types used across
//! all modules: Money, TenantId, EntityId, AuditInfo, and the
//! shared error hierarchy.

pub mod audit;
pub mod entity_id;
pub mod error;
pub mod money;
pub mod tenant;

// Re-exports for convenience
pub use audit::AuditInfo;
pub use entity_id::EntityId;
pub use error::{DomainError, DomainResult};
pub use money::Money;
pub use tenant::TenantId;
