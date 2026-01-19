// ABOUTME: 'ralph init' command implementation
// ABOUTME: Initializes a new Ralph project with templates and directory structure

use ralph_lib::Result;
use std::fs;
use std::path::Path;

/// Configuration for init command
pub struct InitConfig {
    pub dry_run: bool,
    pub verbose: bool,
}

/// Initialize a new Ralph project
pub fn run(config: &InitConfig) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if config.verbose {
        println!("Initializing Ralph project in {}", cwd.display());
    }

    // Create directory structure
    let dirs = [
        "ralph/tasks",
        "docs/ralph",
        ".github/agents",
        ".githooks",
    ];

    for dir in &dirs {
        let path = cwd.join(dir);
        if config.dry_run {
            println!("[dry-run] Would create directory: {}", path.display());
        } else {
            fs::create_dir_all(&path)?;
            if config.verbose {
                println!("Created directory: {}", path.display());
            }
        }
    }

    // Create template files
    create_template_file(
        &cwd,
        ".github/agents/ralph-planner.agent.md",
        RALPH_PLANNER_TEMPLATE,
        config,
    )?;

    create_template_file(
        &cwd,
        ".github/agents/ralph-implementer.agent.md",
        RALPH_IMPLEMENTER_TEMPLATE,
        config,
    )?;

    create_template_file(
        &cwd,
        ".githooks/commit-msg",
        COMMIT_MSG_HOOK_TEMPLATE,
        config,
    )?;

    // Create validation.json if it doesn't exist
    let validation_path = cwd.join("ralph/validation.json");
    if !validation_path.exists() || config.dry_run {
        create_template_file(&cwd, "ralph/validation.json", VALIDATION_JSON_TEMPLATE, config)?;
    }

    // Set commit-msg hook as executable
    if !config.dry_run {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let hook_path = cwd.join(".githooks/commit-msg");
            if hook_path.exists() {
                let mut perms = fs::metadata(&hook_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&hook_path, perms)?;
            }
        }
    }

    if !config.dry_run {
        println!("✅ Ralph project initialized successfully!");
        println!();
        println!("Next steps:");
        println!("  1. Run: git config core.hooksPath .githooks");
        println!("  2. Create a feature: ralph plan <feature-slug>");
    }

    Ok(())
}

fn create_template_file(
    base: &Path,
    relative_path: &str,
    content: &str,
    config: &InitConfig,
) -> Result<()> {
    let path = base.join(relative_path);

    if config.dry_run {
        println!("[dry-run] Would create file: {}", path.display());
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, content)?;

    if config.verbose {
        println!("Created file: {}", path.display());
    }

    Ok(())
}

const RALPH_PLANNER_TEMPLATE: &str = r#"---
name: ralph-planner
tools: ["read", "search", "edit"]
---
Minimum 10 clarifying questions before first PRD draft. Never assume. Architecture + diagrams.
Planning is re-entrant: append to Planning Log, rewrite managed blocks. Planning Log is append-only.
"#;

const RALPH_IMPLEMENTER_TEMPLATE: &str = r#"---
name: ralph-implementer
tools: ["read", "search", "edit", "shell"]
---
Implement one requirement per iteration. Update PRD status only after validation passes.
Append one ledger event per iteration. Run fmt -> lint -> typecheck (short-circuit on failure).
Full test sweep every 5th iteration.
"#;

const COMMIT_MSG_HOOK_TEMPLATE: &str = r#"#!/usr/bin/env bash
set -euo pipefail
exec ralph hook commit-msg "$1"
"#;

const VALIDATION_JSON_TEMPLATE: &str = r#"{
  "schemaVersion": "1.0",
  "profiles": {
    "rust-cargo": {
      "detect": { "anyFilesExist": ["Cargo.toml"] },
      "commands": {
        "fmt": ["cargo fmt --all -- --check"],
        "lint": ["cargo clippy --all-targets --all-features -- -D warnings"],
        "typecheck": ["cargo check --all-targets --all-features"],
        "test": ["cargo test --all-features"]
      }
    }
  }
}
"#;

