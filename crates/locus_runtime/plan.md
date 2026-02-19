# locus_runtime — Plan

The orchestrator for locus.codes. Ties together all crates into a cohesive agent loop.

**Philosophy**: Amp-style simplicity — think, plan, execute, observe. LocusGraph for memory.

---

## Purpose

- **Agent loop** — the core while(not_done) cycle
- **Memory injection** — recall before every LLM call
- **Tool dispatch** — route tool calls through ToolBus
- **Event streaming** — emit SessionEvents to TUI
- **Error recovery** — handle failures gracefully
- **Context management** — compress when near token limit

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              RUNTIME                                        │
│                                                                             │
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────┐   ┌──────────────┐   │
│  │   Session   │   │ LocusGraph   │   │     LLM     │   │   ToolBus    │   │
│  │   Manager   │   │   Client     │   │   Client    │   │              │   │
│  └─────────────┘   └──────────────┘   └─────────────┘   └──────────────┘   │
│         ▲                 ▲                  ▲                   ▲          │
│         │                 │                  │                   │          │
│         └─────────────────┴──────────────────┴───────────────────┘          │
│                                    │                                        │
│                              ┌─────┴─────┐                                  │
│                              │   Agent   │                                  │
│                              │   Loop    │                                  │
│                              └─────┬─────┘                                  │
│                                    │                                        │
│                              ┌─────┴─────┐                                  │
│                              │  Event    │──────▶ TUI (locus_ui)            │
│                              │  Channel  │                                  │
│                              └───────────┘                                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Modules

### 1. `runtime`

The main orchestrator struct. Owns all components and runs the agent loop.

```rust
pub struct Runtime {
    pub session: Session,
    pub locus_graph: Arc<LocusGraphClient>,
    pub llm_client: Arc<LlmClient>,
    pub toolbus: Arc<ToolBus>,
    pub event_tx: mpsc::Sender<SessionEvent>,
    pub config: RuntimeConfig,
}

pub struct RuntimeConfig {
    pub model: String,                    // e.g., "claude-sonnet-4-20250514"
    pub provider: LlmProvider,            // Anthropic, OpenAI, Ollama, ZAI
    pub max_turns: Option<u32>,
    pub context_limit: u64,               // token limit before compression
    pub memory_limit: u8,                 // max memories to retrieve (default: 10)
    pub sandbox: SandboxPolicy,
    pub repo_root: PathBuf,
}

pub enum LlmProvider {
    Anthropic,
    OpenAI,
    Ollama,
    ZAI,
}

impl Runtime {
    pub async fn new(config: RuntimeConfig, event_tx: mpsc::Sender<SessionEvent>) -> Result<Self>;
    
    /// Main entry point — run the agent until session ends
    pub async fn run(&mut self, initial_message: String) -> Result<SessionStatus>;
    
    /// Process a single user message
    pub async fn process_message(&mut self, message: String) -> Result<()>;
    
    /// Graceful shutdown
    pub async fn shutdown(&mut self) -> Result<()>;
}
```

### 2. `agent_loop`

The core loop — Amp-style simplicity.

```rust
impl Runtime {
    pub async fn agent_loop(&mut self) -> Result<SessionStatus> {
        loop {
            // 1. Wait for user input (or continue if tools pending)
            let message = match self.wait_for_input().await {
                Some(msg) => msg,
                None => break, // session ended
            };
            
            // 2. Store user intent
            self.store_user_intent(&message).await;
            
            // 3. Recall memories BEFORE LLM call
            let memories = self.recall_memories(&message).await;
            
            // 4. Build prompt with memories
            let prompt = self.build_prompt(&message, &memories);
            
            // 5. Stream LLM response
            let mut stream = self.llm_client.stream(prompt).await?;
            
            // 6. Process chunks
            while let Some(chunk) = stream.next().await {
                match chunk {
                    LlmChunk::Text(text) => {
                        self.emit(SessionEvent::TextDelta(text)).await;
                    }
                    LlmChunk::Thinking(text) => {
                        self.emit(SessionEvent::ThinkingDelta(text)).await;
                    }
                    LlmChunk::ToolUse(tool) => {
                        self.handle_tool_call(tool).await?;
                    }
                }
            }
            
            // 7. Store decision
            self.store_decision().await;
            
            // 8. Check context window
            if self.near_context_limit() {
                self.compress_context().await?;
            }
            
            // 9. Check termination
            if self.should_terminate() {
                break;
            }
        }
        
        Ok(self.session.status.clone())
    }
}
```

### 3. `memory`

Memory recall and storage helpers.

```rust
impl Runtime {
    /// Recall relevant memories before LLM call
    async fn recall_memories(&mut self, query: &str) -> String {
        let result = self.locus_graph
            .retrieve_memories(
                query,
                Some(self.config.memory_limit as u64),
                self.relevant_context_ids(),
                None,
            )
            .await
            .unwrap_or(ContextResult::default());
        
        // Notify TUI
        self.emit(SessionEvent::MemoryRecall {
            query: query.to_string(),
            items_found: result.items_found,
        }).await;
        
        result.memories
    }
    
    /// Build context_ids for query (project, user, recent session)
    fn relevant_context_ids(&self) -> Option<Vec<String>> {
        Some(vec![
            format!("project:{}", self.repo_hash()),
            "decisions".to_string(),
            "errors".to_string(),
            "user_intent".to_string(),
            format!("session:{}", self.session.id.0),
        ])
    }
    
    /// Store user intent
    async fn store_user_intent(&self, message: &str) {
        let _ = self.locus_graph.store_user_intent(
            message,
            &self.summarize_intent(message),
        ).await; // fire-and-forget
    }
    
    /// Store AI decision after turn
    async fn store_decision(&self) {
        let _ = self.locus_graph.store_decision(
            &self.last_decision_summary(),
            Some(&self.last_reasoning()),
        ).await;
    }
    
    /// Store tool run result
    async fn store_tool_run(&self, tool: &ToolUse, result: &ToolResultData) {
        let _ = self.locus_graph.store_tool_run(
            &tool.name,
            &tool.args,
            &result.output,
            result.duration_ms,
            result.is_error,
        ).await;
    }
    
    /// Store error
    async fn store_error(&self, context: &str, error: &str, file: Option<&str>) {
        let _ = self.locus_graph.store_error(context, error, file).await;
    }
}
```

### 4. `tool_handler`

Execute tools via ToolBus and handle results.

```rust
impl Runtime {
    async fn handle_tool_call(&mut self, tool: ToolUse) -> Result<()> {
        // Emit tool start
        self.emit(SessionEvent::ToolStart(tool.clone())).await;
        
        // Execute via ToolBus
        let start = std::time::Instant::now();
        let result = self.toolbus.call(&tool.name, tool.args.clone()).await;
        let duration_ms = start.elapsed().as_millis() as u64;
        
        let tool_result = match result {
            Ok((output, _history_id)) => {
                ToolResultData {
                    output,
                    duration_ms,
                    is_error: false,
                }
            }
            Err(e) => {
                self.store_error("tool_execution", &e.to_string(), tool.file_path.as_ref().map(|p| p.to_str())).await;
                ToolResultData {
                    output: serde_json::json!({ "error": e.to_string() }),
                    duration_ms,
                    is_error: true,
                }
            }
        };
        
        // Store tool run (non-blocking)
        self.store_tool_run(&tool, &tool_result).await;
        
        // Emit tool done
        self.emit(SessionEvent::ToolDone {
            tool_use_id: tool.id.clone(),
            result: tool_result.clone(),
        }).await;
        
        // Add to session turns
        self.add_tool_result(tool, tool_result);
        
        Ok(())
    }
}
```

### 5. `context`

Build prompts and manage context window.

```rust
impl Runtime {
    /// Build the full prompt for LLM
    fn build_prompt(&self, message: &str, memories: &str) -> Prompt {
        Prompt {
            system: self.build_system_prompt(),
            memories: memories.to_string(),
            session_context: self.build_session_context(),
            conversation: self.build_conversation(message),
        }
    }
    
    fn build_system_prompt(&self) -> String {
        format!(
            r#"You are locus.codes, a terminal-native coding agent.

## Role
You help users write, refactor, debug, and understand code.

## Tools Available
{}

## Safety Rules
- Never run destructive commands without confirmation
- Never commit secrets to version control
- Always verify file paths before editing

## Memory
You have access to memories from previous sessions. Use them to maintain consistency and learn from past decisions.
"#,
            self.format_available_tools()
        )
    }
    
    fn build_session_context(&self) -> String {
        format!(
            r#"## Current Session
- Working directory: {}
- Repository: {}
- Active task: {}
- Files recently modified: {}
"#,
            self.config.repo_root.display(),
            self.repo_name(),
            self.current_task(),
            self.recent_files().join(", "),
        )
    }
    
    fn build_conversation(&self, new_message: &str) -> Vec<Message> {
        let mut messages = Vec::new();
        
        // Add previous turns
        for turn in &self.session.turns {
            messages.push(turn.to_message());
        }
        
        // Add new user message
        messages.push(Message {
            role: Role::User,
            content: new_message.to_string(),
        });
        
        messages
    }
    
    /// Check if context is near limit
    fn near_context_limit(&self) -> bool {
        self.estimate_tokens() > (self.config.context_limit as f64 * 0.85) as u64
    }
    
    /// Compress context when near limit
    async fn compress_context(&mut self) -> Result<()> {
        self.emit(SessionEvent::Status(
            "Context near limit, compressing...".to_string()
        ))).await;
        
        // Use LocusGraph insights to summarize
        let summary = self.locus_graph.generate_insights(
            "Summarize the conversation so far, preserving key decisions and context",
            None,
            Some(20),
            None,
            None,
        ).await?;
        
        // Replace old turns with summary
        self.compress_turns(&summary.insight);
        
        self.emit(SessionEvent::Status(
            format!("Context compressed. Summary: {}", summary.insight)
        )).await;
        
        Ok(())
    }
}
```

### 6. `event`

Event emission helpers.

```rust
impl Runtime {
    async fn emit(&self, event: SessionEvent) {
        let _ = self.event_tx.send(event).await;
    }
    
    async fn emit_error(&self, error: String) {
        self.emit(SessionEvent::Error(error)).await;
    }
    
    async fn emit_turn_start(&self, role: Role) {
        self.emit(SessionEvent::TurnStart { role }).await;
    }
    
    async fn emit_turn_end(&self) {
        self.emit(SessionEvent::TurnEnd).await;
    }
    
    async fn emit_session_end(&self, status: SessionStatus) {
        self.emit(SessionEvent::SessionEnd { status }).await;
    }
}
```

### 7. `session_manager`

Manage session state.

```rust
impl Runtime {
    /// Create a new session
    fn create_session(&mut self) -> Session {
        Session {
            id: SessionId(uuid::Uuid::new_v4().to_string()),
            status: SessionStatus::Active,
            repo_root: self.config.repo_root.clone(),
            config: SessionConfig {
                model: self.config.model.clone(),
                provider: format!("{:?}", self.config.provider),
                max_turns: self.config.max_turns,
                sandbox_policy: self.config.sandbox.clone(),
            },
            turns: Vec::new(),
            created_at: chrono::Utc::now(),
        }
    }
    
    /// Add a turn to the session
    fn add_turn(&mut self, turn: Turn) {
        self.session.turns.push(turn);
    }
    
    /// Add tool result to current turn
    fn add_tool_result(&mut self, tool: ToolUse, result: ToolResultData) {
        // Add to the last assistant turn
        if let Some(last_turn) = self.session.turns.last_mut() {
            if last_turn.role == Role::Assistant {
                last_turn.blocks.push(ContentBlock::ToolUse(tool));
                last_turn.blocks.push(ContentBlock::ToolResult(result));
            }
        }
    }
    
    /// Get current task from session
    fn current_task(&self) -> String {
        // Extract from first user intent or latest user message
        self.session.turns
            .iter()
            .find(|t| t.role == Role::User)
            .and_then(|t| t.blocks.iter().find_map(|b| match b {
                ContentBlock::Text(s) => Some(s.clone()),
                _ => None,
            }))
            .unwrap_or_else(|| "No active task".to_string())
    }
    
    /// Get recently modified files
    fn recent_files(&self) -> Vec<String> {
        // Extract from recent tool calls
        self.session.turns
            .iter()
            .rev()
            .take(5)
            .flat_map(|t| t.blocks.iter())
            .filter_map(|b| match b {
                ContentBlock::ToolUse(t) => t.file_path.as_ref().map(|p| p.display().to_string()),
                _ => None,
            })
            .collect()
    }
}
```

---

## Event Flow

```
User Input
    │
    ▼
┌──────────────────┐
│ Runtime receives │
│ message          │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐     ┌─────────────────┐
│ Store user       │────▶│ LocusGraph      │ (async, non-blocking)
│ intent           │     │ store_user_intent│
└────────┬─────────┘     └─────────────────┘
         │
         ▼
┌──────────────────┐     ┌─────────────────┐
│ Recall memories  │────▶│ LocusGraph      │
│                  │◀────│ retrieve        │
└────────┬─────────┘     └─────────────────┘
         │
         ▼
┌──────────────────┐     ┌─────────────────┐
│ Emit MemoryRecall│────▶│ TUI             │
│ event            │     │ "📚 5 memories" │
└────────┬─────────┘     └─────────────────┘
         │
         ▼
┌──────────────────┐
│ Build prompt     │
│ (system + memory │
│ + session + conv)│
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ LLM stream       │
│                  │
└────────┬─────────┘
         │
         ├─────────────────────────────┐
         │                             │
         ▼                             ▼
┌──────────────────┐          ┌─────────────────┐
│ TextDelta        │          │ ToolUse         │
│ ThinkingDelta    │          │                 │
└────────┬─────────┘          └────────┬────────┘
         │                             │
         ▼                             ▼
┌──────────────────┐          ┌─────────────────┐
│ TUI renders      │          │ ToolBus.call()  │
│ streaming text   │          └────────┬────────┘
└──────────────────┘                   │
                                       ▼
                              ┌─────────────────┐
                              │ Store tool run  │
                              │ (LocusGraph)    │
                              └────────┬────────┘
                                       │
                                       ▼
                              ┌─────────────────┐
                              │ Emit ToolDone   │
                              │ to TUI          │
                              └─────────────────┘
```

---

## Error Handling

```rust
impl Runtime {
    async fn handle_error(&mut self, error: RuntimeError) -> Result<()> {
        match error {
            RuntimeError::ToolFailed { tool, message } => {
                self.store_error("tool", &message, Some(&tool)).await;
                self.emit_error(format!("Tool '{}' failed: {}", tool, message)).await;
                // Continue running — don't crash on tool failure
            }
            RuntimeError::LlmFailed(message) => {
                self.store_error("llm", &message, None).await;
                self.emit_error(format!("LLM error: {}", message)).await;
                self.session.status = SessionStatus::Failed(message);
            }
            RuntimeError::ContextOverflow => {
                self.compress_context().await?;
            }
            RuntimeError::MemoryFailed(message) => {
                // Non-blocking — continue without memory
                tracing::warn!("Memory recall failed: {}", message);
            }
        }
        Ok(())
    }
}
```

---

## Dependencies

```toml
[dependencies]
locus-core = { path = "../locus_core" }
locus-graph = { path = "../locus_graph" }
locus-toolbus = { path = "../locus_toolbus" }
locus-llms = { path = "../locus_llms" }

tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "2"
tracing = "0.1"
chrono = "0.4"
uuid = { version = "1", features = ["v4"] }
```

---

## Configuration

### RuntimeConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | `"claude-sonnet-4-20250514"` | LLM model to use |
| `provider` | `LlmProvider` | `Anthropic` | LLM provider |
| `max_turns` | `Option<u32>` | `None` | Max turns per session |
| `context_limit` | `u64` | `200000` | Token limit before compression |
| `memory_limit` | `u8` | `10` | Max memories to retrieve |
| `sandbox` | `SandboxPolicy` | — | File/command restrictions |
| `repo_root` | `PathBuf` | — | Repository root directory |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `LOCUS_MODEL` | Override default model |
| `LOCUS_PROVIDER` | LLM provider (anthropic, openai, ollama, zai) |
| `LOCUS_MAX_TURNS` | Maximum turns per session |
| `LOCUS_CONTEXT_LIMIT` | Token limit |

---

## Build Order

1. `config` — `RuntimeConfig`, `LlmProvider`
2. `error` — `RuntimeError` enum
3. `session_manager` — Session creation and state management
4. `event` — Event emission helpers
5. `memory` — Recall and storage helpers
6. `context` — Prompt building and compression
7. `tool_handler` — Tool execution via ToolBus
8. `agent_loop` — The main loop
9. `runtime` — Top-level orchestrator struct

---

## Key Principles

1. **Simple loop** — Amp/Claude Code style, no complex DAGs
2. **Recall first** — Always query LocusGraph before LLM call
3. **Fire-and-forget storage** — Memory writes never block
4. **Graceful degradation** — Work without memory if LocusGraph fails
5. **Stream everything** — Text, thinking, tools all stream to TUI
6. **Compress when needed** — Auto-compress near context limit
7. **Learn from actions** — Every tool run stored in LocusGraph

---

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_agent_loop_processes_user_message() {
        let (tx, mut rx) = mpsc::channel(100);
        let runtime = Runtime::new(test_config(), tx).await.unwrap();
        
        // Should emit MemoryRecall event
        runtime.process_message("hello".to_string()).await.unwrap();
        
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, SessionEvent::MemoryRecall { .. }));
    }
    
    #[tokio::test]
    async fn test_tool_call_stores_result() {
        // Test that tool calls are stored in LocusGraph
    }
    
    #[tokio::test]
    async fn test_context_compression_near_limit() {
        // Test that context compresses when near limit
    }
}
```
