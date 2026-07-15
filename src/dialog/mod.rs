//! GUI dialog frontends, one submodule per platform.

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::{confirm_autofill, show};
