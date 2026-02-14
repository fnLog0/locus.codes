use crate::textarea::TextArea;
use std::sync::OnceLock;

/// Example prompts that showcase LocusCode's strengths
const LOCUS_CODE_PROMPTS: &[&str] = &[
    "Dockerize my app",
    "Create github actions workflow to automate building and deploying my app on ECS",
    "Load test my service to right-size resources needed",
    "Analyze costs of my cloud account",
];

/// Example shell commands for shell mode
const SHELL_PROMPTS: &[&str] = &[" "];

// Generate a random index once per session
static LOCUS_CODE_INDEX: OnceLock<usize> = OnceLock::new();
static SHELL_INDEX: OnceLock<usize> = OnceLock::new();

pub fn get_placeholder_prompt(textarea: &TextArea) -> &'static str {
    if textarea.is_shell_mode() {
        // Return a random shell prompt (selected once per session)
        let index = SHELL_INDEX.get_or_init(|| {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            use std::time::{SystemTime, UNIX_EPOCH};

            let mut hasher = DefaultHasher::new();
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .hash(&mut hasher);
            hasher.finish() as usize % SHELL_PROMPTS.len()
        });
        SHELL_PROMPTS[*index]
    } else {
        // Return a random LocusCode prompt (selected once per session)
        let index = LOCUS_CODE_INDEX.get_or_init(|| {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            use std::time::{SystemTime, UNIX_EPOCH};

            let mut hasher = DefaultHasher::new();
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .hash(&mut hasher);
            hasher.finish() as usize % LOCUS_CODE_PROMPTS.len()
        });
        LOCUS_CODE_PROMPTS[*index]
    }
}
