//! TenantId — strongly-typed newtype for multi-tenant isolation.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Tenant identifier newtype.
///
/// Every data row in SutraERP belongs to a tenant.
/// The tenant ID is extracted from the request context
/// (header or JWT claim) and injected into every query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantId(Uuid);

impl TenantId {
    /// Create a new random TenantId.
    pub fn new() -> Self {
        TenantId(Uuid::new_v4())
    }

    /// Create from an existing UUID.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        TenantId(uuid)
    }

    /// Access the inner UUID.
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Consume and return the inner UUID.
    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for TenantId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_different_ids() {
        let a = TenantId::new();
        let b = TenantId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_from_uuid_roundtrip() {
        let uuid = Uuid::new_v4();
        let tid = TenantId::from_uuid(uuid);
        assert_eq!(*tid.as_uuid(), uuid);
    }
}
