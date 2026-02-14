//! Secrets safety: detect and redact sensitive data (plan §1.2).

use regex::Regex;
use std::collections::HashSet;

lazy_static::lazy_static! {
    // Common API key prefixes
    static ref API_KEY_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(sk-[a-zA-Z0-9]{20,})").unwrap(), // OpenAI
        Regex::new(r"(?i)(AKIA[0-9A-Z]{16})").unwrap(), // AWS Access Key
        Regex::new(r"(?i)(AIza[0-9A-Za-z\-_]{35})").unwrap(), // Google API
        Regex::new(r"(?i)(ghp_[a-zA-Z0-9]{36})").unwrap(), // GitHub PAT
        Regex::new(r"(?i)(xox[baprs]-[0-9]{12}-[0-9]{12}-[0-9]{12}-[a-zA-Z0-9]{32})").unwrap(), // Slack
        Regex::new(r"(?i)(pk_live_[a-zA-Z0-9]{24})").unwrap(), // Stripe
        Regex::new(r"(?i)(Bearer [a-zA-Z0-9\-_\.]+)").unwrap(), // Bearer tokens
        Regex::new(r"(?i)(Basic [a-zA-Z0-9\-_\.:]+=*)").unwrap(), // Basic auth
    ];

    // Password/connection string patterns
    static ref SECRET_PATTERNS: Vec<Regex> = vec![
        Regex::new(r#"(?i)(password\s*=\s*['"][^'"]+['"])"#).unwrap(),
        Regex::new(r#"(?i)(passwd\s*=\s*['"][^'"]+['"])"#).unwrap(),
        Regex::new(r#"(?i)(api[_-]?key\s*=\s*['"][^'"]+['"])"#).unwrap(),
        Regex::new(r#"(?i)(secret\s*=\s*['"][^'"]+['"])"#).unwrap(),
        Regex::new(r#"(?i)(token\s*=\s*['"][^'"]+['"])"#).unwrap(),
        Regex::new(r"(?i)(mongodb://[^@]+:[^@]+@)").unwrap(),
        Regex::new(r"(?i)(postgres://[^@]+:[^@]+@)").unwrap(),
        Regex::new(r"(?i)(mysql://[^@]+:[^@]+@)").unwrap(),
    ];

    // Base64 encoded strings (likely secrets)
    static ref BASE64_PATTERN: Regex = Regex::new(r"[A-Za-z0-9+/]{40,}={0,2}").unwrap();
}

/// Detected secret with position information
#[derive(Debug, Clone)]
pub struct DetectedSecret {
    pub kind: SecretKind,
    pub matched_text: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    ApiKey,
    Password,
    ConnectionString,
    Base64Secret,
}

impl std::fmt::Display for SecretKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretKind::ApiKey => write!(f, "API Key"),
            SecretKind::Password => write!(f, "Password"),
            SecretKind::ConnectionString => write!(f, "Connection String"),
            SecretKind::Base64Secret => write!(f, "Base64 Encoded Secret"),
        }
    }
}

/// Secret detector for scanning text for sensitive data
pub struct SecretDetector {
    enable_base64_detection: bool,
    blocked_env_vars: HashSet<String>,
}

impl SecretDetector {
    pub fn new() -> Self {
        Self {
            enable_base64_detection: false, // Disabled by default (high false positive rate)
            blocked_env_vars: Self::default_blocked_env_vars(),
        }
    }

    /// Enable/disable base64 detection (can be noisy)
    pub fn with_base64_detection(mut self, enable: bool) -> Self {
        self.enable_base64_detection = enable;
        self
    }

    /// Set custom blocked environment variables
    pub fn with_blocked_env_vars(mut self, vars: Vec<String>) -> Self {
        self.blocked_env_vars = vars.into_iter().collect();
        self
    }

    fn default_blocked_env_vars() -> HashSet<String> {
        vec![
            "OPENAI_API_KEY".to_string(),
            "ANTHROPIC_API_KEY".to_string(),
            "AWS_ACCESS_KEY_ID".to_string(),
            "AWS_SECRET_ACCESS_KEY".to_string(),
            "GITHUB_TOKEN".to_string(),
            "GITHUB_PAT".to_string(),
            "SLACK_TOKEN".to_string(),
            "STRIPE_API_KEY".to_string(),
            "DATABASE_URL".to_string(),
            "POSTGRES_URL".to_string(),
            "MONGO_URL".to_string(),
            "REDIS_URL".to_string(),
            "API_KEY".to_string(),
            "SECRET".to_string(),
            "PASSWORD".to_string(),
            "TOKEN".to_string(),
            "PRIVATE_KEY".to_string(),
            "AUTH_TOKEN".to_string(),
        ]
        .into_iter()
        .collect()
    }

    /// Scan text for secrets
    pub fn scan(&self, text: &str) -> Vec<DetectedSecret> {
        let mut secrets = Vec::new();

        // Check API key patterns
        for pattern in API_KEY_PATTERNS.iter() {
            for mat in pattern.find_iter(text) {
                secrets.push(DetectedSecret {
                    kind: SecretKind::ApiKey,
                    matched_text: mat.as_str().to_string(),
                    start: mat.start(),
                    end: mat.end(),
                });
            }
        }

        // Check password/connection patterns
        for pattern in SECRET_PATTERNS.iter() {
            for mat in pattern.find_iter(text) {
                secrets.push(DetectedSecret {
                    kind: SecretKind::Password,
                    matched_text: mat.as_str().to_string(),
                    start: mat.start(),
                    end: mat.end(),
                });
            }
        }

        // Check base64 (if enabled)
        if self.enable_base64_detection {
            for mat in BASE64_PATTERN.find_iter(text) {
                // Only flag if it's unusually long or looks like a secret
                if mat.as_str().len() >= 40 {
                    secrets.push(DetectedSecret {
                        kind: SecretKind::Base64Secret,
                        matched_text: mat.as_str().to_string(),
                        start: mat.start(),
                        end: mat.end(),
                    });
                }
            }
        }

        secrets
    }

    /// Filter environment variables, removing sensitive ones
    pub fn filter_env_vars(&self, env: &[(String, String)]) -> Vec<(String, String)> {
        env.iter()
            .filter(|(k, _)| !self.blocked_env_vars.contains(k))
            .cloned()
            .collect()
    }

    /// Redact secrets from text
    pub fn redact(text: &str, secrets: &[DetectedSecret]) -> String {
        if secrets.is_empty() {
            return text.to_string();
        }

        let mut result = String::new();
        let mut last_end = 0;

        for secret in secrets {
            // Add text before the secret
            if secret.start > last_end {
                result.push_str(&text[last_end..secret.start]);
            }
            // Add redaction
            result.push_str("[REDACTED]");
            last_end = secret.end;
        }

        // Add remaining text
        if last_end < text.len() {
            result.push_str(&text[last_end..]);
        }

        result
    }

    /// Check if any secrets were detected
    pub fn has_secrets(&self, text: &str) -> bool {
        !self.scan(text).is_empty()
    }
}

impl Default for SecretDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_openai_key() {
        let detector = SecretDetector::new();
        let text = "sk-proj-XGSoj6zXTRu7Xa0i8YwjC0WhQPCAqJ-bkDcIp00opVNJM6c1F1VthebjBCtqHk";
        let secrets = detector.scan(text);
        assert!(!secrets.is_empty());
        assert_eq!(secrets[0].kind, SecretKind::ApiKey);
    }

    #[test]
    fn test_detect_aws_key() {
        let detector = SecretDetector::new();
        let text = "AKIAIOSFODNN7EXAMPLE";
        let secrets = detector.scan(text);
        assert!(!secrets.is_empty());
        assert_eq!(secrets[0].kind, SecretKind::ApiKey);
    }

    #[test]
    fn test_detect_password() {
        let detector = SecretDetector::new();
        let text = "password=\"mysecretpassword\"";
        let secrets = detector.scan(text);
        assert!(!secrets.is_empty());
        assert_eq!(secrets[0].kind, SecretKind::Password);
    }

    #[test]
    fn test_redact() {
        let detector = SecretDetector::new();
        let text = "sk-proj-abc123 and normal text";
        let secrets = detector.scan(text);
        let redacted = detector.redact(text, &secrets);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("sk-proj"));
    }

    #[test]
    fn test_filter_env_vars() {
        let detector = SecretDetector::new();
        let env = vec![
            ("OPENAI_API_KEY".to_string(), "sk-123".to_string()),
            ("EDITOR".to_string(), "vim".to_string()),
        ];
        let filtered = detector.filter_env_vars(&env);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "EDITOR");
    }
}
