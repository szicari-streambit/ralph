// ABOUTME: Append-only ledger for tracking implementation events
// ABOUTME: Supports JSONL format with optional AVRO serialization

use crate::{RalphError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Optional message or details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl LedgerEvent {
    /// Create a new event with current timestamp
    pub fn new(iteration: u32, requirement: impl Into<String>, status: EventStatus) -> Self {
        Self {
            timestamp: Utc::now(),
            iteration,
            requirement: requirement.into(),
            status,
            validation_passed: None,
            message: None,
        }
    }

    /// Set validation result
    pub fn with_validation(mut self, passed: bool) -> Self {
        self.validation_passed = Some(passed);
        self
    }

    /// Set message
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Append-only ledger for implementation events
#[derive(Debug, Default)]
pub struct Ledger {
    path: Option<std::path::PathBuf>,
    events: Vec<LedgerEvent>,
}

impl Ledger {
    /// Create a new empty in-memory ledger
    pub fn new() -> Self {
        Self {
            path: None,
            events: Vec::new(),
        }
    }

    /// Load an existing ledger from a JSONL file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut events = Vec::new();

        if path.exists() {
            let file = File::open(path)?;
            let reader = BufReader::new(file);

            for (line_num, line) in reader.lines().enumerate() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let event: LedgerEvent = serde_json::from_str(&line).map_err(|e| {
                    RalphError::Ledger(format!("Failed to parse line {}: {}", line_num + 1, e))
                })?;
                events.push(event);
            }
        }

        Ok(Self {
            path: Some(path.to_path_buf()),
            events,
        })
    }

    /// Create a new ledger at the given path (creates file if not exists)
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Create empty file if it doesn't exist
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(Self {
            path: Some(path.to_path_buf()),
            events: Vec::new(),
        })
    }

    /// Get all events
    pub fn events(&self) -> &[LedgerEvent] {
        &self.events
    }

    /// Append a new event to the ledger
    pub fn append(&mut self, event: LedgerEvent) -> Result<()> {
        // First, append to file atomically if we have a path
        if let Some(ref path) = self.path {
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;

            let json = serde_json::to_string(&event)?;
            writeln!(file, "{json}")?;
            file.flush()?;
        }

        // Then add to in-memory list
        self.events.push(event);
        Ok(())
    }

    /// Get the latest iteration number
    pub fn latest_iteration(&self) -> u32 {
        self.events.iter().map(|e| e.iteration).max().unwrap_or(0)
    }

    /// Get events for a specific requirement
    pub fn events_for_requirement(&self, req_id: &str) -> Vec<&LedgerEvent> {
        self.events
            .iter()
            .filter(|e| e.requirement == req_id)
            .collect()
    }

    /// Check if the last event for a requirement was a failure
    pub fn is_requirement_failed(&self, req_id: &str) -> bool {
        self.events_for_requirement(req_id)
            .last()
            .is_some_and(|e| e.status == EventStatus::Failed)
    }

    /// Get the count of iterations where full tests were run
    pub fn full_test_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.iteration % 5 == 0)
            .filter(|e| e.validation_passed.is_some())
            .count()
    }

    /// Export ledger to AVRO format for schema evolution
    pub fn to_avro(&self) -> Result<Vec<u8>> {
        use apache_avro::{types::Record, Schema, Writer};

        let schema = Schema::parse_str(LEDGER_AVRO_SCHEMA)
            .map_err(|e| RalphError::Ledger(format!("Invalid AVRO schema: {e}")))?;

        let mut writer = Writer::new(&schema, Vec::new());

        for event in &self.events {
            let mut record = Record::new(&schema)
                .ok_or_else(|| RalphError::Ledger("Failed to create AVRO record".to_string()))?;

            record.put("timestamp", event.timestamp.to_rfc3339());
            record.put("iteration", i64::from(event.iteration));
            record.put("requirement", event.requirement.clone());
            record.put(
                "status",
                match event.status {
                    EventStatus::Started => "started",
                    EventStatus::InProgress => "in_progress",
                    EventStatus::Done => "done",
                    EventStatus::Failed => "failed",
                },
            );
            record.put(
                "validationPassed",
                event.validation_passed.map(apache_avro::types::Value::Boolean),
            );
            record.put(
                "message",
                event.message.clone().map(apache_avro::types::Value::String),
            );

            writer
                .append(record)
                .map_err(|e| RalphError::Ledger(format!("Failed to write AVRO record: {e}")))?;
        }

        writer
            .into_inner()
            .map_err(|e| RalphError::Ledger(format!("Failed to finalize AVRO: {e}")))
    }

    /// Save ledger to AVRO file
    pub fn save_avro(&self, path: impl AsRef<Path>) -> Result<()> {
        let data = self.to_avro()?;
        std::fs::write(path, data)?;
        Ok(())
    }
}

/// AVRO schema for ledger events
pub const LEDGER_AVRO_SCHEMA: &str = r#"{
    "type": "record",
    "name": "LedgerEvent",
    "namespace": "com.ralph",
    "fields": [
        {"name": "timestamp", "type": "string"},
        {"name": "iteration", "type": "long"},
        {"name": "requirement", "type": "string"},
        {"name": "status", "type": {"type": "enum", "name": "EventStatus", "symbols": ["started", "in_progress", "done", "failed"]}},
        {"name": "validationPassed", "type": ["null", "boolean"], "default": null},
        {"name": "message", "type": ["null", "string"], "default": null}
    ]
}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn sample_event() -> LedgerEvent {
        LedgerEvent::new(1, "REQ-01", EventStatus::Started)
    }

    #[test]
    fn test_event_creation() {
        let event = sample_event();
        assert_eq!(event.iteration, 1);
        assert_eq!(event.requirement, "REQ-01");
        assert_eq!(event.status, EventStatus::Started);
        assert!(event.validation_passed.is_none());
    }

    #[test]
    fn test_event_with_validation() {
        let event = sample_event().with_validation(true);
        assert_eq!(event.validation_passed, Some(true));
    }

    #[test]
    fn test_event_with_message() {
        let event = sample_event().with_message("Test message");
        assert_eq!(event.message, Some("Test message".to_string()));
    }

    #[test]
    fn test_ledger_append_inmemory() {
        let mut ledger = Ledger::new();
        ledger.append(sample_event()).unwrap();
        assert_eq!(ledger.events().len(), 1);
    }

    #[test]
    fn test_ledger_file_roundtrip() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        // Create and append
        {
            let mut ledger = Ledger::create(path).unwrap();
            ledger.append(sample_event()).unwrap();
            ledger
                .append(LedgerEvent::new(2, "REQ-02", EventStatus::Done))
                .unwrap();
        }

        // Reload and verify
        let ledger = Ledger::from_file(path).unwrap();
        assert_eq!(ledger.events().len(), 2);
        assert_eq!(ledger.events()[0].requirement, "REQ-01");
        assert_eq!(ledger.events()[1].requirement, "REQ-02");
    }

    #[test]
    fn test_ledger_latest_iteration() {
        let mut ledger = Ledger::new();
        assert_eq!(ledger.latest_iteration(), 0);

        ledger.append(sample_event()).unwrap();
        assert_eq!(ledger.latest_iteration(), 1);

        ledger
            .append(LedgerEvent::new(5, "REQ-01", EventStatus::Done))
            .unwrap();
        assert_eq!(ledger.latest_iteration(), 5);
    }

    #[test]
    fn test_events_for_requirement() {
        let mut ledger = Ledger::new();
        ledger.append(sample_event()).unwrap();
        ledger
            .append(LedgerEvent::new(2, "REQ-02", EventStatus::Started))
            .unwrap();
        ledger
            .append(LedgerEvent::new(3, "REQ-01", EventStatus::Done))
            .unwrap();

        let req1_events = ledger.events_for_requirement("REQ-01");
        assert_eq!(req1_events.len(), 2);
    }

    #[test]
    fn test_is_requirement_failed() {
        let mut ledger = Ledger::new();
        ledger.append(sample_event()).unwrap();
        assert!(!ledger.is_requirement_failed("REQ-01"));

        ledger
            .append(LedgerEvent::new(2, "REQ-01", EventStatus::Failed))
            .unwrap();
        assert!(ledger.is_requirement_failed("REQ-01"));
    }

    #[test]
    fn test_event_serialization() {
        let event = sample_event().with_validation(true);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"iteration\":1"));
        assert!(json.contains("\"requirement\":\"REQ-01\""));
        assert!(json.contains("\"status\":\"started\""));
        assert!(json.contains("\"validationPassed\":true"));
    }

    #[test]
    fn test_avro_serialization() {
        let mut ledger = Ledger::new();
        ledger.append(sample_event()).unwrap();
        ledger
            .append(
                LedgerEvent::new(2, "REQ-01", EventStatus::Done)
                    .with_validation(true)
                    .with_message("Completed successfully"),
            )
            .unwrap();

        let avro_data = ledger.to_avro().unwrap();
        assert!(!avro_data.is_empty());
        // AVRO data should have the magic bytes and schema
        assert!(avro_data.len() > 10);
    }

    #[test]
    fn test_avro_file_roundtrip() {
        let temp = NamedTempFile::new().unwrap();
        let mut ledger = Ledger::new();
        ledger.append(sample_event()).unwrap();
        ledger.save_avro(temp.path()).unwrap();

        let data = std::fs::read(temp.path()).unwrap();
        assert!(!data.is_empty());
    }
}
