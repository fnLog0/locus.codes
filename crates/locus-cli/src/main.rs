//! locus-cli — entry point for locus.codes
//!
//! Best practice: clap derive for args, anyhow for error propagation.

use anyhow::Result;
use clap::Parser;
use locus_runtime::{run_app, Mode};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "locus")]
#[command(about = "Terminal-native coding agent with deterministic memory", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Boot the TUI and start the agent
    Run {
        /// Operating mode: rush (fast), smart (balanced), deep (thorough)
        #[arg(long, value_enum, default_value_t = CliMode::Smart)]
        mode: CliMode,

        /// Repository path (default: detect from cwd by walking up to .git)
        #[arg(long, value_name = "PATH")]
        repo: Option<PathBuf>,
    },
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
enum CliMode {
    Rush,
    #[default]
    Smart,
    Deep,
}

impl From<CliMode> for Mode {
    fn from(m: CliMode) -> Self {
        match m {
            CliMode::Rush => Mode::Rush,
            CliMode::Smart => Mode::Smart,
            CliMode::Deep => Mode::Deep,
        }
    }
}

fn main() -> Result<()> {
    // Load .env file from current directory or parent directories
    dotenvy::dotenv().ok();

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Run { mode, repo }) => run_app(mode.into(), repo)?,
        None => run_app(Mode::Smart, None)?,
    }
    Ok(())
}
