//! `locus run` command - start the runtime agent.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use tokio::sync::mpsc;

use locus_core::SessionEvent;
use locus_runtime::{Runtime, RuntimeConfig};
use locusgraph_observability::{init_from_env, shutdown};

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

    // Initialize tracing/observability (console + optional OTLP from env)
    if let Err(e) = init_from_env() {
        output::warning(&format!("Observability init failed (continuing): {}", e));
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
            let result = runtime.run(initial_message, None).await;

            // Shutdown
            let _ = runtime.shutdown().await;

            // Wait for event handler
            event_handle.abort();

            match result {
                Ok(status) => {
                    println!();
                    output::success(&format!("Session completed: {:?}", status));
                    let summary = runtime.session.build_summary();
                    output::session_summary(&summary);
                }
                Err(e) => {
                    println!();
                    output::error(&format!("Runtime error: {}", e));
                    let summary = runtime.session.build_summary();
                    output::session_summary(&summary);
                }
            }

            // Optional: continue in a new session (extend to next session)
            loop {
                println!();
                output::dim("Continue in new session? (y/n):");
                let mut input = String::new();
                if std::io::stdin().read_line(&mut input).is_err() || input.is_empty() {
                    break;
                }
                if !input.trim().eq_ignore_ascii_case("y") && !input.trim().eq_ignore_ascii_case("yes") {
                    break;
                }

                output::dim("Enter message for next session (or empty to exit):");
                let mut next_input = String::new();
                if std::io::stdin().read_line(&mut next_input).is_err() {
                    break;
                }
                let next_message = next_input.trim().to_string();
                if next_message.is_empty() {
                    break;
                }

                let (next_tx, mut next_rx) = mpsc::channel::<SessionEvent>(256);
                let event_handle_next = tokio::spawn(async move {
                    use console::style;
                    while let Some(event) = next_rx.recv().await {
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
                                std::io::stdout().flush().ok();
                            }
                            SessionEvent::ThinkingDelta { thinking } => {
                                print!("{}", style(thinking).dim());
                                std::io::stdout().flush().ok();
                            }
                            SessionEvent::ToolStart { tool_use } => {
                                println!("\n  {} {}", style("Tool:").yellow(), style(&tool_use.name).bold());
                            }
                            SessionEvent::ToolDone { result, .. } => {
                                let preview = if result.is_error {
                                    format!("Error: {}", result.output)
                                } else {
                                    let s = result.output.to_string();
                                    if s.len() > 200 { format!("{}...", &s[..200]) } else { s }
                                };
                                println!("  {} {}", style("Result:").dim(), preview);
                            }
                            SessionEvent::Error { error } => eprintln!("\n{} {}", style("Error:").red(), error),
                            SessionEvent::Status { message } => output::dim(&format!("  {}", message)),
                            SessionEvent::MemoryRecall { items_found, .. } => {
                                if items_found > 0 {
                                    output::dim(&format!("  Recalled {} memories", items_found));
                                }
                            }
                            _ => {}
                        }
                    }
                });

                let prev_session = &runtime.session;
                let config = runtime.config.clone();
                let toolbus = std::sync::Arc::clone(&runtime.toolbus);
                let locus_graph = std::sync::Arc::clone(&runtime.locus_graph);
                let llm_client = std::sync::Arc::clone(&runtime.llm_client);

                match Runtime::new_continuing(prev_session, config, next_tx, toolbus, locus_graph, llm_client) {
                    Ok(mut next_runtime) => {
                        output::dim(&format!("  New session (continues from {})\n", prev_session.id));
                        let next_result = next_runtime.run(next_message, None).await;
                        let _ = next_runtime.shutdown().await;
                        event_handle_next.abort();

                        match next_result {
                            Ok(status) => {
                                println!();
                                output::success(&format!("Session completed: {:?}", status));
                                let summary = next_runtime.session.build_summary();
                                output::session_summary(&summary);
                                runtime = next_runtime;
                            }
                            Err(e) => {
                                println!();
                                output::error(&format!("Runtime error: {}", e));
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        event_handle_next.abort();
                        output::error(&format!("Failed to start continued session: {}", e));
                        break;
                    }
                }
            }
        }
        Err(e) => {
            event_handle.abort();
            output::error(&format!("Failed to start runtime: {}", e));
            output::dim("Make sure LocusGraph is configured: locus config graph");
        }
    }

    shutdown();
    Ok(())
}
