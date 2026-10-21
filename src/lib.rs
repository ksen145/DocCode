//! Библиотека DocCode — генератор документации для кода.

// Переводы UI: locales/*.yml вшиваются в крейт на этапе компиляции (rust-i18n).
rust_i18n::i18n!("locales", fallback = "en");

pub mod ai;
pub mod app;
pub mod config;
pub mod core;
pub mod docs;
pub mod index;
pub mod prompts;
pub mod ui;
pub mod utils;

/// Версия пакета.
pub const VERSION: &str = "0.1.0";
