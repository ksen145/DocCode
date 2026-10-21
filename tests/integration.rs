//! Интеграционные тесты: сканер, парсеры и промпты.

use doccode::core::parser::{get_parser, get_parser_for_path};
use doccode::core::scanner::{language_for_path, scan_directory};
use doccode::prompts::manager::PromptsManager;
use doccode::prompts::template::substitute;
use std::path::PathBuf;

#[test]
fn scans_own_source_directory() {
    let root = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let files = scan_directory(&root);

    // Должны найти хотя бы src/main.rs и не попадать в target/
    assert!(!files.is_empty(), "сканер не нашёл файлов");
    assert!(
        files
            .iter()
            .all(|f| !f.path.to_string_lossy().contains("target")),
        "target/ не должен сканироваться"
    );
    assert!(files.iter().any(|f| f.path.ends_with("main.rs")));
}

#[test]
fn language_detection_works() {
    assert_eq!(language_for_path(&PathBuf::from("lib.py")), Some("python"));
    assert_eq!(language_for_path(&PathBuf::from("lib.rs")), Some("rust"));
    assert_eq!(language_for_path(&PathBuf::from("README.md")), None);
}

#[test]
fn parser_factory_works_by_path_and_language() {
    assert!(get_parser_for_path(&PathBuf::from("src/a.rs")).is_some());
    assert!(get_parser_for_path(&PathBuf::from("src/a.py")).is_some());
    assert!(get_parser_for_path(&PathBuf::from("site/index.html")).is_some());
    assert!(get_parser_for_path(&PathBuf::from("assets/style.css")).is_some());
    assert!(get_parser_for_path(&PathBuf::from("src/a.md")).is_none());
    assert!(get_parser("go").is_none());
}

#[test]
fn parses_html_and_css_entities_from_paths() {
    let html = get_parser_for_path(&PathBuf::from("public/index.html")).unwrap();
    let entities = html.parse(
        "<html><head><title>Docs</title></head><body><div id=\"app\"></div></body></html>",
        &PathBuf::from("public/index.html"),
    );
    assert!(entities
        .iter()
        .any(|e| e.name == "app" && e.kind == "element"));
    assert!(entities
        .iter()
        .any(|e| e.name == "Docs" && e.kind == "element"));

    let css = get_parser_for_path(&PathBuf::from("public/app.css")).unwrap();
    let entities = css.parse(
        ".layout { display: flex; }\n#root { color: red; }",
        &PathBuf::from("public/app.css"),
    );
    assert!(entities
        .iter()
        .any(|e| e.name == "layout" && e.kind == "css_class"));
    assert!(entities
        .iter()
        .any(|e| e.name == "root" && e.kind == "css_id"));
}

#[test]
fn parses_python_entities_with_code() {
    let parser = get_parser("python").unwrap();
    let code = "def process(items):\n    return len(items)\n\nclass Store:\n    pass\n";
    let entities = parser.parse(code, &PathBuf::from("app.py"));

    let func = entities
        .iter()
        .find(|e| e.name == "process")
        .expect("function");
    assert_eq!(func.kind, "function");
    assert_eq!(func.line, 1);
    assert!(func
        .code
        .as_deref()
        .unwrap_or("")
        .contains("return len(items)"));

    let class = entities.iter().find(|e| e.name == "Store").expect("class");
    assert_eq!(class.kind, "class");
    assert_eq!(class.line, 4);
}

#[test]
fn parses_rust_entities() {
    let parser = get_parser("rust").unwrap();
    let code = "pub struct Config {\n    pub x: u8,\n}\n\npub fn run() {}\n";
    let entities = parser.parse(code, &PathBuf::from("lib.rs"));

    assert!(entities
        .iter()
        .any(|e| e.name == "Config" && e.kind == "struct"));
    assert!(entities
        .iter()
        .any(|e| e.name == "run" && e.kind == "function"));
}

#[test]
fn prompts_yaml_matches_schema() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manager = PromptsManager::new(PathBuf::from(manifest_dir).join("prompts.yaml"));

    let config = manager
        .load()
        .expect("prompts.yaml должен соответствовать схеме");
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

    // require_prompt с неизвестным именем даёт ошибку с перечнем имён
    let err = manager.require_prompt("nonexistent").unwrap_err();
    assert!(err.contains("project_plan"));
}

#[test]
fn prompt_substitution_replaces_all_vars() {
    let template = "kind={{kind}} name={{name}} line={{line}} again={{name}}";
    let result = substitute(
        template,
        &[("kind", "function"), ("name", "foo"), ("line", "10")],
    );
    assert_eq!(result, "kind=function name=foo line=10 again=foo");
}
