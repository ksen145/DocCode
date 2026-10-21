//! Сканер файлов проекта: обход дерева и определение языка.

use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// Поддерживаемые расширения файлов.
pub const SUPPORTED_LANGUAGES: &[(&str, &str)] = &[
    ("py", "python"),
    ("rs", "rust"),
    ("js", "javascript"),
    ("jsx", "javascript"),
    ("ts", "typescript"),
    ("tsx", "typescript"),
    ("html", "html"),
    ("css", "css"),
];

/// Директории, которые никогда не сканируем.
const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    "docs",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    "venv",
    ".idea",
    ".vscode",
];

/// Результат сканирования одного файла.
#[derive(Debug, Clone)]
pub struct FileScanResult {
    /// Путь к файлу.
    pub path: PathBuf,
    /// Язык (по расширению), None если не поддерживается.
    pub language: Option<&'static str>,
    /// Размер файла в байтах.
    pub size: u64,
}

/// Определяем, пропускать ли директорию.
fn should_skip(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    name.starts_with('.') || SKIP_DIRS.contains(&name.as_ref())
}

/// Сгенерированный DocCode сайт: не индексируем его при повторном запуске,
/// даже если папка документации лежит внутри проекта и не называется `docs`.
fn is_generated_site(entry: &DirEntry) -> bool {
    if entry.file_name().to_string_lossy() != "index.html" {
        return false;
    }
    std::fs::read_to_string(entry.path())
        .map(|text| text.contains("<meta name=\"generator\" content=\"DocCode\">"))
        .unwrap_or(false)
}

/// Определение языка по расширению файла.
pub fn language_for_path(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    SUPPORTED_LANGUAGES
        .iter()
        .find(|(e, _)| *e == ext)
        .map(|(_, lang)| *lang)
}

/// Рекурсивный обход проекта с фильтрацией по расширению.
pub fn scan_directory(root: impl AsRef<Path>) -> Vec<FileScanResult> {
    let root = root.as_ref();
    let mut results = Vec::new();

    let walker = WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !should_skip(e));

    for entry in walker.flatten() {
        if !entry.file_type().is_file() || is_generated_site(&entry) {
            continue;
        }
        if let Some(language) = language_for_path(entry.path()) {
            results.push(FileScanResult {
                path: entry.path().to_path_buf(),
                language: Some(language),
                size: entry.metadata().map(|m| m.len()).unwrap_or(0),
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_detection() {
        assert_eq!(language_for_path(Path::new("a/b.py")), Some("python"));
        assert_eq!(language_for_path(Path::new("a/b.rs")), Some("rust"));
        assert_eq!(language_for_path(Path::new("a/b.tsx")), Some("typescript"));
        assert_eq!(language_for_path(Path::new("a/b.md")), None);
    }

    #[test]
    fn generated_site_index_is_skipped() {
        let dir =
            std::env::temp_dir().join(format!("doccode_scanner_generated_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("site")).unwrap();
        std::fs::write(
            dir.join("site/index.html"),
            "<meta name=\"generator\" content=\"DocCode\">",
        )
        .unwrap();
        std::fs::write(dir.join("site/page.html"), "<html><body>page</body></html>").unwrap();
        std::fs::write(dir.join("site/main.rs"), "fn main() {}").unwrap();

        let files = scan_directory(&dir);
        let names: Vec<String> = files
            .iter()
            .map(|file| file.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(!names.contains(&"index.html".to_string()));
        assert!(names.contains(&"page.html".to_string()));
        assert!(names.contains(&"main.rs".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
