//! User model and auth DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user account in the ERP.
///
/// Mirrors the `app_users` table. `password_hash` is excluded from
/// serialization so user objects can never leak password hashes over the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUser {
    /// Unique user id.
    pub id: Uuid,
    /// Tenant the user belongs to (multi-tenant at the database level).
    pub tenant_id: Uuid,
    /// Login email. Unique per tenant (`UNIQUE(tenant_id, email)`).
    pub email: String,
    /// Argon2id PHC hash of the password.
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// Display name.
    pub full_name: String,
    /// Whether the account may log in.
    pub is_active: bool,
    /// Whether the user bypasses per-tenant role checks (platform admin).
    pub is_superadmin: bool,
    /// Last successful login timestamp.
    pub last_login_at: Option<DateTime<Utc>>,
    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Request body for `POST /api/v1/auth/register`.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUser {
    /// Tenant to create the user in.
    pub tenant_id: Uuid,
    /// Login email.
    pub email: String,
    /// Plaintext password — hashed with argon2id before storage.
    pub password: String,
    /// Display name.
    pub full_name: String,
}

/// Request body for `POST /api/v1/auth/login`.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    /// Login email.
    pub email: String,
    /// Plaintext password.
    pub password: String,
    /// Optional tenant discriminator. Required when the same email exists in
    /// more than one tenant.
    pub tenant_id: Option<Uuid>,
}

/// Public user payload returned by login/me endpoints.
#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    /// User id.
    pub id: Uuid,
    /// Login email.
    pub email: String,
    /// Display name.
    pub full_name: String,
    /// Tenant id.
    pub tenant_id: Uuid,
    /// Role codes held by the user.
    pub roles: Vec<String>,
}

/// Response body for `POST /api/v1/auth/login`.
#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    /// Signed HS256 JWT (24h validity).
    pub token: String,
    /// Public user information.
    pub user: UserInfo,
}

impl AppUser {
    /// Convert to the public user payload, with roles resolved separately.
    pub fn to_user_info(&self, roles: Vec<String>) -> UserInfo {
        UserInfo {
            id: self.id,
            email: self.email.clone(),
            full_name: self.full_name.clone(),
            tenant_id: self.tenant_id,
            roles,
        }
    }
}
