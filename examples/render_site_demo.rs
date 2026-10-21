//! Демонстрационный рендер сайта без LLM: собирает `SiteDoc` вручную и пишет
//! `index.html`. Полезен для проверки HTML/JS и вёрстки без обращения к Ollama.
//!
//! ```text
//! cargo run --example render_site_demo -- [папка]
//! ```

use doccode::docs::site::{
    ComponentDoc, DocStatus, EntityDoc, EntryPointDoc, FileDoc, SiteDoc, SiteStats, Verification,
};
use doccode::docs::site_html::{markdown_to_html, render_site};
use std::path::{Path, PathBuf};

fn component(
    id: &str,
    name: &str,
    description: &str,
    files: &[&str],
    status: DocStatus,
    verification: Option<Verification>,
    children: Vec<ComponentDoc>,
) -> ComponentDoc {
    let markdown = format!(
        "# {name}\n\n## Назначение\n\n{description}\n\n## Ключевые файлы и сущности\n\n\
         | Файл | Сущность | Строка | Роль |\n| :--- | :--- | :--- | :--- |\n\
         | `{}` | `main` | `{}:1` | точка входа |\n\n\
         ## Как это работает\n\nКомпонент читает конфигурацию и запускает сервер.\n\n\
         ## Связи с другими частями\n\nВызывает хранилище и API.\n\n\
         ## На что обратить внимание\n\nОбрабатывайте ошибки открытия файла.\n",
        files.first().copied().unwrap_or("src/main.rs"),
        files.first().copied().unwrap_or("src/main.rs")
    );
    ComponentDoc {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        html: markdown_to_html(&markdown),
        markdown,
        files: files.iter().map(|file| (*file).to_string()).collect(),
        children,
        status,
        verification,
    }
}

fn file(path: &str, language: &str, component: &str, entities: Vec<EntityDoc>) -> FileDoc {
    FileDoc {
        path: path.to_string(),
        language: language.to_string(),
        component: Some(component.to_string()),
        entities,
    }
}

fn main() -> Result<(), String> {
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("doccode_render_demo"));
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;

    let verified = Verification {
        factual: Some(0.93),
        completeness: Some(4),
        accepted: true,
        note: "D1: точность 0.93, полнота 4/5 — принято".to_string(),
    };
    let review = Verification {
        factual: Some(0.78),
        completeness: Some(4),
        accepted: false,
        note: "D1: точность 0.78, полнота 4/5 — требует проверки".to_string(),
    };

    let api = component(
        "c3",
        "HTTP API",
        "Принимает запросы и вызывает сервисный слой.",
        &["src/api.rs"],
        DocStatus::NeedsReview,
        Some(review),
        Vec::new(),
    );
    let core = component(
        "c2",
        "Ядро",
        "Конфигурация, хранилище и бизнес-логика.",
        &["src/config.rs", "src/store.rs"],
        DocStatus::Verified,
        Some(verified.clone()),
        vec![api],
    );
    let entry = component(
        "c1",
        "Точка входа",
        "Запускает приложение и связывает компоненты.",
        &["src/main.rs"],
        DocStatus::Verified,
        Some(verified.clone()),
        vec![core],
    );

    let markdown = "# demo\n\n## Назначение\n\nУчебный проект с тремя слоями.\n\n\
                    ## Как запускается\n\n`cargo run` (`src/main.rs:4`).\n\n\
                    ## Архитектура\n\nТочка входа создаёт конфигурацию, хранилище и сервер.\n\n\
                    ## Основные компоненты\n\n- **Точка входа** — запуск.\n- **Ядро** — логика.\n\n\
                    ## Поток данных\n\nmain → Config → Store → API.\n\n\
                    ## С чего начать чтение кода\n\n1. `src/main.rs:4`\n2. `src/config.rs:8`\n";
    let mut stats = SiteStats {
        files: 4,
        entities: 7,
        components: 3,
        chunks: 11,
        verified: 2,
        needs_review: 1,
        index_backend: "qdrant-edge".to_string(),
        ..Default::default()
    };
    stats.unverified = 0;
    stats.failed = 0;

    let files = vec![
        file(
            "src/main.rs",
            "rust",
            "c1",
            vec![EntityDoc {
                name: "main".to_string(),
                kind: "function".to_string(),
                line: 4,
                code: Some("fn main() {\n    let config = Config::from_env()?;\n    let store = Store::open(&config.db)?;\n    Server::new(config, store).run();\n}".to_string()),
            }],
        ),
        file(
            "src/config.rs",
            "rust",
            "c2",
            vec![EntityDoc {
                name: "Config".to_string(),
                kind: "struct".to_string(),
                line: 8,
                code: Some("pub struct Config {\n    pub db: PathBuf,\n    pub listen: String,\n}".to_string()),
            }],
        ),
        file(
            "src/store.rs",
            "rust",
            "c2",
            vec![EntityDoc {
                name: "Store".to_string(),
                kind: "struct".to_string(),
                line: 3,
                code: Some("pub struct Store {\n    records: HashMap<String, String>,\n}".to_string()),
            }],
        ),
        file(
            "src/api.rs",
            "rust",
            "c3",
            vec![EntityDoc {
                name: "Server".to_string(),
                kind: "struct".to_string(),
                line: 5,
                code: Some("pub struct Server {\n    config: Config,\n}".to_string()),
            }],
        ),
    ];

    let doc = SiteDoc {
        project: "demo".to_string(),
        generated_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs().to_string())
            .unwrap_or_default(),
        root: std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        stats,
        d1_min_factuality: 0.85,
        d1_min_completeness: 3,
        overview_markdown: markdown.to_string(),
        overview_html: markdown_to_html(markdown),
        overview_status: DocStatus::Verified,
        overview_verification: Some(verified),
        entry_points: vec![EntryPointDoc {
            path: "src/main.rs".to_string(),
            line: 4,
            description: "main запускает приложение".to_string(),
        }],
        components: vec![entry],
        files,
        warnings: vec!["Демонстрационный рендер без LLM.".to_string()],
    };

    let path = out.join("index.html");
    std::fs::write(&path, render_site(&doc)).map_err(|e| e.to_string())?;
    print_path(&path);
    Ok(())
}

fn print_path(path: &Path) {
    println!("HTML: {}", path.display());
}
