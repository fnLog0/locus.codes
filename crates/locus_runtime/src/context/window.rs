//! Context window management — token estimation and compression.

use locus_core::{ContentBlock, Session, SessionEvent, Turn};
use locus_graph::{InsightsOptions, LocusGraphClient};
use tokio::sync::mpsc;
use tracing::info;

use crate::error::RuntimeError;

/// Check if the session is approaching context limit.
///
/// Uses a simple token estimation based on character count.
/// Returns true if estimated tokens exceed 85% of the limit.
pub fn near_context_limit(session: &Session, context_limit: u64) -> bool {
    let estimated_tokens = estimate_session_tokens(session);
    let threshold = (context_limit as f64 * 0.85) as u64;
    estimated_tokens > threshold
}

/// Estimate token count for a session.
///
/// Uses a rough heuristic of ~4 characters per token.
fn estimate_session_tokens(session: &Session) -> u64 {
    let mut char_count = 0usize;

    for turn in &session.turns {
        for block in &turn.blocks {
            match block {
                ContentBlock::Text { text } => char_count += text.len(),
                ContentBlock::Thinking { thinking } => char_count += thinking.len(),
                ContentBlock::Error { error } => char_count += error.len(),
                ContentBlock::ToolUse { tool_use } => {
                    char_count += tool_use.name.len();
                    char_count += tool_use.args.to_string().len();
                }
                ContentBlock::ToolResult { tool_result } => {
                    char_count += tool_result.output.to_string().len();
                }
            }
        }
    }

    // Rough estimate: ~4 characters per token
    (char_count / 4) as u64
}

/// Compress context when approaching limit.
///
/// Uses LocusGraph to generate a summary of the conversation and
/// replaces old turns with a summary turn.
pub async fn compress_context(
    locus_graph: &LocusGraphClient,
    session: &mut Session,
    event_tx: &mpsc::Sender<SessionEvent>,
) -> Result<(), RuntimeError> {
    info!("Compressing context for session {}", session.id.as_str());

    let _ = event_tx
        .send(SessionEvent::status("Context near limit, compressing..."))
        .await;

    // Build a summary prompt from the turns
    let turns_summary = summarize_turns(&session.turns);

    // Use LocusGraph insights to compress
    let options = InsightsOptions::new().limit(20);

    let insight_result = locus_graph
        .generate_insights(
            &format!(
                "Summarize this conversation, preserving key decisions and context:\n\n{}",
                turns_summary
            ),
            Some(options),
        )
        .await
        .map_err(|e| RuntimeError::MemoryFailed(e.to_string()))?;

    // Keep only the last few turns and prepend a summary
    let keep_count = session.turns.len().saturating_sub(3).max(1);
    let summary = insight_result.insight;

    // Create a summary turn
    let summary_turn = Turn::system()
        .with_block(ContentBlock::text(format!(
            "[Context Summary]\n{}",
            summary
        )));

    // Replace old turns with summary
    let recent_turns: Vec<Turn> = session.turns.drain(keep_count..).collect();
    session.turns.clear();
    session.turns.push(summary_turn);
    session.turns.extend(recent_turns);

    let _ = event_tx
        .send(SessionEvent::status(format!(
            "Context compressed. {} turns remaining.",
            session.turn_count()
        )))
        .await;

    Ok(())
}

/// Create a text summary of turns for compression.
fn summarize_turns(turns: &[Turn]) -> String {
    turns
        .iter()
        .map(|t| {
            let role = match t.role {
                locus_core::Role::User => "User",
                locus_core::Role::Assistant => "Assistant",
                locus_core::Role::System => "System",
                locus_core::Role::Tool => "Tool",
            };

            let content: String = t
                .blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            format!("**{}**: {}", role, content.chars().take(500).collect::<String>())
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use locus_core::SessionConfig;

    #[test]
    fn test_near_context_limit_false() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let session = Session::new(std::path::PathBuf::from("/repo"), config);
        let limit = 100_000u64;

        assert!(!near_context_limit(&session, limit));
    }

    #[test]
    fn test_near_context_limit_true() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let mut session = Session::new(std::path::PathBuf::from("/repo"), config);

        let large_text = "x".repeat(400_000);
        session.add_turn(Turn::user().with_block(ContentBlock::text(large_text)));

        let limit = 100_000u64;

        assert!(near_context_limit(&session, limit));
    }

    #[test]
    fn test_estimate_session_tokens() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let mut session = Session::new(std::path::PathBuf::from("/repo"), config);

        // 400 chars should be ~100 tokens
        session.add_turn(
            Turn::user().with_block(ContentBlock::text("x".repeat(400))),
        );

        let tokens = estimate_session_tokens(&session);

        assert_eq!(tokens, 100);
    }

    #[test]
    fn test_summarize_turns() {
        let turns = vec![
            Turn::user().with_block(ContentBlock::text("User message")),
            Turn::assistant().with_block(ContentBlock::text("Assistant response")),
        ];

        let summary = summarize_turns(&turns);

        assert!(summary.contains("User"));
        assert!(summary.contains("Assistant"));
        assert!(summary.contains("User message"));
        assert!(summary.contains("Assistant response"));
    }
}
