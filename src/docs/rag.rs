//! Векторный контекст проекта: чанки кода → эмбеддинги → Qdrant → промпт.
//!
//! LLM никогда не получает файлы целиком: только фрагменты, найденные
//! семантическим поиском в Qdrant. Это одновременно уменьшает промпт и
//! заставляет модель опираться на действительно релевантный код.

use crate::ai::client::AiClient;
use crate::core::model::ProjectModel;
use crate::index::vector_store::VectorPoint;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Верхняя граница числа чанков, чтобы индексация большого проекта не шла вечно.
const MAX_CHUNKS: usize = 3000;
/// Максимальная длина кода одного чанка (голова + хвост).
const MAX_CODE_CHARS: usize = 6000;
/// Строк кода в файловом чанке, если из файла не извлечено сущностей.
const FILE_CHUNK_LINES: usize = 80;

/// Данные чанка, которые сохраняются в payload Qdrant и возвращаются при поиске.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPayload {
    /// Относительный путь к файлу.
    pub path: String,
    /// Начальная строка (1-based).
    pub line: u32,
    /// Вид: function/struct/class/file...
    pub kind: String,
    /// Имя сущности или имя файла.
    pub name: String,
    /// Исходный код фрагмента.
    pub code: String,
}

/// Чанк до индексации.
#[derive(Debug, Clone)]
struct CodeChunk {
    id: String,
    payload: ChunkPayload,
    embedding_text: String,
}

/// Метаданные индекса на диске.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMeta {
    /// Размерность векторов.
    pub dimension: usize,
    /// Количество проиндексированных чанков.
    pub chunks: usize,
    /// Модель эмбеддингов (для диагностики).
    pub embedding_model: String,
}

/// Результат индексации.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Размерность векторов.
    pub dimension: usize,
    /// Проиндексировано чанков.
    pub chunks: usize,
    /// Пропущено чанков (ошибка эмбеддинга).
    pub skipped: usize,
    /// Файлов в проекте.
    pub files: usize,
    /// Каталог индекса.
    pub index_dir: String,
    /// Бэкенд индекса.
    pub backend: String,
}

/// Найденный векторный фрагмент.
#[derive(Debug, Clone)]
pub struct RetrievedChunk {
    pub path: String,
    pub line: u32,
    pub kind: String,
    pub name: String,
    pub code: String,
    /// Косинусная близость (больше — ближе).
    pub score: f32,
}

impl RetrievedChunk {
    /// Ссылка для разработчика: `path:line`.
    pub fn location(&self) -> String {
        format!("{}:{}", self.path, self.line)
    }
}

/// Каталог индекса проекта в кэше приложения.
pub fn default_index_dir(root: &Path) -> PathBuf {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let id = blake3::hash(canonical.to_string_lossy().as_bytes())
        .to_hex()
        .to_string();
    let base = dirs::cache_dir().unwrap_or_else(|| canonical.join(".doccode-cache"));
    base.join("doccode")
        .join("projects")
        .join(&id[..16])
        .join("index")
}

fn meta_path(dir: &Path) -> PathBuf {
    dir.join("meta.json")
}

/// Загрузка метаданных индекса.
pub fn load_meta(dir: &Path) -> Result<IndexMeta, String> {
    let path = meta_path(dir);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "Векторный индекс не найден ({}): {}. Запустите индексацию проекта.",
            path.display(),
            e
        )
    })?;
    serde_json::from_str(&text).map_err(|e| format!("Некорректный meta.json индекса: {e}"))
}

/// Построение индекса: чанки → эмбеддинги → Qdrant (или in-memory fallback).
pub async fn build_index(
    model: &ProjectModel,
    root: &Path,
    embedder: &dyn AiClient,
    embedding_model: &str,
) -> Result<IndexStats, String> {
    let chunks = make_chunks(model, root);
    if chunks.is_empty() {
        return Err("Не найдено ни одного фрагмента кода для индексации".to_string());
    }

    let mut points = Vec::new();
    let mut skipped = 0usize;
    let mut dimension = 0usize;
    let mut first_error: Option<String> = None;

    for chunk in chunks.iter().take(MAX_CHUNKS) {
        match embedder.embed(&chunk.embedding_text).await {
            Ok(vector) if !vector.is_empty() => {
                if dimension == 0 {
                    dimension = vector.len();
                }
                if vector.len() != dimension {
                    skipped += 1;
                    first_error.get_or_insert_with(|| {
                        format!("размерность эмбеддинга {} != {}", vector.len(), dimension)
                    });
                    continue;
                }
                points.push(VectorPoint {
                    id: chunk.id.clone(),
                    vector,
                    payload: serde_json::to_string(&chunk.payload)
                        .unwrap_or_else(|_| "{}".to_string()),
                });
            }
            Ok(_) => {
                skipped += 1;
                first_error.get_or_insert_with(|| "пустой эмбеддинг".to_string());
            }
            Err(e) => {
                skipped += 1;
                first_error.get_or_insert(e);
            }
        }
    }

    if points.is_empty() {
        return Err(format!(
            "Не удалось получить ни одного эмбеддинга (модель `{}`): {}",
            embedding_model,
            first_error.unwrap_or_else(|| "нет ответа от Ollama".to_string())
        ));
    }

    let dir = default_index_dir(root);
    reset_index(&dir)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Не удалось создать {}: {e}", dir.display()))?;

    let meta = IndexMeta {
        dimension,
        chunks: points.len(),
        embedding_model: embedding_model.to_string(),
    };

    // Сначала записываем векторы, затем meta: при сбое не остаётся индекс,
    // который выглядит готовым, но не содержит данных.
    add_points(&dir, dimension, points)?;
    std::fs::write(
        meta_path(&dir),
        serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Не удалось записать meta.json: {e}"))?;

    Ok(IndexStats {
        dimension,
        chunks: meta.chunks,
        skipped,
        files: model.files.len(),
        index_dir: dir.display().to_string(),
        backend: backend_name().to_string(),
    })
}

/// Поиск по индексу пачкой запросов.
///
/// `allowed_files` ограничивает выдачу файлами компонента (для документации
/// конкретного модуля), `limit` — общее число фрагментов после дедупликации.
pub async fn retrieve(
    embedder: &dyn AiClient,
    root: &Path,
    queries: &[String],
    per_query: usize,
    limit: usize,
    allowed_files: Option<&HashSet<String>>,
) -> Result<Vec<RetrievedChunk>, String> {
    let dir = default_index_dir(root);
    let meta = load_meta(&dir)?;
    let searcher = Searcher::open(&dir, meta.dimension)?;

    let mut best: HashMap<String, RetrievedChunk> = HashMap::new();
    let mut failures = Vec::new();

    for query in queries {
        let vector = match embedder.embed(query).await {
            Ok(vector) if !vector.is_empty() => vector,
            Ok(_) => {
                failures.push(format!("пустой эмбеддинг запроса «{query}»"));
                continue;
            }
            Err(e) => {
                failures.push(format!("«{query}»: {e}"));
                continue;
            }
        };

        let take = if allowed_files.is_some() {
            (per_query * 6).max(per_query)
        } else {
            (per_query * 2).max(per_query)
        };
        let hits = searcher.search(&vector, take)?;
        for (point, score) in hits {
            let Some(payload) = parse_payload(&point.payload) else {
                continue;
            };
            if let Some(allowed) = allowed_files {
                if !allowed.contains(&payload.path) {
                    continue;
                }
            }
            let chunk = RetrievedChunk {
                path: payload.path,
                line: payload.line,
                kind: payload.kind,
                name: payload.name,
                code: payload.code,
                score,
            };
            best.entry(chunk.location())
                .and_modify(|existing| {
                    if score > existing.score {
                        *existing = chunk.clone();
                    }
                })
                .or_insert(chunk);
        }
    }

    if best.is_empty() {
        return Err(format!(
            "Векторный поиск не дал результатов: {}",
            failures.join("; ")
        ));
    }

    let mut chunks: Vec<RetrievedChunk> = best.into_values().collect();
    chunks.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    chunks.truncate(limit);
    Ok(chunks)
}

/// Форматирование найденных чанков для промпта (с жёстким лимитом символов).
pub fn format_context(chunks: &[RetrievedChunk], max_chars: usize) -> String {
    let mut out = String::new();
    for chunk in chunks {
        let header = format!(
            "=== {} | {} `{}` | близость {:.3} ===\n",
            chunk.location(),
            chunk.kind,
            chunk.name,
            chunk.score
        );
        let body = format!("{}\n\n", chunk.code.trim());
        if out.len() + header.len() + body.len() > max_chars {
            break;
        }
        out.push_str(&header);
        out.push_str(&body);
    }
    if out.is_empty() {
        "Контекст не найден.".to_string()
    } else {
        out
    }
}

fn make_chunks(model: &ProjectModel, root: &Path) -> Vec<CodeChunk> {
    let mut chunks = Vec::new();
    let mut used_ids = HashSet::new();
    let mut chunked_files: HashSet<String> = HashSet::new();

    let mut file_keys: Vec<&String> = model.entities.keys().collect();
    file_keys.sort();
    for file in file_keys {
        let entities = &model.entities[file];
        for entity in entities {
            let Some(code) = entity
                .code
                .as_deref()
                .filter(|code| !code.trim().is_empty())
            else {
                continue;
            };
            let code = truncate_code(code);
            let id = unique_id(
                &mut used_ids,
                format!("{}:{}:{}:{}", file, entity.line, entity.kind, entity.name),
            );
            let payload = ChunkPayload {
                path: file.clone(),
                line: entity.line,
                kind: entity.kind.clone(),
                name: entity.name.clone(),
                code: code.clone(),
            };
            let embedding_text = format!(
                "Файл: {file}\n{} `{}` (строка {})\n{code}",
                entity.kind, entity.name, entity.line
            );
            chunked_files.insert(file.clone());
            chunks.push(CodeChunk {
                id,
                payload,
                embedding_text,
            });
        }
    }

    // Файлы без извлечённых сущностей (например, HTML/CSS/скрипты без
    // поддерживаемых конструкций) режем на строковые окна. Сюда же попадают
    // файлы, у которых сущности есть, но tree-sitter не сохранил их код.
    for file in &model.files {
        if chunked_files.contains(file) {
            continue;
        }
        let path = root.join(file);
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            continue;
        }
        for (window, start) in (0..lines.len()).step_by(FILE_CHUNK_LINES).enumerate() {
            let end = (start + FILE_CHUNK_LINES).min(lines.len());
            let code = truncate_code(&lines[start..end].join("\n"));
            if code.trim().is_empty() {
                continue;
            }
            let line = (start + 1) as u32;
            let name = Path::new(file)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file.clone());
            let id = unique_id(&mut used_ids, format!("{file}:{line}:file:{window}"));
            let payload = ChunkPayload {
                path: file.clone(),
                line,
                kind: "file".to_string(),
                name: name.clone(),
                code: code.clone(),
            };
            let embedding_text = format!("Файл: {file} (строки {line}-{end})\n{code}");
            chunks.push(CodeChunk {
                id,
                payload,
                embedding_text,
            });
        }
    }

    chunks
}

fn truncate_code(code: &str) -> String {
    let chars: Vec<char> = code.chars().collect();
    if chars.len() <= MAX_CODE_CHARS {
        return code.to_string();
    }
    let marker = "\n... (фрагмент сокращён) ...\n";
    let budget = MAX_CODE_CHARS.saturating_sub(marker.chars().count());
    let head_len = budget * 2 / 3;
    let tail_len = budget - head_len;
    let head: String = chars.iter().take(head_len).collect();
    let tail: String = chars.iter().skip(chars.len() - tail_len).collect();
    format!("{head}{marker}{tail}")
}

fn unique_id(used: &mut HashSet<String>, base: String) -> String {
    if used.insert(base.clone()) {
        return base;
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base}#{suffix}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn parse_payload(payload: &str) -> Option<ChunkPayload> {
    serde_json::from_str(payload).ok()
}

fn reset_index(dir: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Не удалось очистить индекс {}: {e}", dir.display())),
    }
}

fn backend_name() -> &'static str {
    if cfg!(feature = "qdrant") {
        "qdrant-edge"
    } else {
        "in-memory"
    }
}

#[cfg(feature = "qdrant")]
fn add_points(dir: &Path, dimension: usize, points: Vec<VectorPoint>) -> Result<(), String> {
    use crate::index::qdrant_store::QdrantVectorStore;
    let mut store = QdrantVectorStore::open(dir, dimension)?;
    store.add_batch(points)
}

#[cfg(not(feature = "qdrant"))]
fn add_points(dir: &Path, _dimension: usize, points: Vec<VectorPoint>) -> Result<(), String> {
    use crate::index::vector_store::VectorStore;
    let path = dir.join("index.json");
    let mut store = VectorStore::load(&path);
    store.add_batch(points)?;
    store.save(&path)
}

/// Открытый один раз поисковый бэкенд.
///
/// Раньше индекс открывался на каждый запрос: для Qdrant Edge это означало
/// повторную загрузку шарда и восстановление WAL (и шумные логи) на каждый
/// из десятков запросов. Теперь [`retrieve`] открывает его однократно.
enum Searcher {
    #[cfg(feature = "qdrant")]
    Qdrant(crate::index::qdrant_store::QdrantVectorStore),
    #[cfg(not(feature = "qdrant"))]
    Memory(crate::index::vector_store::VectorStore),
}

impl Searcher {
    fn open(dir: &Path, dimension: usize) -> Result<Self, String> {
        #[cfg(feature = "qdrant")]
        {
            Ok(Self::Qdrant(
                crate::index::qdrant_store::QdrantVectorStore::open(dir, dimension)?,
            ))
        }
        #[cfg(not(feature = "qdrant"))]
        {
            let _ = dimension;
            Ok(Self::Memory(crate::index::vector_store::VectorStore::load(
                dir.join("index.json"),
            )))
        }
    }

    fn search(&self, vector: &[f32], top_k: usize) -> Result<Vec<(VectorPoint, f32)>, String> {
        match self {
            #[cfg(feature = "qdrant")]
            Self::Qdrant(store) => store.search(vector, top_k),
            #[cfg(not(feature = "qdrant"))]
            Self::Memory(store) => Ok(store.search(vector, top_k)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parser::Entity;

    fn model() -> ProjectModel {
        let entity = Entity {
            name: "run".to_string(),
            kind: "function".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            line: 3,
            code: Some("fn run() { println!(\"hi\"); }".to_string()),
        };
        ProjectModel {
            name: "demo".to_string(),
            entities: HashMap::from([("src/main.rs".to_string(), vec![entity])]),
            files: vec!["src/main.rs".to_string(), "assets/app.css".to_string()],
        }
    }

    #[test]
    fn chunks_are_entity_level_and_file_level() {
        let dir = std::env::temp_dir().join(format!("doccode_rag_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn run() {}\n").unwrap();
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(dir.join("assets/app.css"), ".a { color: red; }\n").unwrap();

        let chunks = make_chunks(&model(), &dir);
        assert!(chunks.iter().any(|c| c.payload.kind == "function"));
        assert!(chunks.iter().any(|c| c.payload.kind == "file"));
        assert!(chunks.iter().any(|c| c.payload.path == "assets/app.css"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn files_with_entities_but_without_code_get_file_chunks() {
        let dir = std::env::temp_dir().join(format!("doccode_rag_empty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/empty.rs"), "// code without parsed body\n").unwrap();

        let mut demo = model();
        demo.files.push("src/empty.rs".to_string());
        demo.entities.insert(
            "src/empty.rs".to_string(),
            vec![Entity {
                name: "Empty".to_string(),
                kind: "struct".to_string(),
                file_path: PathBuf::from("src/empty.rs"),
                line: 1,
                code: None,
            }],
        );

        let chunks = make_chunks(&demo, &dir);
        assert!(chunks
            .iter()
            .any(|chunk| { chunk.payload.path == "src/empty.rs" && chunk.payload.kind == "file" }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn code_is_truncated_without_losing_tail() {
        let long = format!("{}\ntail_marker", "a".repeat(MAX_CODE_CHARS * 2));
        let truncated = truncate_code(&long);
        assert!(truncated.chars().count() <= MAX_CODE_CHARS);
        assert!(truncated.chars().count() < long.chars().count());
        assert!(truncated.contains("фрагмент сокращён"));
        assert!(truncated.ends_with("tail_marker"));
    }

    #[test]
    fn context_has_source_locations_and_limit() {
        let chunks = vec![RetrievedChunk {
            path: "src/main.rs".to_string(),
            line: 10,
            kind: "function".to_string(),
            name: "main".to_string(),
            code: "fn main() {}".to_string(),
            score: 0.9,
        }];
        let context = format_context(&chunks, 10_000);
        assert!(context.contains("src/main.rs:10"));
        assert!(context.contains("fn main()"));
        assert_eq!(format_context(&chunks, 5), "Контекст не найден.");
    }

    #[test]
    fn payload_roundtrip() {
        let payload = ChunkPayload {
            path: "a.rs".to_string(),
            line: 1,
            kind: "function".to_string(),
            name: "f".to_string(),
            code: "fn f() {}".to_string(),
        };
        let raw = serde_json::to_string(&payload).unwrap();
        let parsed = parse_payload(&raw).unwrap();
        assert_eq!(parsed.path, "a.rs");
        assert_eq!(parsed.name, "f");
    }
}
