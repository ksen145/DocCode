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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;
    use std::collections::BTreeMap;

    /// Рекурсивно сворачивает YAML-словарь в плоскую карту dotted-ключей.
    fn flatten(value: &Value, prefix: &str, out: &mut BTreeMap<String, String>) {
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
                out.insert(prefix.to_string(), text.clone());
            }
            other => panic!("неожиданное значение для {prefix}: {other:?}"),
        }
    }

    fn locale(file: &str) -> BTreeMap<String, String> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("не удалось прочитать {}: {e}", path.display()));
        let yaml: Value = serde_yaml::from_str(&text)
            .unwrap_or_else(|e| panic!("{file}: некорректный YAML: {e}"));
        let mut keys = BTreeMap::new();
        flatten(&yaml, "", &mut keys);
        keys
    }

    #[test]
    fn locale_files_have_matching_keys_and_real_translations() {
        let ru = locale("locales/ru.yml");
        let en = locale("locales/en.yml");
        assert_eq!(
            ru.keys().collect::<Vec<_>>(),
            en.keys().collect::<Vec<_>>(),
            "наборы ключей ru.yml и en.yml разошлись"
        );
        let differing = ru
            .iter()
            .filter(|(key, value)| {
                en.get(*key)
                    .is_some_and(|translation| translation != *value)
            })
            .count();
        assert!(
            differing >= ru.len() / 2,
            "подозрительно мало различий: {differing}"
        );
    }

    #[test]
    fn backend_registers_en_and_ru() {
        assert_eq!(crate::_rust_i18n_available_locales(), vec!["en", "ru"]);
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
