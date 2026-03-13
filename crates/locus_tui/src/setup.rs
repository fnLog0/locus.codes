//! Interactive setup wizard state transitions and persistence.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};

use crate::animation::Shimmer;
use crate::state::{Screen, SetupState, SetupStep, TuiState};

pub const PROVIDERS: [(&str, &str, &str); 2] = [
    (
        "anthropic",
        "Anthropic",
        "Claude models (sonnet, opus, haiku)",
    ),
    ("zai", "ZAI", "GLM models (glm-5, glm-4-plus)"),
];

const GRAPH_CHOICES: [&str; 2] = ["Configure now", "Skip for now"];
const DONE_SHIMMER_DURATION: Duration = Duration::from_secs(2);

pub fn provider_id_for_cursor(cursor: usize) -> &'static str {
    PROVIDERS[cursor.min(PROVIDERS.len().saturating_sub(1))].0
}

pub fn provider_label(provider: &str) -> &'static str {
    PROVIDERS
        .iter()
        .find(|(id, _, _)| *id == provider)
        .map(|(_, label, _)| *label)
        .unwrap_or("Unknown")
}

pub fn provider_description(cursor: usize) -> &'static str {
    PROVIDERS[cursor.min(PROVIDERS.len().saturating_sub(1))].2
}

pub fn provider_env_var(provider: &str) -> Result<&'static str> {
    match provider {
        "anthropic" => Ok("ANTHROPIC_API_KEY"),
        "zai" => Ok("ZAI_API_KEY"),
        other => Err(anyhow!("Unsupported provider '{}'", other)),
    }
}

pub fn graph_choice_label(cursor: usize) -> &'static str {
    GRAPH_CHOICES[cursor.min(GRAPH_CHOICES.len().saturating_sub(1))]
}

pub fn setup_progress(step: SetupStep) -> usize {
    match step {
        SetupStep::Welcome => 0,
        SetupStep::SelectProvider => 1,
        SetupStep::EnterApiKey => 2,
        SetupStep::LocusGraphChoice
        | SetupStep::LocusGraphUrl
        | SetupStep::LocusGraphSecret
        | SetupStep::LocusGraphId => 3,
        SetupStep::Confirm => 4,
        SetupStep::Done => 5,
    }
}

pub fn footer_hints(step: SetupStep) -> &'static [(&'static str, &'static str)] {
    match step {
        SetupStep::Welcome => &[("Enter", "begin")],
        SetupStep::SelectProvider => &[("Up/Down", "select"), ("Enter", "confirm")],
        SetupStep::EnterApiKey => &[("Enter", "continue"), ("Esc", "back")],
        SetupStep::LocusGraphChoice => {
            &[("Up/Down", "select"), ("Enter", "confirm"), ("Esc", "back")]
        }
        SetupStep::LocusGraphUrl | SetupStep::LocusGraphSecret | SetupStep::LocusGraphId => {
            &[("Enter", "continue"), ("Esc", "back")]
        }
        SetupStep::Confirm => &[("Enter", "save & start"), ("Esc", "back")],
        SetupStep::Done => &[("Enter", "start chatting")],
    }
}

pub fn mask_preview(value: &str) -> String {
    if value.is_empty() {
        return "not set".to_string();
    }
    if value.chars().count() <= 8 {
        return "*".repeat(value.chars().count());
    }
    let prefix: String = value.chars().take(4).collect();
    let suffix: String = value
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}...{}", prefix, suffix)
}

pub fn mask_for_input(value: &str) -> String {
    "*".repeat(value.chars().count())
}

pub fn handle_setup_enter(state: &mut TuiState) {
    state.setup.error_message = None;

    match state.setup.step {
        SetupStep::Welcome => {
            state.setup.step = SetupStep::SelectProvider;
        }
        SetupStep::SelectProvider => {
            let provider = provider_id_for_cursor(state.setup.provider_cursor).to_string();
            state.setup.selected_provider = Some(provider);
            state.setup.step = SetupStep::EnterApiKey;
        }
        SetupStep::EnterApiKey => {
            if state.setup.api_key.trim().is_empty() {
                state.setup.error_message = Some("Enter an API key to continue.".to_string());
            } else {
                state.setup.step = SetupStep::LocusGraphChoice;
            }
        }
        SetupStep::LocusGraphChoice => {
            state.setup.configure_graph = state.setup.graph_choice_cursor == 0;
            state.setup.step = if state.setup.configure_graph {
                SetupStep::LocusGraphUrl
            } else {
                SetupStep::Confirm
            };
        }
        SetupStep::LocusGraphUrl => {
            if !is_valid_graph_url(&state.setup.graph_url) {
                state.setup.error_message =
                    Some("Enter a valid URL like https://grpc-dev.locusgraph.com:443".to_string());
            } else {
                state.setup.step = SetupStep::LocusGraphSecret;
            }
        }
        SetupStep::LocusGraphSecret => {
            if state.setup.graph_secret.trim().is_empty() {
                state.setup.error_message =
                    Some("Enter a LocusGraph secret or go back and skip it.".to_string());
            } else {
                state.setup.step = SetupStep::LocusGraphId;
            }
        }
        SetupStep::LocusGraphId => {
            if state.setup.graph_id.trim().is_empty() {
                state.setup.error_message = Some("Graph ID cannot be empty.".to_string());
            } else {
                state.setup.step = SetupStep::Confirm;
            }
        }
        SetupStep::Confirm => match save_setup_config(&state.setup) {
            Ok(()) => {
                state.setup.step = SetupStep::Done;
                state.setup.done_shimmer = Some(Shimmer::new());
                state.setup.done_started_at = Some(Instant::now());
                state.status = "Configuration saved to ~/.locus/locus.db".to_string();
                state.status_set_at = Some(Instant::now());
                state.status_permanent = false;
            }
            Err(err) => {
                state.setup.error_message = Some(err.to_string());
            }
        },
        SetupStep::Done => {
            state.screen = Screen::Main;
            state.needs_redraw = true;
            return;
        }
    }

    state.needs_redraw = true;
}

pub fn handle_setup_back(state: &mut TuiState) {
    state.setup.error_message = None;
    state.setup.step = match state.setup.step {
        SetupStep::Welcome => SetupStep::Welcome,
        SetupStep::SelectProvider => SetupStep::Welcome,
        SetupStep::EnterApiKey => SetupStep::SelectProvider,
        SetupStep::LocusGraphChoice => SetupStep::EnterApiKey,
        SetupStep::LocusGraphUrl => SetupStep::LocusGraphChoice,
        SetupStep::LocusGraphSecret => SetupStep::LocusGraphUrl,
        SetupStep::LocusGraphId => SetupStep::LocusGraphSecret,
        SetupStep::Confirm => {
            if state.setup.configure_graph {
                SetupStep::LocusGraphId
            } else {
                SetupStep::LocusGraphChoice
            }
        }
        SetupStep::Done => SetupStep::Done,
    };
    state.needs_redraw = true;
}

pub fn handle_setup_up(state: &mut TuiState) {
    match state.setup.step {
        SetupStep::SelectProvider => {
            if state.setup.provider_cursor == 0 {
                state.setup.provider_cursor = PROVIDERS.len().saturating_sub(1);
            } else {
                state.setup.provider_cursor -= 1;
            }
        }
        SetupStep::LocusGraphChoice => {
            state.setup.graph_choice_cursor = state.setup.graph_choice_cursor.saturating_sub(1);
        }
        _ => {}
    }
    state.needs_redraw = true;
}

pub fn handle_setup_down(state: &mut TuiState) {
    match state.setup.step {
        SetupStep::SelectProvider => {
            state.setup.provider_cursor = (state.setup.provider_cursor + 1) % PROVIDERS.len();
        }
        SetupStep::LocusGraphChoice => {
            state.setup.graph_choice_cursor =
                (state.setup.graph_choice_cursor + 1).min(GRAPH_CHOICES.len() - 1);
        }
        _ => {}
    }
    state.needs_redraw = true;
}

pub fn handle_setup_char(state: &mut TuiState, c: char) {
    if c.is_control() {
        return;
    }
    if let Some(input) = current_input_mut(&mut state.setup) {
        input.push(c);
        state.setup.error_message = None;
        state.needs_redraw = true;
    }
}

pub fn handle_setup_backspace(state: &mut TuiState) {
    if let Some(input) = current_input_mut(&mut state.setup) {
        input.pop();
        state.setup.error_message = None;
        state.needs_redraw = true;
    }
}

pub fn tick_setup_animation(state: &mut TuiState) {
    if state.screen != Screen::Setup || state.setup.step != SetupStep::Done {
        return;
    }

    let Some(started_at) = state.setup.done_started_at else {
        return;
    };

    if started_at.elapsed() > DONE_SHIMMER_DURATION {
        state.setup.done_shimmer = None;
        state.setup.done_started_at = None;
        return;
    }

    if let Some(shimmer) = &mut state.setup.done_shimmer {
        shimmer.tick();
        state.needs_redraw = true;
    }
}

fn current_input_mut(setup: &mut SetupState) -> Option<&mut String> {
    match setup.step {
        SetupStep::EnterApiKey => Some(&mut setup.api_key),
        SetupStep::LocusGraphUrl => Some(&mut setup.graph_url),
        SetupStep::LocusGraphSecret => Some(&mut setup.graph_secret),
        SetupStep::LocusGraphId => Some(&mut setup.graph_id),
        _ => None,
    }
}

fn is_valid_graph_url(url: &str) -> bool {
    let trimmed = url.trim();
    !trimmed.is_empty() && !trimmed.contains(' ') && trimmed.contains("://")
}

fn save_setup_config(setup: &SetupState) -> Result<()> {
    let provider = setup
        .selected_provider
        .as_deref()
        .ok_or_else(|| anyhow!("No provider selected"))?;
    let api_env = provider_env_var(provider)?;
    let locus_dir = global_locus_dir()?;
    let conn = locus_core::db::open_db_at(&locus_dir)?;

    locus_core::db::set_config(&conn, api_env, &format!("\"{}\"", setup.api_key))?;
    locus_core::db::set_config(&conn, "LOCUS_PROVIDER", provider)?;

    if setup.configure_graph {
        locus_core::db::set_config(&conn, "LOCUSGRAPH_AGENT_SECRET", &setup.graph_secret)?;
        locus_core::db::set_config(&conn, "LOCUSGRAPH_SERVER_URL", &setup.graph_url)?;
        locus_core::db::set_config(&conn, "LOCUSGRAPH_GRAPH_ID", &setup.graph_id)?;
    }

    let config = locus_core::db::get_config(&conn)?;
    locus_core::db::sync_env_file(&locus_dir, &config)?;

    unsafe {
        std::env::set_var(api_env, &setup.api_key);
        std::env::set_var("LOCUS_PROVIDER", provider);
        if setup.configure_graph {
            std::env::set_var("LOCUSGRAPH_AGENT_SECRET", &setup.graph_secret);
            std::env::set_var("LOCUSGRAPH_SERVER_URL", &setup.graph_url);
            std::env::set_var("LOCUSGRAPH_GRAPH_ID", &setup.graph_id);
        }
    }

    Ok(())
}

fn global_locus_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    let locus_dir = home.join(".locus");
    std::fs::create_dir_all(&locus_dir)?;
    Ok(locus_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TuiState;

    #[test]
    fn setup_flow_advances_for_minimum_path() {
        let mut state = TuiState::default();
        state.screen = Screen::Setup;

        handle_setup_enter(&mut state);
        assert_eq!(state.setup.step, SetupStep::SelectProvider);

        handle_setup_enter(&mut state);
        assert_eq!(state.setup.step, SetupStep::EnterApiKey);

        state.setup.api_key = "test-key".to_string();
        handle_setup_enter(&mut state);
        assert_eq!(state.setup.step, SetupStep::LocusGraphChoice);

        state.setup.graph_choice_cursor = 1;
        handle_setup_enter(&mut state);
        assert_eq!(state.setup.step, SetupStep::Confirm);
    }

    #[test]
    fn mask_preview_shows_prefix_and_suffix() {
        assert_eq!(mask_preview("abcdefgh1234"), "abcd...1234");
        assert_eq!(mask_preview("short"), "*****");
    }
}
