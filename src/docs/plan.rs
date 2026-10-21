//! Разбор ответа LLM «план проекта» и детерминированный fallback.
//!
//! Модель возвращает JSON с обзором и деревом компонентов. Локальные модели
//! часто оборачивают JSON в Markdown-забор или добавляют текст — парсер
//! извлекает первый сбалансированный объект/массив и валидирует его.

use crate::core::model::ProjectModel;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Ответ модели с планом проекта.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawPlan {
    /// Краткое описание проекта (может быть пустым — обзор генерируется отдельно).
    #[serde(default)]
    pub summary: String,
    /// Точки входа, найденные моделью.
    #[serde(default, alias = "entrypoints")]
    pub entry_points: Vec<RawEntryPoint>,
    /// Компоненты (могут быть вложенными).
    #[serde(default, alias = "modules")]
    pub components: Vec<RawComponent>,
}

/// Точка входа, предложенная моделью.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawEntryPoint {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub description: String,
}

/// Компонент проекта.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawComponent {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub children: Vec<RawComponent>,
}

impl RawComponent {
    /// Все файлы компонента, включая вложенные.
    pub fn all_files(&self) -> Vec<String> {
        let mut files = self.files.clone();
        for child in &self.children {
            files.extend(child.all_files());
        }
        files
    }

    /// Число файлов во всём поддереве.
    pub fn file_count(&self) -> usize {
        self.all_files().len()
    }
}

/// Извлекает первый сбалансированный JSON-объект или массив из текста модели.
///
/// Поддерживает Markdown-заборы, пояснения до/после JSON и строки с экранами.
pub fn extract_json(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let start = bytes.iter().position(|b| *b == b'{' || *b == b'[')?;
    let open = bytes[start];
    let close = if open == b'{' { b'}' } else { b']' };

    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match *byte {
            b'"' => in_string = true,
            b if b == open => depth += 1,
            b if b == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=index]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Разбор плана из ответа LLM.
pub fn parse_plan(text: &str) -> Result<RawPlan, String> {
    let json = extract_json(text)
        .ok_or_else(|| "В ответе LLM не найден JSON-объект или массив".to_string())?;

    if json.trim_start().starts_with('[') {
        let components: Vec<RawComponent> =
            serde_json::from_str(json).map_err(|e| format!("JSON-массив компонентов: {e}"))?;
        return Ok(RawPlan {
            components,
            ..Default::default()
        });
    }

    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("JSON плана: {e}"))?;

    // Некоторые модели кладут полезную нагрузку в `data`/`project`.
    let object = value
        .get("data")
        .or_else(|| value.get("project"))
        .filter(|v| v.is_object())
        .unwrap_or(&value);

    serde_json::from_value::<RawPlan>(object.clone()).map_err(|e| format!("Схема плана: {e}"))
}

/// Детерминированный fallback: группировка по каталогам или языкам.
///
/// Используется, если LLM вернула невалидный JSON: документация всё равно
/// будет построена, но структура будет менее осмысленной.
pub fn fallback_plan(model: &ProjectModel) -> RawPlan {
    let files: Vec<&String> = if model.files.is_empty() {
        let mut keys: Vec<&String> = model.entities.keys().collect();
        keys.sort();
        keys
    } else {
        let mut files: Vec<&String> = model.files.iter().collect();
        files.sort();
        files
    };

    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in &files {
        let key = directory_key(file);
        groups.entry(key).or_default().push((*file).clone());
    }

    let components = groups
        .into_iter()
        .map(|(name, files)| RawComponent {
            description: format!(
                "Файлов: {}. Сущностей: {}.",
                files.len(),
                count_entities(model, &files)
            ),
            name: pretty_name(&name),
            files,
            children: Vec::new(),
        })
        .collect();

    RawPlan {
        summary: format!(
            "{}: {} файлов, {} сущностей.",
            model.name,
            model.files.len().max(model.entities.len()),
            model.all_entities().count()
        ),
        entry_points: Vec::new(),
        components,
    }
}

/// Ключ группировки: первый каталог, а если он один — первые два.
fn directory_key(file: &str) -> String {
    let parts: Vec<&str> = file.split('/').collect();
    match parts.as_slice() {
        [_only] => "Корень проекта".to_string(),
        [first, ..] => {
            if parts.len() > 2 && *first == "src" {
                format!("{}/{}", first, parts[1])
            } else {
                (*first).to_string()
            }
        }
        [] => "Корень проекта".to_string(),
    }
}

fn pretty_name(key: &str) -> String {
    match key {
        "src" => "Исходный код".to_string(),
        "tests" | "test" => "Тесты".to_string(),
        "benches" => "Бенчмарки".to_string(),
        "examples" => "Примеры".to_string(),
        "scripts" => "Скрипты".to_string(),
        "Корень проекта" => "Корень проекта".to_string(),
        other => other.to_string(),
    }
}

fn count_entities(model: &ProjectModel, files: &[String]) -> usize {
    files
        .iter()
        .map(|file| model.entities.get(file).map(Vec::len).unwrap_or(0))
        .sum()
}

/// Нормализует путь из ответа модели в относительный путь проекта.
///
/// Отбрасывает абсолютный префикс, `./`, обратные слэши и кавычки.
pub fn normalize_path(raw: &str, known: &HashMap<String, String>) -> Option<String> {
    let cleaned = raw
        .trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .replace('\\', "/");
    let cleaned = cleaned.trim_start_matches("./");

    if let Some(exact) = known.get(cleaned) {
        return Some(exact.clone());
    }

    // Если модель вернула абсолютный путь — ищем известный суффикс.
    let mut best: Option<(&String, usize)> = None;
    for relative in known.keys() {
        // Сравниваем по границе пути, иначе `notsrc/main.rs` ошибочно
        // совпадёт с `src/main.rs`.
        let matches = cleaned == relative.as_str() || cleaned.ends_with(&format!("/{relative}"));
        if matches
            && best
                .as_ref()
                .map(|(_, len)| relative.len() > *len)
                .unwrap_or(true)
        {
            best = Some((relative, relative.len()));
        }
    }
    best.map(|(relative, _)| relative.clone())
}

/// Строит карту известных путей: нормализованный вариант → канонический.
pub fn known_files(model: &ProjectModel) -> HashMap<String, String> {
    let mut known = HashMap::new();
    for file in &model.files {
        known.insert(file.clone(), file.clone());
        known.insert(
            file.trim_start_matches("./").replace('\\', "/"),
            file.clone(),
        );
    }
    for file in model.entities.keys() {
        known.insert(file.clone(), file.clone());
        known.insert(
            file.trim_start_matches("./").replace('\\', "/"),
            file.clone(),
        );
    }
    known
}

/// Проверяет, что все файлы из плана существуют, и добавляет пропущенные
/// в компонент «Прочие файлы».
pub fn sanitize_plan(plan: &mut RawPlan, model: &ProjectModel) -> Vec<String> {
    let known = known_files(model);
    let mut warnings = Vec::new();
    let mut assigned: BTreeSet<String> = BTreeSet::new();

    fn clean_component(
        component: &mut RawComponent,
        known: &HashMap<String, String>,
        assigned: &mut BTreeSet<String>,
        warnings: &mut Vec<String>,
    ) {
        let mut files = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for raw in component.files.drain(..) {
            match normalize_path(&raw, known) {
                Some(path) => {
                    // Родитель и дочерний компонент могут осознанно делить
                    // файл (обзор поддерева + детализация). Дубликаты
                    // удаляем только внутри одного компонента.
                    if seen.insert(path.clone()) {
                        files.push(path.clone());
                    }
                    assigned.insert(path);
                }
                None => warnings.push(format!(
                    "План содержит неизвестный файл `{}` — пропущен",
                    raw.trim()
                )),
            }
        }
        component.files = files;

        let mut children = Vec::new();
        for mut child in component.children.drain(..) {
            clean_component(&mut child, known, assigned, warnings);
            if !child.files.is_empty() || !child.children.is_empty() {
                if child.name.trim().is_empty() {
                    child.name = "Без названия".to_string();
                }
                children.push(child);
            }
        }
        component.children = children;

        if component.name.trim().is_empty()
            && (!component.files.is_empty() || !component.children.is_empty())
        {
            component.name = "Без названия".to_string();
        }
    }

    let mut components = Vec::new();
    for mut component in plan.components.drain(..) {
        clean_component(&mut component, &known, &mut assigned, &mut warnings);
        if !component.files.is_empty() || !component.children.is_empty() {
            components.push(component);
        }
    }
    plan.components = components;

    // Если модель не дала ни одного валидного компонента, не прячем все
    // файлы в «Прочие файлы»: вызывающий код применит fallback-группировку.
    if plan.components.is_empty() {
        return warnings;
    }

    let mut unassigned: Vec<String> = model
        .files
        .iter()
        .filter(|file| !assigned.contains(*file))
        .cloned()
        .collect();
    unassigned.sort();
    if !unassigned.is_empty() {
        warnings.push(format!(
            "{} файлов не попали в план LLM — добавлен компонент «Прочие файлы»",
            unassigned.len()
        ));
        plan.components.push(RawComponent {
            name: "Прочие файлы".to_string(),
            description:
                "Файлы, которые LLM не отнесла к компонентам. Проверьте структуру вручную."
                    .to_string(),
            files: unassigned,
            children: Vec::new(),
        });
    }

    warnings
}

/// Кандидаты в точки входа по имени файла (без анализа эндпоинтов).
pub fn entry_candidates(model: &ProjectModel) -> Vec<(String, u32)> {
    let mut candidates = Vec::new();
    for file in &model.files {
        let lower = file.to_ascii_lowercase();
        let name = lower.rsplit('/').next().unwrap_or(&lower);
        let score = match name {
            "main.rs" | "main.py" | "main.ts" | "main.js" => 100,
            "app.py" | "server.py" | "server.js" | "server.ts" => 90,
            "index.js" | "index.ts" | "index.tsx" | "index.jsx" => 80,
            "manage.py" | "cli.py" | "cli.rs" => 70,
            _ => 0,
        };
        if score == 0 {
            continue;
        }
        let line = model
            .entities
            .get(file)
            .and_then(|entities| {
                entities
                    .iter()
                    .find(|entity| {
                        entity.name == "main" || entity.name == "run" || entity.name == "start"
                    })
                    .or_else(|| entities.first())
            })
            .map(|entity| entity.line)
            .unwrap_or(1);
        candidates.push((file.clone(), line));
    }

    candidates.sort_by(|a, b| {
        let score = |path: &str| {
            let lower = path.to_ascii_lowercase();
            if lower.starts_with("src/") {
                2
            } else if lower.contains("/bin/") || lower.starts_with("bin/") {
                1
            } else {
                0
            }
        };
        score(&b.0).cmp(&score(&a.0)).then_with(|| a.0.cmp(&b.0))
    });
    candidates.dedup_by(|a, b| a.0 == b.0);
    candidates.truncate(5);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parser::Entity;
    use std::path::PathBuf;

    fn model() -> ProjectModel {
        let mut entities = HashMap::new();
        entities.insert(
            "src/main.rs".to_string(),
            vec![Entity {
                name: "main".to_string(),
                kind: "function".to_string(),
                file_path: PathBuf::from("src/main.rs"),
                line: 7,
                code: Some("fn main() {}".to_string()),
            }],
        );
        ProjectModel {
            name: "demo".to_string(),
            entities,
            files: vec![
                "src/main.rs".to_string(),
                "src/core/mod.rs".to_string(),
                "tests/it.rs".to_string(),
            ],
        }
    }

    #[test]
    fn extracts_json_from_markdown_fence() {
        let text = "Вот план:\n```json\n{\"components\":[{\"name\":\"Ядро\",\"files\":[\"src/main.rs\"]}]}\n```\nГотово";
        let plan = parse_plan(text).unwrap();
        assert_eq!(plan.components.len(), 1);
        assert_eq!(plan.components[0].name, "Ядро");
    }

    #[test]
    fn extracts_array_plan() {
        let plan = parse_plan("[{\"name\":\"A\",\"files\":[\"a.rs\"]}]").unwrap();
        assert_eq!(plan.components[0].name, "A");
    }

    #[test]
    fn braces_inside_strings_do_not_break_extraction() {
        let text = "prefix {\"summary\":\"a } b\",\"components\":[]} suffix";
        let json = extract_json(text).unwrap();
        assert!(json.ends_with('}'));
        assert!(parse_plan(text).is_ok());
    }

    #[test]
    fn normalizes_unix_and_windows_paths() {
        let known = known_files(&model());
        assert_eq!(
            normalize_path(".\\src\\main.rs", &known).as_deref(),
            Some("src/main.rs")
        );
        assert_eq!(
            normalize_path("D:/work/demo/src/core/mod.rs", &known).as_deref(),
            Some("src/core/mod.rs")
        );
        assert_eq!(
            normalize_path("/src/main.rs", &known).as_deref(),
            Some("src/main.rs")
        );
        assert!(normalize_path("nope.rs", &known).is_none());
        assert!(normalize_path("notsrc/main.rs", &known).is_none());
    }

    #[test]
    fn sanitize_adds_unassigned_files_and_drops_unknown() {
        let mut plan = RawPlan {
            components: vec![RawComponent {
                name: "Ядро".to_string(),
                files: vec!["src/main.rs".to_string(), "ghost.rs".to_string()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let warnings = sanitize_plan(&mut plan, &model());
        assert!(warnings.iter().any(|w| w.contains("ghost.rs")));
        assert_eq!(plan.components.len(), 2);
        let other = plan
            .components
            .iter()
            .find(|c| c.name == "Прочие файлы")
            .unwrap();
        assert!(other.files.contains(&"tests/it.rs".to_string()));
        assert!(other.files.contains(&"src/core/mod.rs".to_string()));
    }

    #[test]
    fn sanitize_allows_parent_and_child_to_share_files() {
        let mut plan = RawPlan {
            components: vec![RawComponent {
                name: "Ядро".to_string(),
                files: vec![
                    "src/main.rs".to_string(),
                    "src/core/mod.rs".to_string(),
                    "tests/it.rs".to_string(),
                ],
                children: vec![RawComponent {
                    name: "Точка входа".to_string(),
                    files: vec!["src/main.rs".to_string()],
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let warnings = sanitize_plan(&mut plan, &model());
        assert!(!warnings.iter().any(|w| w.contains("Прочие файлы")));
        assert_eq!(plan.components[0].files.len(), 3);
        assert_eq!(plan.components[0].children[0].files, vec!["src/main.rs"]);
    }

    #[test]
    fn fallback_groups_src_by_second_level() {
        let plan = fallback_plan(&model());
        assert!(plan.components.iter().any(|c| c.name == "src/core"));
        assert!(plan.components.iter().any(|c| c.name == "Тесты"));
    }

    #[test]
    fn entry_candidates_prefer_src_main() {
        let candidates = entry_candidates(&model());
        assert_eq!(candidates[0].0, "src/main.rs");
        assert_eq!(candidates[0].1, 7);
    }
}
