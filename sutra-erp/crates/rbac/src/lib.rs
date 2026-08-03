//! Tenant-scoped role based access control for SutraERP.
pub mod engine;
pub mod middleware;
pub mod models;
pub mod seed;
pub mod scope;
pub use engine::PermissionEngine;
pub use middleware::{PermissionDenied, RequirePermission, UserContext};
pub use scope::ScopeFilter;
