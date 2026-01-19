// ABOUTME: Append-only ledger for tracking implementation events
// ABOUTME: Supports JSONL format with optional AVRO serialization

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a ledger event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Started,
    InProgress,
    Done,
    Failed,
}

/// A single event in the ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEvent {
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Iteration number (1-based)
    pub iteration: u32,
    /// Requirement ID this event relates to
    pub requirement: String,
    /// Status of the event
    pub status: EventStatus,
    /// Whether validation passed (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_passed: Option<bool>,
}

/// Append-only ledger for implementation events
#[derive(Debug, Default)]
pub struct Ledger {
    events: Vec<LedgerEvent>,
}

impl Ledger {
    /// Create a new empty ledger
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Get all events
    pub fn events(&self) -> &[LedgerEvent] {
        &self.events
    }
}

