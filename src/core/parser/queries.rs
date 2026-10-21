//! S-expression queries для tree-sitter: выборка узлов-сущностей.
//!
//! Каждая query содержит capture `@name` (имя сущности) и capture с kind
//! (`@function`, `@class`, ...), по которому определяется тип сущности.

/// Python: функции, асинхронные функции, классы.
pub const PYTHON_QUERY: &str = r#"
(function_definition name: (identifier) @name) @function
(class_definition name: (identifier) @name) @class
"#;

/// Rust: функции, структуры, перечисления, трейты, модули, константы, impl-блоки.
pub const RUST_QUERY: &str = r#"
(function_item name: (identifier) @name) @function
(struct_item name: (type_identifier) @name) @struct
(enum_item name: (type_identifier) @name) @enum
(trait_item name: (type_identifier) @name) @trait
(const_item name: (identifier) @name) @const
(type_item name: (type_identifier) @name) @type
(mod_item name: (identifier) @name) @module
(impl_item type: (type_identifier) @name) @impl
"#;

/// JavaScript: функции, классы, методы, стрелочные функции в переменных.
pub const JAVASCRIPT_QUERY: &str = r#"
(function_declaration name: (identifier) @name) @function
(generator_function_declaration name: (identifier) @name) @function
(class_declaration name: (identifier) @name) @class
(method_definition name: (_) @name) @method
(variable_declarator name: (identifier) @name value: (arrow_function) @function)
"#;

/// TypeScript: то же + интерфейсы.
pub const TYPESCRIPT_QUERY: &str = r#"
(function_declaration name: (identifier) @name) @function
(generator_function_declaration name: (identifier) @name) @function
(class_declaration name: (identifier) @name) @class
(interface_declaration name: (type_identifier) @name) @interface
(method_definition name: (_) @name) @method
(variable_declarator name: (identifier) @name value: (arrow_function) @function)
"#;

/// HTML: id-атрибуты элементов (якоря/идентификаторы) и заголовок страницы.
///
/// `@_attr`/`@_tag` — вспомогательные capture'ы для предикатов (kind берётся
/// из `@element`, имя — из `@name`).
pub const HTML_QUERY: &str = r#"
(element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @name)))
  (#eq? @_attr "id")
) @element

(element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (attribute_value) @name))
  (#eq? @_attr "id")
) @element

(element
  (start_tag (tag_name) @_tag)
  (text) @name
  (#eq? @_tag "title")
) @element
"#;

/// CSS: классы, id-селекторы и именованные @keyframes.
///
/// Kind-захват лежит на `rule_set`, чтобы в `code` попало тело правила,
/// а не только селектор.
pub const CSS_QUERY: &str = r#"
(rule_set (selectors (class_selector (class_name) @name))) @css_class
(rule_set (selectors (id_selector (id_name) @name))) @css_id
(rule_set (selectors (descendant_selector (class_selector (class_name) @name)))) @css_class
(rule_set (selectors (child_selector (class_selector (class_name) @name)))) @css_class
(rule_set (selectors (sibling_selector (class_selector (class_name) @name)))) @css_class
(rule_set (selectors (adjacent_sibling_selector (class_selector (class_name) @name)))) @css_class
(keyframes_statement (keyframes_name) @name) @keyframes
"#;
