//! Announcement feature module: splits DTOs, auth, and handler into focused files.

pub mod handler;
pub mod dto;
pub mod announcement;

// Re-export the public handler so existing call sites can import from crate root.
pub use handler::handle_announcement;


