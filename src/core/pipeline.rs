//! Пайплайн анализа проекта: сканирование → парсинг → модель.
//!
//! Намеренно **не** ищет API-эндпоинты и не вызывает LLM: продукт является
//! оболочкой над LLM, а вся смысловая работа (выделение компонентов,
//! архитектура, описания) выполняется по векторному контексту в `docs::site`.
//!
//! Здесь собирается только детерминированная «карта» проекта: относительные
//! пути файлов, извлечённые tree-sitter сущности и их исходный код. Эта карта
//! затем чанкуется и индексируется в Qdrant.

use crate::core::model::ProjectModel;
use crate::core::parser::{get_parser_for_path, Entity};
use crate::core::scanner::scan_directory;
use std::collections::{BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

/// Результат анализа проекта.
#[derive(Debug, Clone, Default)]
pub struct Analysis {
    /// Модель проекта (файлы и сущности).
    pub model: ProjectModel,
    /// Сканировано файлов с поддерживаемым языком.
    pub files_scanned: usize,
}

/// Относительный путь с `/` в качестве разделителя (для HTML и payload Qdrant).
pub fn relative_slash_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Синхронный анализ директории проекта (без LLM и без поиска эндпоинтов).
///
/// Файлы, которые не удалось прочитать, пропускаются с предупреждением
/// в лог (graceful degradation), а не прерывают весь анализ.
pub fn analyze_project_sync(root: &Path) -> Result<Analysis, String> {
    if !root.is_dir() {
        return Err(format!("Директория не найдена: {}", root.display()));
    }

    let files = scan_directory(root);
    let files_scanned = files.len();

    let mut entities: HashMap<String, Vec<Entity>> = HashMap::new();
    let mut project_files = BTreeSet::new();

    for file in &files {
        let relative = relative_slash_path(root, &file.path);
        if relative.is_empty() {
            continue;
        }
        project_files.insert(relative.clone());

        let content = match std::fs::read_to_string(&file.path) {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!("Не удалось прочитать {}: {}", file.path.display(), e);
                continue;
            }
        };

        if let Some(parser) = get_parser_for_path(&file.path) {
            let mut parsed = parser.parse(&content, Path::new(&relative));
            for entity in &mut parsed {
                entity.file_path = PathBuf::from(&relative);
            }
            if !parsed.is_empty() {
                entities.insert(relative, parsed);
            }
        }
    }

    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let model = ProjectModel {
        name,
        entities,
        files: project_files.into_iter().collect(),
    };

    Ok(Analysis {
        model,
        files_scanned,
    })
}

/// Асинхронная обёртка для запуска из UI-цикла (`Task::perform`).
pub async fn analyze_project(root: PathBuf) -> Result<Analysis, String> {
    analyze_project_sync(&root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_directory_is_an_error() {
        let err = analyze_project_sync(Path::new("no_such_dir_xyz")).unwrap_err();
        assert!(err.contains("не найдена"));
    }

    #[test]
    fn analyzes_own_source_directory_with_relative_paths() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let analysis = analyze_project_sync(&root).expect("анализ собственного проекта");

        assert!(analysis.files_scanned > 0, "файлы должны найтись");
        assert!(
            analysis.model.entities.keys().any(|k| k.ends_with(".rs")),
            "должны быть сущности из .rs файлов"
        );
        assert!(
            analysis
                .model
                .files
                .iter()
                .all(|path| !path.contains("target")),
            "target/ не должен попадать в карту проекта"
        );
        assert!(
            analysis
                .model
                .files
                .iter()
                .any(|path| path == "src/main.rs"),
            "файлы должны быть относительными: {:?}",
            analysis.model.files.iter().take(5).collect::<Vec<_>>()
        );
    }

    #[test]
    fn broken_files_are_skipped_without_failing_analysis() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("doccode_pipeline_{}_{}", std::process::id(), stamp));
        std::fs::create_dir_all(&dir).unwrap();

        // Невалидный UTF-8: read_to_string упадёт, но анализ должен продолжиться.
        std::fs::write(dir.join("broken.rs"), [0xC3u8, 0x28, 0xFF]).unwrap();
        std::fs::write(dir.join("good.py"), "def ok():\n    return 1\n").unwrap();

        let analysis = analyze_project_sync(&dir).expect("битые файлы не роняют анализ");
        assert_eq!(analysis.files_scanned, 2, "оба файла просканированы");
        assert!(
            analysis
                .model
                .entities
                .values()
                .flatten()
                .any(|e| e.name == "ok"),
            "валидный файл всё равно разобран"
        );
        assert!(
            !analysis
                .model
                .entities
                .keys()
                .any(|k| k.ends_with("broken.rs")),
            "битый файл не попадает в модель"
        );
        assert!(
            analysis.model.files.iter().any(|path| path == "broken.rs"),
            "но файл остаётся в карте проекта"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn relative_paths_use_forward_slashes() {
        let root = Path::new("D:/work/demo");
        let path = Path::new("D:/work/demo/src/core/mod.rs");
        assert_eq!(relative_slash_path(root, path), "src/core/mod.rs");
    }
}
