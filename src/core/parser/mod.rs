//! Модель сущностей кода и извлечение их парсером.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod languages;
pub mod queries;

pub use languages::parse;

/// Сущность кода (функция, класс, модуль и т.д.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Имя сущности.
    pub name: String,
    /// Kind: function, method, class, struct, enum, interface, module, const...
    pub kind: String,
    /// Путь к файлу.
    pub file_path: PathBuf,
    /// Номер строки определения (1-based).
    pub line: u32,
    /// Исходный текст сущности (для передачи в LLM).
    pub code: Option<String>,
}
