//! `locus providers` subcommands.

use anyhow::{anyhow, Result};
use locus_llms::{AnthropicProvider, ProviderRegistry, ZaiProvider};

use crate::cli::ProvidersAction;
use crate::output;

struct ProviderInfo {
    id: String,
    has_key: bool,
    models: &'static [&'static str],
}

const ANTHROPIC_MODELS: &[&str] = &[
    "claude-opus-4-0-20250514",
    "claude-sonnet-4-0-20250514",
    "claude-3-5-sonnet-20241022",
    "claude-3-5-haiku-20241022",
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
];

const ZAI_MODELS: &[&str] = &[
    "glm-4-plus",
    "glm-4-air",
    "glm-4-airx",
    "glm-4-flash",
    "glm-4-long",
    "glm-4v-plus",
    "glm-4v-flash",
];

fn build_registry() -> (ProviderRegistry, Vec<ProviderInfo>) {
    let mut registry = ProviderRegistry::new();
    let mut infos = Vec::new();

    // Try Anthropic
    if let Ok(provider) = AnthropicProvider::from_env() {
        registry = registry.register("anthropic", provider);
        infos.push(ProviderInfo {
            id: "anthropic".to_string(),
            has_key: true,
            models: ANTHROPIC_MODELS,
        });
    } else {
        infos.push(ProviderInfo {
            id: "anthropic".to_string(),
            has_key: false,
            models: ANTHROPIC_MODELS,
        });
    }

    // Try Z.AI
    if let Ok(provider) = ZaiProvider::from_env() {
        registry = registry.register("zai", provider);
        infos.push(ProviderInfo {
            id: "zai".to_string(),
            has_key: true,
            models: ZAI_MODELS,
        });
    } else {
        infos.push(ProviderInfo {
            id: "zai".to_string(),
            has_key: false,
            models: ZAI_MODELS,
        });
    }

    (registry, infos)
}

pub async fn handle(action: ProvidersAction) -> Result<()> {
    match action {
        ProvidersAction::List => list().await,
        ProvidersAction::Info { provider } => info(&provider).await,
        ProvidersAction::Test { provider } => test(&provider).await,
        ProvidersAction::Models { provider } => models(&provider).await,
    }
}

async fn list() -> Result<()> {
    let (_, infos) = build_registry();

    output::header("Registered Providers");

    let mut table = output::table();
    table.set_header(vec![
        comfy_table::Cell::new("Provider")
            .fg(comfy_table::Color::Cyan)
            .add_attribute(comfy_table::Attribute::Bold),
        comfy_table::Cell::new("Status")
            .fg(comfy_table::Color::Cyan)
            .add_attribute(comfy_table::Attribute::Bold),
        comfy_table::Cell::new("Models")
            .fg(comfy_table::Color::Cyan)
            .add_attribute(comfy_table::Attribute::Bold),
    ]);

    for info in &infos {
        let status = if info.has_key {
            comfy_table::Cell::new("configured").fg(comfy_table::Color::Green)
        } else {
            comfy_table::Cell::new("missing API key").fg(comfy_table::Color::Yellow)
        };
        let models_str = format_models(info.models);
        table.add_row(vec![
            comfy_table::Cell::new(&info.id).fg(comfy_table::Color::Green),
            status,
            comfy_table::Cell::new(models_str),
        ]);
    }

    println!("{table}");

    Ok(())
}

fn format_models(models: &[&str]) -> String {
    if models.len() <= 3 {
        models.join(", ")
    } else {
        format!("{}, {}, {} (+{} more)", models[0], models[1], models[2], models.len() - 3)
    }
}

async fn info(provider_id: &str) -> Result<()> {
    let (registry, infos) = build_registry();
    let provider = registry.get_provider(provider_id)?;

    let has_key = infos.iter().any(|i| i.id == provider_id && i.has_key);
    let status = if has_key { "configured" } else { "missing API key" };

    output::header(&format!("Provider: {}", provider_id));
    output::kv("id", provider.provider_id());
    output::kv("status", status);

    Ok(())
}

async fn test(provider_id: &str) -> Result<()> {
    let (registry, _) = build_registry();
    let provider = registry.get_provider(provider_id)?;

    let spinner = output::spinner(&format!("Testing {} connectivity...", provider_id));

    // Test by listing models
    match provider.list_models().await {
        Ok(models) => {
            output::spinner_success(
                &spinner,
                &format!("{} is reachable ({} models)", provider_id, models.len()),
            );
            Ok(())
        }
        Err(e) => {
            output::spinner_error(&spinner, &format!("{} connection failed", provider_id));
            Err(anyhow!("Provider test failed: {}", e))
        }
    }
}

async fn models(provider_id: &str) -> Result<()> {
    let (registry, _) = build_registry();
    let provider = registry.get_provider(provider_id)?;

    let spinner = output::spinner(&format!("Fetching models for {}...", provider_id));

    match provider.list_models().await {
        Ok(models) => {
            spinner.abandon();

            output::header(&format!("Models for {}", provider_id));

            if models.is_empty() {
                output::dim("No models returned (provider may not support listing)");
                return Ok(());
            }

            let mut table = output::table();
            output::table_header(&mut table, "Model", "");

            let items: Vec<_> = models
                .iter()
                .map(|m| {
                    output::table_row(&mut table, m, "");
                    (m.as_str(), "")
                })
                .collect();

            output::table_print(&table, &items);

            Ok(())
        }
        Err(e) => {
            output::spinner_error(&spinner, "Failed to fetch models");
            Err(anyhow!("Failed to list models: {}", e))
        }
    }
}
