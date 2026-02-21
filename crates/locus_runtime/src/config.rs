//! Runtime configuration for locus.codes

use std::path::PathBuf;
use std::str::FromStr;

use locus_core::SandboxPolicy;

/// LLM provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LlmProvider {
    #[default]
    Anthropic,
    OpenAI,
    Ollama,
    ZAI,
}

impl LlmProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmProvider::Anthropic => "anthropic",
            LlmProvider::OpenAI => "openai",
            LlmProvider::Ollama => "ollama",
            LlmProvider::ZAI => "zai",
        }
    }
}

impl FromStr for LlmProvider {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(LlmProvider::Anthropic),
            "openai" => Ok(LlmProvider::OpenAI),
            "ollama" => Ok(LlmProvider::Ollama),
            "zai" | "z.ai" => Ok(LlmProvider::ZAI),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// LLM model to use
    pub model: String,
    /// LLM provider
    pub provider: LlmProvider,
    /// Maximum turns per session (None = unlimited)
    pub max_turns: Option<u32>,
    /// Token limit before context compression
    pub context_limit: u64,
    /// Maximum memories to retrieve per query
    pub memory_limit: u8,
    /// Maximum tokens to spend on tool schemas per LLM call
    pub tool_token_budget: u32,
    /// Maximum tokens for LLM response generation
    pub max_tokens: u32,
    /// Sandbox policy for file/command access
    pub sandbox: SandboxPolicy,
    /// Repository root directory
    pub repo_root: PathBuf,
}

impl RuntimeConfig {
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            model: "claude-sonnet-4-20250514".to_string(),
            provider: LlmProvider::default(),
            max_turns: None,
            context_limit: 200_000,
            memory_limit: 10,
            tool_token_budget: 3800,
            max_tokens: 8192,
            sandbox: SandboxPolicy::default(),
            repo_root,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_provider(mut self, provider: LlmProvider) -> Self {
        self.provider = provider;
        if provider == LlmProvider::ZAI && self.model == "claude-sonnet-4-20250514" {
            self.model = std::env::var("ZAI_MODEL").unwrap_or_else(|_| "glm-5".to_string());
        }
        self
    }

    pub fn with_max_turns(mut self, max: u32) -> Self {
        self.max_turns = Some(max);
        self
    }

    pub fn with_context_limit(mut self, limit: u64) -> Self {
        self.context_limit = limit;
        self
    }

    pub fn with_memory_limit(mut self, limit: u8) -> Self {
        self.memory_limit = limit;
        self
    }

    pub fn with_tool_token_budget(mut self, budget: u32) -> Self {
        self.tool_token_budget = budget;
        self
    }

    pub fn with_sandbox(mut self, sandbox: SandboxPolicy) -> Self {
        self.sandbox = sandbox;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Load configuration from environment variables
    pub fn from_env(repo_root: PathBuf) -> Self {
        let mut config = Self::new(repo_root);

        if let Ok(model) = std::env::var("LOCUS_MODEL") {
            config.model = model;
        }

        if let Ok(provider_str) = std::env::var("LOCUS_PROVIDER") {
            if let Ok(provider) = provider_str.parse::<LlmProvider>() {
                config.provider = provider;
            }
        } else {
            // If no LOCUS_PROVIDER, infer from API keys so e.g. ZAI_API_KEY alone uses Zai
            if std::env::var("ZAI_API_KEY").is_ok() {
                config.provider = LlmProvider::ZAI;
            } else if std::env::var("OPENAI_API_KEY").is_ok() {
                config.provider = LlmProvider::OpenAI;
            } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                config.provider = LlmProvider::Anthropic;
            }
        }

        // When using Zai, default model must be a Z.AI model (e.g. glm-5), not Anthropic's
        if config.provider == LlmProvider::ZAI
            && config.model == "claude-sonnet-4-20250514"
        {
            config.model = std::env::var("ZAI_MODEL").unwrap_or_else(|_| "glm-5".to_string());
        }

        if let Ok(max_turns) = std::env::var("LOCUS_MAX_TURNS") {
            if let Ok(val) = max_turns.parse::<u32>() {
                config.max_turns = Some(val);
            }
        }

        if let Ok(limit) = std::env::var("LOCUS_CONTEXT_LIMIT") {
            if let Ok(val) = limit.parse::<u64>() {
                config.context_limit = val;
            }
        }

        if let Ok(budget) = std::env::var("LOCUS_TOOL_BUDGET") {
            if let Ok(val) = budget.parse::<u32>() {
                config.tool_token_budget = val;
            }
        }

        if let Ok(max_tokens) = std::env::var("LOCUS_MAX_TOKENS") {
            if let Ok(val) = max_tokens.parse::<u32>() {
                config.max_tokens = val;
            }
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_provider_as_str() {
        assert_eq!(LlmProvider::Anthropic.as_str(), "anthropic");
        assert_eq!(LlmProvider::OpenAI.as_str(), "openai");
        assert_eq!(LlmProvider::Ollama.as_str(), "ollama");
        assert_eq!(LlmProvider::ZAI.as_str(), "zai");
    }

    #[test]
    fn test_llm_provider_from_str() {
        assert_eq!("anthropic".parse(), Ok(LlmProvider::Anthropic));
        assert_eq!("ANTHROPIC".parse(), Ok(LlmProvider::Anthropic));
        assert_eq!("z.ai".parse(), Ok(LlmProvider::ZAI));
        assert!("unknown".parse::<LlmProvider>().is_err());
    }

    #[test]
    fn test_runtime_config_new() {
        let config = RuntimeConfig::new(PathBuf::from("/repo"));
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.provider, LlmProvider::Anthropic);
        assert_eq!(config.max_turns, None);
        assert_eq!(config.context_limit, 200_000);
        assert_eq!(config.memory_limit, 10);
        assert_eq!(config.tool_token_budget, 3800);
        assert_eq!(config.max_tokens, 8192);
    }

    #[test]
    fn test_runtime_config_builder() {
        let config = RuntimeConfig::new(PathBuf::from("/repo"))
            .with_model("gpt-4")
            .with_provider(LlmProvider::OpenAI)
            .with_max_turns(10)
            .with_context_limit(100_000)
            .with_memory_limit(5)
            .with_tool_token_budget(2000)
            .with_max_tokens(16384);

        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.provider, LlmProvider::OpenAI);
        assert_eq!(config.max_turns, Some(10));
        assert_eq!(config.context_limit, 100_000);
        assert_eq!(config.memory_limit, 5);
        assert_eq!(config.tool_token_budget, 2000);
        assert_eq!(config.max_tokens, 16384);
    }
}
