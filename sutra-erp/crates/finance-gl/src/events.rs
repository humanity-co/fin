//! General Ledger domain events.
//!
//! These are the internal (data-only) event structs used within the GL module
//! for writing to the outbox. They mirror the public event types in `sutra-events`
//! but are owned by the GL module to avoid circular dependencies.

use chrono::{DateTime, Utc};
use serde::Serialize;

/// Internal event data for outbox serialization — owned by the GL module.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum GlEventData {
    JournalCreated {
        journal_id: String,
        journal_number: String,
        journal_type: String,
        status: String,
        created_by: String,
        occurred_at: DateTime<Utc>,
    },
    JournalPosted {
        journal_id: String,
        journal_number: String,
        total_debit: i64,
        total_credit: i64,
        period_id: String,
        posted_by: String,
        occurred_at: DateTime<Utc>,
    },
    JournalReversed {
        original_journal_id: String,
        reversing_journal_id: String,
        reason: String,
        reversed_by: String,
        occurred_at: DateTime<Utc>,
    },
}

/// Handle a GL event from the outbox (called by event dispatcher).
/// This is a placeholder for eventual read-model projection updates.
pub async fn handle_gl_event(event_type: &str, _payload: &serde_json::Value) {
    match event_type {
        "JournalCreated" => {
            tracing::debug!("GL event: JournalCreated");
        }
        "JournalPosted" => {
            tracing::debug!("GL event: JournalPosted — triggers trial balance refresh");
        }
        "JournalReversed" => {
            tracing::debug!("GL event: JournalReversed");
        }
        _ => {
            tracing::warn!("Unknown GL event type: {}", event_type);
        }
    }
}
