//! Реализации парсеров для языков через tree-sitter.

use super::{Entity, LanguageParser};
use std::path::Path;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};

/// Парсер поверх tree-sitter: грамматика + query выборки сущностей.
pub struct TsParser {
    language: tree_sitter::Language,
    query_src: &'static str,
}

impl TsParser {
    pub fn new(language: tree_sitter::Language, query_src: &'static str) -> Self {
        Self {
            language,
            query_src,
        }
    }

    /// Извлечение сущностей по query: capture `@name` + capture-kind.
    fn extract(&self, content: &str, file_path: &Path) -> Vec<Entity> {
        let mut parser = Parser::new();
        if let Err(e) = parser.set_language(&self.language) {
            tracing::error!("set_language failed: {}", e);
            return vec![];
        }
        let Some(tree) = parser.parse(content, None) else {
            return vec![];
        };
        let Ok(query) = Query::new(&self.language, self.query_src) else {
            tracing::error!("query compile failed for {}", file_path.display());
            return vec![];
        };

        let bytes = content.as_bytes();
        let mut cursor = QueryCursor::new();
        let mut entities = Vec::new();

        let mut matches =
            cursor.matches(&query, tree.root_node(), |node: tree_sitter::Node<'_>| {
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
}

impl LanguageParser for TsParser {
    fn parse(&self, content: &str, file_path: &Path) -> Vec<Entity> {
        self.extract(content, file_path)
    }
}

/// Выбор парсера по имени языка (см. scanner::SUPPORTED_LANGUAGES).
pub fn by_language(language: &str) -> Option<Box<dyn LanguageParser>> {
    use super::queries;

    let parser = match language {
        "python" => TsParser::new(tree_sitter_python::LANGUAGE.into(), queries::PYTHON_QUERY),
        "rust" => TsParser::new(tree_sitter_rust::LANGUAGE.into(), queries::RUST_QUERY),
        "javascript" => TsParser::new(
            tree_sitter_javascript::LANGUAGE.into(),
            queries::JAVASCRIPT_QUERY,
        ),
        "typescript" => TsParser::new(
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            queries::TYPESCRIPT_QUERY,
        ),
        "html" => TsParser::new(tree_sitter_html::LANGUAGE.into(), queries::HTML_QUERY),
        "css" => TsParser::new(tree_sitter_css::LANGUAGE.into(), queries::CSS_QUERY),
        _ => return None,
    };
    Some(Box::new(parser))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_python_entities() {
        let parser = by_language("python").unwrap();
        let code = "def hello(a, b):\n    return a + b\n\nclass Foo:\n    pass\n";
        let entities = parser.parse(code, &PathBuf::from("test.py"));
        let names: Vec<_> = entities
            .iter()
            .map(|e| (e.name.as_str(), e.kind.as_str()))
            .collect();
        assert!(names.contains(&("hello", "function")));
        assert!(names.contains(&("Foo", "class")));
    }

    #[test]
    fn parse_rust_entities() {
        let parser = by_language("rust").unwrap();
        let code = "struct Foo { x: u32 }\nfn bar() {}\nmod inner {}\n";
        let entities = parser.parse(code, &PathBuf::from("test.rs"));
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
        let parser = by_language("javascript").unwrap();
        let code = "function run() {}\nclass App {}\nconst f = () => {};\n";
        let entities = parser.parse(code, &PathBuf::from("test.js"));
        let names: Vec<_> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"run"));
        assert!(names.contains(&"App"));
        assert!(names.contains(&"f"));
    }

    #[test]
    fn parse_html_entities_ids_and_title() {
        let parser = by_language("html").unwrap();
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
        let entities = parser.parse(code, &PathBuf::from("index.html"));

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
        let parser = by_language("css").unwrap();
        let code = r#"
.btn { color: red; }
.card .title { font-weight: bold; }
#header { margin: 0; }
@keyframes spin { from { opacity: 0; } to { opacity: 1; } }
"#;
        let entities = parser.parse(code, &PathBuf::from("style.css"));

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
    fn html_and_css_factories_are_wired() {
        assert!(by_language("html").is_some());
        assert!(by_language("css").is_some());
        assert!(by_language("go").is_none());
    }
}
