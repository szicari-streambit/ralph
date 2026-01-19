// ABOUTME: Error types for Ralph operations
// ABOUTME: Defines RalphError enum covering all failure modes

use thiserror::Error;

/// Errors that can occur during Ralph operations
#[derive(Error, Debug)]
pub enum RalphError {
    /// I/O error reading or writing files
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing or serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// PRD schema validation failed
    #[error("PRD validation error: {0}")]
    PrdValidation(String),

    /// Ledger operation failed
    #[error("Ledger error: {0}")]
    Ledger(String),

    /// Validation profile error
    #[error("Validation profile error: {0}")]
    ValidationProfile(String),

    /// Command execution failed
    #[error("Command failed: {0}")]
    Command(String),

    /// Git operation failed
    #[error("Git error: {0}")]
    Git(String),

    /// Copilot CLI error
    #[error("Copilot error: {0}")]
    Copilot(String),
}

