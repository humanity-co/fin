//! SutraERP — Authentication Kernel
//!
//! Password hashing (argon2id), HS256 JWT issuance/validation, the
//! authenticated [`UserContext`] identity type, and the Axum [`auth_layer`]
//! middleware that validates `Authorization: Bearer <token>` headers and
//! injects the caller's identity into request extensions — ready for the
//! RBAC permission layer to consume.
//!
//! ## Security properties
//!
//! - Passwords are hashed with **argon2id** (memory-hard, v19, default OWASP
//!   parameters) — never stored in plaintext.
//! - Tokens are signed with **HS256** using a `JWT_SECRET` environment
//!   variable (24-hour expiry). The secret is never hardcoded; auth fails
//!   closed when `JWT_SECRET` is not configured.
//! - The middleware returns `401` for missing, malformed, expired, or
//!   invalid tokens and never passes the request through unauthenticated.

pub mod errors;
pub mod hashing;
pub mod jwt;
pub mod middleware;
pub mod models;

pub use errors::AuthError;
pub use middleware::UserContext;
pub use models::{AppUser, CreateUser, LoginRequest, LoginResponse, UserInfo};
