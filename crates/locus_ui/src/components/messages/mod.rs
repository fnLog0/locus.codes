//! Message component split into role, content, tool, and message.
//!
//! User and assistant are differentiated by a colored bar (█/·) on the same side; assistant is duller.

mod content;
mod message;
mod role;
mod tool;

pub use content::ContentBlock;
pub use message::Message;
pub use role::Role;
pub use tool::{ToolDisplay, ToolStatus};
