#[cfg(windows)]
pub mod app;
pub mod asr;
pub mod audio;
#[cfg(windows)]
pub mod bridge;
pub mod bundle;
pub mod edit;
#[cfg(windows)]
pub mod platform;
pub mod store;
#[cfg(windows)]
pub mod ui;
pub mod update;
pub mod update_trust;
mod vad;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
