//! `locus toolbus` subcommands.

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use locus_toolbus::ToolBus;
use serde_json::Value as JsonValue;

use crate::cli::ToolbusAction;
use crate::output;

pub async fn handle(action: ToolbusAction) -> Result<()> {
    let repo_root = find_repo_root()?;
    let bus = ToolBus::new(repo_root);

    match action {
        ToolbusAction::List => list(&bus),
        ToolbusAction::Info { tool } => info(&bus, &tool),
        ToolbusAction::Call { tool, args } => call(&bus, &tool, &args).await,
    }
}

fn find_repo_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut path = cwd.as_path();

    loop {
        if path.join(".git").exists() {
            return Ok(path.to_path_buf());
        }
        match path.parent() {
            Some(p) => path = p,
            None => break,
        }
    }

    Err(anyhow!(
        "Not in a git repository. locus requires a git repo to run."
    ))
}

fn list(bus: &ToolBus) -> Result<()> {
    let tools = bus.list_tools();

    if tools.is_empty() {
        output::dim("No tools registered");
        return Ok(());
    }

    output::header("Registered Tools");

    let mut table = output::table();
    output::table_header(&mut table, "Tool", "Description");

    let items: Vec<_> = tools
        .iter()
        .map(|t| {
            output::table_row(&mut table, &t.name, &t.description);
            (t.name.as_str(), t.description.as_str())
        })
        .collect();

    output::table_print(&table, &items);

    Ok(())
}

fn info(bus: &ToolBus, tool_name: &str) -> Result<()> {
    let tools = bus.list_tools();
    let tool = tools
        .iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| anyhow!("Tool not found: {}", tool_name))?;

    output::header(&format!("Tool: {}", tool.name));
    output::dim(&tool.description);
    println!();
    output::header("Parameters");
    output::json_pretty(&tool.parameters);

    Ok(())
}

async fn call(bus: &ToolBus, tool_name: &str, args_str: &str) -> Result<()> {
    let args: JsonValue =
        serde_json::from_str(args_str).map_err(|e| anyhow!("Invalid JSON arguments: {}", e))?;

    let spinner = output::spinner(&format!("Calling {}...", tool_name));

    let (result, duration_ms) = bus.call(tool_name, args).await?;

    output::spinner_success(&spinner, &format!("Completed in {}ms", duration_ms));
    println!();
    output::json_pretty(&result);

    Ok(())
}
