//! Управление конфигурацией промптов (YAML).

use crate::prompts::PromptsConfig;
use std::path::PathBuf;

/// Менеджер промптов.
pub struct PromptsManager {
    /// Путь к файлу промптов.
    path: PathBuf,
}

impl PromptsManager {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
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
}
