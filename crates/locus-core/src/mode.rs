//! Mode: Rush / Smart / Deep

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Mode {
    Rush,
    #[default]
    Smart,
    Deep,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Rush => write!(f, "Rush"),
            Mode::Smart => write!(f, "Smart"),
            Mode::Deep => write!(f, "Deep"),
        }
    }
}
