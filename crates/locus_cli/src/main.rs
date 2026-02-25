//! CLI entry point for locus.codes.

mod cli;
mod commands;
mod output;

use clap::Parser;

use crate::cli::Cli;

/// Strip one level of surrounding double quotes from a config value (DB stores quoted).
fn unquote_config_value(v: &str) -> &str {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        &v[1..v.len() - 1]
    } else {
        v
    }
}

/// Load configuration: DB first (global then project overrides), then env files.
/// Runtime uses env only (LocusGraphConfig::from_env()); it does not read the DB.
/// Order: 1) ~/.locus/locus.db  2) .locus/locus.db (project)  3) ~/.locus/env  4) .locus/env  5) .env (project root)
fn load_locus_config() {
    // 1) Load from global DB ~/.locus/locus.db config table
    if let Some(home) = dirs::home_dir() {
        let locus_dir = home.join(".locus");
        let db_path = locus_dir.join(locus_core::db::LOCUS_DB);
        if db_path.exists() {
            if let Ok(conn) = locus_core::db::open_db_at(&locus_dir) {
                if let Ok(pairs) = locus_core::db::get_config(&conn) {
                    for (k, v) in pairs {
                        let _ = unsafe { std::env::set_var(&k, unquote_config_value(&v)) };
                    }
                }
            }
        }
    }
    // 2) Load from project DB (cwd or parent .locus/locus.db); overrides global
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd;
        for _ in 0..32 {
            let locus_dir = dir.join(".locus");
            let db_path = locus_dir.join(locus_core::db::LOCUS_DB);
            if db_path.exists() {
                if let Ok(conn) = locus_core::db::open_db_at(&locus_dir) {
                    if let Ok(pairs) = locus_core::db::get_config(&conn) {
                        for (k, v) in pairs {
                            let _ = unsafe { std::env::set_var(&k, unquote_config_value(&v)) };
                        }
                    }
                }
                break;
            }
            if let Some(parent) = dir.parent() {
                dir = parent.to_path_buf();
            } else {
                break;
            }
        }
    }
    // 3) Fallback: env files (for older installs or if DB missing)
    if let Some(home) = dirs::home_dir() {
        let config_path = home.join(".locus").join("env");
        if config_path.exists() {
            let _ = dotenvy::from_path(&config_path);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd;
        for _ in 0..32 {
            let project_env = dir.join(".locus").join("env");
            if project_env.exists() {
                let _ = dotenvy::from_path(&project_env);
                break;
            }
            if let Some(parent) = dir.parent() {
                dir = parent.to_path_buf();
            } else {
                break;
            }
        }
    }
    // 4) Project root .env (so LOCUSGRAPH_*, API keys, etc. are used when not in DB)
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd;
        for _ in 0..32 {
            let env_file = dir.join(".env");
            if env_file.exists() {
                let _ = dotenvy::from_path(&env_file);
                break;
            }
            if let Some(parent) = dir.parent() {
                dir = parent.to_path_buf();
            } else {
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    load_locus_config();
    let cli = Cli::parse();
    output::init(cli.output);

    if let Err(e) = commands::handle(cli).await {
        output::error(&e.to_string());
        std::process::exit(1);
    }
}
