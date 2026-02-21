//! Tracing layer that forwards formatted log lines to a sink (e.g. TUI debug traces).

use std::fmt::Write;

use tracing::field::Visit;
use tracing_subscriber::layer::{Context, Layer};

use crate::config::LogSink;

/// Builds a single line from an event: "[LEVEL] target: message key=value ..."
struct LineVisitor {
    buf: String,
}

impl LineVisitor {
    fn new() -> Self {
        Self {
            buf: String::with_capacity(256),
        }
    }

    fn finish(self) -> String {
        self.buf
    }
}

impl Visit for LineVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            if !self.buf.is_empty() {
                self.buf.push(' ');
            }
            self.buf.push_str(value);
        } else {
            if !self.buf.is_empty() {
                self.buf.push(' ');
            }
            write!(self.buf, "{}={:?}", field.name(), value).ok();
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let name = field.name();
        if name == "message" {
            if !self.buf.is_empty() {
                self.buf.push(' ');
            }
            write!(self.buf, "{:?}", value).ok();
        } else {
            if !self.buf.is_empty() {
                self.buf.push(' ');
            }
            write!(self.buf, "{}={:?}", name, value).ok();
        }
    }
}

/// Layer that sends each formatted event to the given sink when present. The sink must not block.
pub(crate) fn tui_log_layer(sink: Option<LogSink>) -> TuiLogLayer {
    TuiLogLayer { sink }
}

#[derive(Clone)]
pub(crate) struct TuiLogLayer {
    sink: Option<LogSink>,
}

impl<S> Layer<S> for TuiLogLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let level = *event.metadata().level();
        let target = event.metadata().target();
        let mut visitor = LineVisitor::new();
        event.record(&mut visitor);
        let rest = visitor.finish();
        let line = if rest.is_empty() {
            format!("[{}] {}", level, target)
        } else {
            format!("[{}] {}: {}", level, target, rest)
        };
        const MAX_LEN: usize = 32_000;
        let line = if line.len() > MAX_LEN {
            let trunc: String = line.chars().take(MAX_LEN).collect();
            format!("{}… ({} chars)", trunc, line.len())
        } else {
            line
        };
        if let Some(ref sink) = self.sink {
            sink(line);
        }
    }
}
