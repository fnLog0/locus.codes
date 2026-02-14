//! locus-core — shared types: events, mode, session (no UI/runtime deps).

pub mod event_bus;
pub mod events;
pub mod mode;
pub mod session;

pub use event_bus::{event_bus, EventRx, EventTx};
pub use events::RuntimeEvent;
pub use mode::Mode;
pub use session::{detect_repo_root, SessionState};
