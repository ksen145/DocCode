//! Схема и обработка промптов (YAML-конфигурация, шаблоны).

pub mod manager;
pub mod schema;
pub mod template;

pub use schema::{Prompt, PromptsConfig};

use std::path::{Path, PathBuf};

/// Встроенный вариант prompts.yaml — гарантия наличия промптов даже без файла на диске.
pub const DEFAULT_PROMPTS_YAML: &str = include_str!("../../prompts.yaml");

/// Загрузка текста промптов с fallback-цепочкой.
///
/// Порядок поиска: указанный путь → `./prompts.yaml` → `prompts.yaml` рядом
/// с бинарником → встроенный вариант.
///
/// Возвращает `(реально прочитанный путь, текст)`; `None` в первом элементе
/// означает, что текст взят из встроенного варианта (файла на диске нет).
pub fn load_with_fallback(path: &Path) -> (Option<PathBuf>, String) {
    if let Ok(text) = std::fs::read_to_string(path) {
        return (Some(path.to_path_buf()), text);
    }

    let mut candidates = vec![PathBuf::from("prompts.yaml")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("prompts.yaml"));
        }
    }
    for candidate in candidates {
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            return (Some(candidate), text);
        }
    }

    (None, DEFAULT_PROMPTS_YAML.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_defaults_match_schema() {
        let config: PromptsConfig =
            serde_yaml::from_str(DEFAULT_PROMPTS_YAML).expect("встроенный YAML должен парситься");
        assert_eq!(config.version, 1);
        for name in [
            "project_plan",
            "project_overview",
            "component_doc",
            "d1_factuality",
            "d1_completeness",
        ] {
            assert!(config.prompts.contains_key(name), "нет промпта {name}");
        }
    }

    #[test]
    fn fallback_always_provides_valid_prompts() {
        let (source, text) = load_with_fallback(Path::new("no/such/prompts.yaml"));
        let config: PromptsConfig =
            serde_yaml::from_str(&text).expect("fallback-текст должен соответствовать схеме");
        assert_eq!(config.version, 1);
        // Либо нашли файл на диске, либо отдали встроенный вариант.
        if source.is_none() {
            assert_eq!(text, DEFAULT_PROMPTS_YAML);
        }
    }
}
