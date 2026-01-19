// ABOUTME: PRD (Product Requirements Document) data structures and parsing
// ABOUTME: Matches the JSON Schema defined in schemas/prd.schema.json

use serde::{Deserialize, Serialize};

/// Status of a requirement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
}

/// A single requirement in a PRD
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    /// Unique identifier (e.g., "REQ-01")
    pub id: String,
    /// Short title
    pub title: String,
    /// Current status
    pub status: RequirementStatus,
    /// Acceptance criteria (Given/When/Then format)
    pub acceptance_criteria: Vec<String>,
}

/// Product Requirements Document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prd {
    /// Schema version
    pub schema_version: String,
    /// URL-safe identifier
    pub slug: String,
    /// Human-readable title
    pub title: String,
    /// Current run identifier
    pub active_run_id: String,
    /// Validation profiles to use
    pub validation_profiles: Vec<String>,
    /// List of requirements
    pub requirements: Vec<Requirement>,
}

