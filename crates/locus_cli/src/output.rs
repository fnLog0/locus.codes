//! Terminal output helpers — dual-mode: styled text for humans, structured JSON for machines.
//!
//! Uses:
//! - `console` for colors (respects NO_COLOR, auto-disables when piped)
//! - `comfy-table` for structured data
//! - `indicatif` for progress/spinners

use std::sync::atomic::{AtomicBool, Ordering};

use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::cli::OutputFormat;

// ── Global format flag ─────────────────────────────────────────────

static JSON_MODE: AtomicBool = AtomicBool::new(false);

pub fn init(format: OutputFormat) {
    if matches!(format, OutputFormat::Json) {
        JSON_MODE.store(true, Ordering::Relaxed);
    }
}

fn is_json() -> bool {
    JSON_MODE.load(Ordering::Relaxed)
}

// ── JSON envelope ──────────────────────────────────────────────────

#[derive(Serialize)]
struct Msg<'a> {
    level: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<&'a JsonValue>,
}

fn emit_json(level: &str, message: &str, data: Option<&JsonValue>) {
    let msg = Msg {
        level,
        message,
        data,
    };
    let json = serde_json::to_string(&msg).unwrap_or_else(|_| {
        format!(
            "{{\"level\":\"{level}\",\"message\":\"{message}\"}}"
        )
    });
    println!("{json}");
}

// ── Public helpers ─────────────────────────────────────────────────

pub fn header(text: &str) {
    if is_json() {
        emit_json("info", text, None);
    } else {
        println!("{}", style(text).bold().cyan());
    }
}

pub fn success(text: &str) {
    if is_json() {
        emit_json("success", text, None);
    } else {
        println!("{} {}", style("✓").green(), style(text).bright());
    }
}

pub fn error(text: &str) {
    if is_json() {
        let msg = Msg {
            level: "error",
            message: text,
            data: None,
        };
        let json = serde_json::to_string(&msg).unwrap_or_default();
        eprintln!("{json}");
    } else {
        eprintln!("{} {}", style("✗").red(), style(text).bright());
    }
}

pub fn warning(text: &str) {
    if is_json() {
        emit_json("warning", text, None);
    } else {
        println!("{} {}", style("!").yellow(), style(text).bright());
    }
}

pub fn dim(text: &str) {
    if is_json() {
        emit_json("info", text, None);
    } else {
        println!("{}", style(text).dim());
    }
}

pub fn json_pretty(value: &JsonValue) {
    if is_json() {
        emit_json("data", "", Some(value));
    } else {
        let formatted =
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
        println!("{formatted}");
    }
}

/// Print a key-value pair with styled key.
pub fn kv(key: &str, value: &str) {
    if is_json() {
        let data = serde_json::json!({ key: value });
        emit_json("info", key, Some(&data));
    } else {
        println!("  {} {}", style(key).cyan().bold(), value);
    }
}

/// Print session summary at end of run (human-readable or JSON).
pub fn session_summary(summary: &locus_core::SessionSummary) {
    if is_json() {
        let data = serde_json::json!({
            "session_id": summary.session_id,
            "status": format!("{:?}", summary.status),
            "run_duration_ms": summary.run_duration_ms,
            "total_prompt_tokens": summary.total_prompt_tokens,
            "total_completion_tokens": summary.total_completion_tokens,
            "total_tokens": summary.total_tokens(),
            "turn_count": summary.turn_count,
            "tools_used": summary.tools_used,
            "first_user_message": summary.first_user_message,
        });
        emit_json("session_summary", "Session ended", Some(&data));
    } else {
        println!();
        println!("{}", style("── Session summary ──").dim().bold());
        println!("  {} {}", style("Session ID:").cyan().bold(), summary.session_id);
        println!("  {} {:?}", style("Status:").cyan().bold(), summary.status);
        println!(
            "  {} {}",
            style("Run duration:").cyan().bold(),
            summary.run_duration_display()
        );
        println!(
            "  {} {} ({} prompt, {} completion)",
            style("Tokens:").cyan().bold(),
            summary.total_tokens(),
            summary.total_prompt_tokens,
            summary.total_completion_tokens
        );
        println!("  {} {}", style("Turns:").cyan().bold(), summary.turn_count);
        if !summary.tools_used.is_empty() {
            println!(
                "  {} {}",
                style("Tools used:").cyan().bold(),
                summary.tools_used.join(", ")
            );
        }
        if let Some(ref msg) = summary.first_user_message {
            println!("  {} {}", style("First message:").cyan().bold(), msg);
        }
        println!("{}", style("─────────────────────").dim());
    }
}

// ── Tables ─────────────────────────────────────────────────────────

/// Create a styled table for listing items.
pub fn table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Add a header row to the table.
pub fn table_header(table: &mut Table, col1: &str, col2: &str) {
    table.set_header(vec![
        Cell::new(col1)
            .fg(Color::Cyan)
            .add_attribute(comfy_table::Attribute::Bold),
        Cell::new(col2)
            .fg(Color::Cyan)
            .add_attribute(comfy_table::Attribute::Bold),
    ]);
}

/// Add a row to the table.
pub fn table_row(table: &mut Table, name: &str, desc: &str) {
    table.add_row(vec![Cell::new(name).fg(Color::Green), Cell::new(desc)]);
}

/// Print a table (JSON mode emits items array instead).
pub fn table_print(table: &Table, items: &[(&str, &str)]) {
    if is_json() {
        let items: Vec<_> = items
            .iter()
            .map(|(name, desc)| serde_json::json!({ "name": name, "description": desc }))
            .collect();
        let data = serde_json::json!({ "items": items });
        emit_json("list", "", Some(&data));
    } else {
        println!("{table}");
    }
}

// ── Spinners ───────────────────────────────────────────────────────

/// Create a spinner for async operations.
pub fn spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner
}

/// Finish spinner with success message.
pub fn spinner_success(spinner: &ProgressBar, message: &str) {
    spinner.abandon();
    if is_json() {
        emit_json("success", message, None);
    } else {
        println!("{} {}", style("✓").green(), message);
    }
}

/// Finish spinner with error message.
pub fn spinner_error(spinner: &ProgressBar, message: &str) {
    spinner.abandon();
    if is_json() {
        emit_json("error", message, None);
    } else {
        eprintln!("{} {}", style("✗").red(), message);
    }
}
