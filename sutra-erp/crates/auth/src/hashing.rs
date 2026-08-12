//! Password hashing utilities.
//!
//! Uses **argon2id** (v19) with the default OWASP-recommended parameters and a
//! random 16-byte salt per password. Verification is constant-time on the
//! hash comparison via `password-hash`'s `PasswordVerifier`.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::errors::{AuthError, AuthResult};

/// Hash a plaintext password with argon2id.
///
/// Returns the PHC-formatted hash string (e.g.
/// `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`), suitable for storage in
/// `app_users.password_hash`.
pub fn hash_password(plain: &str) -> AuthResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default(); // Argon2id, v19, OWASP defaults
    argon2
        .hash_password(plain.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AuthError::Hashing(e.to_string()))
}

/// Verify a plaintext password against a stored PHC hash.
///
/// Returns `Ok(true)` on match, `Ok(false)` on mismatch, and `Err` only if
/// the stored hash is malformed or verification fails outright.
pub fn verify_password(plain: &str, hash: &str) -> AuthResult<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| AuthError::Hashing(e.to_string()))?;
    match Argon2::default().verify_password(plain.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(AuthError::Hashing(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let hash = hash_password("Sutra@2026!secure").expect("hash should succeed");
        assert!(hash.starts_with("$argon2id$"), "expected argon2id PHC hash");
        assert!(verify_password("Sutra@2026!secure", &hash).expect("verify should not err"));
        assert!(!verify_password("wrong-password", &hash).expect("verify should not err"));
    }

    #[test]
    fn hashes_are_unique_per_salt() {
        let a = hash_password("same-password").unwrap();
        let b = hash_password("same-password").unwrap();
        assert_ne!(a, b, "random salt must produce distinct hashes");
    }
}
