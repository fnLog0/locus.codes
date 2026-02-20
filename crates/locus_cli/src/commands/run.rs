//! `locus run` command - start the runtime agent.

use std::path::PathBuf;

use anyhow::Result;
use tokio::sync::mpsc;

use locus_core::SessionEvent;
use locus_runtime::{Runtime, RuntimeConfig};

use crate::output;

pub async fn handle(
    model: Option<String>,
    provider: Option<String>,
    workdir: Option<String>,
    max_turns: Option<u32>,
    max_tokens: Option<u32>,
    prompt: Option<String>,
) -> Result<()> {
    // Determine working directory
    let repo_root = match workdir {
        Some(w) => PathBuf::from(w),
        None => std::env::current_dir()?,
    };

    // Parse provider
    let llm_provider = provider
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    // Build config
    let mut config = RuntimeConfig::from_env(repo_root.clone())
        .with_provider(llm_provider);

    if let Some(m) = &model {
        config = config.with_model(m);
    }

    if let Some(max) = max_turns {
        config = config.with_max_turns(max);
    }

    if let Some(tokens) = max_tokens {
        config = config.with_max_tokens(tokens);
    }

    output::header("Locus Runtime");
    println!("  Repository: {}", repo_root.display());
    println!("  Model: {}", config.model);
    println!("  Provider: {}", config.provider);
    if let Some(max) = config.max_turns {
        println!("  Max turns: {}", max);
    }
    println!();

    // Get initial message
    let initial_message = match prompt {
        Some(p) => p,
        None => {
            output::dim("Enter your initial message (Ctrl+C to cancel):");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            input.trim().to_string()
        }
    };

    if initial_message.is_empty() {
        output::warning("No message provided, exiting.");
        return Ok(());
    }

    // Create event channel
    let (event_tx, mut event_rx) = mpsc::channel::<SessionEvent>(256);

    // Spawn event handler
    let event_handle = tokio::spawn(async move {
        use console::style;
        while let Some(event) = event_rx.recv().await {
            match event {
                SessionEvent::TurnStart { role } => {
                    let role_str = match role {
                        locus_core::Role::User => style("User").cyan(),
                        locus_core::Role::Assistant => style("Assistant").green(),
                        locus_core::Role::Tool => style("Tool").yellow(),
                        locus_core::Role::System => style("System").magenta(),
                    };
                    println!("\n[{}]", role_str);
                }
                SessionEvent::TextDelta { text } => {
                    print!("{}", text);
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
                SessionEvent::ThinkingDelta { thinking } => {
                    print!("{}", style(thinking).dim());
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
                SessionEvent::ToolStart { tool_use } => {
                    println!("\n  {} {}", style("Tool:").yellow(), style(&tool_use.name).bold());
                }
                SessionEvent::ToolDone { result, .. } => {
                    let preview = if result.is_error {
                        format!("Error: {}", result.output)
                    } else {
                        let content_str = result.output.to_string();
                        if content_str.len() > 200 {
                            format!("{}...", &content_str[..200])
                        } else {
                            content_str
                        }
                    };
                    println!("  {} {}", style("Result:").dim(), preview);
                }
                SessionEvent::Error { error } => {
                    eprintln!("\n{} {}", style("Error:").red(), error);
                }
                SessionEvent::Status { message } => {
                    output::dim(&format!("  {}", message));
                }
                SessionEvent::MemoryRecall { items_found, .. } => {
                    if items_found > 0 {
                        output::dim(&format!("  Recalled {} memories", items_found));
                    }
                }
                _ => {}
            }
        }
    });

    // Create and run runtime
    output::dim("Starting runtime...\n");

    match Runtime::new(config, event_tx).await {
        Ok(mut runtime) => {
            let result = runtime.run(initial_message).await;

            // Shutdown
            let _ = runtime.shutdown().await;

            // Wait for event handler
            event_handle.abort();

            match result {
                Ok(status) => {
                    println!();
                    output::success(&format!("Session completed: {:?}", status));
                }
                Err(e) => {
                    println!();
                    output::error(&format!("Runtime error: {}", e));
                }
            }
        }
        Err(e) => {
            event_handle.abort();
            output::error(&format!("Failed to start runtime: {}", e));
            output::dim("Make sure LocusGraph is configured: locus config graph");
        }
    }

    Ok(())
}
