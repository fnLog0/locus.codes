//! Message role (user, assistant, system) and display bar.

/// Message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Role {
    /// One-character bar shown at the start of the first line (same side for all roles).
    /// User = █, Assistant = ·, System = !
    pub fn bar_char(&self) -> &'static str {
        match self {
            Role::User => "█",
            Role::Assistant => "·",
            Role::System => "!",
        }
    }

    /// Legacy label; prefer bar_char for display.
    pub fn label(&self) -> &'static str {
        match self {
            Role::User => "YOU",
            Role::Assistant => "ASSISTANT",
            Role::System => "SYSTEM",
        }
    }
}
