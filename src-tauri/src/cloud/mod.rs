//! M3 primitives. No automatic connection or native agent-store access.
pub mod crypto;
pub mod desktop;
pub mod drive;
pub mod oauth;
pub mod pending;
pub mod queue;
pub mod vault;
pub type Result<T> = std::result::Result<T, String>;

pub mod wizard;
pub mod wizard_desktop;
