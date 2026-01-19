// ABOUTME: 'ralph plan' command implementation
// ABOUTME: Launches interactive planning session with GitHub Copilot CLI

use ralph_lib::{MarkdownPrd, Prd, Requirement, RequirementStatus, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Configuration for plan command
pub struct PlanConfig {
    pub slug: String,
    pub dry_run: bool,
    pub verbose: bool,
}

/// Start or resume a planning session
pub fn run(config: PlanConfig) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let task_dir = cwd.join("ralph/tasks").join(&config.slug);
    let prd_path = task_dir.join("prd.json");
    let md_path = cwd.join("docs/ralph").join(&config.slug).join("prd.md");

    if config.verbose {
        println!("Planning feature: {}", config.slug);
        println!("Task directory: {}", task_dir.display());
    }

    // Create task directory if needed
    if !task_dir.exists() {
        if config.dry_run {
            println!("[dry-run] Would create directory: {}", task_dir.display());
        } else {
            fs::create_dir_all(&task_dir)?;
            if config.verbose {
                println!("Created task directory: {}", task_dir.display());
            }
        }
    }

    // Create initial PRD if it doesn't exist
    let prd = if prd_path.exists() {
        Prd::from_file(&prd_path)?
    } else {
        let new_prd = create_initial_prd(&config.slug);
        if config.dry_run {
            println!("[dry-run] Would create PRD: {}", prd_path.display());
        } else {
            new_prd.save(&prd_path)?;
            if config.verbose {
                println!("Created initial PRD: {}", prd_path.display());
            }
        }
        new_prd
    };

    // Create or update markdown PRD
    if !config.dry_run {
        ensure_markdown_prd(&prd, &md_path)?;
    } else {
        println!("[dry-run] Would update markdown: {}", md_path.display());
    }

    // Launch Copilot planning session
    if config.dry_run {
        println!("[dry-run] Would launch: copilot --agent=ralph-planner --model claude-opus-4.5");
        println!("[dry-run] Working directory: {}", task_dir.display());
    } else {
        println!("🚀 Launching planning session for '{}'...", config.slug);
        println!();
        println!("PRD location: {}", prd_path.display());
        println!("Markdown doc: {}", md_path.display());
        println!();

        launch_copilot_planner(&task_dir)?;
    }

    Ok(())
}

fn create_initial_prd(slug: &str) -> Prd {
    let run_id = format!(
        "{}-{}",
        slug,
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );

    Prd {
        schema_version: "1.0".to_string(),
        slug: slug.to_string(),
        title: slug.replace('-', " ").to_string(),
        active_run_id: run_id,
        validation_profiles: vec!["rust-cargo".to_string()],
        requirements: vec![Requirement {
            id: "REQ-01".to_string(),
            title: "Initial requirement".to_string(),
            status: RequirementStatus::Todo,
            acceptance_criteria: vec!["Define acceptance criteria during planning".to_string()],
        }],
    }
}

fn ensure_markdown_prd(prd: &Prd, md_path: &Path) -> Result<()> {
    if let Some(parent) = md_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if md_path.exists() {
        // Load existing and preserve planning log
        let existing = MarkdownPrd::from_file(md_path)?;
        let planning_log = existing.get_section("PLANNING_LOG").map(String::from);
        prd.save_markdown(md_path, planning_log.as_deref())?;
    } else {
        prd.save_markdown(md_path, None)?;
    }

    Ok(())
}

fn launch_copilot_planner(working_dir: &Path) -> Result<()> {
    let status = Command::new("copilot")
        .args(["--agent=ralph-planner", "--model", "claude-opus-4.5"])
        .current_dir(working_dir)
        .status();

    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                println!("✅ Planning session completed");
            } else {
                println!("⚠️  Planning session exited with status: {}", exit_status);
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                println!("❌ Error: 'copilot' command not found");
                println!("   Please install GitHub Copilot CLI: https://docs.github.com/en/copilot/github-copilot-in-the-cli");
            } else {
                return Err(e.into());
            }
        }
    }

    Ok(())
}

