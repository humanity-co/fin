//! EntityId — generic newtype over UUID for aggregate root identity.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;
use uuid::Uuid;

/// Generic EntityId newtype for aggregate root identifiers.
///
/// The type parameter `T` provides compile-time type safety so that
/// a `JournalId` cannot accidentally be used where a `VendorId` is expected.
///
/// # Examples
///
/// ```rust
/// use sutra_core::EntityId;
///
/// struct Journal;
/// type JournalId = EntityId<Journal>;
///
/// let id = JournalId::new();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct EntityId<T> {
    id: Uuid,
    #[serde(skip)]
    _phantom: PhantomData<T>,
}

impl<T> EntityId<T> {
    /// Create a new random v7 EntityId (time-ordered for aggregate roots).
    pub fn new() -> Self {
        EntityId {
            id: Uuid::now_v7(),
            _phantom: PhantomData,
        }
    }

    /// Create a new random v4 EntityId (for non-aggregate-root entities).
    pub fn new_v4() -> Self {
        EntityId {
            id: Uuid::new_v4(),
            _phantom: PhantomData,
        }
    }

    /// Create from an existing UUID.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        EntityId {
            id: uuid,
            _phantom: PhantomData,
        }
    }

    /// Access the inner UUID.
    pub const fn as_uuid(&self) -> &Uuid {
        &self.id
    }

    /// Consume and return the inner UUID.
    pub fn into_inner(self) -> Uuid {
        self.id
    }
}

impl<T> Default for EntityId<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Display for EntityId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Journal;
    struct Vendor;

    #[test]
    fn test_type_safety() {
        let j1: EntityId<Journal> = EntityId::new();
        let j2: EntityId<Journal> = EntityId::new();

        // Same type compares fine
        assert_ne!(j1, j2);

        // Different types: uncomment to see compile error
        // let v: EntityId<Vendor> = j1; // won't compile
    }

    #[test]
    fn test_v7_is_time_ordered() {
        let first = EntityId::<()>::new();
        let second = EntityId::<()>::new();
        // UUID v7 is time-ordered: first should be <= second
        assert!(first.as_uuid().as_timestamp() <= second.as_uuid().as_timestamp());
    }
}
