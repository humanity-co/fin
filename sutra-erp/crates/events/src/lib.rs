//! SutraERP — Event Definitions & Outbox Infrastructure
//!
//! This crate defines the `DomainEvent` trait, the `OutboxMessage` struct,
//! and per-module event enums. It is the contract layer for the
//! transactional-outbox event-driven architecture.

pub mod domain_event;
pub mod events;
pub mod outbox;

// Re-exports
pub use domain_event::DomainEvent;
pub use events::*;
pub use outbox::OutboxMessage;
