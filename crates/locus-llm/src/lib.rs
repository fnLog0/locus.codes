//! locus-llm — model interface, prompt builder, response parser (plan §1.1).

mod client;
mod openai;
mod parse;
mod prompt;
mod secrets;
mod types;

pub use client::{ModelClient, OllamaClient};
pub use openai::OpenAIClient;
pub use parse::parse_response;
pub use prompt::{
    build_prompt, tool_definitions, PATCH_AGENT_SYSTEM, REPO_AGENT_SYSTEM,
    TEST_AGENT_SYSTEM, DEBUG_AGENT_SYSTEM, SEARCH_AGENT_SYSTEM, COMMIT_AGENT_SYSTEM,
};
pub use secrets::{DetectedSecret, SecretDetector, SecretKind};
pub use types::{CompletionRequest, CompletionResponse, ModeLimits, ToolCall};
