// ABOUTME: 'ralph status' command implementation
// ABOUTME: Displays PRD status, requirements, and ledger events

use ralph_lib::{Ledger, Prd, RequirementStatus, Result};
use std::fs;
use std::path::Path;

/// Configuration for status command
pub struct StatusConfig {
    pub slug: Option<String>,
    pub verbose: bool,
}

/// Show status of PRD requirements and ledger
pub fn run(config: &StatusConfig) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let tasks_dir = cwd.join("ralph/tasks");

    if !tasks_dir.exists() {
        println!("No Ralph tasks found. Run 'ralph init' first.");
        return Ok(());
    }

    match &config.slug {
        Some(slug) => show_feature_status(&cwd, slug, config.verbose)?,
        None => show_all_features(&tasks_dir, config.verbose)?,
    }

    Ok(())
}

fn show_all_features(tasks_dir: &Path, verbose: bool) -> Result<()> {
    let entries = fs::read_dir(tasks_dir)?;

    let mut features: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                features.push(name.to_string());
            }
        }
    }

    if features.is_empty() {
        println!("No features found. Create one with 'ralph plan <slug>'.");
        return Ok(());
    }

    println!("📋 Ralph Features\n");

    for slug in &features {
        let prd_path = tasks_dir.join(slug).join("prd.json");
        if prd_path.exists() {
            match Prd::from_file(&prd_path) {
                Ok(prd) => {
                    let (done, total) = count_requirements(&prd);
                    let progress = if total > 0 {
                        format!("{done}/{total}")
                    } else {
                        "0/0".to_string()
                    };
                    println!("  {} [{}] {}", status_icon(done, total), progress, prd.title);

                    if verbose {
                        for req in &prd.requirements {
                            println!(
                                "    {} {} - {}",
                                req_status_icon(&req.status),
                                req.id,
                                req.title
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("  ❓ {slug} (error: {e})");
                }
            }
        }
    }

    Ok(())
}

fn show_feature_status(cwd: &Path, slug: &str, verbose: bool) -> Result<()> {
    let task_dir = cwd.join("ralph/tasks").join(slug);
    let prd_path = task_dir.join("prd.json");
    let ledger_path = task_dir.join("ledger.jsonl");

    if !prd_path.exists() {
        println!("❌ Feature '{slug}' not found");
        return Ok(());
    }

    let prd = Prd::from_file(&prd_path)?;

    println!("📋 {}\n", prd.title);
    println!("Slug: {}", prd.slug);
    println!("Run ID: {}", prd.active_run_id);
    println!("Profiles: {}", prd.validation_profiles.join(", "));
    println!();

    // Show requirements
    println!("Requirements:");
    for req in &prd.requirements {
        println!(
            "  {} {} - {}",
            req_status_icon(&req.status),
            req.id,
            req.title
        );
        if verbose {
            for ac in &req.acceptance_criteria {
                println!("      • {ac}");
            }
        }
    }

    // Show ledger summary if exists
    if ledger_path.exists() {
        let ledger = Ledger::from_file(&ledger_path)?;
        let events = ledger.events();

        if !events.is_empty() {
            println!();
            println!("Ledger ({} events):", events.len());
            println!("  Latest iteration: {}", ledger.latest_iteration());

            if verbose {
                println!();
                for event in events.iter().rev().take(10) {
                    println!(
                        "  [{}] {} {} {:?}{}",
                        event.timestamp.format("%Y-%m-%d %H:%M"),
                        event.iteration,
                        event.requirement,
                        event.status,
                        event
                            .validation_passed
                            .map_or("", |v| if v { " ✅" } else { " ❌" })
                    );
                }
            }
        }
    }

    Ok(())
}

fn count_requirements(prd: &Prd) -> (usize, usize) {
    let total = prd.requirements.len();
    let done = prd
        .requirements
        .iter()
        .filter(|r| r.status == RequirementStatus::Done)
        .count();
    (done, total)
}

fn status_icon(done: usize, total: usize) -> &'static str {
    if done == total && total > 0 {
        "✅"
    } else if done > 0 {
        "🔄"
    } else {
        "⬜"
    }
}

fn req_status_icon(status: &RequirementStatus) -> &'static str {
    match status {
        RequirementStatus::Todo => "⬜",
        RequirementStatus::InProgress => "🔄",
        RequirementStatus::Done => "✅",
        RequirementStatus::Blocked => "🚫",
    }
}

