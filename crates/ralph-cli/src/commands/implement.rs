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
    /// Enable continuous looping until success or max iterations (default: true)
    pub loop_enabled: bool,
    /// Maximum number of iterations before stopping (default: 10)
    pub max_iterations: u32,
}

/// Run the implementation loop
pub fn run(config: &ImplementConfig) -> Result<()> {
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

    // Check for uncommitted changes
    if has_uncommitted_changes() {
        println!("⚠️  Warning: You have uncommitted changes");
        if config.verbose {
            println!("   Consider committing or stashing before implementation");
        }
    }

    let mut prd = Prd::from_file(&prd_path)?;
    let mut ledger = if ledger_path.exists() {
        Ledger::from_file(&ledger_path)?
    } else {
        Ledger::create(&ledger_path)?
    };

    // Ensure we're on the correct branch
    let branch_name = format!("ralph/{}/{}", config.slug, prd.active_run_id);
    ensure_branch(&branch_name, config.dry_run, config.verbose)?;

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

    if config.loop_enabled {
        // Autonomous loop mode - iterate until success or max iterations
        let mut iteration_count = 0;
        loop {
            iteration_count += 1;

            // Check safety limit
            if iteration_count > config.max_iterations {
                println!(
                    "⛔ Max iterations ({}) reached - stopping",
                    config.max_iterations
                );
                break;
            }

            // Run one iteration
            let validation_passed = run_single_iteration(
                config,
                &cwd,
                &prd_path,
                &mut prd,
                &mut ledger,
                validation_config.as_ref(),
            )?;

            if validation_passed {
                println!("✅ Task complete!");
                break;
            }

            // Validation failed - loop will retry automatically
            if iteration_count < config.max_iterations {
                println!(
                    "🔄 Validation failed, retrying (attempt {}/{})",
                    iteration_count + 1,
                    config.max_iterations
                );
            }
        }
    } else {
        // Single iteration mode (--once flag)
        run_single_iteration(
            config,
            &cwd,
            &prd_path,
            &mut prd,
            &mut ledger,
            validation_config.as_ref(),
        )?;
    }

    Ok(())
}

/// Run a single iteration of the implementation loop
///
/// Returns Ok(true) if validation passed, Ok(false) if validation failed
fn run_single_iteration(
    config: &ImplementConfig,
    cwd: &Path,
    prd_path: &Path,
    prd: &mut Prd,
    ledger: &mut Ledger,
    validation_config: Option<&ValidationConfig>,
) -> Result<bool> {
    // Find next requirement to implement
    let next_req = prd
        .requirements
        .iter()
        .find(|r| r.status == RequirementStatus::Todo || r.status == RequirementStatus::InProgress)
        .cloned();

    let Some(req) = next_req else {
        println!("✅ All requirements are complete!");
        return Ok(true);
    };

    let iteration = ledger.latest_iteration() + 1;
    let run_full_tests = iteration % 5 == 0;

    println!(
        "🔄 Iteration {} - Implementing {}: {}",
        iteration, req.id, req.title
    );

    if config.dry_run {
        println!("[dry-run] Would run implementation for {}", req.id);
        println!("[dry-run] Would run validation (full_tests: {run_full_tests})");
        return Ok(true);
    }

    // Mark requirement as in progress
    prd.update_requirement_status(&req.id, RequirementStatus::InProgress);
    prd.save(prd_path)?;

    // Log start event
    ledger.append(LedgerEvent::new(iteration, &req.id, EventStatus::Started))?;

    // Generate prompt and launch Copilot
    let prompt = generate_prompt(prd, &req, ledger, iteration, run_full_tests);

    println!("📝 Launching Copilot implementer...");
    let copilot_success = launch_copilot_implementer(cwd, &prompt);

    // Run validation
    let (validation_passed, validation_output) = if let Some(vc) = validation_config {
        if let Some(profile) = prd.validation_profiles.first().and_then(|p| vc.get(p)) {
            println!("🔍 Running validation...");
            let results = profile.run_all(cwd, run_full_tests);
            let all_passed = results.iter().all(|r| r.success);

            // Capture output from first failed stage
            let failed_output = results
                .iter()
                .find(|r| !r.success)
                .map(|r| format!("Stage: {:?}\n\n{}", r.stage, r.output));

            for result in &results {
                let icon = if result.success { "✅" } else { "❌" };
                println!("  {} {:?}", icon, result.stage);
            }

            (all_passed, failed_output)
        } else {
            (true, None)
        }
    } else {
        (true, None)
    };

    // Update status based on results
    let (final_status, event_status) = if copilot_success && validation_passed {
        (RequirementStatus::Done, EventStatus::Done)
    } else {
        (RequirementStatus::InProgress, EventStatus::Failed)
    };

    prd.update_requirement_status(&req.id, final_status);
    prd.save(prd_path)?;

    // Build ledger event with validation output if available
    let mut event =
        LedgerEvent::new(iteration, &req.id, event_status).with_validation(validation_passed);
    if let Some(output) = validation_output {
        event = event.with_validation_output(output);
    }
    ledger.append(event)?;

    if validation_passed {
        println!("✅ Iteration {iteration} complete");
    } else {
        println!("❌ Iteration {iteration} failed validation");
    }

    Ok(validation_passed)
}

fn generate_prompt(
    prd: &Prd,
    req: &ralph_lib::Requirement,
    ledger: &Ledger,
    iteration: u32,
    run_full_tests: bool,
) -> String {
    let mut prompt = format!(
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
            .map(|ac| format!("- {ac}"))
            .collect::<Vec<_>>()
            .join("\n"),
        if run_full_tests { " -> test" } else { "" }
    );

    // Add validation failure feedback if previous iteration failed
    if iteration > 1 {
        if let Some(validation_output) = ledger.get_last_validation_failure(&req.id) {
            prompt.push_str("\n\n⚠️  PREVIOUS ITERATION FAILED VALIDATION:\n\n");
            prompt.push_str(&validation_output);
            prompt.push_str(
                "\n\nPlease fix the validation errors above before proceeding. \
                 Make sure to run the appropriate commands (e.g., cargo fmt) to resolve issues.",
            );
        }
    }

    prompt
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
                println!("❌ Error launching copilot: {e}");
            }
            false
        }
    }
}

fn has_uncommitted_changes() -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false)
}

fn ensure_branch(branch_name: &str, dry_run: bool, verbose: bool) -> Result<()> {
    // Check if branch exists
    let branch_exists = Command::new("git")
        .args(["rev-parse", "--verify", branch_name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    // Get current branch
    let current_branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if current_branch == branch_name {
        if verbose {
            println!("Already on branch: {branch_name}");
        }
        return Ok(());
    }

    if dry_run {
        if branch_exists {
            println!("[dry-run] Would checkout branch: {branch_name}");
        } else {
            println!("[dry-run] Would create and checkout branch: {branch_name}");
        }
        return Ok(());
    }

    if branch_exists {
        println!("📌 Checking out branch: {branch_name}");
        let status = Command::new("git")
            .args(["checkout", branch_name])
            .status()?;
        if !status.success() {
            println!("⚠️  Failed to checkout branch, continuing on current branch");
        }
    } else {
        println!("🌿 Creating branch: {branch_name}");
        let status = Command::new("git")
            .args(["checkout", "-b", branch_name])
            .status()?;
        if !status.success() {
            println!("⚠️  Failed to create branch, continuing on current branch");
        }
    }

    Ok(())
}
