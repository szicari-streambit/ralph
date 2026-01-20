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

    // Count requirements by status
    let total_reqs = prd.requirements.len();
    let done_reqs = prd
        .requirements
        .iter()
        .filter(|r| r.status == RequirementStatus::Done)
        .count();
    let remaining_reqs = total_reqs - done_reqs;

    if config.verbose {
        println!("Implementing feature: {}", config.slug);
        println!("PRD: {}", prd_path.display());
        println!("Ledger: {}", ledger_path.display());
        println!("Current iteration: {}", ledger.latest_iteration() + 1);
    }

    println!(
        "📊 Progress: {}/{} requirements complete ({} remaining)",
        done_reqs, total_reqs, remaining_reqs
    );

    if config.loop_enabled {
        println!(
            "🔄 Starting implementation loop (max {} iterations)",
            config.max_iterations
        );
        println!();

        // Autonomous loop mode - iterate through requirements until all done or max iterations
        let mut iteration_count = 0;
        loop {
            iteration_count += 1;

            // Check safety limit
            if iteration_count > config.max_iterations {
                println!(
                    "⛔ Max iterations ({}) reached - stopping",
                    config.max_iterations
                );
                let remaining = prd
                    .requirements
                    .iter()
                    .filter(|r| r.status != RequirementStatus::Done)
                    .count();
                if remaining > 0 {
                    println!("   {} requirements still incomplete", remaining);
                }
                break;
            }

            // Run one iteration
            let all_done = run_single_iteration(
                config,
                &cwd,
                &prd_path,
                &mut prd,
                &mut ledger,
                validation_config.as_ref(),
            )?;

            // If all requirements are complete, we're done
            if all_done {
                println!("✅ All requirements complete!");
                break;
            }

            // Continue to next requirement
            println!();
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
/// Returns Ok(true) if all requirements are complete, Ok(false) if there's more work to do
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
        // No more requirements to implement
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
        // In dry-run, simulate success but indicate more work remains
        return Ok(false);
    }

    // Mark requirement as in progress
    prd.update_requirement_status(&req.id, RequirementStatus::InProgress);
    prd.save(prd_path)?;

    // Log start event
    ledger.append(LedgerEvent::new(iteration, &req.id, EventStatus::Started))?;

    // Generate prompt and launch Copilot
    let prompt = generate_prompt(prd, &req, ledger, iteration, run_full_tests);

    println!("📝 Launching Copilot implementer...");
    let copilot_success = launch_copilot_implementer(cwd, &prompt, config.verbose);

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
        // Summarize validation output to keep it concise and avoid API request body size issues
        let summary = summarize_validation_output(&output, config.verbose);
        event = event.with_validation_output(summary);
    }
    ledger.append(event)?;

    if validation_passed {
        println!("✅ Iteration {iteration} complete");
    } else {
        println!("❌ Iteration {iteration} failed validation");
    }

    // Return false to indicate there may be more requirements to process
    Ok(false)
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

            // Truncate validation output to prevent API request body size issues
            // Keep first 2000 chars which should be enough to show the key errors
            const MAX_VALIDATION_OUTPUT: usize = 2000;
            if validation_output.len() > MAX_VALIDATION_OUTPUT {
                prompt.push_str(&validation_output[..MAX_VALIDATION_OUTPUT]);
                prompt.push_str(&format!(
                    "\n\n... (truncated {} chars) ...\n",
                    validation_output.len() - MAX_VALIDATION_OUTPUT
                ));
            } else {
                prompt.push_str(&validation_output);
            }

            prompt.push_str(
                "\n\n🚨 YOU MUST FIX THESE ERRORS BEFORE FINISHING.\n\
                 Read the error output above and fix the root cause.\n\
                 DO NOT finish your work until validation passes.",
            );
        }
    }

    prompt
}

/// Smart truncation of validation output
/// Keeps first N lines and last M lines to preserve context and final errors
fn smart_truncate_validation_output(output: &str, max_chars: usize) -> String {
    if output.len() <= max_chars {
        return output.to_string();
    }

    let lines: Vec<&str> = output.lines().collect();
    let total_lines = lines.len();

    // Strategy: Keep first 15 lines (usually contains the error type and first occurrence)
    // and last 10 lines (usually contains the summary or final error)
    let first_n = 15.min(total_lines / 2);
    let last_m = 10.min(total_lines / 2);

    let mut result = String::new();

    // Add first N lines
    for line in lines.iter().take(first_n) {
        result.push_str(line);
        result.push('\n');
    }

    // Add truncation marker
    let omitted = total_lines.saturating_sub(first_n + last_m);
    if omitted > 0 {
        result.push_str(&format!("\n... ({} lines omitted) ...\n\n", omitted));
    }

    // Add last M lines
    for line in lines.iter().skip(total_lines.saturating_sub(last_m)) {
        result.push_str(line);
        result.push('\n');
    }

    // If still too long, hard truncate
    if result.len() > max_chars {
        result.truncate(max_chars);
        result.push_str("...\n(truncated to fit size limit)");
    }

    result
}

/// Summarize validation output using copilot CLI
/// Returns a concise summary (3-5 bullet points) of the validation errors
fn summarize_validation_output(validation_output: &str, verbose: bool) -> String {
    if validation_output.is_empty() {
        return String::new();
    }

    let prompt = format!(
        "Summarize the following validation errors into 3-5 concise bullet points. \
         Focus on the root causes and actionable fixes. Do not include explanations, \
         just the bullet points:\n\n{}",
        validation_output
    );

    if verbose {
        println!("🤖 Summarizing validation output with copilot...");
    }

    let result = Command::new("copilot")
        .args([
            "-p",
            &prompt,
            "--model",
            "gpt-5-mini",
            "--silent",
            "--allow-all-tools",
        ])
        .output();

    match result {
        Ok(cmd_output) if cmd_output.status.success() => {
            let summary = String::from_utf8_lossy(&cmd_output.stdout)
                .trim()
                .to_string();
            if verbose {
                println!("✅ Validation summary generated ({} chars)", summary.len());
            }
            summary
        }
        Ok(cmd_output) => {
            eprintln!(
                "⚠️  Failed to summarize validation output: {}",
                String::from_utf8_lossy(&cmd_output.stderr)
            );
            // Fallback: smart truncation
            smart_truncate_validation_output(validation_output, 2000)
        }
        Err(e) => {
            eprintln!("⚠️  Error calling copilot for summarization: {e}");
            // Fallback: smart truncation
            smart_truncate_validation_output(validation_output, 2000)
        }
    }
}

fn launch_copilot_implementer(working_dir: &Path, prompt: &str, verbose: bool) -> bool {
    let mut args = vec![
        "-p",
        prompt,
        "--agent=ralph-implementer",
        "--model",
        "gpt-5-mini",
        "--allow-all-tools",
        "--allow-all-paths",
    ];

    // Add debug logging when verbose is enabled
    if verbose {
        args.push("--log-level");
        args.push("debug");
    }

    let status = Command::new("copilot")
        .args(&args)
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
