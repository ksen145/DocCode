//! Схема промптов (YAML).

use serde::{Deserialize, Serialize};

/// Промпт с переменными.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    /// Описание промпта.
    pub description: String,

    /// Шаблон с {{var}} подстановкой.
    pub template: String,
}

/// Конфигурация промптов.
///
/// `temperature` и `max_tokens` намеренно отсутствуют: это параметры
/// приложения, а не шаблонов.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsConfig {
    /// Версия схемы.
    pub version: u32,

    /// Сами промпты.
    pub prompts: std::collections::HashMap<String, Prompt>,
}
