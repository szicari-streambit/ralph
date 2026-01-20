// ABOUTME: Ralph CLI entry point for PRD automation
// ABOUTME: Provides subcommands: init, plan, implement, status, hook

mod commands;

use clap::{Parser, Subcommand};

/// Ralph CLI - Automated PRD implementation using GitHub Copilot
#[derive(Parser)]
#[command(name = "ralph")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Ralph project
    Init {
        /// Preview actions without executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Start or resume planning session for a feature
    Plan {
        /// Feature slug (URL-safe identifier)
        slug: String,
        /// Preview actions without executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Run implementation loop for a feature
    Implement {
        /// Feature slug (URL-safe identifier)
        slug: String,
        /// Preview actions without executing
        #[arg(long)]
        dry_run: bool,
        /// Run only one iteration instead of looping until success
        #[arg(long)]
        once: bool,
        /// Maximum number of iterations (default: 10)
        #[arg(long, default_value = "10")]
        max_iterations: u32,
    },
    /// Show status of PRD requirements and ledger
    Status {
        /// Optional feature slug (shows all if omitted)
        slug: Option<String>,
    },
    /// Git hook handlers
    Hook {
        #[command(subcommand)]
        hook_type: HookType,
    },
}

#[derive(Subcommand)]
enum HookType {
    /// Validate commit message references a requirement
    CommitMsg {
        /// Path to the commit message file
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { dry_run } => commands::init::run(&commands::init::InitConfig {
            dry_run,
            verbose: cli.verbose,
        }),
        Commands::Plan { slug, dry_run } => commands::plan::run(&commands::plan::PlanConfig {
            slug,
            dry_run,
            verbose: cli.verbose,
        }),
        Commands::Implement {
            slug,
            dry_run,
            once,
            max_iterations,
        } => commands::implement::run(&commands::implement::ImplementConfig {
            slug,
            dry_run,
            verbose: cli.verbose,
            loop_enabled: !once,
            max_iterations,
        }),
        Commands::Status { slug } => commands::status::run(&commands::status::StatusConfig {
            slug,
            verbose: cli.verbose,
        }),
        Commands::Hook { hook_type } => match hook_type {
            HookType::CommitMsg { file } => {
                commands::hook::commit_msg(&commands::hook::CommitMsgConfig {
                    file,
                    verbose: cli.verbose,
                })
            }
        },
    };

    if let Err(e) = result {
        eprintln!("❌ Error: {e}");
        std::process::exit(1);
    }
}
