//! Конфигурация приложения: Ollama, параметры генерации, политика D1 и пути.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Конфигурация подключения к Ollama.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBackendConfig {
    /// URL API (по умолчанию http://localhost:11434).
    pub base_url: Option<String>,
    /// Имя модели генерации.
    pub model: String,
    /// Модель эмбеддингов для векторного индекса кода.
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    /// Модель D1 для контроля качества документации.
    #[serde(default = "default_d1_model")]
    pub d1_model: String,
}

/// Значение по умолчанию для модели эмбеддингов.
fn default_embedding_model() -> String {
    "nomic-embed-text".to_string()
}

/// Значение по умолчанию для модели D1.
fn default_d1_model() -> String {
    "d1".to_string()
}

impl Default for AiBackendConfig {
    fn default() -> Self {
        Self {
            base_url: Some("http://localhost:11434".to_string()),
            // Рекомендуемая модель генерации для Ollama.
            model: "gemma4:e4b".to_string(),
            embedding_model: default_embedding_model(),
            d1_model: default_d1_model(),
        }
    }
}

/// Параметры генерации текста.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Температура сэмплирования: 0.0–2.0.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Ограничение длины ответа в токенах.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Разрешить модели рассуждать перед ответом (`think` в Ollama).
    #[serde(default = "default_think")]
    pub think: bool,
}

fn default_temperature() -> f32 {
    0.15
}

fn default_max_tokens() -> u32 {
    4000
}

fn default_think() -> bool {
    true
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            think: default_think(),
        }
    }
}

/// Настраиваемая политика контроля качества D1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct D1Config {
    /// Минимальная вероятность фактической точности (0.0–1.0).
    #[serde(default = "default_min_factuality")]
    pub min_factuality: f32,
    /// Минимальная оценка полноты по шкале 1–5.
    #[serde(default = "default_min_completeness")]
    pub min_completeness: u8,
    /// Сколько раз повторять генерацию при отказе D1.
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
}

fn default_min_factuality() -> f32 {
    0.85
}

fn default_min_completeness() -> u8 {
    3
}

fn default_max_retries() -> u8 {
    1
}

impl Default for D1Config {
    fn default() -> Self {
        Self {
            min_factuality: default_min_factuality(),
            min_completeness: default_min_completeness(),
            max_retries: default_max_retries(),
        }
    }
}

impl D1Config {
    /// Проверка диапазонов перед сохранением/запуском.
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.min_factuality) {
            return Err("Порог точности D1 должен быть от 0.0 до 1.0".to_string());
        }
        if !(1..=5).contains(&self.min_completeness) {
            return Err("Порог полноты D1 должен быть от 1 до 5".to_string());
        }
        if self.max_retries > 5 {
            return Err("Число повторов D1 должно быть от 0 до 5".to_string());
        }
        Ok(())
    }
}

/// Тема интерфейса: системная (по умолчанию) или явно выбранная.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Следовать теме операционной системы.
    #[default]
    System,
    /// Всегда светлая тема.
    Light,
    /// Всегда тёмная тема.
    Dark,
}

impl ThemeMode {
    /// Тёмная ли тема с учётом настройки операционной системы.
    pub fn is_dark(self) -> bool {
        match self {
            Self::Light => false,
            Self::Dark => true,
            Self::System => matches!(dark_light::detect(), dark_light::Mode::Dark),
        }
    }
}

/// Конфигурация приложения.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// AI-бэкенд.
    pub ai_backend: AiBackendConfig,
    /// Параметры генерации (temperature/max_tokens/think).
    #[serde(default)]
    pub generation: GenerationConfig,
    /// Настройки контроля качества D1.
    #[serde(default)]
    pub d1: D1Config,
    /// Путь к файлу промптов (YAML).
    pub prompts_path: PathBuf,
    /// Путь к проекту для документирования.
    pub project_path: Option<PathBuf>,
    /// Путь для записи документации.
    pub docs_path: Option<PathBuf>,
    /// Тема GUI: системная, светлая или тёмная.
    #[serde(default)]
    pub theme: ThemeMode,
    /// Язык интерфейса: "ru"/"en".
    #[serde(default = "default_lang")]
    pub lang: String,
    /// Проверять сгенерированную документацию моделью D1.
    #[serde(default = "default_verify_d1")]
    pub verify_d1: bool,
}

/// D1-проверка включена по умолчанию: без неё качество документации не контролируется.
fn default_verify_d1() -> bool {
    true
}

/// Значение по умолчанию для `lang` (отсутствует в старых config.toml).
fn default_lang() -> String {
    "ru".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

impl AppConfig {
    /// Конфигурация по умолчанию (пути от dirs::config_dir()).
    pub fn default_config() -> Self {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            ai_backend: AiBackendConfig::default(),
            generation: GenerationConfig::default(),
            d1: D1Config::default(),
            prompts_path: config_dir.join("doccode").join("prompts.yaml"),
            project_path: None,
            docs_path: None,
            theme: ThemeMode::System,
            lang: "ru".to_string(),
            verify_d1: default_verify_d1(),
        }
    }

    /// Полный путь к config.toml.
    pub fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.join("doccode").join("config.toml")
    }

    /// Загрузка config.toml (при отсутствии — значения по умолчанию).
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default_config());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Не удалось прочитать {}: {}", path.display(), e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Некорректный TOML в {}: {}", path.display(), e))
    }

    /// Сохранение config.toml (создаёт родительскую директорию).
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Не удалось создать {}: {}", parent.display(), e))?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, content)
            .map_err(|e| format!("Не удалось записать {}: {}", path.display(), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_serializes() {
        let cfg = AppConfig::default_config();
        let text = toml::to_string(&cfg).unwrap();
        let back: AppConfig = toml::from_str(&text).unwrap();
        assert_eq!(back.ai_backend.model, "gemma4:e4b");
        assert_eq!(back.theme, ThemeMode::System);
        assert_eq!(back.lang, "ru");
        assert_eq!(back.generation.temperature, 0.15);
        assert_eq!(back.generation.max_tokens, 4000);
        assert!(back.generation.think);
        assert_eq!(back.d1.min_factuality, 0.85);
        assert_eq!(back.d1.min_completeness, 3);
        assert_eq!(back.d1.max_retries, 1);
    }

    #[test]
    fn old_config_without_lang_still_loads() {
        // Старые config.toml не содержат новых полей — serde подставит default.
        let old = r#"
prompts_path = "prompts.yaml"
theme = "light"

[ai_backend]
model = "qwen2.5-coder:7b"
"#;
        let cfg: AppConfig = toml::from_str(old).expect("старый конфиг должен загружаться");
        assert_eq!(cfg.lang, "ru");
        assert_eq!(cfg.theme, ThemeMode::Light);
        assert_eq!(cfg.generation.temperature, 0.15);
        assert_eq!(cfg.generation.max_tokens, 4000);
        assert!(cfg.generation.think);
        assert_eq!(cfg.d1.min_factuality, 0.85);
        assert_eq!(cfg.d1.min_completeness, 3);
        assert_eq!(cfg.d1.max_retries, 1);
    }

    #[test]
    fn missing_theme_defaults_to_system() {
        let old = r#"
prompts_path = "prompts.yaml"

[ai_backend]
model = "qwen2.5-coder:7b"
"#;
        let cfg: AppConfig = toml::from_str(old).expect("конфиг без темы должен загружаться");
        assert_eq!(cfg.theme, ThemeMode::System);
    }

    #[test]
    fn theme_mode_roundtrips_as_lowercase() {
        for (mode, text) in [
            (ThemeMode::System, "system"),
            (ThemeMode::Light, "light"),
            (ThemeMode::Dark, "dark"),
        ] {
            let mut cfg = AppConfig::default_config();
            cfg.theme = mode;
            let serialized = toml::to_string(&cfg).unwrap();
            assert!(serialized.contains(&format!("theme = \"{text}\"")));
            let back: AppConfig = toml::from_str(&serialized).unwrap();
            assert_eq!(back.theme, mode);
        }
    }

    #[test]
    fn generation_and_d1_settings_roundtrip() {
        let mut cfg = AppConfig::default_config();
        cfg.generation.temperature = 0.4;
        cfg.generation.max_tokens = 8192;
        cfg.generation.think = false;
        cfg.d1.min_factuality = 0.72;
        cfg.d1.min_completeness = 4;
        cfg.d1.max_retries = 2;
        let text = toml::to_string(&cfg).unwrap();
        let back: AppConfig = toml::from_str(&text).unwrap();
        assert_eq!(back.generation.temperature, 0.4);
        assert_eq!(back.generation.max_tokens, 8192);
        assert!(!back.generation.think);
        assert_eq!(back.d1.min_factuality, 0.72);
        assert_eq!(back.d1.min_completeness, 4);
        assert_eq!(back.d1.max_retries, 2);
    }

    #[test]
    fn d1_settings_are_validated() {
        let mut cfg = D1Config::default();
        assert!(cfg.validate().is_ok());
        cfg.min_factuality = 1.2;
        assert!(cfg.validate().is_err());
        cfg.min_factuality = 0.8;
        cfg.min_completeness = 0;
        assert!(cfg.validate().is_err());
        cfg.min_completeness = 3;
        cfg.max_retries = 6;
        assert!(cfg.validate().is_err());
    }
}
