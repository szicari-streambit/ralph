// ABOUTME: PRD (Product Requirements Document) data structures and parsing
// ABOUTME: Matches the JSON Schema defined in schemas/prd.schema.json

use crate::{RalphError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Status of a requirement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequirementStatus {
    Todo,
    #[serde(rename = "in_progress")]
    InProgress,
    Done,
    Blocked,
}

/// A single requirement in a PRD
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl Prd {
    /// Load a PRD from a JSON file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid JSON.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::from_json(&content)
    }

    /// Parse a PRD from a JSON string
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is invalid.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(RalphError::from)
    }

    /// Serialize the PRD to a JSON string
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(RalphError::from)
    }

    /// Serialize the PRD to a pretty-printed JSON string
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_json_pretty(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(RalphError::from)
    }

    /// Save the PRD to a JSON file
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the file cannot be written.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = self.to_json_pretty()?;
        std::fs::write(path.as_ref(), json)?;
        Ok(())
    }

    /// Validate this PRD against the JSON schema
    ///
    /// # Errors
    ///
    /// Returns an error if the schema file cannot be read or validation fails.
    pub fn validate_schema(&self, schema_path: impl AsRef<Path>) -> Result<()> {
        let schema_content = std::fs::read_to_string(schema_path.as_ref())?;
        let schema: serde_json::Value = serde_json::from_str(&schema_content)?;
        let instance = serde_json::to_value(self)?;

        let compiled = jsonschema::JSONSchema::compile(&schema)
            .map_err(|e| RalphError::PrdValidation(format!("Invalid schema: {e}")))?;

        if let Err(errors) = compiled.validate(&instance) {
            let messages: Vec<String> = errors.map(|e| e.to_string()).collect();
            return Err(RalphError::PrdValidation(messages.join("; ")));
        }
        Ok(())
    }

    /// Generate markdown documentation for this PRD
    #[must_use]
    pub fn to_markdown(&self) -> String {
        use std::fmt::Write;
        let mut md = String::new();
        let _ = writeln!(md, "# {}\n", self.title);
        let _ = writeln!(md, "**Slug:** `{}`\n", self.slug);
        let _ = writeln!(md, "**Run ID:** `{}`\n", self.active_run_id);
        let _ = writeln!(
            md,
            "**Validation Profiles:** {}\n",
            self.validation_profiles.join(", ")
        );
        md.push_str("## Requirements\n\n");

        for req in &self.requirements {
            let status_icon = match req.status {
                RequirementStatus::Todo => "⬜",
                RequirementStatus::InProgress => "🔄",
                RequirementStatus::Done => "✅",
                RequirementStatus::Blocked => "🚫",
            };
            let _ = writeln!(md, "### {} {} - {}\n", status_icon, req.id, req.title);
            md.push_str("**Acceptance Criteria:**\n\n");
            for ac in &req.acceptance_criteria {
                let _ = writeln!(md, "- {ac}");
            }
            md.push('\n');
        }
        md
    }

    /// Update requirement status by ID
    pub fn update_requirement_status(&mut self, req_id: &str, status: RequirementStatus) -> bool {
        if let Some(req) = self.requirements.iter_mut().find(|r| r.id == req_id) {
            req.status = status;
            true
        } else {
            false
        }
    }

    /// Generate markdown with RALPH markers for managed sections
    #[must_use]
    pub fn to_markdown_with_markers(&self, planning_log: Option<&str>) -> String {
        use std::fmt::Write;
        let mut md = String::new();
        let _ = writeln!(md, "# {}\n", self.title);
        let _ = writeln!(
            md,
            "Canonical machine PRD: ralph/tasks/{}/prd.json\n",
            self.slug
        );

        // Planning log section
        md.push_str("<!-- RALPH:BEGIN PLANNING_LOG -->\n");
        if let Some(log) = planning_log {
            md.push_str(log);
            if !log.ends_with('\n') {
                md.push('\n');
            }
        }
        md.push_str("<!-- RALPH:END PLANNING_LOG -->\n");

        md
    }

    /// Save markdown PRD to a file
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or the file cannot be written.
    pub fn save_markdown(&self, path: impl AsRef<Path>, planning_log: Option<&str>) -> Result<()> {
        let md = self.to_markdown_with_markers(planning_log);
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path.as_ref(), md)?;
        Ok(())
    }
}

/// Manages markdown files with RALPH markers
pub struct MarkdownPrd {
    content: String,
}

impl MarkdownPrd {
    /// Load from an existing markdown file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Ok(Self { content })
    }

    /// Create with initial content
    #[must_use]
    pub fn new(content: String) -> Self {
        Self { content }
    }

    /// Get the current content
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Extract content from a marked section
    #[must_use]
    pub fn get_section(&self, marker: &str) -> Option<&str> {
        let begin = format!("<!-- RALPH:BEGIN {marker} -->");
        let end = format!("<!-- RALPH:END {marker} -->");

        let start_idx = self.content.find(&begin)?;
        let end_idx = self.content.find(&end)?;

        if start_idx < end_idx {
            let content_start = start_idx + begin.len();
            let section = &self.content[content_start..end_idx];
            Some(section.trim())
        } else {
            None
        }
    }

    /// Append to a marked section (preserves existing content)
    pub fn append_to_section(&mut self, marker: &str, text: &str) {
        use std::fmt::Write;
        let begin = format!("<!-- RALPH:BEGIN {marker} -->");
        let end = format!("<!-- RALPH:END {marker} -->");

        if let Some(end_idx) = self.content.find(&end) {
            // Insert before the END marker
            let insert_pos = end_idx;
            let new_line = if self.content[..insert_pos].ends_with('\n') {
                ""
            } else {
                "\n"
            };
            self.content
                .insert_str(insert_pos, &format!("{new_line}{text}\n"));
        } else {
            // Section doesn't exist - add it at the end
            let _ = write!(self.content, "\n{begin}\n{text}\n{end}\n");
        }
    }

    /// Save to file
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or the file cannot be written.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path.as_ref(), &self.content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn sample_prd() -> Prd {
        Prd {
            schema_version: "1.0".to_string(),
            slug: "test-feature".to_string(),
            title: "Test Feature".to_string(),
            active_run_id: "test-20260119-1".to_string(),
            validation_profiles: vec!["rust-cargo".to_string()],
            requirements: vec![Requirement {
                id: "REQ-01".to_string(),
                title: "Test requirement".to_string(),
                status: RequirementStatus::Todo,
                acceptance_criteria: vec!["Given X, when Y, then Z".to_string()],
            }],
        }
    }

    #[test]
    fn test_prd_roundtrip_json() {
        let prd = sample_prd();
        let json = prd.to_json().unwrap();
        let parsed = Prd::from_json(&json).unwrap();
        assert_eq!(prd, parsed);
    }

    #[test]
    fn test_prd_file_roundtrip() {
        let prd = sample_prd();
        let temp = NamedTempFile::new().unwrap();
        prd.save(temp.path()).unwrap();
        let loaded = Prd::from_file(temp.path()).unwrap();
        assert_eq!(prd, loaded);
    }

    #[test]
    fn test_requirement_status_serialization() {
        assert_eq!(
            serde_json::to_string(&RequirementStatus::Todo).unwrap(),
            "\"todo\""
        );
        assert_eq!(
            serde_json::to_string(&RequirementStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&RequirementStatus::Done).unwrap(),
            "\"done\""
        );
        assert_eq!(
            serde_json::to_string(&RequirementStatus::Blocked).unwrap(),
            "\"blocked\""
        );
    }

    #[test]
    fn test_prd_to_markdown() {
        let prd = sample_prd();
        let md = prd.to_markdown();
        assert!(md.contains("# Test Feature"));
        assert!(md.contains("**Slug:** `test-feature`"));
        assert!(md.contains("⬜ REQ-01"));
    }

    #[test]
    fn test_update_requirement_status() {
        let mut prd = sample_prd();
        assert!(prd.update_requirement_status("REQ-01", RequirementStatus::Done));
        assert_eq!(prd.requirements[0].status, RequirementStatus::Done);
        assert!(!prd.update_requirement_status("REQ-99", RequirementStatus::Done));
    }

    #[test]
    fn test_parse_example_prd() {
        let json = r#"{"schemaVersion":"1.0","slug":"example-feature","title":"Example feature","activeRunId":"example-20260119-1","validationProfiles":["rust-cargo"],"requirements":[{"id":"REQ-01","title":"Add endpoint","status":"todo","acceptanceCriteria":["Given valid request, when calling POST /v1/example, then returns 200"]}]}"#;
        let prd = Prd::from_json(json).unwrap();
        assert_eq!(prd.slug, "example-feature");
        assert_eq!(prd.requirements.len(), 1);
        assert_eq!(prd.requirements[0].status, RequirementStatus::Todo);
    }

    #[test]
    fn test_to_markdown_with_markers() {
        let prd = sample_prd();
        let md = prd.to_markdown_with_markers(Some("Initial planning notes"));
        assert!(md.contains("# Test Feature"));
        assert!(md.contains("Canonical machine PRD: ralph/tasks/test-feature/prd.json"));
        assert!(md.contains("<!-- RALPH:BEGIN PLANNING_LOG -->"));
        assert!(md.contains("Initial planning notes"));
        assert!(md.contains("<!-- RALPH:END PLANNING_LOG -->"));
    }

    #[test]
    fn test_markdown_prd_get_section() {
        let content = "# Title\n\n<!-- RALPH:BEGIN PLANNING_LOG -->\nSome notes\n<!-- RALPH:END PLANNING_LOG -->\n";
        let md = MarkdownPrd::new(content.to_string());
        assert_eq!(md.get_section("PLANNING_LOG"), Some("Some notes"));
    }

    #[test]
    fn test_markdown_prd_append_to_section() {
        let content = "# Title\n\n<!-- RALPH:BEGIN PLANNING_LOG -->\nFirst note\n<!-- RALPH:END PLANNING_LOG -->\n";
        let mut md = MarkdownPrd::new(content.to_string());
        md.append_to_section("PLANNING_LOG", "Second note");
        assert!(md.content().contains("First note"));
        assert!(md.content().contains("Second note"));
    }

    #[test]
    fn test_markdown_prd_append_creates_section() {
        let content = "# Title\n";
        let mut md = MarkdownPrd::new(content.to_string());
        md.append_to_section("NEW_SECTION", "New content");
        assert!(md.content().contains("<!-- RALPH:BEGIN NEW_SECTION -->"));
        assert!(md.content().contains("New content"));
        assert!(md.content().contains("<!-- RALPH:END NEW_SECTION -->"));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_requirement_status() -> impl Strategy<Value = RequirementStatus> {
        prop_oneof![
            Just(RequirementStatus::Todo),
            Just(RequirementStatus::InProgress),
            Just(RequirementStatus::Done),
            Just(RequirementStatus::Blocked),
        ]
    }

    fn arb_requirement() -> impl Strategy<Value = Requirement> {
        (
            "[A-Z]{3}-[0-9]{2}",
            "[a-z ]{5,20}",
            arb_requirement_status(),
            prop::collection::vec("[a-z ]{10,30}", 1..3),
        )
            .prop_map(|(id, title, status, criteria)| Requirement {
                id,
                title,
                status,
                acceptance_criteria: criteria,
            })
    }

    fn arb_prd() -> impl Strategy<Value = Prd> {
        (
            "[a-z-]{5,15}",
            "[A-Za-z ]{5,20}",
            "[a-z0-9-]{10,20}",
            prop::collection::vec(arb_requirement(), 1..5),
        )
            .prop_map(|(slug, title, run_id, requirements)| Prd {
                schema_version: "1.0".to_string(),
                slug,
                title,
                active_run_id: run_id,
                validation_profiles: vec!["rust-cargo".to_string()],
                requirements,
            })
    }

    proptest! {
        #[test]
        fn prd_json_roundtrip(prd in arb_prd()) {
            let json = prd.to_json().unwrap();
            let parsed = Prd::from_json(&json).unwrap();
            prop_assert_eq!(prd, parsed);
        }

        #[test]
        fn requirement_status_roundtrip(status in arb_requirement_status()) {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: RequirementStatus = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(status, parsed);
        }

        #[test]
        fn prd_markdown_contains_requirements(prd in arb_prd()) {
            let md = prd.to_markdown();
            for req in &prd.requirements {
                prop_assert!(md.contains(&req.id));
            }
        }
    }
}
