// ABOUTME: Core library for Ralph CLI providing PRD automation functionality
// ABOUTME: Includes PRD parsing, validation, ledger management, and validation profiles

pub mod error;
pub mod ledger;
pub mod prd;
pub mod validation;

pub use error::RalphError;
pub use ledger::{EventStatus, Ledger, LedgerEvent};
pub use prd::{MarkdownPrd, Prd, Requirement, RequirementStatus};
pub use validation::{ValidationConfig, ValidationProfile, ValidationResult, ValidationStage};

/// Result type alias using [`RalphError`]
pub type Result<T> = std::result::Result<T, RalphError>;
