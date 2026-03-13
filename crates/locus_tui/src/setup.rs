//! Interactive setup wizard handlers and persistence.

use anyhow::{Result, anyhow, bail};

use crate::state::{Screen, SetupProvider, SetupState, SetupStep, TuiState};

const GRAPH_URL_PREFIXES: &[&str] = &["http://", "https://"];

pub fn handle_setup_enter(state: &mut TuiState) {
    state.setup.error_message = None;

    let result = match state.setup.step {
        SetupStep::Welcome => {
            state.setup.step = SetupStep::SelectProvider;
            Ok(())
        }
        SetupStep::SelectProvider => {
            state.setup.selected_provider = Some(
                SetupProvider::ALL[state
                    .setup
                    .provider_cursor
                    .min(SetupProvider::ALL.len() - 1)],
            );
            state.setup.step = SetupStep::EnterApiKey;
            Ok(())
        }
        SetupStep::EnterApiKey => validate_non_empty("API key", &state.setup.api_key).map(|_| {
            state.setup.step = SetupStep::LocusGraphChoice;
        }),
        SetupStep::LocusGraphChoice => {
            state.setup.configure_graph = state.setup.graph_choice_cursor == 0;
            state.setup.step = if state.setup.configure_graph {
                SetupStep::LocusGraphUrl
            } else {
                SetupStep::Confirm
            };
            Ok(())
        }
        SetupStep::LocusGraphUrl => validate_graph_url(&state.setup.graph_url).map(|_| {
            state.setup.step = SetupStep::LocusGraphSecret;
        }),
        SetupStep::LocusGraphSecret => {
            validate_non_empty("LocusGraph secret", &state.setup.graph_secret).map(|_| {
                state.setup.step = SetupStep::LocusGraphId;
            })
        }
        SetupStep::LocusGraphId => {
            validate_non_empty("Graph ID", &state.setup.graph_id).map(|_| {
                state.setup.step = SetupStep::Confirm;
            })
        }
        SetupStep::Confirm => save_setup_config(&state.setup).map(|_| {
            state.setup.step = SetupStep::Done;
            state.setup.done_shimmer = Some(crate::animation::Shimmer::new());
            state.setup.done_shimmer_started_at = Some(std::time::Instant::now());
            state.status = "Setup complete".to_string();
            state.status_set_at = Some(std::time::Instant::now());
            state.status_permanent = false;
        }),
        SetupStep::Done => {
            state.screen = Screen::Main;
            state.status = "Ready".to_string();
            state.status_set_at = Some(std::time::Instant::now());
            state.status_permanent = false;
            Ok(())
        }
    };

    if let Err(err) = result {
        state.setup.error_message = Some(err.to_string());
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
        SetupStep::Done => SetupStep::Confirm,
    };
    if state.setup.step != SetupStep::Done {
        state.setup.done_shimmer = None;
        state.setup.done_shimmer_started_at = None;
    }
    state.needs_redraw = true;
}

pub fn handle_setup_up(state: &mut TuiState) {
    match state.setup.step {
        SetupStep::SelectProvider => {
            state.setup.provider_cursor = state.setup.provider_cursor.saturating_sub(1);
        }
        SetupStep::LocusGraphChoice => {
            state.setup.graph_choice_cursor = state.setup.graph_choice_cursor.saturating_sub(1);
        }
        _ => return,
    }
    state.setup.error_message = None;
    state.needs_redraw = true;
}

pub fn handle_setup_down(state: &mut TuiState) {
    match state.setup.step {
        SetupStep::SelectProvider => {
            state.setup.provider_cursor =
                (state.setup.provider_cursor + 1).min(SetupProvider::ALL.len().saturating_sub(1));
        }
        SetupStep::LocusGraphChoice => {
            state.setup.graph_choice_cursor = (state.setup.graph_choice_cursor + 1).min(1);
        }
        _ => return,
    }
    state.setup.error_message = None;
    state.needs_redraw = true;
}

pub fn handle_setup_char(state: &mut TuiState, c: char) {
    if c.is_control() {
        return;
    }
    let Some(field) = active_field_mut(&mut state.setup) else {
        return;
    };
    field.push(c);
    state.setup.error_message = None;
    state.needs_redraw = true;
}

pub fn handle_setup_backspace(state: &mut TuiState) {
    let Some(field) = active_field_mut(&mut state.setup) else {
        return;
    };
    field.pop();
    state.setup.error_message = None;
    state.needs_redraw = true;
}

fn active_field_mut(setup: &mut SetupState) -> Option<&mut String> {
    match setup.step {
        SetupStep::EnterApiKey => Some(&mut setup.api_key),
        SetupStep::LocusGraphUrl => Some(&mut setup.graph_url),
        SetupStep::LocusGraphSecret => Some(&mut setup.graph_secret),
        SetupStep::LocusGraphId => Some(&mut setup.graph_id),
        _ => None,
    }
}

fn validate_non_empty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} cannot be empty");
    }
    Ok(())
}

fn validate_graph_url(value: &str) -> Result<()> {
    validate_non_empty("LocusGraph URL", value)?;
    if GRAPH_URL_PREFIXES
        .iter()
        .any(|prefix| value.starts_with(prefix))
    {
        return Ok(());
    }
    bail!("LocusGraph URL must start with http:// or https://");
}

fn save_setup_config(setup: &SetupState) -> Result<()> {
    let provider = setup
        .selected_provider
        .ok_or_else(|| anyhow!("No provider selected"))?;

    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    let locus_dir = home.join(".locus");
    std::fs::create_dir_all(&locus_dir)?;

    let conn = locus_core::db::open_db_at(&locus_dir)?;
    locus_core::db::set_config(&conn, provider.env_var(), &format!("\"{}\"", setup.api_key))?;
    locus_core::db::set_config(&conn, "LOCUS_PROVIDER", provider.id())?;

    if setup.configure_graph {
        locus_core::db::set_config(&conn, "LOCUSGRAPH_AGENT_SECRET", &setup.graph_secret)?;
        locus_core::db::set_config(&conn, "LOCUSGRAPH_SERVER_URL", &setup.graph_url)?;
        locus_core::db::set_config(&conn, "LOCUSGRAPH_GRAPH_ID", &setup.graph_id)?;
    }

    let config = locus_core::db::get_config(&conn)?;
    locus_core::db::sync_env_file(&locus_dir, &config)?;

    unsafe {
        std::env::set_var(provider.env_var(), &setup.api_key);
        std::env::set_var("LOCUS_PROVIDER", provider.id());
        if setup.configure_graph {
            std::env::set_var("LOCUSGRAPH_AGENT_SECRET", &setup.graph_secret);
            std::env::set_var("LOCUSGRAPH_SERVER_URL", &setup.graph_url);
            std::env::set_var("LOCUSGRAPH_GRAPH_ID", &setup.graph_id);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TuiState;

    #[test]
    fn backspace_edits_active_field() {
        let mut state = TuiState::new();
        state.setup.step = SetupStep::EnterApiKey;
        state.setup.api_key = "abc".to_string();
        handle_setup_backspace(&mut state);
        assert_eq!(state.setup.api_key, "ab");
    }

    #[test]
    fn enter_advances_when_api_key_is_present() {
        let mut state = TuiState::new();
        state.setup.step = SetupStep::EnterApiKey;
        state.setup.api_key = "sk-test".to_string();
        handle_setup_enter(&mut state);
        assert_eq!(state.setup.step, SetupStep::LocusGraphChoice);
    }

    #[test]
    fn invalid_graph_url_sets_error() {
        let mut state = TuiState::new();
        state.setup.step = SetupStep::LocusGraphUrl;
        state.setup.graph_url = "grpc-dev.locusgraph.com".to_string();
        handle_setup_enter(&mut state);
        assert_eq!(state.setup.step, SetupStep::LocusGraphUrl);
        assert!(state.setup.error_message.is_some());
    }
}
