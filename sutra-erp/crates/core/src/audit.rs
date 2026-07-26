//! AuditInfo — shared audit trail information for every entity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Audit trail metadata attached to every entity.
///
/// Every table row carries these columns:
/// - `created_by` / `created_at` — who and when created the record
/// - `updated_by` / `updated_at` — who and when last modified the record
///
/// Financial records are append-only (no `updated_*` changes after posting).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditInfo {
    /// UUID of the user who created this record.
    pub created_by: Uuid,
    /// Timestamp when this record was created.
    pub created_at: DateTime<Utc>,
    /// UUID of the user who last updated this record.
    pub updated_by: Uuid,
    /// Timestamp when this record was last updated.
    pub updated_at: DateTime<Utc>,
}

impl AuditInfo {
    /// Create a new AuditInfo with the given user as both creator and updater.
    pub fn new(created_by: Uuid) -> Self {
        let now = Utc::now();
        AuditInfo {
            created_by,
            created_at: now,
            updated_by: created_by,
            updated_at: now,
        }
    }

    /// Touch the audit info — mark as updated by the given user now.
    pub fn touch(&mut self, updated_by: Uuid) {
        self.updated_by = updated_by;
        self.updated_at = Utc::now();
    }
}

impl Default for AuditInfo {
    fn default() -> Self {
        AuditInfo {
            created_by: Uuid::nil(),
            created_at: Utc::now(),
            updated_by: Uuid::nil(),
            updated_at: Utc::now(),
        }
    }
}
