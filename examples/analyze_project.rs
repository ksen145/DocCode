//! Анализ реального проекта без LLM: scan → parse → карта файлов и сущностей.
//!
//! Запуск:
//! ```text
//! cargo run --example analyze_project -- D:/work/my-project
//! ```

use doccode::core::pipeline::analyze_project_sync;
use std::collections::BTreeMap;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());

    let started = Instant::now();
    match analyze_project_sync(std::path::Path::new(&path)) {
        Ok(analysis) => {
            let entities_total = analysis.model.all_entities().count();

            println!("Проект:            {}", analysis.model.name);
            println!("Файлов (языки):    {}", analysis.files_scanned);
            println!("Файлов в карте:    {}", analysis.model.files.len());
            println!("Файлов с сущностями: {}", analysis.model.entities.len());
            println!("Сущностей всего:   {}", entities_total);

            let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
            for entity in analysis.model.all_entities() {
                *kinds.entry(entity.kind.as_str()).or_default() += 1;
            }
            println!("По видам:");
            for (kind, count) in &kinds {
                println!("  {kind:<12} {count}");
            }

            println!("Затрачено:         {} мс", started.elapsed().as_millis());
        }
        Err(e) => {
            eprintln!("Ошибка: {e}");
            std::process::exit(1);
        }
    }
}
