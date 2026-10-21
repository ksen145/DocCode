//! Локализация интерфейса (русский/английский) на rust-i18n.
//!
//! Переводы лежат в `locales/*.yml` и вшиваются в крейт на этапе компиляции
//! макросом `rust_i18n::i18n!` (см. `src/lib.rs`); новый язык — новый YAML-файл
//! плюс вариант в `Lang`. `Lang::t` — единственная точка доступа, возвращает
//! `Cow<'static, str>` (обычно без аллокации). Неизвестный ключ возвращает сам
//! ключ (fallback — видимая подсказка).

use std::borrow::Cow;

/// Язык интерфейса.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Ru,
    En,
}

impl Lang {
    /// Переключение языка.
    pub fn toggle(self) -> Self {
        match self {
            Lang::Ru => Lang::En,
            Lang::En => Lang::Ru,
        }
    }

    /// Код языка для конфигурации ("ru"/"en").
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::En => "en",
        }
    }

    /// Разбор кода языка (неизвестный код → русский по умолчанию).
    pub fn from_code(code: &str) -> Self {
        match code {
            "en" => Lang::En,
            _ => Lang::Ru,
        }
    }

    /// Перевод ключа на текущий язык.
    pub fn t(self, key: &'static str) -> Cow<'static, str> {
        rust_i18n::t!(key, locale = self.code())
    }
}

/// Все ключи словаря (для тестов полноты).
pub const KEYS: &[&str] = &[
    "app.title",
    "app.tagline",
    "nav.welcome",
    "nav.project",
    "nav.ai",
    "nav.prompts",
    "nav.progress",
    "nav.results",
    "nav.generate",
    "nav.back",
    "nav.section.setup",
    "nav.section.run",
    "topbar.workspace",
    "sidebar.no_project",
    "sidebar.status.ready",
    "sidebar.status.working",
    "sidebar.status.done",
    "common.save",
    "common.saved",
    "common.enabled",
    "common.disabled",
    "common.theme_light",
    "common.theme_dark",
    "common.theme_system",
    "welcome.title",
    "welcome.subtitle",
    "welcome.hint",
    "welcome.eyebrow",
    "welcome.start",
    "welcome.configure",
    "welcome.recommended",
    "welcome.recommended_generation",
    "welcome.recommended_quality",
    "welcome.project",
    "welcome.model",
    "welcome.ai",
    "welcome.quality",
    "welcome.not_set",
    "welcome.how",
    "welcome.step1.title",
    "welcome.step1.body",
    "welcome.step2.title",
    "welcome.step2.body",
    "welcome.step3.title",
    "welcome.step3.body",
    "project.title",
    "project.hint",
    "project.path",
    "project.path_hint",
    "project.docs",
    "project.docs_hint",
    "project.browse",
    "project.choose_path",
    "project.choose_docs",
    "project.paths_card",
    "project.quality_card",
    "project.saved",
    "project.save_error",
    "project.d1_verification",
    "project.d1_verification_hint",
    "project.open_ai",
    "project.edit_d1",
    "project.factuality_short",
    "project.completeness_short",
    "project.retries_short",
    "ai.title",
    "ai.hint",
    "ai.recommended_badge",
    "ai.recommended_models",
    "ai.recommended_hint",
    "ai.connection_card",
    "ai.url",
    "ai.url_hint",
    "ai.models_card",
    "ai.model",
    "ai.model_hint",
    "ai.embedding_model",
    "ai.embedding_model_hint",
    "ai.generation_card",
    "ai.generation_hint",
    "ai.temperature",
    "ai.temperature_hint",
    "ai.max_tokens",
    "ai.max_tokens_hint",
    "ai.d1_card",
    "ai.d1_model",
    "ai.d1_model_hint",
    "ai.d1_min_factuality",
    "ai.d1_min_factuality_hint",
    "ai.d1_min_completeness",
    "ai.d1_min_completeness_hint",
    "ai.d1_max_retries",
    "ai.d1_max_retries_hint",
    "ai.invalid_temperature",
    "ai.temperature_range",
    "ai.invalid_max_tokens",
    "ai.max_tokens_range",
    "ai.invalid_factuality",
    "ai.factuality_range",
    "ai.invalid_completeness",
    "ai.completeness_range",
    "ai.invalid_retries",
    "ai.retries_range",
    "ai.saved",
    "ai.save_error",
    "ai.test",
    "ai.testing",
    "ai.ok",
    "ai.fail",
    "prompts.title",
    "prompts.path",
    "prompts.hint",
    "prompts.missing",
    "prompts.validate",
    "prompts.valid",
    "prompts.invalid",
    "prompts.saved",
    "prompts.save_error",
    "prompts.reset",
    "prompts.parameters_title",
    "prompts.parameters_hint",
    "prompts.open_ai",
    "progress.title",
    "progress.subtitle",
    "progress.current",
    "progress.stage.scan",
    "progress.stage.parse",
    "progress.stage.index",
    "progress.stage.plan",
    "progress.stage.generate",
    "progress.stage.finalize",
    "progress.stage.done",
    "progress.working",
    "progress.hint",
    "results.title",
    "results.subtitle",
    "results.again",
    "results.written",
    "results.failures",
    "results.scanned",
    "results.indexed",
    "results.components",
    "results.verified",
    "results.needs_review",
    "results.unverified",
    "results.open",
    "results.opened",
    "results.open_error",
    "results.files",
    "results.errors",
    "results.empty",
    "results.empty_title",
    "results.success_title",
    "results.success_body",
    "results.error_title",
    "results.error_body",
    "results.d1_card",
    "results.back",
    "error.project_empty",
    "error.project_missing",
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    /// Рекурсивно сворачивает YAML-словарь в плоский список dotted-ключей.
    fn flatten(value: &Value, prefix: &str, out: &mut Vec<String>) {
        match value {
            Value::Mapping(map) => {
                for (k, v) in map {
                    let segment = k.as_str().expect("строковый ключ в YAML переводов");
                    let path = if prefix.is_empty() {
                        segment.to_string()
                    } else {
                        format!("{prefix}.{segment}")
                    };
                    flatten(v, &path, out);
                }
            }
            Value::String(text) => {
                assert!(!text.is_empty(), "пустое значение перевода для {prefix}");
                out.push(prefix.to_string());
            }
            other => panic!("неожиданное значение для {prefix}: {other:?}"),
        }
    }

    #[test]
    fn all_keys_have_translations_in_both_locales() {
        for key in KEYS {
            for lang in [Lang::Ru, Lang::En] {
                let value = lang.t(key);
                assert!(
                    !value.is_empty(),
                    "пустой перевод {} для {key}",
                    lang.code()
                );
                assert_ne!(
                    value.as_ref(),
                    *key,
                    "нет перевода {} для {key} (возвращён сам ключ)",
                    lang.code()
                );
            }
        }
    }

    /// Строгая проверка файлов переводов: каждый ключ KEYS есть именно в ru.yml
    /// и именно в en.yml (fallback на en маскировал бы пропуск в ru.yml).
    #[test]
    fn yaml_locale_files_contain_all_keys() {
        for file in ["locales/ru.yml", "locales/en.yml"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("не удалось прочитать {}: {e}", path.display()));
            let yaml: Value = serde_yaml::from_str(&text)
                .unwrap_or_else(|e| panic!("{file}: некорректный YAML: {e}"));
            let mut keys = Vec::new();
            flatten(&yaml, "", &mut keys);
            for key in KEYS {
                assert!(
                    keys.iter().any(|k| k.as_str() == *key),
                    "в {file} нет ключа {key}"
                );
            }
        }
    }

    #[test]
    fn backend_registers_en_and_ru() {
        assert_eq!(crate::_rust_i18n_available_locales(), vec!["en", "ru"]);
    }

    #[test]
    fn at_least_some_keys_differ_between_languages() {
        let differing = KEYS
            .iter()
            .filter(|k| Lang::Ru.t(k) != Lang::En.t(k))
            .count();
        assert!(
            differing >= KEYS.len() / 2,
            "подозрительно мало различий: {differing}"
        );
    }

    #[test]
    fn unknown_key_falls_back_to_itself() {
        assert_eq!(Lang::Ru.t("no.such.key"), "no.such.key");
        assert_eq!(Lang::En.t("no.such.key"), "no.such.key");
    }

    #[test]
    fn lang_codes_and_toggle() {
        assert_eq!(Lang::Ru.code(), "ru");
        assert_eq!(Lang::En.code(), "en");
        assert_eq!(Lang::Ru.toggle(), Lang::En);
        assert_eq!(Lang::En.toggle(), Lang::Ru);
        assert_eq!(Lang::from_code("en"), Lang::En);
        assert_eq!(Lang::from_code("de"), Lang::Ru);
    }
}
