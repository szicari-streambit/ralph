// ABOUTME: 'ralph implement' command implementation
// ABOUTME: Runs unattended implementation loop with GitHub Copilot CLI

use ralph_lib::{
    EventStatus, Ledger, LedgerEvent, Prd, RequirementStatus, Result, ValidationConfig,
};
use std::path::Path;
use std::process::Command;

/// Configuration for implement command
pub struct ImplementConfig {
    pub slug: String,
    pub dry_run: bool,
    pub verbose: bool,
}

/// Run the implementation loop
pub fn run(config: ImplementConfig) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let task_dir = cwd.join("ralph/tasks").join(&config.slug);
    let prd_path = task_dir.join("prd.json");
    let ledger_path = task_dir.join("ledger.jsonl");
    let validation_path = cwd.join("ralph/validation.json");

    // Verify PRD exists
    if !prd_path.exists() {
        println!("❌ Error: PRD not found at {}", prd_path.display());
        println!("   Run 'ralph plan {}' first", config.slug);
        return Ok(());
    }

    let mut prd = Prd::from_file(&prd_path)?;
    let mut ledger = if ledger_path.exists() {
        Ledger::from_file(&ledger_path)?
    } else {
        Ledger::create(&ledger_path)?
    };

    // Load validation config
    let validation_config = if validation_path.exists() {
        Some(ValidationConfig::from_file(&validation_path)?)
    } else {
        None
    };

    if config.verbose {
        println!("Implementing feature: {}", config.slug);
        println!("PRD: {}", prd_path.display());
        println!("Ledger: {}", ledger_path.display());
        println!("Current iteration: {}", ledger.latest_iteration() + 1);
    }

    // Find next requirement to implement
    let next_req = prd
        .requirements
        .iter()
        .find(|r| r.status == RequirementStatus::Todo || r.status == RequirementStatus::InProgress)
        .cloned();

    let Some(req) = next_req else {
        println!("✅ All requirements are complete!");
        return Ok(());
    };

    let iteration = ledger.latest_iteration() + 1;
    let run_full_tests = iteration % 5 == 0;

    println!("🔄 Iteration {} - Implementing {}: {}", iteration, req.id, req.title);

    if config.dry_run {
        println!("[dry-run] Would run implementation for {}", req.id);
        println!("[dry-run] Would run validation (full_tests: {})", run_full_tests);
        return Ok(());
    }

    // Mark requirement as in progress
    prd.update_requirement_status(&req.id, RequirementStatus::InProgress);
    prd.save(&prd_path)?;

    // Log start event
    ledger.append(LedgerEvent::new(iteration, &req.id, EventStatus::Started))?;

    // Generate prompt and launch Copilot
    let prompt = generate_prompt(&prd, &req, iteration, run_full_tests);

    println!("📝 Launching Copilot implementer...");
    let copilot_success = launch_copilot_implementer(&cwd, &prompt);

    // Run validation
    let validation_passed = if let Some(ref vc) = validation_config {
        if let Some(profile) = prd.validation_profiles.first().and_then(|p| vc.get(p)) {
            println!("🔍 Running validation...");
            let results = profile.run_all(&cwd, run_full_tests);
            let all_passed = results.iter().all(|r| r.success);

            for result in &results {
                let icon = if result.success { "✅" } else { "❌" };
                println!("  {} {:?}", icon, result.stage);
            }

            all_passed
        } else {
            true
        }
    } else {
        true
    };

    // Update status based on results
    let (final_status, event_status) = if copilot_success && validation_passed {
        (RequirementStatus::Done, EventStatus::Done)
    } else {
        (RequirementStatus::InProgress, EventStatus::Failed)
    };

    prd.update_requirement_status(&req.id, final_status);
    prd.save(&prd_path)?;

    ledger.append(
        LedgerEvent::new(iteration, &req.id, event_status).with_validation(validation_passed),
    )?;

    if validation_passed {
        println!("✅ Iteration {} complete", iteration);
    } else {
        println!("❌ Iteration {} failed validation", iteration);
    }

    Ok(())
}

fn generate_prompt(prd: &Prd, req: &ralph_lib::Requirement, iteration: u32, run_full_tests: bool) -> String {
    format!(
        "Implement requirement {} for feature '{}' (iteration {}).\n\n\
         Title: {}\n\n\
         Acceptance Criteria:\n{}\n\n\
         Validation: fmt -> lint -> typecheck{}\n\n\
         Update PRD status only after validation passes.",
        req.id,
        prd.slug,
        iteration,
        req.title,
        req.acceptance_criteria
            .iter()
            .map(|ac| format!("- {}", ac))
            .collect::<Vec<_>>()
            .join("\n"),
        if run_full_tests { " -> test" } else { "" }
    )
}

fn launch_copilot_implementer(working_dir: &Path, prompt: &str) -> bool {
    let status = Command::new("copilot")
        .args([
            "-p",
            prompt,
            "--agent=ralph-implementer",
            "--model",
            "gpt-5-mini",
            "--allow-all-tools",
            "--allow-all-paths",
        ])
        .current_dir(working_dir)
        .status();

    match status {
        Ok(exit_status) => exit_status.success(),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                println!("❌ Error: 'copilot' command not found");
            } else {
                println!("❌ Error launching copilot: {}", e);
            }
            false
        }
    }
}

