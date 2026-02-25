//! `locus config` subcommands. Config stored in locus.db config table; .locus/env synced for sourcing.

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::cli::ConfigAction;
use crate::output;
use locus_core::db;

const PROVIDERS: &[(&str, &str, &str)] = &[
    ("anthropic", "ANTHROPIC_API_KEY", "Claude models (opus, sonnet, haiku)"),
    ("zai", "ZAI_API_KEY", "GLM models (glm-5, glm-4-plus, etc.)"),
    ("tinyfish", "TINYFISH_API_KEY", "TinyFish web automation"),
];

pub async fn handle(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Api { provider } => configure_api(provider).await,
        ConfigAction::Graph { url, graph_id } => configure_graph(url, graph_id).await,
    }
}

async fn configure_api(provider: Option<String>) -> Result<()> {
    let selected = match provider {
        Some(p) => {
            let p_lower = p.to_lowercase();
            PROVIDERS
                .iter()
                .find(|(id, _, _)| *id == p_lower)
                .ok_or_else(|| anyhow!("Unknown provider '{}'. Available: {}", p, 
                    PROVIDERS.iter().map(|(id, _, _)| *id).collect::<Vec<_>>().join(", ")))?
        }
        None => select_provider()?,
    };

    let (id, env_var, description) = selected;
    
    output::header(&format!("Configure {}", id));
    println!("  {}", description);
    println!();

    // Check if already set
    if let Ok(current) = env::var(env_var) {
        let masked = mask_key(&current);
        println!("  Current: {}", masked);
        println!();
    }

    // Prompt for new key
    let key = prompt_api_key(id)?;
    
    if key.is_empty() {
        output::warning("No key entered, cancelled.");
        return Ok(());
    }

    let locus_dir = get_global_locus_dir()?;
    save_config_key(&locus_dir, env_var, &format!("\"{}\"", key))?;

    output::success(&format!("Saved {} to {}", env_var, locus_dir.join("locus.db").display()));
    println!();
    output::dim("Run 'source ~/.locus/env' or restart your shell to apply.");

    Ok(())
}

async fn configure_graph(url: Option<String>, graph_id: Option<String>) -> Result<()> {
    output::header("Configure LocusGraph");
    println!("  Memory and context storage for Locus agent");
    println!();

    // Check current config
    let current_secret = env::var("LOCUSGRAPH_AGENT_SECRET").ok();
    let current_url = env::var("LOCUSGRAPH_SERVER_URL").ok();
    let current_graph_id = env::var("LOCUSGRAPH_GRAPH_ID").ok();

    if let Some(secret) = &current_secret {
        println!("  Current secret: {}", mask_key(secret));
    }
    if let Some(u) = &current_url {
        println!("  Current URL: {}", u);
    }
    if let Some(id) = &current_graph_id {
        println!("  Current graph ID: {}", id);
    }
    println!();

    // Prompt for agent secret (required)
    println!("Enter LocusGraph agent secret:");
    let secret = prompt_api_key("LocusGraph")?;

    if secret.is_empty() {
        output::warning("No secret entered, cancelled.");
        return Ok(());
    }

    // Use provided values or defaults
    let final_url = url.unwrap_or_else(|| {
        println!();
        println!("Enter server URL [http://127.0.0.1:50051]:");
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let trimmed = input.trim();
        if trimmed.is_empty() {
            "http://127.0.0.1:50051".to_string()
        } else {
            trimmed.to_string()
        }
    });

    let final_graph_id = graph_id.unwrap_or_else(|| {
        println!();
        println!("Enter graph ID [locus-agent]:");
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let trimmed = input.trim();
        if trimmed.is_empty() {
            "locus-agent".to_string()
        } else {
            trimmed.to_string()
        }
    });

    let locus_dir = get_global_locus_dir()?;
    save_graph_config_db(&locus_dir, &secret, &final_url, &final_graph_id)?;

    output::success(&format!("Saved LocusGraph config to {}", locus_dir.join("locus.db").display()));
    println!();
    output::dim("Run 'source ~/.locus/env' or restart your shell to apply.");

    Ok(())
}

fn select_provider() -> Result<&'static (&'static str, &'static str, &'static str)> {
    println!("Select a provider to configure:\n");
    
    for (i, (id, _, desc)) in PROVIDERS.iter().enumerate() {
        let status = if env::var(PROVIDERS[i].1).is_ok() {
            console::style("(configured)").green()
        } else {
            console::style("(not set)").dim()
        };
        println!("  {}) {} {} - {}", i + 1, id, status, desc);
    }
    println!();

    print!("Enter choice [1-{}]: ", PROVIDERS.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let choice: usize = input.trim().parse()
        .map_err(|_| anyhow!("Invalid choice"))?;
    
    if choice < 1 || choice > PROVIDERS.len() {
        return Err(anyhow!("Choice must be 1-{}", PROVIDERS.len()));
    }

    Ok(&PROVIDERS[choice - 1])
}

fn prompt_api_key(provider: &str) -> Result<String> {
    use crossterm::{
        event::{self, Event, KeyCode, KeyModifiers},
        terminal,
    };
    
    println!("Enter API key for {}:", provider);
    print!("> ");
    io::stdout().flush()?;

    let mut key = String::new();

    terminal::enable_raw_mode()?;
    
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Enter => {
                        println!();
                        break;
                    }
                    KeyCode::Backspace => {
                        if !key.is_empty() {
                            key.pop();
                            print!("\x08 \x08");
                            io::stdout().flush()?;
                        }
                    }
                    KeyCode::Char(c) => {
                        if k.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                            terminal::disable_raw_mode()?;
                            println!();
                            return Ok(String::new());
                        }
                        key.push(c);
                        print!("*");
                        io::stdout().flush()?;
                    }
                    _ => {}
                }
            }
        }
    }
    
    terminal::disable_raw_mode()?;

    Ok(key)
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    format!("{}...{}", &key[..4], &key[key.len()-4..])
}

fn get_global_locus_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not find home directory"))?;
    let locus_dir = home.join(".locus");
    std::fs::create_dir_all(&locus_dir)?;
    Ok(locus_dir)
}

/// Save one config key to DB and sync env file.
fn save_config_key(locus_dir: &PathBuf, key: &str, value: &str) -> Result<()> {
    let conn = db::open_db_at(locus_dir)?;
    db::set_config(&conn, key, value)?;
    let config = db::get_config(&conn)?;
    db::sync_env_file(locus_dir, &config)?;
    Ok(())
}

/// Save LocusGraph keys to DB and sync env file.
fn save_graph_config_db(
    locus_dir: &PathBuf,
    secret: &str,
    url: &str,
    graph_id: &str,
) -> Result<()> {
    let conn = db::open_db_at(locus_dir)?;
    db::set_config(&conn, "LOCUSGRAPH_AGENT_SECRET", &format!("\"{}\"", secret))?;
    db::set_config(&conn, "LOCUSGRAPH_SERVER_URL", &format!("\"{}\"", url))?;
    db::set_config(&conn, "LOCUSGRAPH_GRAPH_ID", &format!("\"{}\"", graph_id))?;
    let config = db::get_config(&conn)?;
    db::sync_env_file(locus_dir, &config)?;
    Ok(())
}
