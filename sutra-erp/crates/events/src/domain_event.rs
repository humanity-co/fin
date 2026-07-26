//! DomainEvent trait — implemented by all domain events.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt::Debug;

/// Trait that every domain event must implement.
///
/// Events are serializable (for outbox storage) and carry
/// metadata such as the aggregate type and ID they originated from.
pub trait DomainEvent: Debug + Serialize + Send + Sync + 'static {
    /// The type of aggregate that emitted this event (e.g., "Journal").
    fn aggregate_type() -> &'static str;

    /// The ID of the aggregate instance.
    fn aggregate_id(&self) -> String;

    /// The event type name (e.g., "JournalPosted").
    fn event_type(&self) -> &'static str;

    /// The event version for schema evolution.
    fn event_version(&self) -> u32 {
        1
    }

    /// When the event occurred.
    fn occurred_at(&self) -> DateTime<Utc>;
}
