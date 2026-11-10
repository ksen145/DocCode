//! Извлечение сущностей из кода через tree-sitter.

use super::Entity;
use std::path::Path;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};

/// Извлечение сущностей из содержимого файла.
///
/// `language` — имя из [`crate::core::scanner::SUPPORTED_LANGUAGES`];
/// для неподдерживаемого языка возвращается пустой список.
pub fn parse(language: &str, content: &str, file_path: &Path) -> Vec<Entity> {
    use super::queries;

    let (language, query_src) = match language {
        "python" => (tree_sitter_python::LANGUAGE.into(), queries::PYTHON_QUERY),
        "rust" => (tree_sitter_rust::LANGUAGE.into(), queries::RUST_QUERY),
        "javascript" => (
            tree_sitter_javascript::LANGUAGE.into(),
            queries::JAVASCRIPT_QUERY,
        ),
        "typescript" => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            queries::TYPESCRIPT_QUERY,
        ),
        "html" => (tree_sitter_html::LANGUAGE.into(), queries::HTML_QUERY),
        "css" => (tree_sitter_css::LANGUAGE.into(), queries::CSS_QUERY),
        _ => return Vec::new(),
    };

    let mut parser = Parser::new();
    if let Err(e) = parser.set_language(&language) {
        tracing::error!("set_language failed: {}", e);
        return vec![];
    }
    let Some(tree) = parser.parse(content, None) else {
        return vec![];
    };
    let Ok(query) = Query::new(&language, query_src) else {
        tracing::error!("query compile failed for {}", file_path.display());
        return vec![];
    };

    let bytes = content.as_bytes();
    let mut cursor = QueryCursor::new();
    let mut entities = Vec::new();

    let mut matches = cursor.matches(&query, tree.root_node(), |node: tree_sitter::Node<'_>| {
        std::iter::once(&bytes[node.byte_range()])
    });

    while let Some(m) = matches.next() {
        let mut name: Option<&str> = None;
        let mut kind = "entity";
        let mut definition: Option<tree_sitter::Node> = None;

        for capture in m.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            if capture_name == "name" {
                name = capture.node.utf8_text(bytes).ok();
            } else if capture_name.starts_with('_') {
                // Вспомогательные capture'ы (@_attr, @_tag) — только для
                // предикатов #eq?/#match?, kind из них не берём.
            } else {
                kind = capture_name;
                definition = Some(capture.node);
            }
        }

        if let (Some(name), Some(def)) = (name, definition) {
            entities.push(Entity {
                name: name.to_string(),
                kind: kind.to_string(),
                file_path: file_path.to_path_buf(),
                line: def.start_position().row as u32 + 1,
                code: def.utf8_text(bytes).ok().map(str::to_string),
            });
        }
    }

    entities
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_python_entities() {
        let code = "def hello(a, b):\n    return a + b\n\nclass Foo:\n    pass\n";
        let entities = parse("python", code, &PathBuf::from("test.py"));
        let names: Vec<_> = entities
            .iter()
            .map(|e| (e.name.as_str(), e.kind.as_str()))
            .collect();
        assert!(names.contains(&("hello", "function")));
        assert!(names.contains(&("Foo", "class")));
    }

    #[test]
    fn parse_rust_entities() {
        let code = "struct Foo { x: u32 }\nfn bar() {}\nmod inner {}\n";
        let entities = parse("rust", code, &PathBuf::from("test.rs"));
        let names: Vec<_> = entities
            .iter()
            .map(|e| (e.name.as_str(), e.kind.as_str()))
            .collect();
        assert!(names.contains(&("Foo", "struct")));
        assert!(names.contains(&("bar", "function")));
        assert!(names.contains(&("inner", "module")));
    }

    #[test]
    fn parse_js_entities() {
        let code = "function run() {}\nclass App {}\nconst f = () => {};\n";
        let entities = parse("javascript", code, &PathBuf::from("test.js"));
        let names: Vec<_> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"run"));
        assert!(names.contains(&"App"));
        assert!(names.contains(&"f"));
    }

    #[test]
    fn parse_html_entities_ids_and_title() {
        let code = r#"<!DOCTYPE html>
<html>
<head><title>Мой сайт</title></head>
<body>
  <div id="main" class="wrapper">Hello</div>
  <section id=footer></section>
  <p class="no-id-here">text</p>
</body>
</html>
"#;
        let entities = parse("html", code, &PathBuf::from("index.html"));

        let main = entities
            .iter()
            .find(|e| e.name == "main")
            .expect("id=main должен найтись");
        assert_eq!(main.kind, "element");
        assert!(main.code.as_deref().unwrap_or("").contains("wrapper"));

        // Неквотированное значение id=footer тоже распознаётся.
        assert!(entities
            .iter()
            .any(|e| e.name == "footer" && e.kind == "element"));

        // Заголовок страницы.
        let title = entities
            .iter()
            .find(|e| e.name == "Мой сайт")
            .expect("title должен найтись");
        assert_eq!(title.kind, "element");

        // class-атрибут не превращается в сущность.
        assert!(
            !entities
                .iter()
                .any(|e| e.name == "wrapper" || e.name == "no-id-here"),
            "лишние сущности: {:?}",
            entities.iter().map(|e| &e.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn parse_css_entities_classes_ids_keyframes() {
        let code = r#"
.btn { color: red; }
.card .title { font-weight: bold; }
#header { margin: 0; }
@keyframes spin { from { opacity: 0; } to { opacity: 1; } }
"#;
        let entities = parse("css", code, &PathBuf::from("style.css"));

        let btn = entities
            .iter()
            .find(|e| e.name == "btn" && e.kind == "css_class")
            .expect(".btn найдена");
        // code — всё правило, включая тело.
        assert!(btn.code.as_deref().unwrap_or("").contains("color: red"));

        // Вложенный селектор .card .title тоже виден.
        assert!(entities
            .iter()
            .any(|e| e.name == "title" && e.kind == "css_class"));
        assert!(entities
            .iter()
            .any(|e| e.name == "header" && e.kind == "css_id"));
        assert!(entities
            .iter()
            .any(|e| e.name == "spin" && e.kind == "keyframes"));
    }

    #[test]
    fn unknown_language_returns_nothing() {
        assert!(parse("go", "package main", &PathBuf::from("main.go")).is_empty());
    }
}
