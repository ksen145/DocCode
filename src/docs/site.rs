//! Генерация интерактивного HTML-сайта документации.
//!
//! Поток:
//! 1. [`build_index`] — чанки кода → эмбеддинги → Qdrant;
//! 2. [`plan_site`] — LLM по векторному контексту строит дерево компонентов
//!    и общий обзор проекта;
//! 3. [`describe_component`] — LLM документирует один компонент, D1 проверяет
//!    факты и полноту;
//! 4. [`finalize_site`] — детерминированный рендер `index.html`.

use crate::ai::client::{AiClient, GenerateOptions};
use crate::ai::d1::{D1Policy, D1Request};
use crate::ai::make_client;
use crate::config::{AiBackendConfig, D1Config, GenerationConfig};
use crate::core::model::ProjectModel;
use crate::core::parser::Entity;
use crate::core::scanner::language_for_path;
use crate::docs::plan::{
    entry_candidates, fallback_plan, known_files, normalize_path, parse_plan, sanitize_plan,
    RawComponent, RawPlan,
};
use crate::docs::rag::{self, IndexStats};
use crate::docs::site_html;
use crate::prompts::template::substitute;
use crate::prompts::{load_with_fallback, Prompt, PromptsConfig};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Максимальный размер кода в общем контексте проекта.
const OVERVIEW_CONTEXT_CHARS: usize = 26_000;
/// Максимальный размер кода в контексте одного компонента.
const COMPONENT_CONTEXT_CHARS: usize = 18_000;
/// Критерии оценки полноты для D1.
const COMPLETENESS_CRITERIA: &[&str] = &[
    "1 — почти пусто",
    "2 — только общие слова",
    "3 — достаточно, чтобы начать работу",
    "4 — хорошо: есть файлы, сущности и связи",
    "5 — исчерпывающе для разработчика",
];

/// Запросы для выборки контекста при планировании и обзоре.
const PROJECT_QUERIES: &[&str] = &[
    "точка входа, запуск программы, main",
    "архитектура приложения, слои и модули",
    "назначение проекта, бизнес-логика",
    "конфигурация, настройки, параметры запуска",
    "взаимодействие компонентов и поток данных",
    "хранение данных, внешние сервисы и API",
    "обработка ошибок, логирование",
    "пользовательский интерфейс и представления",
    "тесты и примеры использования",
];

/// Конфигурация генерации сайта.
#[derive(Debug, Clone)]
pub struct SiteConfig {
    pub project_root: PathBuf,
    pub docs_dir: PathBuf,
    pub prompts_path: PathBuf,
    pub ai: AiBackendConfig,
    pub generation: GenerationConfig,
    pub d1: D1Config,
    pub verify_d1: bool,
    pub language: String,
}

impl SiteConfig {
    /// Язык, который передаётся в промпты (`ru`/`en`).
    ///
    /// Формат — предложный падеж, потому что шаблоны говорят
    /// «на {{language}} языке».
    pub fn language_name(&self) -> &str {
        match self.language.as_str() {
            "en" => "английском",
            _ => "русском",
        }
    }
}

/// Статус документа.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocStatus {
    /// Ещё не сгенерирован.
    Pending,
    /// D1 приняла документ.
    Verified,
    /// D1 нашла проблемы.
    NeedsReview,
    /// D1 не запускалась/недоступна.
    Unverified,
    /// Генерация завершилась ошибкой.
    Failed,
}

impl DocStatus {
    pub fn label(self) -> &'static str {
        match self {
            DocStatus::Pending => "ожидает",
            DocStatus::Verified => "проверено D1",
            DocStatus::NeedsReview => "требует проверки",
            DocStatus::Unverified => "не проверено",
            DocStatus::Failed => "ошибка",
        }
    }

    /// CSS-класс для HTML.
    pub fn css(self) -> &'static str {
        match self {
            DocStatus::Pending => "pending",
            DocStatus::Verified => "verified",
            DocStatus::NeedsReview => "review",
            DocStatus::Unverified => "unverified",
            DocStatus::Failed => "failed",
        }
    }
}

/// Результат проверки D1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub factual: Option<f32>,
    pub completeness: Option<u8>,
    pub accepted: bool,
    pub note: String,
}

/// Точка входа программы.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPointDoc {
    pub path: String,
    pub line: u32,
    pub description: String,
}

/// Компонент проекта (логический, с вложенностью).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDoc {
    pub id: String,
    pub name: String,
    /// Короткое описание из плана.
    pub description: String,
    /// Markdown, сгенерированный LLM.
    pub markdown: String,
    /// HTML для страницы.
    pub html: String,
    /// Файлы, закреплённые непосредственно за этим компонентом.
    pub files: Vec<String>,
    /// Вложенные компоненты.
    pub children: Vec<ComponentDoc>,
    pub status: DocStatus,
    pub verification: Option<Verification>,
}

impl ComponentDoc {
    /// Все файлы поддерева без повторов (порядок — родитель, затем дети).
    pub fn all_files(&self) -> Vec<String> {
        let mut files = Vec::new();
        let mut seen = HashSet::new();
        fn walk(component: &ComponentDoc, files: &mut Vec<String>, seen: &mut HashSet<String>) {
            for file in &component.files {
                if seen.insert(file.clone()) {
                    files.push(file.clone());
                }
            }
            for child in &component.children {
                walk(child, files, seen);
            }
        }
        walk(self, &mut files, &mut seen);
        files
    }

    /// Все id поддерева в порядке обхода.
    pub fn all_ids(&self) -> Vec<String> {
        let mut ids = vec![self.id.clone()];
        for child in &self.children {
            ids.extend(child.all_ids());
        }
        ids
    }
}

/// Сущность в файле (для дерева файлов и быстрого перехода к коду).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDoc {
    pub name: String,
    pub kind: String,
    pub line: u32,
    pub code: Option<String>,
}

/// Файл проекта.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDoc {
    pub path: String,
    pub language: String,
    pub component: Option<String>,
    pub entities: Vec<EntityDoc>,
}

/// Статистика сайта.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SiteStats {
    pub files: usize,
    pub entities: usize,
    pub components: usize,
    pub chunks: usize,
    pub verified: usize,
    pub needs_review: usize,
    pub unverified: usize,
    pub failed: usize,
    pub index_backend: String,
}

/// Готовый документ сайта.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteDoc {
    pub project: String,
    pub generated_at: String,
    pub root: String,
    pub stats: SiteStats,
    /// Порог точности, с которым проверялся этот документ.
    #[serde(default = "default_site_min_factuality")]
    pub d1_min_factuality: f32,
    /// Порог полноты, с которым проверялся этот документ.
    #[serde(default = "default_site_min_completeness")]
    pub d1_min_completeness: u8,
    pub overview_markdown: String,
    pub overview_html: String,
    pub overview_status: DocStatus,
    pub overview_verification: Option<Verification>,
    pub entry_points: Vec<EntryPointDoc>,
    pub components: Vec<ComponentDoc>,
    pub files: Vec<FileDoc>,
    pub warnings: Vec<String>,
}

fn default_site_min_factuality() -> f32 {
    crate::config::D1Config::default().min_factuality
}

fn default_site_min_completeness() -> u8 {
    crate::config::D1Config::default().min_completeness
}

/// Промежуточное состояние: план уже построен, описания компонентов ещё нет.
#[derive(Debug, Clone)]
pub struct SiteDraft {
    pub doc: SiteDoc,
    pub pending: Vec<String>,
}

/// Результат документирования одного компонента.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentResult {
    pub id: String,
    pub markdown: String,
    pub html: String,
    pub status: DocStatus,
    pub verification: Option<Verification>,
}

/// Шаг 1: индексация кода в Qdrant.
pub async fn build_index(config: &SiteConfig, model: &ProjectModel) -> Result<IndexStats, String> {
    let client = make_client(&config.ai);
    rag::build_index(
        model,
        &config.project_root,
        client.as_ref(),
        &config.ai.embedding_model,
    )
    .await
}

/// Шаг 2: план компонентов + общий обзор проекта.
pub async fn plan_site(
    config: &SiteConfig,
    model: &ProjectModel,
    index: &IndexStats,
) -> Result<SiteDraft, String> {
    let prompts = load_prompts(&config.prompts_path)?;
    let client = make_client(&config.ai);
    let mut warnings = Vec::new();
    let mut verification_notes = Vec::new();

    let query_strings: Vec<String> = PROJECT_QUERIES
        .iter()
        .map(|query| (*query).to_string())
        .collect();
    let chunks = match rag::retrieve(
        client.as_ref(),
        &config.project_root,
        &query_strings,
        4,
        42,
        None,
    )
    .await
    {
        Ok(chunks) => chunks,
        Err(e) => {
            warnings.push(format!("Векторный контекст обзора не получен: {e}"));
            Vec::new()
        }
    };
    let context = rag::format_context(&chunks, OVERVIEW_CONTEXT_CHARS);
    let outline = project_outline(model);

    let plan_prompt = render_prompt(
        &prompts,
        "project_plan",
        &[
            ("project", model.name.as_str()),
            ("outline", outline.as_str()),
            ("context", context.as_str()),
            ("language", config.language_name()),
        ],
    )?;
    let options = generation_options(config);
    let mut plan = match client.generate(&plan_prompt, &options).await {
        Ok(result) => match parse_plan(&result.content) {
            Ok(mut plan) => {
                let plan_warnings = sanitize_plan(&mut plan, model);
                warnings.extend(plan_warnings);
                plan
            }
            Err(e) => {
                warnings.push(format!(
                    "LLM вернула невалидный план ({e}). Использована группировка по каталогам."
                ));
                fallback_plan(model)
            }
        },
        Err(e) => {
            warnings.push(format!(
                "Не удалось получить план от LLM ({e}). Использована группировка по каталогам."
            ));
            fallback_plan(model)
        }
    };
    if plan.components.is_empty() {
        warnings.push(
            "После проверки плана не осталось ни одного компонента — использована группировка по каталогам."
                .to_string(),
        );
        plan = fallback_plan(model);
    }

    let known = known_files(model);
    let mut counter = 0usize;
    let components = build_components(&plan.components, &mut counter);
    let mut assigned = HashMap::new();
    assign_file_components(&components, &mut assigned);
    let pending: Vec<String> = components.iter().flat_map(ComponentDoc::all_ids).collect();

    let files = build_file_docs(model, &assigned);
    let entry_points = build_entry_points(model, &plan, &known);

    let stats = SiteStats {
        files: files.len(),
        entities: files.iter().map(|file| file.entities.len()).sum(),
        components: pending.len(),
        chunks: index.chunks,
        verified: 0,
        needs_review: 0,
        unverified: 0,
        failed: 0,
        index_backend: index.backend.clone(),
    };

    // Обзор проекта: отдельный запрос, чтобы не перегружать JSON-план.
    let components_outline = components_outline(&components);
    let entry_points_text = entry_points
        .iter()
        .map(|entry| format!("- {}:{} — {}", entry.path, entry.line, entry.description))
        .collect::<Vec<_>>()
        .join("\n");
    let stats_text = format!(
        "файлов: {}, сущностей: {}, компонентов: {}, векторных фрагментов: {}",
        stats.files, stats.entities, stats.components, stats.chunks
    );
    let overview_prompt = render_prompt(
        &prompts,
        "project_overview",
        &[
            ("project", model.name.as_str()),
            ("stats", stats_text.as_str()),
            ("components", components_outline.as_str()),
            ("entry_points", entry_points_text.as_str()),
            ("context", context.as_str()),
            ("language", config.language_name()),
        ],
    );

    let (overview_markdown, overview_status, overview_verification) = match overview_prompt {
        Ok(prompt) => {
            generate_verified(
                config,
                client.as_ref(),
                &prompts,
                &prompt,
                &context,
                "обзор проекта",
                &mut warnings,
                &mut verification_notes,
            )
            .await
        }
        Err(e) => {
            warnings.push(e);
            (String::new(), DocStatus::Unverified, None)
        }
    };

    let overview_markdown = if overview_markdown.trim().is_empty() {
        fallback_overview(model, &stats, &components, &entry_points)
    } else {
        overview_markdown
    };

    let doc = SiteDoc {
        project: model.name.clone(),
        generated_at: unix_timestamp(),
        root: config.project_root.display().to_string(),
        d1_min_factuality: config.d1.min_factuality,
        d1_min_completeness: config.d1.min_completeness,
        stats,
        overview_html: site_html::markdown_to_html(&overview_markdown),
        overview_markdown,
        overview_status,
        overview_verification,
        entry_points,
        components,
        files,
        warnings,
    };

    // Состояния компонентов выставляются после их генерации; в плане они pending.
    Ok(SiteDraft { doc, pending })
}

/// Шаг 3: документирование одного компонента по векторному контексту.
pub async fn describe_component(
    config: &SiteConfig,
    component: &ComponentDoc,
    all_files: &HashSet<String>,
) -> Result<ComponentResult, String> {
    let prompts = load_prompts(&config.prompts_path)?;
    let client = make_client(&config.ai);

    let files: Vec<String> = component
        .all_files()
        .into_iter()
        .filter(|file| all_files.contains(file))
        .collect();
    if files.is_empty() {
        let markdown = format!(
            "# {}\n\n{}\n\n_Компонент не содержит файлов; описание взято из плана._",
            component.name,
            component.description.trim()
        );
        return Ok(ComponentResult {
            id: component.id.clone(),
            html: site_html::markdown_to_html(&markdown),
            markdown,
            status: DocStatus::Unverified,
            verification: None,
        });
    }

    let mut queries = vec![
        format!("{}: {}", component.name, component.description),
        format!(
            "компонент {}, обязанности, взаимодействие, поток данных",
            component.name
        ),
    ];
    for file in files.iter().take(12) {
        queries.push(format!("{file} {}", component.name));
    }

    let allowed: HashSet<String> = files.iter().cloned().collect();
    let chunks = rag::retrieve(
        client.as_ref(),
        &config.project_root,
        &queries,
        3,
        26,
        Some(&allowed),
    )
    .await?;
    let context = rag::format_context(&chunks, COMPONENT_CONTEXT_CHARS);
    let files_outline = component_files_outline(component, &files);
    let prompt = render_prompt(
        &prompts,
        "component_doc",
        &[
            ("name", component.name.as_str()),
            ("description", component.description.as_str()),
            ("files", files_outline.as_str()),
            ("context", context.as_str()),
            ("language", config.language_name()),
        ],
    )?;

    let mut warnings = Vec::new();
    let mut _notes = Vec::new();
    let (markdown, status, verification) = generate_verified(
        config,
        client.as_ref(),
        &prompts,
        &prompt,
        &context,
        &format!("компонент «{}»", component.name),
        &mut warnings,
        &mut _notes,
    )
    .await;

    if markdown.trim().is_empty() {
        return Err(format!(
            "LLM вернула пустой документ для компонента «{}»",
            component.name
        ));
    }

    Ok(ComponentResult {
        id: component.id.clone(),
        html: site_html::markdown_to_html(&markdown),
        markdown,
        status,
        verification,
    })
}

/// Применение результата к дереву.
pub fn apply_component_result(draft: &mut SiteDraft, result: ComponentResult) {
    fn update(components: &mut [ComponentDoc], result: &ComponentResult) -> bool {
        for component in components {
            if component.id == result.id {
                component.markdown = result.markdown.clone();
                component.html = result.html.clone();
                component.status = result.status;
                component.verification = result.verification.clone();
                return true;
            }
            if update(&mut component.children, result) {
                return true;
            }
        }
        false
    }
    update(&mut draft.doc.components, &result);
}

/// Пометить компонент ошибкой, не прерывая остальные.
pub fn mark_component_failed(draft: &mut SiteDraft, component_id: &str, error: &str) {
    fn update(components: &mut [ComponentDoc], id: &str, error: &str) -> bool {
        for component in components {
            if component.id == id {
                component.status = DocStatus::Failed;
                component.markdown = format!(
                    "# {}\n\n_Не удалось сгенерировать описание._\n\n```text\n{}\n```",
                    component.name, error
                );
                component.html = site_html::markdown_to_html(&component.markdown);
                return true;
            }
            if update(&mut component.children, id, error) {
                return true;
            }
        }
        false
    }
    update(&mut draft.doc.components, component_id, error);
}

/// Шаг 4: финальный HTML.
pub fn finalize_site(config: &SiteConfig, draft: &mut SiteDraft) -> Result<PathBuf, String> {
    let mut doc = draft.doc.clone();
    refresh_component_stats(&mut doc);
    std::fs::create_dir_all(&config.docs_dir)
        .map_err(|e| format!("Не удалось создать {}: {e}", config.docs_dir.display()))?;
    let path = config.docs_dir.join("index.html");
    let html = site_html::render_site(&doc);
    std::fs::write(&path, html)
        .map_err(|e| format!("Не удалось записать {}: {e}", path.display()))?;
    draft.doc = doc;
    Ok(path)
}

/// Публичная обёртка: найти компонент по id.
pub fn find_component<'a>(components: &'a [ComponentDoc], id: &str) -> Option<&'a ComponentDoc> {
    for component in components {
        if component.id == id {
            return Some(component);
        }
        if let Some(found) = find_component(&component.children, id) {
            return Some(found);
        }
    }
    None
}

fn refresh_component_stats(doc: &mut SiteDoc) {
    fn walk(components: &[ComponentDoc], stats: &mut SiteStats) {
        for component in components {
            match component.status {
                DocStatus::Verified => stats.verified += 1,
                DocStatus::NeedsReview => stats.needs_review += 1,
                DocStatus::Unverified | DocStatus::Pending => stats.unverified += 1,
                DocStatus::Failed => stats.failed += 1,
            }
            walk(&component.children, stats);
        }
    }
    doc.stats.verified = 0;
    doc.stats.needs_review = 0;
    doc.stats.unverified = 0;
    doc.stats.failed = 0;
    walk(&doc.components, &mut doc.stats);
}

fn load_prompts(path: &Path) -> Result<PromptsConfig, String> {
    let (source, text) = load_with_fallback(path);
    if source.is_none() {
        tracing::warn!(
            "Файл промптов {} не найден — используется встроенный набор",
            path.display()
        );
    }
    serde_yaml::from_str(&text).map_err(|e| {
        format!(
            "Не удалось прочитать промпты {}: {e}. Исправьте файл или удалите его, \
             чтобы применился встроенный набор.",
            source
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<встроенный prompts.yaml>".to_string())
        )
    })
}

fn generation_options(config: &SiteConfig) -> GenerateOptions {
    GenerateOptions {
        temperature: Some(config.generation.temperature),
        max_tokens: Some(config.generation.max_tokens),
    }
}

fn render_prompt(
    prompts: &PromptsConfig,
    name: &str,
    vars: &[(&str, &str)],
) -> Result<String, String> {
    let prompt = prompts.prompts.get(name).ok_or_else(|| {
        format!(
            "Промпт '{}' не найден (доступны: {})",
            name,
            prompts
                .prompts
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    Ok(substitute(&prompt.template, vars))
}

fn prompt_text<'a>(prompts: &'a PromptsConfig, name: &str, fallback: &'a str) -> &'a str {
    prompts
        .prompts
        .get(name)
        .map(|prompt: &Prompt| prompt.template.as_str())
        .unwrap_or(fallback)
}

/// Генерация + проверка D1 + один повтор при отказе.
#[allow(clippy::too_many_arguments)]
async fn generate_verified(
    config: &SiteConfig,
    client: &dyn AiClient,
    prompts: &PromptsConfig,
    prompt: &str,
    source_context: &str,
    label: &str,
    warnings: &mut Vec<String>,
    notes: &mut Vec<String>,
) -> (String, DocStatus, Option<Verification>) {
    let options = generation_options(config);
    let max_retries = config.d1.max_retries as usize;
    let mut current_prompt = prompt.to_string();
    let mut last_markdown = String::new();
    let mut last_status = DocStatus::Unverified;
    let mut last_verification = None;

    for attempt in 0..=max_retries {
        let result = match client.generate(&current_prompt, &options).await {
            Ok(result) => result,
            Err(e) => {
                warnings.push(format!("LLM не сгенерировала {label}: {e}"));
                return (last_markdown, DocStatus::Failed, last_verification);
            }
        };
        let markdown = result.content.trim().to_string();
        if markdown.is_empty() {
            warnings.push(format!("LLM вернула пустой ответ для {label}"));
            return (last_markdown, DocStatus::Failed, last_verification);
        }
        last_markdown = markdown.clone();

        if !config.verify_d1 {
            notes.push(format!("{label}: D1-проверка отключена"));
            return (markdown, DocStatus::Unverified, None);
        }

        let (status, verification) = verify_document(
            client,
            prompts,
            &markdown,
            source_context,
            D1Policy {
                min_factuality: config.d1.min_factuality,
                min_completeness: config.d1.min_completeness,
            },
        )
        .await;
        last_status = status;
        last_verification = verification.clone();

        match status {
            DocStatus::Verified => return (markdown, status, verification),
            DocStatus::NeedsReview if attempt < max_retries => {
                let feedback = verification
                    .as_ref()
                    .map(|v| v.note.clone())
                    .unwrap_or_default();
                warnings.push(format!(
                    "{label}: D1 не приняла документ ({}), повторная генерация",
                    feedback
                ));
                current_prompt = format!(
                    "{prompt}\n\nВАЖНО: предыдущая версия не прошла проверку D1 ({feedback}).\n\
                     Перепиши документ строго по фрагментам кода выше. Не добавляй функций, \
                     которых нет в коде; для каждого утверждения используй path:line.\n"
                );
            }
            DocStatus::NeedsReview => return (markdown, status, verification),
            _ => return (markdown, status, verification),
        }
    }

    (last_markdown, last_status, last_verification)
}

/// Проверка документа моделью D1.
async fn verify_document(
    client: &dyn AiClient,
    prompts: &PromptsConfig,
    markdown: &str,
    source_context: &str,
    policy: D1Policy,
) -> (DocStatus, Option<Verification>) {
    let factuality = prompt_text(
        prompts,
        "d1_factuality",
        "Все технические утверждения документации должны подтверждаться исходным кодом.",
    );
    let completeness = prompt_text(
        prompts,
        "d1_completeness",
        "Оцени, достаточно ли документации разработчику для начала работы с компонентом.",
    );
    let criteria: Vec<String> = COMPLETENESS_CRITERIA
        .iter()
        .map(|value| (*value).to_string())
        .collect();
    let state = format!(
        "=== ДОКУМЕНТАЦИЯ ===\n{markdown}\n\n=== ИСХОДНЫЙ КОД (векторная выборка) ===\n{source_context}"
    );
    let request = D1Request::for_documentation("", state, factuality, completeness, &criteria);

    match client.verify(&request).await {
        Ok(response) => {
            let evaluation = response.evaluate_with(policy);
            let verification = Verification {
                factual: evaluation.factual,
                completeness: evaluation.completeness,
                accepted: evaluation.accepted,
                note: evaluation.label(),
            };
            let status = if evaluation.accepted {
                DocStatus::Verified
            } else {
                DocStatus::NeedsReview
            };
            (status, Some(verification))
        }
        Err(e) => {
            let note = format!("D1 недоступна: {e}");
            (
                DocStatus::Unverified,
                Some(Verification {
                    factual: None,
                    completeness: None,
                    accepted: false,
                    note,
                }),
            )
        }
    }
}

fn build_components(raw: &[RawComponent], counter: &mut usize) -> Vec<ComponentDoc> {
    raw.iter()
        .map(|component| {
            *counter += 1;
            let id = format!("c{counter}");
            ComponentDoc {
                id,
                name: component.name.clone(),
                description: component.description.clone(),
                markdown: String::new(),
                html: String::new(),
                files: component.files.clone(),
                children: build_components(&component.children, counter),
                status: DocStatus::Pending,
                verification: None,
            }
        })
        .collect()
}

/// Привязка файла к компоненту для страницы файла.
///
/// Дочерние компоненты перекрывают родительские: если родитель перечисляет
/// всё поддерево, конкретный файл всё равно ведёт в свою детализацию.
fn assign_file_components(components: &[ComponentDoc], assigned: &mut HashMap<String, String>) {
    for component in components {
        for file in &component.files {
            assigned.insert(file.clone(), component.id.clone());
        }
        assign_file_components(&component.children, assigned);
    }
}

fn build_file_docs(model: &ProjectModel, assigned: &HashMap<String, String>) -> Vec<FileDoc> {
    let mut paths: Vec<String> = if model.files.is_empty() {
        model.entities.keys().cloned().collect()
    } else {
        model.files.clone()
    };
    paths.sort();
    paths.dedup();

    paths
        .into_iter()
        .map(|path| {
            let language = language_for_path(Path::new(&path))
                .unwrap_or("text")
                .to_string();
            let entities = model
                .entities
                .get(&path)
                .map(|entities| entities.iter().map(entity_doc).collect::<Vec<EntityDoc>>())
                .unwrap_or_default();
            FileDoc {
                component: assigned.get(&path).cloned(),
                path,
                language,
                entities,
            }
        })
        .collect()
}

fn entity_doc(entity: &Entity) -> EntityDoc {
    EntityDoc {
        name: entity.name.clone(),
        kind: entity.kind.clone(),
        line: entity.line,
        code: entity
            .code
            .as_ref()
            .map(|code| truncate_for_html(code, 5000)),
    }
}

fn truncate_for_html(code: &str, max_chars: usize) -> String {
    let chars: Vec<char> = code.chars().collect();
    if chars.len() <= max_chars {
        return code.to_string();
    }
    let head: String = chars.iter().take(max_chars).collect();
    format!("{head}\n... (сокращено)")
}

fn build_entry_points(
    model: &ProjectModel,
    plan: &RawPlan,
    known: &HashMap<String, String>,
) -> Vec<EntryPointDoc> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for (path, line) in entry_candidates(model) {
        if seen.insert(path.clone()) {
            let description = plan
                .entry_points
                .iter()
                .find(|entry| entry.path.contains(&path) || path.contains(&entry.path))
                .map(|entry| entry.description.trim().to_string())
                .filter(|description| !description.is_empty())
                .unwrap_or_else(|| "Точка запуска программы".to_string());
            result.push(EntryPointDoc {
                path,
                line,
                description,
            });
        }
    }

    for entry in &plan.entry_points {
        let Some(path) = normalize_path(&entry.path, known) else {
            continue;
        };
        if !seen.insert(path.clone()) {
            continue;
        }
        let line = model
            .entities
            .get(&path)
            .and_then(|entities| entities.first())
            .map(|entity| entity.line)
            .unwrap_or(1);
        result.push(EntryPointDoc {
            path,
            line,
            description: entry.description.trim().to_string(),
        });
    }

    result.truncate(8);
    result
}

fn project_outline(model: &ProjectModel) -> String {
    let mut out = String::new();
    let mut files: Vec<&String> = model.files.iter().collect();
    files.sort();
    for file in files {
        let entities = model.entities.get(file);
        let entity_text = entities
            .map(|entities| {
                entities
                    .iter()
                    .map(|entity| format!("{} `{}`@{}", entity.kind, entity.name, entity.line))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "—".to_string());
        out.push_str(&format!("- {file}: {entity_text}\n"));
        if out.len() > 18_000 {
            out.push_str("- ... (список файлов сокращён)\n");
            break;
        }
    }
    out
}

fn components_outline(components: &[ComponentDoc]) -> String {
    fn walk(components: &[ComponentDoc], depth: usize, out: &mut String) {
        for component in components {
            let indent = "  ".repeat(depth);
            out.push_str(&format!(
                "{indent}- {} ({} файлов): {}\n",
                component.name,
                component.all_files().len(),
                component.description
            ));
            walk(&component.children, depth + 1, out);
        }
    }
    let mut out = String::new();
    walk(components, 0, &mut out);
    out
}

fn component_files_outline(component: &ComponentDoc, files: &[String]) -> String {
    let mut out = String::new();
    for file in files {
        out.push_str(&format!("- {file}\n"));
    }
    if out.is_empty() {
        out.push_str("- (нет файлов)\n");
    }
    out.push_str(&format!(
        "\nКомпонент: {}\nОписание из плана: {}\n",
        component.name, component.description
    ));
    out
}

fn fallback_overview(
    model: &ProjectModel,
    stats: &SiteStats,
    components: &[ComponentDoc],
    entry_points: &[EntryPointDoc],
) -> String {
    let mut out = format!("# {}\n\n", model.name);
    out.push_str("## Назначение\n\n");
    out.push_str(&format!(
        "Проект `{}` содержит {} файлов и {} сущностей кода.\n\n",
        model.name, stats.files, stats.entities
    ));
    out.push_str("## Точка входа\n\n");
    if entry_points.is_empty() {
        out.push_str("Явная точка входа не найдена — начните с корневых файлов проекта.\n\n");
    } else {
        for entry in entry_points {
            out.push_str(&format!(
                "- `{}:{}` — {}\n",
                entry.path, entry.line, entry.description
            ));
        }
        out.push('\n');
    }
    out.push_str("## Основные компоненты\n\n");
    for component in components {
        out.push_str(&format!(
            "- **{}** — {} ({} файлов)\n",
            component.name,
            component.description,
            component.all_files().len()
        ));
    }
    out
}

fn unix_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::d1::D1Response;

    fn component(id: &str, name: &str, files: &[&str]) -> ComponentDoc {
        ComponentDoc {
            id: id.to_string(),
            name: name.to_string(),
            description: "Описание".to_string(),
            markdown: String::new(),
            html: String::new(),
            files: files.iter().map(|file| (*file).to_string()).collect(),
            children: Vec::new(),
            status: DocStatus::Pending,
            verification: None,
        }
    }

    #[test]
    fn component_tree_ids_and_files() {
        let mut parent = component("c1", "Ядро", &["src/main.rs"]);
        parent
            .children
            .push(component("c2", "Парсер", &["src/parser.rs"]));
        assert_eq!(parent.all_files(), vec!["src/main.rs", "src/parser.rs"]);
        assert_eq!(parent.all_ids(), vec!["c1", "c2"]);

        parent.children[0].files.push("src/main.rs".to_string());
        assert_eq!(parent.all_files(), vec!["src/main.rs", "src/parser.rs"]);
    }

    #[test]
    fn nested_file_assignment_prefers_deepest_component() {
        let mut parent = component("c1", "Ядро", &["src/shared.rs"]);
        parent.children.push(component(
            "c2",
            "Парсер",
            &["src/shared.rs", "src/parser.rs"],
        ));
        let mut assigned = HashMap::new();
        assign_file_components(&[parent], &mut assigned);
        assert_eq!(
            assigned.get("src/shared.rs").map(String::as_str),
            Some("c2")
        );
        assert_eq!(
            assigned.get("src/parser.rs").map(String::as_str),
            Some("c2")
        );
    }

    #[test]
    fn result_updates_nested_component() {
        let mut doc = SiteDoc {
            project: "demo".to_string(),
            generated_at: String::new(),
            root: String::new(),
            stats: SiteStats::default(),
            d1_min_factuality: 0.85,
            d1_min_completeness: 3,
            overview_markdown: String::new(),
            overview_html: String::new(),
            overview_status: DocStatus::Verified,
            overview_verification: None,
            entry_points: Vec::new(),
            components: vec![component("c1", "Ядро", &["a.rs"])],
            files: Vec::new(),
            warnings: Vec::new(),
        };
        doc.components[0]
            .children
            .push(component("c2", "Вложенный", &["b.rs"]));
        let mut draft = SiteDraft {
            doc,
            pending: vec!["c1".to_string(), "c2".to_string()],
        };
        apply_component_result(
            &mut draft,
            ComponentResult {
                id: "c2".to_string(),
                markdown: "текст".to_string(),
                html: "<p>текст</p>".to_string(),
                status: DocStatus::Verified,
                verification: None,
            },
        );
        assert_eq!(draft.doc.components[0].children[0].markdown, "текст");
        assert_eq!(
            find_component(&draft.doc.components, "c2").unwrap().status,
            DocStatus::Verified
        );
    }

    #[test]
    fn d1_policy_thresholds_are_applied() {
        let response = D1Response {
            answers: HashMap::from([
                ("is_factual".to_string(), serde_json::json!({"noul": 0.9})),
                ("completeness".to_string(), serde_json::json!({"score": 4})),
            ]),
        };
        let evaluation = response.evaluate();
        assert!(evaluation.accepted);
    }

    #[test]
    fn stats_count_statuses() {
        let mut doc = SiteDoc {
            project: "demo".to_string(),
            generated_at: String::new(),
            root: String::new(),
            stats: SiteStats::default(),
            d1_min_factuality: 0.85,
            d1_min_completeness: 3,
            overview_markdown: String::new(),
            overview_html: String::new(),
            overview_status: DocStatus::Verified,
            overview_verification: None,
            entry_points: Vec::new(),
            components: vec![
                ComponentDoc {
                    status: DocStatus::Verified,
                    ..component("c1", "A", &[])
                },
                ComponentDoc {
                    status: DocStatus::Failed,
                    ..component("c2", "B", &[])
                },
            ],
            files: Vec::new(),
            warnings: Vec::new(),
        };
        refresh_component_stats(&mut doc);
        assert_eq!(doc.stats.verified, 1);
        assert_eq!(doc.stats.failed, 1);
    }
}
