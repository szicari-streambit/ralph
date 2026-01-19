// ABOUTME: Ralph CLI entry point for PRD automation
// ABOUTME: Provides subcommands: init, plan, implement, status, hook

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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { dry_run } => {
            println!("Initializing Ralph project (dry_run: {dry_run})...");
            // TODO: Implement init command
        }
        Commands::Plan { slug, dry_run } => {
            println!("Planning feature '{slug}' (dry_run: {dry_run})...");
            // TODO: Implement plan command
        }
        Commands::Implement { slug, dry_run } => {
            println!("Implementing feature '{slug}' (dry_run: {dry_run})...");
            // TODO: Implement implement command
        }
        Commands::Status { slug } => {
            match slug {
                Some(s) => println!("Status for feature '{s}'..."),
                None => println!("Status for all features..."),
            }
            // TODO: Implement status command
        }
        Commands::Hook { hook_type } => match hook_type {
            HookType::CommitMsg { file } => {
                println!("Validating commit message from '{file}'...");
                // TODO: Implement commit-msg hook
            }
        },
    }
}

