// ABOUTME: Git hook command implementations
// ABOUTME: Validates commit messages reference valid requirement IDs

use ralph_lib::{Prd, Result};
use std::fs;
use std::path::Path;
use std::process;

/// Configuration for commit-msg hook
pub struct CommitMsgConfig {
    pub file: String,
    pub verbose: bool,
}

/// Validate commit message references a requirement
pub fn commit_msg(config: &CommitMsgConfig) -> Result<()> {
    let message = fs::read_to_string(&config.file)?;

    if config.verbose {
        println!("Validating commit message from: {}", config.file);
    }

    // Check for requirement reference pattern
    let req_pattern = regex_lite::Regex::new(r"REQ-\d+").expect("valid regex");

    let refs: Vec<&str> = req_pattern
        .find_iter(&message)
        .map(|m| m.as_str())
        .collect();

    if refs.is_empty() {
        eprintln!("❌ Commit message must reference a requirement (e.g., REQ-01)");
        eprintln!();
        eprintln!("Examples of valid commit messages:");
        eprintln!("  REQ-01: Add user authentication endpoint");
        eprintln!("  Implement login flow (REQ-01)");
        eprintln!("  [REQ-01] Fix validation bug");
        process::exit(1);
    }

    // Verify requirement exists in some PRD
    let cwd = std::env::current_dir()?;
    let tasks_dir = cwd.join("ralph/tasks");

    if tasks_dir.exists() {
        let valid_reqs = collect_all_requirement_ids(&tasks_dir)?;

        for req_ref in &refs {
            if !valid_reqs.contains(&(*req_ref).to_string()) {
                eprintln!("⚠️  Warning: {req_ref} not found in any PRD");
            } else if config.verbose {
                println!("✅ Found valid requirement: {req_ref}");
            }
        }
    }

    if config.verbose {
        println!("✅ Commit message validation passed");
    }

    Ok(())
}

fn collect_all_requirement_ids(tasks_dir: &Path) -> Result<Vec<String>> {
    let mut ids = Vec::new();

    if let Ok(entries) = fs::read_dir(tasks_dir) {
        for entry in entries.flatten() {
            if entry.file_type()?.is_dir() {
                let prd_path = entry.path().join("prd.json");
                if prd_path.exists() {
                    if let Ok(prd) = Prd::from_file(&prd_path) {
                        for req in &prd.requirements {
                            ids.push(req.id.clone());
                        }
                    }
                }
            }
        }
    }

    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_commit_msg_with_valid_ref() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "REQ-01: Add feature").unwrap();

        let config = CommitMsgConfig {
            file: temp.path().to_string_lossy().to_string(),
            verbose: false,
        };

        // This should not exit(1) since there's a valid pattern
        // We can't fully test this without mocking process::exit
        let _ = config; // Just verify it compiles
    }
}
