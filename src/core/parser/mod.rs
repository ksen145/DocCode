//! Интерфейс парсеров: извлечение сущностей из исходного кода.

use crate::core::scanner::language_for_path;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod languages;
pub mod queries;

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

/// Трейт языкового парсера.
pub trait LanguageParser: Send + Sync {
    /// Извлечь сущности из содержимого файла.
    fn parse(&self, content: &str, file_path: &Path) -> Vec<Entity>;
}

/// Фабрика парсеров по пути к файлу.
pub fn get_parser_for_path(path: &Path) -> Option<Box<dyn LanguageParser>> {
    let lang = language_for_path(path)?;
    get_parser(lang)
}

/// Фабрика парсеров по имени языка (см. scanner::SUPPORTED_LANGUAGES).
pub fn get_parser(language: &str) -> Option<Box<dyn LanguageParser>> {
    languages::by_language(language)
}
