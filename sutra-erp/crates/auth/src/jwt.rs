//! JWT issuance and validation.
//!
//! Tokens are signed with **HS256** using a `JWT_SECRET` environment
//! variable. Claims: `sub` (user id), `tenant_id`, `roles`, `exp`
//! (24 hours from issuance).

use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::{AuthError, AuthResult};

/// JWT claims carried by every SutraERP access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// User id (UUID as string) — JWT standard subject claim.
    pub sub: String,
    /// Tenant id (UUID as string) — the tenant the user belongs to.
    pub tenant_id: String,
    /// Role codes the user holds (e.g. `["CFO", "Accountant"]`).
    pub roles: Vec<String>,
    /// Expiry as a Unix timestamp (seconds).
    pub exp: usize,
}

/// Token lifetime: 24 hours.
pub const TOKEN_TTL_SECS: i64 = 24 * 60 * 60;

/// Read the `JWT_SECRET` from the environment.
///
/// Auth fails closed: a missing secret is an error, never a silent default.
fn jwt_secret() -> AuthResult<String> {
    std::env::var("JWT_SECRET").map_err(|_| AuthError::MissingSecret)
}

/// Create a signed HS256 JWT for the given user and tenant.
///
/// The token expires 24 hours after issuance and carries the user's role
/// codes so downstream middleware and handlers can authorize without a DB
/// round-trip per request.
pub fn create_token(user_id: Uuid, tenant_id: Uuid, roles: Vec<String>) -> AuthResult<String> {
    let exp = (Utc::now().timestamp() + TOKEN_TTL_SECS) as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        roles,
        exp,
    };
    let secret = jwt_secret()?;
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AuthError::InvalidToken(e.to_string()))
}

/// Validate a token's signature, expiry, and claims.
///
/// Returns the decoded claims on success. Expired tokens produce
/// [`AuthError::TokenExpired`]; anything else that fails validation produces
/// [`AuthError::InvalidToken`].
pub fn validate_token(token: &str) -> AuthResult<Claims> {
    let secret = jwt_secret()?;
    let mut validation = Validation::new(Algorithm::HS256);
    // `exp` is required: tokens without an expiry are rejected.
    validation.validate_exp = true;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
        _ => AuthError::InvalidToken(e.to_string()),
    })?;
    Ok(data.claims)
}

/// Validate a token and return the parsed `(user_id, tenant_id)`.
///
/// Convenience for the middleware: performs claim parsing once and fails
/// closed on malformed subject/tenant claims.
pub fn validate_token_identity(token: &str) -> AuthResult<(Uuid, Uuid, Vec<String>)> {
    let claims = validate_token(token)?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AuthError::InvalidClaim("malformed sub (user id)".into()))?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| AuthError::InvalidClaim("malformed tenant_id".into()))?;
    Ok((user_id, tenant_id, claims.roles))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static SET_SECRET: Once = Once::new();

    fn init_secret() {
        SET_SECRET.call_once(|| {
            std::env::set_var("JWT_SECRET", "test-secret-for-unit-tests-only");
        });
    }

    #[test]
    fn token_roundtrip() {
        init_secret();
        let user = Uuid::new_v4();
        let tenant = Uuid::new_v4();
        let token = create_token(user, tenant, vec!["CFO".into()]).expect("sign");
        let (uid, tid, roles) = validate_token_identity(&token).expect("validate");
        assert_eq!(uid, user);
        assert_eq!(tid, tenant);
        assert_eq!(roles, vec!["CFO"]);
    }

    #[test]
    fn rejects_garbage() {
        init_secret();
        assert!(matches!(
            validate_token("not-a-jwt"),
            Err(AuthError::InvalidToken(_))
        ));
    }

    #[test]
    fn rejects_wrong_secret() {
        init_secret();
        let token = create_token(Uuid::new_v4(), Uuid::new_v4(), vec![]).expect("sign");
        std::env::set_var("JWT_SECRET", "a-different-secret");
        // restore for other tests
        let result = validate_token(&token);
        std::env::set_var("JWT_SECRET", "test-secret-for-unit-tests-only");
        assert!(matches!(result, Err(AuthError::InvalidToken(_))));
    }
}
