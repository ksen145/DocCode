//! Живой прогон нового пайплайна: анализ → Qdrant → план → компоненты → index.html.
//!
//! Запуск:
//! ```text
//! cargo run --example site_e2e -- <путь_к_проекту> [модель] [папка_docs]
//! DOCCODE_D1=0 — отключить проверку D1
//! ```
//!
//! Пример использует ту же библиотеку, что и GUI, поэтому проверяет реальный
//! путь: эмбеддинги Ollama, Qdrant Edge, промпты, D1 и рендер HTML.

use doccode::config::AiBackendConfig;
use doccode::core::pipeline;
use doccode::docs::site::{self, SiteConfig};
use std::collections::HashSet;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), String> {
    doccode::utils::logging::init_logger();

    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("Укажите путь к проекту: cargo run --example site_e2e -- <root>")?,
    );
    let model = args.next().unwrap_or_else(|| "gemma4:e4b".to_string());
    let docs = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("doccode_site_e2e"));

    let ai = AiBackendConfig {
        base_url: Some("http://localhost:11434".to_string()),
        model,
        embedding_model: "nomic-embed-text".to_string(),
        d1_model: "d1".to_string(),
    };
    let config = SiteConfig {
        project_root: root.clone(),
        docs_dir: docs.clone(),
        prompts_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("prompts.yaml"),
        ai,
        generation: Default::default(),
        d1: Default::default(),
        verify_d1: std::env::var("DOCCODE_D1")
            .map(|value| value != "0")
            .unwrap_or(true),
        language: "ru".to_string(),
    };

    println!("=== DocCode site e2e ===");
    println!("project: {}", root.display());
    println!("docs:    {}", docs.display());
    println!("model:   {}", config.ai.model);
    println!("D1:      {}\n", config.verify_d1);

    let analysis = pipeline::analyze_project(root).await?;
    println!(
        "1. Анализ: {} файлов, {} сущностей",
        analysis.files_scanned,
        analysis.model.all_entities().count()
    );

    let index = site::build_index(&config, &analysis.model).await?;
    println!(
        "2. Индекс: {} чанков, dim={}, backend={}, пропущено={}",
        index.chunks, index.dimension, index.backend, index.skipped
    );

    let mut draft = site::plan_site(&config, &analysis.model, &index).await?;
    println!(
        "3. План: {} компонентов, обзор {} символов, статус {:?}",
        draft.doc.stats.components,
        draft.doc.overview_markdown.len(),
        draft.doc.overview_status
    );

    let all_files: HashSet<String> = draft
        .doc
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect();
    let mut done = 0usize;
    let mut failed = 0usize;
    while !draft.pending.is_empty() {
        let id = draft.pending.remove(0);
        let Some(component) = site::find_component(&draft.doc.components, &id).cloned() else {
            eprintln!("   ! компонент {id} не найден");
            continue;
        };
        print!("   - {} ... ", component.name);
        match site::describe_component(&config, &component, &all_files).await {
            Ok(result) => {
                println!("{:?}", result.status);
                site::apply_component_result(&mut draft, result);
                done += 1;
            }
            Err(error) => {
                println!("ошибка: {error}");
                site::mark_component_failed(&mut draft, &id, &error);
                failed += 1;
            }
        }
    }

    let path = site::finalize_site(&config, &mut draft)?;
    println!(
        "4. Готово: {done} компонентов, {failed} ошибок, HTML: {} ({} КБ)",
        path.display(),
        std::fs::metadata(&path)
            .map(|meta| meta.len() / 1024)
            .unwrap_or(0)
    );
    for warning in &draft.doc.warnings {
        println!("   warning: {warning}");
    }
    Ok(())
}
