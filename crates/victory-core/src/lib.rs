//! Shared plumbing for every VIctory crate and tool.
//!
//! * [`config`]: where the user's files live (never inside the repository).
//! * [`policy`]: the runtime half of rule #1, refusing to write
//!   game-derived data into the source tree.
//! * [`logging`]: one tracing setup for all binaries.

pub mod batch;
pub mod config;
pub mod env;
pub mod fastboot;
pub mod kbm;
pub mod memmap;
pub mod saves;
pub mod logging;
pub mod policy;
pub mod splash;
pub mod sync;

pub use config::Config;
pub use policy::{PolicyError, ensure_outside_repo};
