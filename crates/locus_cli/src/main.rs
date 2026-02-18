//! CLI entry point for locus.codes.

mod cli;
mod commands;
mod output;

use clap::Parser;

use crate::cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    output::init(cli.output);

    if let Err(e) = commands::handle(cli).await {
        output::error(&e.to_string());
        std::process::exit(1);
    }
}
