//! Authentication middleware.
//!
//! Delegates to the `sutra-auth` JWT AuthLayer, which validates
//! `Authorization: Bearer <token>` headers and injects the authenticated
//! [`sutra_auth::UserContext`] into request extensions.

pub use sutra_auth::middleware::auth_layer as auth_middleware;
