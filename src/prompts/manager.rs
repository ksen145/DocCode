//! Управление конфигурацией промптов (YAML).

use crate::prompts::{Prompt, PromptsConfig};
use std::path::{Path, PathBuf};

/// Менеджер промптов.
pub struct PromptsManager {
    /// Путь к файлу промптов.
    path: PathBuf,
}

impl Default for PromptsManager {
    fn default() -> Self {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            path: config_dir.join("doccode").join("prompts.yaml"),
        }
    }
}

impl PromptsManager {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Загрузка промптов из YAML.
    pub fn load(&self) -> Result<PromptsConfig, String> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("Не удалось прочитать {}: {}", self.path.display(), e))?;
        serde_yaml::from_str(&content)
            .map_err(|e| format!("Некорректный YAML в {}: {}", self.path.display(), e))
    }

    /// Сохранение промптов в YAML.
    pub fn save(&self, config: &PromptsConfig) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Не удалось создать {}: {}", parent.display(), e))?;
        }
        let content = serde_yaml::to_string(config).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, content)
            .map_err(|e| format!("Не удалось записать {}: {}", self.path.display(), e))
    }

    /// Получение промпта по имени (клон — конфиг загружается заново).
    pub fn get_prompt(&self, name: &str) -> Option<Prompt> {
        self.load().ok()?.prompts.get(name).cloned()
    }

    /// Получение промпта или ошибки с перечнем доступных имён.
    pub fn require_prompt(&self, name: &str) -> Result<Prompt, String> {
        let config = self.load()?;
        config.prompts.get(name).cloned().ok_or_else(|| {
            format!(
                "Промпт '{}' не найден (доступны: {})",
                name,
                config
                    .prompts
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    }
}
