//! `locus config` subcommands.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::cli::ConfigAction;
use crate::output;

const PROVIDERS: &[(&str, &str, &str)] = &[
    ("anthropic", "ANTHROPIC_API_KEY", "Claude models (opus, sonnet, haiku)"),
    ("zai", "ZAI_API_KEY", "GLM models (glm-4-plus, glm-4-flash, etc.)"),
];

pub async fn handle(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Api { provider } => configure_api(provider).await,
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

    // Save to config file
    let config_path = get_config_path()?;
    save_api_key(&config_path, env_var, &key)?;
    
    output::success(&format!("Saved {} to {}", env_var, config_path.display()));
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

fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not find home directory"))?;
    let locus_dir = home.join(".locus");
    fs::create_dir_all(&locus_dir)?;
    Ok(locus_dir.join("env"))
}

fn save_api_key(path: &PathBuf, env_var: &str, key: &str) -> Result<()> {
    // Read existing config
    let existing = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    // Parse existing key-value pairs
    let mut config: BTreeMap<String, String> = existing
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("export ") && line.contains('=') {
                let line = line.strip_prefix("export ")?;
                let (key, value) = line.split_once('=')?;
                Some((key.trim().to_string(), value.trim().to_string()))
            } else {
                None
            }
        })
        .collect();

    // Update the key
    config.insert(env_var.to_string(), format!("\"{}\"", key));

    // Write back
    let mut content = String::new();
    content.push_str("# Locus CLI configuration\n");
    content.push_str("# Source this file: source ~/.locus/env\n\n");
    
    for (k, v) in &config {
        content.push_str(&format!("export {}={}\n", k, v));
    }

    fs::write(path, content)?;
    Ok(())
}
