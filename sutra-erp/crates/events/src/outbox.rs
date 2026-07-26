//! OutboxMessage — persistent event record for the transactional outbox pattern.
//!
//! Every domain event is first persisted to the `event_outbox` table
//! within the same database transaction as the aggregate change.
//! A background worker polls the outbox and publishes events to Redis.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of an outbox message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutboxStatus {
    /// Not yet published.
    Pending,
    /// Successfully published to the event bus.
    Published,
    /// Publication failed after max retries.
    Failed,
    /// Manually moved to dead letter queue.
    DeadLettered,
}

/// A message stored in the transactional outbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxMessage {
    /// Unique ID of this outbox record.
    pub outbox_id: Uuid,
    /// The tenant that owns the event.
    pub tenant_id: Uuid,
    /// The aggregate type name (e.g., "Journal").
    pub aggregate_type: String,
    /// The aggregate instance ID.
    pub aggregate_id: String,
    /// The event type name (e.g., "JournalPosted").
    pub event_type: String,
    /// JSON-serialized event payload.
    pub event_payload: serde_json::Value,
    /// Correlation ID for tracing across services.
    pub correlation_id: Option<Uuid>,
    /// Current publication status.
    pub status: OutboxStatus,
    /// Number of publication attempts.
    pub retry_count: i32,
    /// Maximum retries before dead-lettering.
    pub max_retries: i32,
    /// Error message from the last failed attempt.
    pub last_error: Option<String>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When it was successfully published (if ever).
    pub published_at: Option<DateTime<Utc>>,
}

impl OutboxMessage {
    /// Create a new outbox message from a domain event.
    pub fn new<E: super::DomainEvent>(
        tenant_id: Uuid,
        event: &E,
    ) -> Self {
        let aggregate_id = event.aggregate_id();
        let event_type = event.event_type().to_string();
        let aggregate_type = E::aggregate_type().to_string();

        // Serialize the event to JSON value
        let event_payload = serde_json::to_value(event)
            .unwrap_or_else(|_| serde_json::Value::Null);

        OutboxMessage {
            outbox_id: Uuid::now_v7(),
            tenant_id,
            aggregate_type,
            aggregate_id,
            event_type,
            event_payload,
            correlation_id: None,
            status: OutboxStatus::Pending,
            retry_count: 0,
            max_retries: 5,
            last_error: None,
            created_at: Utc::now(),
            published_at: None,
        }
    }

    /// Mark this message as successfully published.
    pub fn mark_published(&mut self) {
        self.status = OutboxStatus::Published;
        self.published_at = Some(Utc::now());
    }

    /// Record a failed publish attempt.
    pub fn record_failure(&mut self, error: &str) {
        self.retry_count += 1;
        self.last_error = Some(error.to_string());
        if self.retry_count >= self.max_retries {
            self.status = OutboxStatus::Failed;
        }
    }
}
