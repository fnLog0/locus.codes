//! TUI run loop: terminal setup, event handling, draw. Optional runtime integration.
//!
//! Key events are read in a dedicated thread so the main loop never blocks on terminal
//! input; this keeps the UI responsive when the stream hangs or the terminal is slow.

use std::io;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use locus_core::SessionEvent;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc as tokio_mpsc;

use crate::runtime_events::apply_session_event;
use crate::state::{ChatItem, Screen, TuiState};
use crate::view;

/// Toggle collapsed state of the last thinking block (key `t` when input empty).
fn toggle_last_think_collapsed(state: &mut TuiState) {
    for item in state.messages.iter_mut().rev() {
        if let ChatItem::Think(m) = item {
            m.collapsed = !m.collapsed;
            state.cache_dirty = true;
            state.needs_redraw = true;
            return;
        }
    }
}

/// Run the TUI: alternate screen, raw mode, event loop. No runtime; Enter echoes as AI.
pub fn run_tui() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState::new();
    state.push_trace_line("[log] TUI started (no runtime). Use Ctrl+D for runtime logs.".to_string());
    let result = run_loop(&mut terminal, &mut state, None, None, None, None, None);

    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    disable_raw_mode()?;

    result
}

/// Run the TUI with runtime: receive [SessionEvent] on `event_rx`, send user messages on Enter via `user_msg_tx`.
/// If `log_rx` is provided, runtime log lines (tracing) are pushed to the debug traces screen (Ctrl+D).
/// If `new_session_tx` is provided, Ctrl+N sends a signal to start a new session (next message uses fresh runtime).
/// If `cancel_tx` is provided, first Ctrl+C during streaming sends cancel (halts run); second Ctrl+C exits TUI.
pub fn run_tui_with_runtime(
    mut event_rx: tokio_mpsc::Receiver<SessionEvent>,
    user_msg_tx: tokio_mpsc::Sender<String>,
    log_rx: Option<tokio_mpsc::Receiver<String>>,
    new_session_tx: Option<tokio_mpsc::Sender<()>>,
    cancel_tx: Option<tokio_mpsc::Sender<()>>,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState::new();
    state.push_trace_line("[log] TUI started with runtime. Runtime logs (Ctrl+D) show tracing output.".to_string());
    let result = run_loop(
        &mut terminal,
        &mut state,
        Some(&mut event_rx),
        Some(&user_msg_tx),
        log_rx,
        new_session_tx.as_ref(),
        cancel_tx.as_ref(),
    );

    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    disable_raw_mode()?;

    result
}

const STATUS_TIMEOUT: Duration = Duration::from_secs(5);

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut TuiState,
    mut event_rx: Option<&mut tokio_mpsc::Receiver<SessionEvent>>,
    user_msg_tx: Option<&tokio_mpsc::Sender<String>>,
    mut log_rx: Option<tokio_mpsc::Receiver<String>>,
    new_session_tx: Option<&tokio_mpsc::Sender<()>>,
    cancel_tx: Option<&tokio_mpsc::Sender<()>>,
) -> anyhow::Result<()> {
    let (key_tx, key_rx) = mpsc::channel();
    let _reader = std::thread::spawn(move || {
        loop {
            if event::poll(Duration::from_millis(50)).unwrap_or(false)
                && let Ok(ev) = event::read()
            {
                let _ = key_tx.send(ev);
            }
        }
    });

    loop {
        // Drain runtime log lines into debug traces (multi-line logs split into separate lines)
        if let Some(ref mut rx) = log_rx {
            while let Ok(line) = rx.try_recv() {
                for l in line.split('\n') {
                    state.push_trace_line(l.to_string());
                }
            }
        }
        // Drain session events from runtime
        if let Some(ref mut rx) = event_rx {
            while let Ok(event) = rx.try_recv() {
                apply_session_event(state, event);
            }
        }
        if state.auto_scroll {
            state.scroll = 0;
        }

        // Status timeout: clear transient status after 5s
        if !state.status_permanent
            && let Some(set_at) = state.status_set_at
            && set_at.elapsed() > STATUS_TIMEOUT
        {
            state.status.clear();
            state.status_set_at = None;
            state.needs_redraw = true;
        }

        let streaming_active = state.is_streaming
            && (!state.current_ai_text.is_empty() || !state.current_think_text.is_empty());
        let should_draw = state.needs_redraw
            || (state.is_streaming && state.current_ai_text.is_empty() && state.current_think_text.is_empty())
            || streaming_active;

        if should_draw {
            state.frame_count = state.frame_count.wrapping_add(1);
            terminal.draw(|f| view::draw(f, state, f.area()))?;
            state.needs_redraw = false;
        }

        if let Ok(ev) = key_rx.try_recv() {
            match ev {
                Event::Key(e) => {
                    if e.kind != KeyEventKind::Press {
                        continue;
                    }
                    match e.code {
                        KeyCode::Char('d') if e.modifiers.contains(KeyModifiers::CONTROL) => {
                            state.screen = match state.screen {
                                Screen::Main => Screen::DebugTraces,
                                Screen::DebugTraces => Screen::Main,
                            };
                            state.needs_redraw = true;
                        }
                        KeyCode::Char('n') if e.modifiers.contains(KeyModifiers::CONTROL) && state.screen == Screen::Main => {
                            if let Some(tx) = new_session_tx {
                                let _ = tx.try_send(());
                                state.push_separator("New session".to_string());
                                state.status = "New session — next message starts fresh".to_string();
                                state.status_set_at = Some(std::time::Instant::now());
                                state.status_permanent = false;
                                state.needs_redraw = true;
                            }
                        }
                        KeyCode::Char('c') if e.modifiers.contains(KeyModifiers::CONTROL) => {
                            if state.is_streaming {
                                if let Some(tx) = cancel_tx {
                                    let _ = tx.try_send(());
                                    state.status = "Cancelling… (Ctrl+C again to quit)".to_string();
                                    state.status_set_at = Some(std::time::Instant::now());
                                    state.needs_redraw = true;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        KeyCode::Char('q') if state.input_buffer.is_empty() => break,
                        KeyCode::Esc if state.screen == Screen::DebugTraces => {
                            state.screen = Screen::Main;
                            state.needs_redraw = true;
                        }
                        KeyCode::Up if state.screen == Screen::DebugTraces => state.trace_scroll_up(1),
                        KeyCode::Down if state.screen == Screen::DebugTraces => state.trace_scroll_down(1),
                        KeyCode::PageUp if state.screen == Screen::DebugTraces => state.trace_scroll_up(10),
                        KeyCode::PageDown if state.screen == Screen::DebugTraces => state.trace_scroll_down(10),
                        KeyCode::Up if state.screen == Screen::Main => state.scroll_up(1),
                        KeyCode::Down if state.screen == Screen::Main => state.scroll_down(1),
                        KeyCode::PageUp if state.screen == Screen::Main => state.scroll_up(5),
                        KeyCode::PageDown if state.screen == Screen::Main => state.scroll_down(5),
                        KeyCode::Enter if state.screen == Screen::Main => {
                            let line = state.input_take();
                            let trimmed = line.trim();
                            if !trimmed.is_empty() {
                                state.push_user(trimmed.to_string(), None);
                                if let Some(tx) = user_msg_tx {
                                    let _ = tx.try_send(trimmed.to_string());
                                } else {
                                    state.push_ai(format!("You said: {}", trimmed), None);
                                }
                            }
                        }
                        KeyCode::Backspace if state.screen == Screen::Main => state.input_backspace(),
                        KeyCode::Char('u') if e.modifiers.contains(KeyModifiers::CONTROL) && state.screen == Screen::Main => state.input_clear_line(),
                        KeyCode::Char('k') if e.modifiers.contains(KeyModifiers::CONTROL) && state.screen == Screen::Main => state.input_kill_to_end(),
                        KeyCode::Char('t') if state.input_buffer.is_empty() && state.screen == Screen::Main => toggle_last_think_collapsed(state),
                        KeyCode::Char('y') if e.modifiers.contains(KeyModifiers::CONTROL) && state.input_buffer.is_empty() && state.screen == Screen::Main => {
                            copy_last_ai_to_clipboard(state);
                        }
                        KeyCode::Char(c) if state.screen == Screen::Main => state.input_insert(c),
                        KeyCode::Left if state.screen == Screen::Main => state.input_cursor_left(),
                        KeyCode::Right if state.screen == Screen::Main => state.input_cursor_right(),
                        KeyCode::Home if state.screen == Screen::Main => state.input_cursor_home(),
                        KeyCode::End if state.screen == Screen::Main => state.input_cursor_end(),
                        KeyCode::Delete if state.screen == Screen::Main => state.input_delete(),
                        _ => {}
                    }
                }
                Event::Resize(_, _) => {
                    state.cache_dirty = true;
                    state.needs_redraw = true;
                }
                Event::Mouse(me) => {
                    match me.kind {
                        MouseEventKind::ScrollUp => {
                            if state.screen == Screen::DebugTraces {
                                state.trace_scroll_up(3);
                            } else {
                                state.scroll_up(3);
                            }
                            state.needs_redraw = true;
                        }
                        MouseEventKind::ScrollDown => {
                            if state.screen == Screen::DebugTraces {
                                state.trace_scroll_down(3);
                            } else {
                                state.scroll_down(3);
                            }
                            state.needs_redraw = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        } else {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    Ok(())
}

/// Copy last AI message to system clipboard (Ctrl+Y when input empty).
fn copy_last_ai_to_clipboard(state: &mut TuiState) {
    let text = state
        .messages
        .iter()
        .rev()
        .find_map(|m| {
            if let ChatItem::Ai(ai) = m {
                Some(ai.text.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();
    if text.is_empty() {
        return;
    }
    if cli_clipboard::set_contents(text).is_ok() {
        state.status = "Copied to clipboard".to_string();
        state.status_set_at = Some(std::time::Instant::now());
        state.status_permanent = false;
        state.needs_redraw = true;
    }
}
