//! Сквозной тест: анализ мини-проекта (Python + JS + Rust) одним пайплайном.

use doccode::core::pipeline::analyze_project_sync;

/// Уникальный временный каталог для фикстуры (pid + метка времени).
fn fixture_root() -> std::path::PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("doccode_fixture_{}_{}", std::process::id(), stamp))
}

#[test]
fn analyzes_python_js_rust_project_end_to_end() {
    let root = fixture_root();
    std::fs::create_dir_all(root.join("src")).expect("создать src/");
    std::fs::create_dir_all(root.join("web")).expect("создать web/");

    std::fs::write(
        root.join("app.py"),
        r#"
from flask import Flask
app = Flask(__name__)

@app.route('/hello', methods=['GET'])
def hello():
    return "hi"

class Greeter:
    pass
"#,
    )
    .expect("записать app.py");

    std::fs::write(
        root.join("web/server.js"),
        r#"
const app = require('express')();
app.get('/items', (req, res) => res.json([]));
function deleteItem(id) {}
"#,
    )
    .expect("записать server.js");

    std::fs::write(
        root.join("src/lib.rs"),
        r#"
pub struct Config {
    pub x: u8,
}

pub fn run() {}

#[get("/health")]
async fn health() {}
"#,
    )
    .expect("записать lib.rs");

    let analysis = analyze_project_sync(&root).expect("мини-проект должен анализироваться");

    // Все три файла с поддерживаемыми языками просканированы.
    assert_eq!(analysis.files_scanned, 3);

    let names: Vec<&str> = analysis
        .model
        .entities
        .values()
        .flatten()
        .map(|e| e.name.as_str())
        .collect();

    // Python
    assert!(names.contains(&"hello"), "python: функция hello");
    assert!(names.contains(&"Greeter"), "python: класс Greeter");
    // JavaScript
    assert!(names.contains(&"deleteItem"), "js: функция deleteItem");
    // Rust
    assert!(names.contains(&"Config"), "rust: структура Config");
    assert!(names.contains(&"run"), "rust: функция run");

    // Все файлы попадают в карту проекта относительными путями.
    assert!(analysis.model.files.contains(&"app.py".to_string()));
    assert!(analysis.model.files.contains(&"web/server.js".to_string()));
    assert!(analysis.model.files.contains(&"src/lib.rs".to_string()));

    std::fs::remove_dir_all(&root).ok();
}
