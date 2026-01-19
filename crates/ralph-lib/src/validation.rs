// ABOUTME: Validation profile system for project-specific checks
// ABOUTME: Supports detection rules and command execution (fmt, lint, typecheck, test)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Detection rules for a validation profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectRules {
    /// Profile applies if any of these files exist
    #[serde(default)]
    pub any_files_exist: Vec<String>,
}

/// Commands for each validation stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCommands {
    /// Format check commands
    #[serde(default)]
    pub fmt: Vec<String>,
    /// Lint commands
    #[serde(default)]
    pub lint: Vec<String>,
    /// Type check commands
    #[serde(default)]
    pub typecheck: Vec<String>,
    /// Test commands
    #[serde(default)]
    pub test: Vec<String>,
}

/// A validation profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationProfile {
    /// Rules for detecting if this profile applies
    pub detect: DetectRules,
    /// Commands to run for validation
    pub commands: ProfileCommands,
}

/// Container for all validation profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationConfig {
    /// Schema version
    pub schema_version: String,
    /// Named profiles
    pub profiles: HashMap<String, ValidationProfile>,
}

