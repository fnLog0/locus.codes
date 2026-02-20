//! CLI entry point for locus.codes.

mod cli;
mod commands;
mod output;

use clap::Parser;

use crate::cli::Cli;

fn load_dotenv() {
    let mut path = std::env::current_dir().unwrap();
    loop {
        let env_path = path.join(".env");
        if env_path.exists() {
            let _ = dotenvy::from_path(&env_path);
            return;
        }
        if !path.pop() {
            return;
        }
    }
}

#[tokio::main]
async fn main() {
    load_dotenv();
    let cli = Cli::parse();
    output::init(cli.output);

    if let Err(e) = commands::handle(cli).await {
        output::error(&e.to_string());
        std::process::exit(1);
    }
}
