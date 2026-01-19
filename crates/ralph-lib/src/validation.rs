// ABOUTME: Validation profile system for project-specific checks
// ABOUTME: Supports detection rules and command execution (fmt, lint, typecheck, test)

use crate::{RalphError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Output};

/// Detection rules for a validation profile
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectRules {
    /// Profile applies if any of these files exist
    #[serde(default)]
    pub any_files_exist: Vec<String>,
}

impl DetectRules {
    /// Check if the detection rules match for the given directory
    pub fn matches(&self, dir: impl AsRef<Path>) -> bool {
        let dir = dir.as_ref();
        self.any_files_exist
            .iter()
            .any(|file| dir.join(file).exists())
    }
}

/// Commands for each validation stage
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// Result of running a validation command
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// The stage that was run
    pub stage: ValidationStage,
    /// Whether the command succeeded
    pub success: bool,
    /// Combined stdout and stderr
    pub output: String,
    /// Exit code if available
    pub exit_code: Option<i32>,
}

/// Validation stages in order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStage {
    Fmt,
    Lint,
    Typecheck,
    Test,
}

impl ValidationStage {
    /// Get all stages in order
    pub fn all() -> &'static [Self] {
        &[Self::Fmt, Self::Lint, Self::Typecheck, Self::Test]
    }

    /// Get short-circuit stages (no test)
    pub fn short_circuit() -> &'static [Self] {
        &[Self::Fmt, Self::Lint, Self::Typecheck]
    }
}

/// A validation profile configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationProfile {
    /// Rules for detecting if this profile applies
    pub detect: DetectRules,
    /// Commands to run for validation
    pub commands: ProfileCommands,
}

impl ValidationProfile {
    /// Get commands for a specific stage
    pub fn commands_for_stage(&self, stage: ValidationStage) -> &[String] {
        match stage {
            ValidationStage::Fmt => &self.commands.fmt,
            ValidationStage::Lint => &self.commands.lint,
            ValidationStage::Typecheck => &self.commands.typecheck,
            ValidationStage::Test => &self.commands.test,
        }
    }

    /// Run validation commands for a stage
    pub fn run_stage(&self, stage: ValidationStage, cwd: impl AsRef<Path>) -> ValidationResult {
        let commands = self.commands_for_stage(stage);
        let cwd = cwd.as_ref();

        for cmd_str in commands {
            let result = run_shell_command(cmd_str, cwd);
            match result {
                Ok(output) => {
                    if !output.status.success() {
                        return ValidationResult {
                            stage,
                            success: false,
                            output: String::from_utf8_lossy(&output.stdout).to_string()
                                + &String::from_utf8_lossy(&output.stderr),
                            exit_code: output.status.code(),
                        };
                    }
                }
                Err(e) => {
                    return ValidationResult {
                        stage,
                        success: false,
                        output: e.to_string(),
                        exit_code: None,
                    };
                }
            }
        }

        ValidationResult {
            stage,
            success: true,
            output: String::new(),
            exit_code: Some(0),
        }
    }

    /// Run all validation stages with short-circuit on failure
    /// If `include_tests` is true, runs all stages. Otherwise skips test stage.
    pub fn run_all(
        &self,
        cwd: impl AsRef<Path>,
        include_tests: bool,
    ) -> Vec<ValidationResult> {
        let cwd = cwd.as_ref();
        let stages = if include_tests {
            ValidationStage::all()
        } else {
            ValidationStage::short_circuit()
        };

        let mut results = Vec::new();
        for &stage in stages {
            let result = self.run_stage(stage, cwd);
            let success = result.success;
            results.push(result);
            if !success {
                break; // Short-circuit on failure
            }
        }
        results
    }
}

/// Run a shell command in the given directory
fn run_shell_command(cmd: &str, cwd: &Path) -> std::io::Result<Output> {
    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(cwd)
        .output()
}

/// Container for all validation profiles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationConfig {
    /// Schema version
    pub schema_version: String,
    /// Named profiles
    pub profiles: HashMap<String, ValidationProfile>,
}

impl ValidationConfig {
    /// Load validation config from a JSON file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::from_json(&content)
    }

    /// Parse validation config from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(RalphError::from)
    }

    /// Detect which profiles apply to the given directory
    pub fn detect_profiles(&self, dir: impl AsRef<Path>) -> Vec<&str> {
        let dir = dir.as_ref();
        self.profiles
            .iter()
            .filter(|(_, profile)| profile.detect.matches(dir))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get a profile by name
    pub fn get(&self, name: &str) -> Option<&ValidationProfile> {
        self.profiles.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_config() -> ValidationConfig {
        let json = r#"{
            "schemaVersion": "1.0",
            "profiles": {
                "rust-cargo": {
                    "detect": { "anyFilesExist": ["Cargo.toml"] },
                    "commands": {
                        "fmt": ["cargo fmt --all -- --check"],
                        "lint": ["cargo clippy -- -D warnings"],
                        "typecheck": ["cargo check"],
                        "test": ["cargo test"]
                    }
                },
                "node-npm": {
                    "detect": { "anyFilesExist": ["package.json"] },
                    "commands": {
                        "fmt": ["npm run fmt"],
                        "lint": ["npm run lint"],
                        "typecheck": ["npm run typecheck"],
                        "test": ["npm test"]
                    }
                }
            }
        }"#;
        ValidationConfig::from_json(json).unwrap()
    }

    #[test]
    fn test_config_parsing() {
        let config = sample_config();
        assert_eq!(config.schema_version, "1.0");
        assert_eq!(config.profiles.len(), 2);
        assert!(config.profiles.contains_key("rust-cargo"));
    }

    #[test]
    fn test_detect_rules_matches() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();

        let rules = DetectRules {
            any_files_exist: vec!["Cargo.toml".to_string()],
        };
        assert!(rules.matches(dir.path()));

        let rules2 = DetectRules {
            any_files_exist: vec!["package.json".to_string()],
        };
        assert!(!rules2.matches(dir.path()));
    }

    #[test]
    fn test_detect_profiles() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();

        let config = sample_config();
        let detected = config.detect_profiles(dir.path());
        assert_eq!(detected.len(), 1);
        assert!(detected.contains(&"rust-cargo"));
    }

    #[test]
    fn test_run_stage_success() {
        let profile = ValidationProfile {
            detect: DetectRules::default(),
            commands: ProfileCommands {
                fmt: vec!["echo 'ok'".to_string()],
                ..Default::default()
            },
        };

        let result = profile.run_stage(ValidationStage::Fmt, ".");
        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn test_run_stage_failure() {
        let profile = ValidationProfile {
            detect: DetectRules::default(),
            commands: ProfileCommands {
                fmt: vec!["exit 1".to_string()],
                ..Default::default()
            },
        };

        let result = profile.run_stage(ValidationStage::Fmt, ".");
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
    }

    #[test]
    fn test_run_all_short_circuits() {
        let profile = ValidationProfile {
            detect: DetectRules::default(),
            commands: ProfileCommands {
                fmt: vec!["echo 'fmt ok'".to_string()],
                lint: vec!["exit 1".to_string()],
                typecheck: vec!["echo 'should not run'".to_string()],
                test: vec!["echo 'should not run'".to_string()],
            },
        };

        let results = profile.run_all(".", false);
        assert_eq!(results.len(), 2); // fmt + lint, then short-circuit
        assert!(results[0].success);
        assert!(!results[1].success);
    }

    #[test]
    fn test_validation_stage_iterators() {
        assert_eq!(ValidationStage::all().len(), 4);
        assert_eq!(ValidationStage::short_circuit().len(), 3);
    }
}
