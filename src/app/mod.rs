//! Корневой модуль приложения DocCode: состояние, сообщения, update/view, запуск iced.
//!
//! Пайплайн:
//! `Scan → Index (Qdrant) → Plan (LLM + overview) → Generate (по компонентам,
//! D1-проверка) → Finalize (index.html) → Results`.

use crate::ai::make_client;
use crate::config::{AiBackendConfig, AppConfig, D1Config, GenerationConfig, ThemeMode};
use crate::core::model::ProjectModel;
use crate::core::pipeline;
use crate::docs::rag::IndexStats;
use crate::docs::site::{self, ComponentResult, SiteConfig, SiteDraft, SiteStats};
use crate::prompts::manager::PromptsManager;
use crate::prompts::schema::PromptsConfig;
use crate::ui::i18n::Lang;
use crate::ui::views;
use iced::widget::text_editor;
use iced::{Element, Task};
use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Стадия генерации (для экрана прогресса).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Генерация ещё не запускалась.
    Idle,
    /// Сканирование и разбор проекта.
    Scan,
    /// Построение векторного индекса в Qdrant.
    Index,
    /// План компонентов и общий обзор.
    Plan,
    /// Документирование компонентов + D1.
    Generate,
    /// Рендер единого index.html.
    Finalize,
    /// Завершено (переход к результатам).
    Done,
}

impl Stage {
    /// Ключ i18n для названия стадии.
    pub fn label_key(self) -> &'static str {
        match self {
            Stage::Idle | Stage::Scan => "progress.stage.scan",
            Stage::Index => "progress.stage.index",
            Stage::Plan => "progress.stage.plan",
            Stage::Generate => "progress.stage.generate",
            Stage::Finalize => "progress.stage.finalize",
            Stage::Done => "progress.stage.done",
        }
    }
}

/// Корневое состояние приложения.
#[derive(Debug)]
pub struct App {
    /// Конфигурация приложения.
    pub config: AppConfig,
    /// Разрешённая тема (с учётом системной настройки на момент запуска).
    resolved_dark: bool,
    /// Текущий экран (переходы обрабатываются в update()).
    pub current_screen: Screen,
    /// Язык интерфейса.
    pub lang: Lang,

    /// Статус для экрана приветствия (ошибки запуска).
    pub status: Option<String>,

    // --- Черновики настроек проекта ---
    /// Путь к проекту (строка поля ввода).
    pub project_path: String,
    /// Папка для документации (строка поля ввода).
    pub docs_path: String,
    /// Статус сохранения настроек проекта.
    pub project_status: Option<String>,

    // --- Черновики подключения к Ollama ---
    /// URL API (строка поля ввода).
    pub ai_url: String,
    /// Имя модели генерации (строка поля ввода).
    pub ai_model: String,
    /// Модель эмбеддингов для Qdrant.
    pub ai_embedding_model: String,
    /// Модель D1 для проверки документации.
    pub ai_d1_model: String,
    /// Температура генерации (строка поля ввода).
    pub ai_temperature: String,
    /// Максимум токенов ответа (строка поля ввода).
    pub ai_max_tokens: String,
    /// Разрешить модели рассуждать перед ответом (`think`).
    pub ai_think: bool,
    /// Порог фактической точности D1.
    pub ai_d1_min_factuality: String,
    /// Порог полноты D1.
    pub ai_d1_min_completeness: String,
    /// Число повторов генерации при отказе D1.
    pub ai_d1_max_retries: String,
    /// Результат проверки связи/сохранения.
    pub ai_status: Option<String>,

    // --- Редактор промптов ---
    /// Содержимое prompts.yaml в редакторе.
    pub prompts_content: text_editor::Content,
    /// Статус валидации/сохранения промптов.
    pub prompts_status: Option<String>,

    // --- Прогресс текущего запуска ---
    /// Текущая стадия.
    pub stage: Stage,
    /// Завершено единиц работы (компонентов).
    pub progress_done: usize,
    /// Всего единиц работы.
    pub progress_total: usize,
    /// Просканировано файлов (в этом запуске).
    pub files_scanned: usize,
    /// Проиндексировано векторных фрагментов.
    pub index_chunks: usize,
    /// Бэкенд индекса (qdrant-edge / in-memory).
    pub index_backend: String,
    /// Очередь id компонентов, ожидающих генерации.
    pub pending: Vec<String>,
    /// Модель последнего анализа (нужна для плана и индексации).
    pub last_model: Option<ProjectModel>,
    /// Черновик сайта между стадиями.
    pub site_draft: Option<SiteDraft>,
    /// Статистика последнего сайта.
    pub site_stats: Option<SiteStats>,

    // --- Результаты последнего запуска ---
    /// Записанные файлы (сейчас — один index.html).
    pub written: Vec<PathBuf>,
    /// Ошибки генерации.
    pub failures: Vec<String>,
}

impl Default for App {
    fn default() -> Self {
        let config = AppConfig::default_config();
        let resolved_dark = config.theme.is_dark();
        let ai = config.ai_backend.clone();
        let generation = config.generation.clone();
        let d1 = config.d1.clone();
        Self {
            config,
            resolved_dark,
            current_screen: Screen::Welcome,
            lang: Lang::default(),
            status: None,
            project_path: String::new(),
            docs_path: String::new(),
            project_status: None,
            ai_url: ai
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string()),
            ai_model: ai.model.clone(),
            ai_embedding_model: ai.embedding_model.clone(),
            ai_d1_model: ai.d1_model.clone(),
            ai_temperature: generation.temperature.to_string(),
            ai_max_tokens: generation.max_tokens.to_string(),
            ai_think: generation.think,
            ai_d1_min_factuality: d1.min_factuality.to_string(),
            ai_d1_min_completeness: d1.min_completeness.to_string(),
            ai_d1_max_retries: d1.max_retries.to_string(),
            ai_status: None,
            prompts_content: text_editor::Content::new(),
            prompts_status: None,
            stage: Stage::Idle,
            progress_done: 0,
            progress_total: 0,
            files_scanned: 0,
            index_chunks: 0,
            index_backend: String::new(),
            pending: Vec::new(),
            last_model: None,
            site_draft: None,
            site_stats: None,
            written: Vec::new(),
            failures: Vec::new(),
        }
    }
}

/// Перечисление экранов приложения.
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Welcome,
    ProjectSettings,
    AiSettings,
    PromptsEditor,
    Progress,
    Results,
}

/// Сообщения UI (Elm-цикл: view → update → Task).
#[derive(Debug, Clone)]
pub enum Message {
    // --- Навигация и темы ---
    Navigate(Screen),
    ToggleTheme,
    ToggleLang,

    // --- Настройки проекта ---
    ProjectPathChanged(String),
    DocsPathChanged(String),
    PickProjectFolder,
    PickDocsFolder,
    SaveProject,
    ToggleD1Verification(bool),

    // --- Подключение к Ollama ---
    AiUrlChanged(String),
    AiModelChanged(String),
    AiEmbeddingModelChanged(String),
    AiD1ModelChanged(String),
    AiTemperatureChanged(String),
    AiMaxTokensChanged(String),
    ToggleAiThinking(bool),
    AiD1FactualityChanged(String),
    AiD1CompletenessChanged(String),
    AiD1RetriesChanged(String),
    SaveAi,
    TestAi,
    AiTestFinished(Result<(), String>),

    // --- Промпты ---
    PromptsAction(text_editor::Action),
    PromptsReload,
    PromptsValidate,
    PromptsSave,

    // --- Генерация ---
    Generate,
    AnalysisFinished(Result<pipeline::Analysis, String>),
    IndexFinished(Result<IndexStats, String>),
    SitePlanned(Result<Box<SiteDraft>, String>),
    ComponentFinished(String, Result<Box<ComponentResult>, String>),
    SiteFinished(Result<(PathBuf, SiteStats), String>),
    OpenSite,
    OpenSiteFinished(Result<(), String>),
    BackToWelcome,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    /// Приложение с загруженной конфигурацией (черновики полей заполняются из неё).
    pub fn with_config(config: AppConfig) -> Self {
        let ai = config.ai_backend.clone();
        let generation = config.generation.clone();
        let d1 = config.d1.clone();
        let lang = Lang::from_code(&config.lang);
        let mut app = Self {
            config,
            lang,
            project_path: String::new(),
            docs_path: String::new(),
            ai_url: ai
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string()),
            ai_model: ai.model.clone(),
            ai_embedding_model: ai.embedding_model.clone(),
            ai_d1_model: ai.d1_model.clone(),
            ai_temperature: generation.temperature.to_string(),
            ai_max_tokens: generation.max_tokens.to_string(),
            ai_think: generation.think,
            ai_d1_min_factuality: d1.min_factuality.to_string(),
            ai_d1_min_completeness: d1.min_completeness.to_string(),
            ai_d1_max_retries: d1.max_retries.to_string(),
            ..Default::default()
        };
        app.resolved_dark = app.config.theme.is_dark();
        app.project_path = app
            .config
            .project_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        app.docs_path = app
            .config
            .docs_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        app
    }

    /// Текущий экран.
    pub fn screen(&self) -> &Screen {
        &self.current_screen
    }

    /// Смена экрана.
    pub fn set_screen(&mut self, screen: Screen) {
        self.current_screen = screen;
    }

    /// Перевод ключа на текущий язык.
    pub fn t(&self, key: &'static str) -> Cow<'static, str> {
        self.lang.t(key)
    }

    /// Включена ли тёмная тема.
    pub fn is_dark_theme(&self) -> bool {
        self.resolved_dark
    }

    /// Директория записи документации для текущего запуска.
    fn docs_output_dir(&self) -> PathBuf {
        let docs = self.docs_path.trim();
        if !docs.is_empty() {
            return PathBuf::from(docs);
        }
        let project = if !self.project_path.trim().is_empty() {
            Some(PathBuf::from(self.project_path.trim()))
        } else {
            self.config.project_path.clone()
        };
        project
            .map(|root| root.join("docs"))
            .unwrap_or_else(|| PathBuf::from("docs"))
    }

    /// Текущее подключение к Ollama, собранное из полей ввода.
    fn draft_ai_backend(&self) -> AiBackendConfig {
        AiBackendConfig {
            base_url: Some(self.ai_url.trim().to_string()),
            model: self.ai_model.trim().to_string(),
            embedding_model: self.ai_embedding_model.trim().to_string(),
            d1_model: self.ai_d1_model.trim().to_string(),
        }
    }

    /// Параметры генерации из полей настроек.
    fn draft_generation(&self) -> Result<GenerationConfig, String> {
        let temperature = parse_f32(&self.ai_temperature).ok_or_else(|| {
            format!(
                "{}: {}",
                self.t("ai.invalid_temperature"),
                self.ai_temperature
            )
        })?;
        if !(0.0..=2.0).contains(&temperature) {
            return Err(self.t("ai.temperature_range").to_string());
        }
        let max_tokens = self
            .ai_max_tokens
            .trim()
            .parse::<u32>()
            .map_err(|_| self.t("ai.invalid_max_tokens").to_string())?;
        if !(64..=32768).contains(&max_tokens) {
            return Err(self.t("ai.max_tokens_range").to_string());
        }
        Ok(GenerationConfig {
            temperature,
            max_tokens,
            think: self.ai_think,
        })
    }

    /// Пороги и повторы D1 из полей настроек.
    fn draft_d1(&self) -> Result<D1Config, String> {
        let min_factuality = parse_f32(&self.ai_d1_min_factuality)
            .ok_or_else(|| self.t("ai.invalid_factuality").to_string())?;
        if !(0.0..=1.0).contains(&min_factuality) {
            return Err(self.t("ai.factuality_range").to_string());
        }
        let min_completeness = self
            .ai_d1_min_completeness
            .trim()
            .parse::<u8>()
            .map_err(|_| self.t("ai.invalid_completeness").to_string())?;
        if !(1..=5).contains(&min_completeness) {
            return Err(self.t("ai.completeness_range").to_string());
        }
        let max_retries = self
            .ai_d1_max_retries
            .trim()
            .parse::<u8>()
            .map_err(|_| self.t("ai.invalid_retries").to_string())?;
        if max_retries > 5 {
            return Err(self.t("ai.retries_range").to_string());
        }
        Ok(D1Config {
            min_factuality,
            min_completeness,
            max_retries,
        })
    }

    /// Конфигурация нового пайплайна для текущего запуска.
    fn site_config(&self) -> Result<SiteConfig, String> {
        let root = if !self.project_path.trim().is_empty() {
            self.project_path.trim().to_string()
        } else {
            self.config
                .project_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        };
        if root.is_empty() {
            return Err(self.t("error.project_empty").to_string());
        }
        let root = PathBuf::from(root);
        if !root.is_dir() {
            return Err(self.t("error.project_missing").to_string());
        }
        Ok(SiteConfig {
            project_root: root,
            docs_dir: self.docs_output_dir(),
            prompts_path: self.config.prompts_path.clone(),
            ai: self.draft_ai_backend(),
            generation: self.draft_generation()?,
            d1: self.draft_d1()?,
            verify_d1: self.config.verify_d1,
            language: self.lang.code().to_string(),
        })
    }
}

/// Число из поля ввода с поддержкой запятой как десятичного разделителя.
fn parse_f32(raw: &str) -> Option<f32> {
    raw.trim().replace(',', ".").parse().ok()
}

/// Сохранение конфига без прерывания работы UI (ошибка — только в лог).
fn save_config_quiet(config: &AppConfig) {
    if let Err(e) = config.save() {
        tracing::warn!("Не удалось сохранить конфиг: {}", e);
    }
}

/// Загрузка промптов в редактор с fallback-цепочкой.
///
/// Если файла нет нигде на диске — встроенный вариант записывается по пути
/// из конфига, чтобы генерация читала существующий файл.
fn load_prompts_into(app: &mut App) {
    let expected = app.config.prompts_path.clone();
    let (source, text) = crate::prompts::load_with_fallback(&expected);

    if let Some(actual) = source {
        app.config.prompts_path = actual;
        app.prompts_content = text_editor::Content::with_text(&text);
        app.prompts_status = None;
        return;
    }

    app.prompts_content = text_editor::Content::with_text(&text);
    app.prompts_status = None;
    if let Some(parent) = expected.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            let status = format!("{}: {}", app.t("prompts.missing"), e);
            app.prompts_status = Some(status);
            return;
        }
    }
    if let Err(e) = std::fs::write(&expected, &text) {
        tracing::warn!("Не удалось записать {}: {}", expected.display(), e);
        let status = format!("{}: {}", app.t("prompts.missing"), e);
        app.prompts_status = Some(status);
    }
}

/// Финализация черновика в `index.html`.
fn finalize_draft(app: &mut App) -> Task<Message> {
    let Some(mut draft) = app.site_draft.take() else {
        app.failures
            .push("Внутренняя ошибка: черновик сайта потерян".to_string());
        app.stage = Stage::Done;
        app.set_screen(Screen::Results);
        return Task::none();
    };
    let config = match app.site_config() {
        Ok(config) => config,
        Err(e) => {
            app.failures.push(e);
            app.stage = Stage::Done;
            app.set_screen(Screen::Results);
            return Task::none();
        }
    };
    app.stage = Stage::Finalize;
    Task::perform(
        async move {
            site::finalize_site(&config, &mut draft).map(|path| (path, draft.doc.stats.clone()))
        },
        Message::SiteFinished,
    )
}

/// Запуск следующего компонента из очереди (или финализация).
fn start_next_component(app: &mut App) -> Task<Message> {
    let Some(id) = app.pending.first().cloned() else {
        return finalize_draft(app);
    };
    app.pending.remove(0);

    let Some(draft) = app.site_draft.as_ref() else {
        app.failures
            .push("Внутренняя ошибка: черновик сайта потерян".to_string());
        app.stage = Stage::Done;
        app.set_screen(Screen::Results);
        return Task::none();
    };
    let Some(component) = site::find_component(&draft.doc.components, &id).cloned() else {
        app.failures
            .push(format!("Компонент `{id}` не найден в дереве"));
        app.progress_done += 1;
        return start_next_component(app);
    };
    let all_files: HashSet<String> = draft.doc.files.iter().map(|f| f.path.clone()).collect();
    let config = match app.site_config() {
        Ok(config) => config,
        Err(e) => {
            app.failures.push(e);
            app.stage = Stage::Done;
            app.set_screen(Screen::Results);
            return Task::none();
        }
    };

    let task_id = id.clone();
    Task::perform(
        async move { site::describe_component(&config, &component, &all_files).await },
        move |result| Message::ComponentFinished(task_id.clone(), result.map(Box::new)),
    )
}

/// Открыть файл в системном приложении.
fn open_path(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.display().to_string()])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Не удалось открыть {}: {e}", path.display()))
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Не удалось открыть {}: {e}", path.display()))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Не удалось открыть {}: {e}", path.display()))
    }
}

/// Начальная директория для диалога выбора папки.
fn dialog_start(current: &str, fallback: Option<&Path>) -> Option<PathBuf> {
    let current = current.trim();
    if !current.is_empty() {
        let path = PathBuf::from(current);
        if path.is_dir() {
            return Some(path);
        }
    }

    fallback
        .filter(|path| path.is_dir())
        .map(Path::to_path_buf)
        .or_else(dirs::home_dir)
}

/// Системный диалог выбора папки.
///
/// Вызывается из `update` (главный поток): диалог модальный и открывается
/// поверх окна приложения, поэтому отдельный поток не нужен.
fn pick_folder(title: &str, start: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().set_title(title);
    if let Some(start) = start.filter(|path| path.is_dir()) {
        dialog = dialog.set_directory(start);
    }
    dialog.pick_folder()
}

/// Цикл обновления состояния (Elm-architecture update).
pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // --- Навигация и темы ---
        Message::Navigate(screen) => {
            app.set_screen(screen);
            app.status = None;
            Task::none()
        }
        Message::ToggleTheme => {
            // System → явная противоположная тема → другая явная → System.
            app.config.theme = match app.config.theme {
                ThemeMode::System if app.is_dark_theme() => ThemeMode::Light,
                ThemeMode::System => ThemeMode::Dark,
                ThemeMode::Dark => ThemeMode::Light,
                ThemeMode::Light => ThemeMode::System,
            };
            app.resolved_dark = app.config.theme.is_dark();
            save_config_quiet(&app.config);
            Task::none()
        }
        Message::ToggleLang => {
            app.lang = app.lang.toggle();
            app.config.lang = app.lang.code().to_string();
            save_config_quiet(&app.config);
            Task::none()
        }

        // --- Настройки проекта ---
        Message::ProjectPathChanged(value) => {
            app.project_path = value;
            app.project_status = None;
            Task::none()
        }
        Message::DocsPathChanged(value) => {
            app.docs_path = value;
            app.project_status = None;
            Task::none()
        }
        Message::PickProjectFolder => {
            let start = dialog_start(&app.project_path, app.config.project_path.as_deref());
            let title = app.t("project.choose_path").to_string();
            if let Some(path) = pick_folder(&title, start.as_deref()) {
                app.project_path = path.display().to_string();
                app.project_status = None;
            }
            Task::none()
        }
        Message::PickDocsFolder => {
            let project_root = if app.project_path.trim().is_empty() {
                app.config.project_path.clone()
            } else {
                Some(PathBuf::from(app.project_path.trim()))
            };
            let start = dialog_start(&app.docs_path, project_root.as_deref());
            let title = app.t("project.choose_docs").to_string();
            if let Some(path) = pick_folder(&title, start.as_deref()) {
                app.docs_path = path.display().to_string();
                app.project_status = None;
            }
            Task::none()
        }
        Message::SaveProject => {
            let raw = app.project_path.trim().to_string();
            if raw.is_empty() {
                app.project_status = Some(app.t("error.project_empty").to_string());
                return Task::none();
            }
            let root = PathBuf::from(&raw);
            if !root.is_dir() {
                app.project_status = Some(app.t("error.project_missing").to_string());
                return Task::none();
            }
            app.config.project_path = Some(root);
            app.config.docs_path = {
                let docs = app.docs_path.trim();
                if docs.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(docs))
                }
            };
            let status = match app.config.save() {
                Ok(()) => app.t("project.saved").to_string(),
                Err(e) => format!("{}: {}", app.t("project.save_error"), e),
            };
            app.project_status = Some(status);
            Task::none()
        }
        Message::ToggleD1Verification(enabled) => {
            app.config.verify_d1 = enabled;
            save_config_quiet(&app.config);
            app.project_status = Some(app.t("common.saved").to_string());
            Task::none()
        }

        // --- Подключение к Ollama ---
        Message::AiUrlChanged(value) => {
            app.ai_url = value;
            app.ai_status = None;
            Task::none()
        }
        Message::AiModelChanged(value) => {
            app.ai_model = value;
            app.ai_status = None;
            Task::none()
        }
        Message::AiEmbeddingModelChanged(value) => {
            app.ai_embedding_model = value;
            app.ai_status = None;
            Task::none()
        }
        Message::AiD1ModelChanged(value) => {
            app.ai_d1_model = value;
            app.ai_status = None;
            Task::none()
        }
        Message::AiTemperatureChanged(value) => {
            app.ai_temperature = value;
            app.ai_status = None;
            Task::none()
        }
        Message::AiMaxTokensChanged(value) => {
            app.ai_max_tokens = value;
            app.ai_status = None;
            Task::none()
        }
        Message::ToggleAiThinking(enabled) => {
            app.ai_think = enabled;
            app.ai_status = None;
            Task::none()
        }
        Message::AiD1FactualityChanged(value) => {
            app.ai_d1_min_factuality = value;
            app.ai_status = None;
            Task::none()
        }
        Message::AiD1CompletenessChanged(value) => {
            app.ai_d1_min_completeness = value;
            app.ai_status = None;
            Task::none()
        }
        Message::AiD1RetriesChanged(value) => {
            app.ai_d1_max_retries = value;
            app.ai_status = None;
            Task::none()
        }
        Message::SaveAi => {
            let generation = match app.draft_generation() {
                Ok(value) => value,
                Err(e) => {
                    app.ai_status = Some(e);
                    return Task::none();
                }
            };
            let d1 = match app.draft_d1() {
                Ok(value) => value,
                Err(e) => {
                    app.ai_status = Some(e);
                    return Task::none();
                }
            };
            app.config.ai_backend = app.draft_ai_backend();
            app.config.generation = generation;
            app.config.d1 = d1;
            let status = match app.config.save() {
                Ok(()) => app.t("ai.saved").to_string(),
                Err(e) => format!("{}: {}", app.t("ai.save_error"), e),
            };
            app.ai_status = Some(status);
            Task::none()
        }
        Message::TestAi => {
            let client = make_client(&app.draft_ai_backend());
            app.ai_status = Some(app.t("ai.testing").to_string());
            Task::perform(async move { client.ping().await }, Message::AiTestFinished)
        }
        Message::AiTestFinished(Ok(())) => {
            app.ai_status = Some(app.t("ai.ok").to_string());
            Task::none()
        }
        Message::AiTestFinished(Err(e)) => {
            let status = format!("{}: {}", app.t("ai.fail"), e);
            app.ai_status = Some(status);
            Task::none()
        }

        // --- Промпты ---
        Message::PromptsAction(action) => {
            app.prompts_content.perform(action);
            Task::none()
        }
        Message::PromptsReload => {
            load_prompts_into(app);
            Task::none()
        }
        Message::PromptsValidate => {
            let text = app.prompts_content.text();
            let status = match serde_yaml::from_str::<PromptsConfig>(&text) {
                Ok(_) => app.t("prompts.valid").to_string(),
                Err(e) => format!("{}: {}", app.t("prompts.invalid"), e),
            };
            app.prompts_status = Some(status);
            Task::none()
        }
        Message::PromptsSave => {
            let text = app.prompts_content.text();
            let parsed = match serde_yaml::from_str::<PromptsConfig>(&text) {
                Ok(config) => config,
                Err(e) => {
                    app.prompts_status = Some(format!("{}: {}", app.t("prompts.invalid"), e));
                    return Task::none();
                }
            };
            let manager = PromptsManager::new(app.config.prompts_path.clone());
            let status = match manager.save(&parsed) {
                Ok(()) => app.t("prompts.saved").to_string(),
                Err(e) => format!("{}: {}", app.t("prompts.save_error"), e),
            };
            app.prompts_status = Some(status);
            Task::none()
        }

        // --- Генерация ---
        Message::Generate => {
            let raw = if !app.project_path.trim().is_empty() {
                app.project_path.trim().to_string()
            } else {
                app.config
                    .project_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            };
            if raw.is_empty() {
                app.status = Some(app.t("error.project_empty").to_string());
                return Task::none();
            }
            let root = PathBuf::from(&raw);
            if !root.is_dir() {
                app.status = Some(app.t("error.project_missing").to_string());
                return Task::none();
            }
            if let Err(e) = app.draft_generation() {
                app.ai_status = Some(e.clone());
                app.status = Some(e);
                app.set_screen(Screen::AiSettings);
                return Task::none();
            }
            if let Err(e) = app.draft_d1() {
                app.ai_status = Some(e.clone());
                app.status = Some(e);
                app.set_screen(Screen::AiSettings);
                return Task::none();
            }

            app.status = None;
            app.set_screen(Screen::Progress);
            app.stage = Stage::Scan;
            app.progress_done = 0;
            app.progress_total = 1;
            app.files_scanned = 0;
            app.index_chunks = 0;
            app.index_backend.clear();
            app.pending.clear();
            app.last_model = None;
            app.site_draft = None;
            app.site_stats = None;
            app.written.clear();
            app.failures.clear();

            Task::perform(
                async move { pipeline::analyze_project(root).await },
                Message::AnalysisFinished,
            )
        }
        Message::AnalysisFinished(Ok(analysis)) => {
            app.files_scanned = analysis.files_scanned;
            app.last_model = Some(analysis.model);
            app.stage = Stage::Index;
            app.progress_done = 0;
            app.progress_total = 1;

            let Some(model) = app.last_model.clone() else {
                return Task::none();
            };
            let config = match app.site_config() {
                Ok(config) => config,
                Err(e) => {
                    app.failures.push(e);
                    app.stage = Stage::Done;
                    app.set_screen(Screen::Results);
                    return Task::none();
                }
            };
            Task::perform(
                async move { site::build_index(&config, &model).await },
                Message::IndexFinished,
            )
        }
        Message::AnalysisFinished(Err(e)) => {
            app.stage = Stage::Done;
            app.failures.push(e);
            app.set_screen(Screen::Results);
            Task::none()
        }
        Message::IndexFinished(Ok(stats)) => {
            app.index_chunks = stats.chunks;
            app.index_backend = stats.backend.clone();
            app.stage = Stage::Plan;
            app.progress_done = 0;
            app.progress_total = 1;

            let Some(model) = app.last_model.clone() else {
                app.failures
                    .push("Внутренняя ошибка: модель проекта потеряна".to_string());
                app.stage = Stage::Done;
                app.set_screen(Screen::Results);
                return Task::none();
            };
            let config = match app.site_config() {
                Ok(config) => config,
                Err(e) => {
                    app.failures.push(e);
                    app.stage = Stage::Done;
                    app.set_screen(Screen::Results);
                    return Task::none();
                }
            };
            Task::perform(
                async move { site::plan_site(&config, &model, &stats).await },
                |result| Message::SitePlanned(result.map(Box::new)),
            )
        }
        Message::IndexFinished(Err(e)) => {
            app.stage = Stage::Done;
            app.failures.push(e);
            app.set_screen(Screen::Results);
            Task::none()
        }
        Message::SitePlanned(Ok(draft)) => {
            let draft = *draft;
            app.pending = draft.pending.clone();
            app.site_stats = Some(draft.doc.stats.clone());
            app.site_draft = Some(draft);
            app.stage = Stage::Generate;
            app.progress_done = 0;
            app.progress_total = app.pending.len();
            start_next_component(app)
        }
        Message::SitePlanned(Err(e)) => {
            app.stage = Stage::Done;
            app.failures.push(e);
            app.set_screen(Screen::Results);
            Task::none()
        }
        Message::ComponentFinished(_id, Ok(result)) => {
            if let Some(draft) = app.site_draft.as_mut() {
                site::apply_component_result(draft, *result);
            }
            app.progress_done += 1;
            start_next_component(app)
        }
        Message::ComponentFinished(id, Err(e)) => {
            tracing::warn!("Компонент {} не сгенерирован: {}", id, e);
            app.failures.push(format!("Компонент `{id}`: {e}"));
            if let Some(draft) = app.site_draft.as_mut() {
                site::mark_component_failed(draft, &id, &e);
            }
            app.progress_done += 1;
            start_next_component(app)
        }
        Message::SiteFinished(Ok((path, stats))) => {
            app.written.push(path);
            app.site_stats = Some(stats);
            app.stage = Stage::Done;
            app.set_screen(Screen::Results);
            Task::none()
        }
        Message::SiteFinished(Err(e)) => {
            app.stage = Stage::Done;
            app.failures.push(e);
            app.set_screen(Screen::Results);
            Task::none()
        }
        Message::OpenSite => match app.written.first().cloned() {
            Some(path) => Task::perform(async move { open_path(&path) }, Message::OpenSiteFinished),
            None => {
                app.status = Some("index.html ещё не создан".to_string());
                Task::none()
            }
        },
        Message::OpenSiteFinished(Ok(())) => {
            app.status = Some(app.t("results.opened").to_string());
            Task::none()
        }
        Message::OpenSiteFinished(Err(e)) => {
            app.status = Some(format!("{}: {}", app.t("results.open_error"), e));
            Task::none()
        }
        Message::BackToWelcome => {
            app.set_screen(Screen::Welcome);
            app.status = None;
            Task::none()
        }
    }
}

/// Корневой view: общий каркас + содержимое текущего экрана.
pub fn view(app: &App) -> Element<'_, Message> {
    let content: Element<'_, Message> = match app.screen() {
        Screen::Welcome => views::welcome::view(app),
        Screen::ProjectSettings => views::project_settings::view(app),
        Screen::AiSettings => views::ai_settings::view(app),
        Screen::PromptsEditor => views::prompts_editor::view(app),
        Screen::Progress => views::progress::view(app),
        Screen::Results => views::results::view(app),
    };

    views::shell(app, content)
}

/// Размер встроенного RGBA-пиксельного файла иконки окна (`make_icon.py`).
const WINDOW_ICON_SIZE: u32 = 128;

/// Запуск GUI-цикла iced с переданной конфигурацией.
pub fn run(config: AppConfig) -> iced::Result {
    let mut application = iced::application("DocCode", update, view).theme(|state: &App| {
        if state.is_dark_theme() {
            iced::Theme::Dark
        } else {
            iced::Theme::Light
        }
    });

    // Иконка окна вшита в бинарь: packaging/icon/make_icon.py → doccode-128.rgba.
    // Сырые пиксели, а не PNG — чтобы не тянуть декодер изображений в рантайм.
    match iced::window::icon::from_rgba(
        include_bytes!("../../packaging/icon/doccode-128.rgba").to_vec(),
        WINDOW_ICON_SIZE,
        WINDOW_ICON_SIZE,
    ) {
        Ok(icon) => {
            application = application.window(iced::window::Settings {
                icon: Some(icon),
                size: iced::Size::new(1140.0, 700.0),
                min_size: Some(iced::Size::new(980.0, 640.0)),
                position: iced::window::Position::Centered,
                ..iced::window::Settings::default()
            });
        }
        Err(err) => tracing::warn!("Иконка окна не загрузилась: {err}"),
    }

    application.run_with(move || {
        let mut app = App::with_config(config);
        // Загрузка промптов в редактор при старте (с fallback).
        load_prompts_into(&mut app);
        (app, Task::none())
    })
}
