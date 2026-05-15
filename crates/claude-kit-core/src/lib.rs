pub mod ai;
pub mod bundle_chat;
pub mod bundles;
pub mod harmonize;
pub mod library;
pub mod marketplace;
pub mod project;
pub mod recommend;
pub mod settings;
pub mod update;

// Single lock shared across all test modules that mutate CLAUDE_KIT_HOME.
#[cfg(test)]
pub static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
