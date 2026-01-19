// ABOUTME: Core library for Ralph CLI providing PRD automation functionality
// ABOUTME: Includes PRD parsing, validation, ledger management, and validation profiles

pub mod error;
pub mod ledger;
pub mod prd;
pub mod validation;

pub use error::RalphError;
pub use ledger::Ledger;
pub use prd::Prd;
pub use validation::ValidationProfile;

/// Result type alias using RalphError
pub type Result<T> = std::result::Result<T, RalphError>;

