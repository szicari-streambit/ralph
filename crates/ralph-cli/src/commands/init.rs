// ABOUTME: 'ralph init' command implementation
// ABOUTME: Initializes a new Ralph project with templates and directory structure

use ralph_lib::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

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

    // Detect git root for placing agent files and create directory structure for project
    // Determine git root: prefer `git rev-parse --show-toplevel`, fall back to searching parents for .git
    let git_root = match Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(out) if out.status.success() => {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            std::path::PathBuf::from(path)
        }
        _ => {
            // Walk parents looking for .git directory
            let mut dir = cwd.clone();
            let mut found = None;
            loop {
                if dir.join(".git").exists() {
                    found = Some(dir.clone());
                    break;
                }
                if !dir.pop() {
                    break;
                }
            }
            if let Some(p) = found {
                p
            } else {
                // Not inside a git repo — initialize one in the current directory (unless dry-run)
                if !config.dry_run {
                    let init = Command::new("git").arg("init").current_dir(&cwd).output()?;
                    if !init.status.success() {
                        return Err(ralph_lib::RalphError::Command(format!(
                            "Failed to initialize git repository: {}",
                            String::from_utf8_lossy(&init.stderr)
                        )));
                    }
                } else if config.verbose {
                    println!(
                        "(dry-run) Would initialize a new git repository in {}",
                        cwd.display()
                    );
                }
                cwd.clone()
            }
        }
    };

    if config.verbose {
        println!("Detected git root: {}", git_root.display());
    }

    let dirs = ["ralph/tasks", "docs/ralph", ".githooks"];

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

    // Create template files. If RALPH_SHARE_DIR is set, prefer files from $RALPH_SHARE_DIR/templates/.github/agents/)
    // otherwise fall back to embedded templates.
    let share_dir = std::env::var("RALPH_SHARE_DIR").ok();

    // Helper to try reading a file from share_dir and write it; returns Ok(true) if used
    fn try_use_shared(
        share_dir: &str,
        rel: &str,
        dest: &std::path::Path,
        verbose: bool,
    ) -> Result<bool> {
        let shared_path = std::path::Path::new(share_dir).join("templates").join(rel);
        if shared_path.exists() {
            // If destination already exists, do not overwrite. In verbose mode indicate skip.
            if dest.exists() {
                if verbose {
                    println!("Skipped existing file: {}", dest.display());
                }
                return Ok(true);
            }
            if verbose {
                println!("Using shared template: {}", shared_path.display());
            }
            let content = std::fs::read_to_string(&shared_path)?;
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(dest, content)?;
            return Ok(true);
        }
        Ok(false)
    }

    let planner_dest = git_root.join(".github/agents/ralph-planner.agent.md");
    let implementer_dest = git_root.join(".github/agents/ralph-implementer.agent.md");

    if let Some(ref sd) = share_dir {
        // Try to copy both agent files from shared dir. If any missing, return error.
        let planner_ok = try_use_shared(
            sd,
            ".github/agents/ralph-planner.agent.md",
            &planner_dest,
            config.verbose,
        )?;
        let implementer_ok = try_use_shared(
            sd,
            ".github/agents/ralph-implementer.agent.md",
            &implementer_dest,
            config.verbose,
        )?;
        if !planner_ok || !implementer_ok {
            return Err(ralph_lib::RalphError::Command(format!("RALPH_SHARE_DIR is set to '{}' but agent files not found in templates/.github/agents/", sd)));
        }
    } else {
        // use embedded templates
        create_template_file(
            &git_root,
            ".github/agents/ralph-planner.agent.md",
            RALPH_PLANNER_TEMPLATE,
            config,
        )?;
        create_template_file(
            &git_root,
            ".github/agents/ralph-implementer.agent.md",
            RALPH_IMPLEMENTER_TEMPLATE,
            config,
        )?;
    }

    create_template_file(
        &cwd,
        ".githooks/commit-msg",
        COMMIT_MSG_HOOK_TEMPLATE,
        config,
    )?;

    // Create validation.json if it doesn't exist
    let validation_path = cwd.join("ralph/validation.json");
    if !validation_path.exists() || config.dry_run {
        create_template_file(
            &cwd,
            "ralph/validation.json",
            VALIDATION_JSON_TEMPLATE,
            config,
        )?;
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
        println!("Planner and Implementer agents installed");
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

    // If file already exists, do not overwrite. In verbose mode indicate it was skipped.
    if path.exists() {
        if config.verbose {
            println!("Skipped existing file: {}", path.display());
        }
        return Ok(());
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
tools: ["read", "search", "edit", "execute"]
---

CRITICAL: You MUST ensure your work passes the repository's validation profiles BEFORE finishing each iteration.

The outer loop will run validation checks as defined in ralph/validation.json (profiles: fmt, lint, typecheck, test).
If validation fails, read the validation output carefully, fix root causes, and re-run the validation commands specified by the active profile.

Implement one requirement per iteration. Update PRD status only after validation passes.
Append one ledger event per iteration. Full test sweep every 5th iteration.
"#;

const COMMIT_MSG_HOOK_TEMPLATE: &str = r#"#!/usr/bin/env bash
set -euo pipefail
exec ralph hook commit-msg "$1"
"#;

const VALIDATION_JSON_TEMPLATE: &str = r#"{
  "schemaVersion": "1.0",
  "profiles": {
    "default": {
      "detect": { "anyFilesExist": [] },
      "commands": {
        "fmt": [],
        "lint": [],
        "typecheck": [],
        "test": []
      }
    }
  }
}
"#;
