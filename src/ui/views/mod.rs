//! Экраны приложения и общий визуальный каркас (sidebar + topbar).

pub mod ai_settings;
pub mod progress;
pub mod project_settings;
pub mod prompts_editor;
pub mod results;
pub mod welcome;

use crate::app::{App, Message, Screen, Stage};
use crate::config::ThemeMode;
use crate::ui::i18n::Lang;
use crate::ui::theme::{self, Palette, BOLD, SEMIBOLD};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};

/// Корневой каркас приложения.
pub fn shell<'a>(app: &'a App, content: Element<'a, Message>) -> Element<'a, Message> {
    let p = palette(app);
    let body = container(content).padding([24.0, 30.0]).width(Length::Fill);
    let divider = container(Space::with_height(0))
        .width(Length::Fill)
        .height(1)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(p.border)),
            ..container::Style::default()
        });
    let main = column![
        topbar(app),
        divider,
        scrollable(body).width(Length::Fill).height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    container(row![sidebar(app), main].height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| theme::canvas(p))
        .into()
}

/// Левая панель: бренд, навигация и быстрые переключатели.
fn sidebar<'a>(app: &'a App) -> Element<'a, Message> {
    let p = palette(app);
    let run_started = app.stage != Stage::Idle;
    let has_results =
        app.site_stats.is_some() || !app.written.is_empty() || !app.failures.is_empty();

    let brand = row![
        container(text("DC").size(16).font(BOLD).color(p.on_accent))
            .padding([8.0, 10.0])
            .style(move |_| theme::logo(p)),
        column![
            text(app.t("app.title")).size(20).font(BOLD).color(p.text),
            text(app.t("app.tagline")).size(10).color(p.muted),
        ]
        .spacing(1),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let status_key = if app.project_path.trim().is_empty() && app.config.project_path.is_none() {
        "sidebar.status.no_project"
    } else if app.stage == Stage::Done {
        "sidebar.status.done"
    } else if app.stage != Stage::Idle {
        "sidebar.status.working"
    } else {
        "sidebar.status.ready"
    };
    let status_color = if app.stage == Stage::Done {
        p.success
    } else if app.stage != Stage::Idle {
        p.warning
    } else {
        p.success
    };
    let project = app.project_path.trim().to_string();
    let project = if project.is_empty() {
        app.config
            .project_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| app.t("sidebar.no_project").to_string())
    } else {
        project
    };

    let status = container(
        column![
            row![
                container(Space::with_height(0))
                    .width(8)
                    .height(8)
                    .style(move |_| theme::dot(p, status_color)),
                text(app.t(status_key))
                    .size(11)
                    .font(SEMIBOLD)
                    .color(p.text),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            text(shorten(&project, 30)).size(10).color(p.muted),
        ]
        .spacing(4),
    )
    .padding(10)
    .width(Length::Fill)
    .style(move |_| theme::inset(p));

    let theme_label = match app.config.theme {
        ThemeMode::System => app.t("common.theme_system"),
        ThemeMode::Light => app.t("common.theme_light"),
        ThemeMode::Dark => app.t("common.theme_dark"),
    };
    let lang_label = if app.lang == Lang::Ru { "EN" } else { "RU" };

    container(
        column![
            brand,
            Space::with_height(22),
            section_label(app, "nav.section.setup"),
            nav_item(app, p, Screen::Welcome, "nav.welcome", true),
            nav_item(app, p, Screen::ProjectSettings, "nav.project", true),
            nav_item(app, p, Screen::AiSettings, "nav.ai", true),
            nav_item(app, p, Screen::PromptsEditor, "nav.prompts", true),
            Space::with_height(14),
            section_label(app, "nav.section.run"),
            nav_item(app, p, Screen::Progress, "nav.progress", run_started),
            nav_item(app, p, Screen::Results, "nav.results", has_results),
            Space::with_height(Length::Fill),
            status,
            Space::with_height(10),
            row![
                button(text(theme_label).size(10).color(p.muted))
                    .width(Length::FillPortion(1))
                    .padding([8.0, 0.0])
                    .style(move |_, status| theme::secondary_button(p, status))
                    .on_press(Message::ToggleTheme),
                button(text(lang_label).size(12).font(SEMIBOLD).color(p.muted))
                    .width(Length::FillPortion(1))
                    .padding([8.0, 0.0])
                    .style(move |_, status| theme::secondary_button(p, status))
                    .on_press(Message::ToggleLang),
            ]
            .spacing(6)
            .width(Length::Fill),
            text(format!("v{}", env!("CARGO_PKG_VERSION")))
                .size(9)
                .color(p.muted),
        ]
        .spacing(4)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .padding([20.0, 16.0])
    .width(248)
    .height(Length::Fill)
    .style(move |_| theme::sidebar(p))
    .into()
}

/// Верхняя панель контента.
fn topbar<'a>(app: &'a App) -> Element<'a, Message> {
    let p = palette(app);
    let model_name = if app.ai_model.trim().is_empty() {
        "—".to_string()
    } else {
        shorten(app.ai_model.trim(), 24)
    };
    let d1_name = if app.ai_d1_model.trim().is_empty() {
        "—".to_string()
    } else {
        shorten(app.ai_d1_model.trim(), 16)
    };
    let model = format!("{model_name}  ·  D1: {d1_name}");
    let action: Element<'a, Message> = if *app.screen() == Screen::Progress {
        text(app.t("progress.working"))
            .size(12)
            .color(p.muted)
            .into()
    } else {
        primary(app, app.t("nav.generate"), Message::Generate)
    };

    row![
        column![
            text(app.t("topbar.workspace"))
                .size(9)
                .font(SEMIBOLD)
                .color(p.muted),
            text(app.t(screen_title_key(app.screen())))
                .size(14)
                .font(SEMIBOLD)
                .color(p.text),
        ]
        .spacing(1),
        Space::with_width(Length::Fill),
        container(text(model).size(11).color(p.muted))
            .padding([6.0, 10.0])
            .style(move |_| theme::inset(p)),
        action,
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([14.0, 24.0])
    .width(Length::Fill)
    .into()
}

fn nav_item<'a>(
    app: &'a App,
    p: Palette,
    screen: Screen,
    key: &'static str,
    enabled: bool,
) -> Element<'a, Message> {
    let active = *app.screen() == screen;
    let dot_color = if active { p.accent } else { p.border };
    let content = row![
        container(Space::with_height(0))
            .width(6)
            .height(6)
            .style(move |_| theme::dot(p, dot_color)),
        text(app.t(key)).size(12).font(if active {
            SEMIBOLD
        } else {
            iced::Font::DEFAULT
        }),
    ]
    .spacing(9)
    .align_y(Alignment::Center);
    let item = button(content)
        .width(Length::Fill)
        .padding([9.0, 11.0])
        .style(move |_, status| theme::nav_button(p, active, status));
    if enabled {
        item.on_press(Message::Navigate(screen)).into()
    } else {
        item.into()
    }
}

fn section_label<'a>(app: &'a App, key: &'static str) -> Element<'a, Message> {
    let p = palette(app);
    text(app.t(key).to_uppercase())
        .size(9)
        .font(SEMIBOLD)
        .color(p.muted)
        .into()
}

/// Заголовок страницы с подзаголовком.
pub(crate) fn page_header<'a>(
    app: &'a App,
    title_key: &'static str,
    subtitle_key: &'static str,
) -> Element<'a, Message> {
    let p = palette(app);
    column![
        text(app.t(title_key)).size(28).font(BOLD).color(p.text),
        text(app.t(subtitle_key)).size(13).color(p.muted),
    ]
    .spacing(5)
    .into()
}

/// Карточка контента.
pub(crate) fn card<'a>(
    app: &App,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let p = palette(app);
    container(content)
        .padding(20)
        .width(Length::Fill)
        .style(move |_| theme::card(p))
        .into()
}

/// Акцентная карточка (hero).
pub(crate) fn hero<'a>(
    app: &App,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let p = palette(app);
    container(content)
        .padding(24)
        .width(Length::Fill)
        .style(move |_| theme::hero(p))
        .into()
}

/// Маленький акцентный бейдж.
pub(crate) fn badge<'a>(app: &App, label: impl Into<String>) -> Element<'a, Message> {
    let p = palette(app);
    container(text(label.into()).size(10).font(SEMIBOLD).color(p.accent))
        .padding([4.0, 8.0])
        .style(move |_| theme::badge(p))
        .into()
}

/// Подпись поля.
pub(crate) fn label<'a>(app: &App, key: &'static str) -> Element<'a, Message> {
    let p = palette(app);
    text(app.t(key))
        .size(12)
        .font(SEMIBOLD)
        .color(p.text)
        .into()
}

/// Подсказка поля.
pub(crate) fn hint<'a>(app: &App, key: &'static str) -> Element<'a, Message> {
    let p = palette(app);
    text(app.t(key)).size(11).color(p.muted).into()
}

/// Поле формы: подпись + контрол + подсказка.
pub(crate) fn field<'a>(
    app: &App,
    label_key: &'static str,
    hint_key: &'static str,
    input: Element<'a, Message>,
) -> Element<'a, Message> {
    column![label(app, label_key), input, hint(app, hint_key)]
        .spacing(5)
        .width(Length::Fill)
        .into()
}

/// Основная кнопка.
pub(crate) fn primary<'a>(
    app: &App,
    label: impl Into<String>,
    message: Message,
) -> Element<'a, Message> {
    let p = palette(app);
    button(text(label.into()).size(13).font(SEMIBOLD))
        .padding([9.0, 14.0])
        .style(move |_, status| theme::primary_button(p, status))
        .on_press(message)
        .into()
}

/// Вторичная кнопка.
pub(crate) fn secondary<'a>(
    app: &App,
    label: impl Into<String>,
    message: Message,
) -> Element<'a, Message> {
    let p = palette(app);
    button(text(label.into()).size(12).font(SEMIBOLD))
        .padding([9.0, 13.0])
        .style(move |_, status| theme::secondary_button(p, status))
        .on_press(message)
        .into()
}

/// Мягкая акцентная кнопка.
pub(crate) fn soft<'a>(
    app: &App,
    label: impl Into<String>,
    message: Message,
) -> Element<'a, Message> {
    let p = palette(app);
    button(text(label.into()).size(12).font(SEMIBOLD))
        .padding([9.0, 13.0])
        .style(move |_, status| theme::soft_button(p, status))
        .on_press(message)
        .into()
}

/// Карточка статистики.
pub(crate) fn stat<'a>(
    app: &App,
    label_key: &'static str,
    value: impl Into<String>,
    accent: bool,
) -> Element<'a, Message> {
    let p = palette(app);
    let value_color = if accent { p.accent } else { p.text };
    container(
        column![
            text(app.t(label_key))
                .size(10)
                .font(SEMIBOLD)
                .color(p.muted),
            text(value.into()).size(24).font(BOLD).color(value_color),
        ]
        .spacing(3),
    )
    .padding(16)
    .width(Length::FillPortion(1))
    .style(move |_| theme::card(p))
    .into()
}

/// Строка статуса (опционально).
pub(crate) fn status_line<'a>(app: &App, status: Option<&'a str>) -> Element<'a, Message> {
    match status {
        Some(value) => status_text(app, value.to_string()),
        None => Space::with_height(0).into(),
    }
}

/// Строка статуса с собственным текстом.
pub(crate) fn status_text<'a>(app: &App, value: impl Into<String>) -> Element<'a, Message> {
    let p = palette(app);
    container(
        row![
            container(Space::with_height(0))
                .width(8)
                .height(8)
                .style(move |_| theme::dot(p, p.accent)),
            text(value.into()).size(12).color(p.text),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([9.0, 11.0])
    .width(Length::Fill)
    .style(move |_| theme::inset(p))
    .into()
}

pub(crate) fn palette(app: &App) -> Palette {
    theme::Palette::new(app.is_dark_theme())
}

fn screen_title_key(screen: &Screen) -> &'static str {
    match screen {
        Screen::Welcome => "nav.welcome",
        Screen::ProjectSettings => "nav.project",
        Screen::AiSettings => "nav.ai",
        Screen::PromptsEditor => "nav.prompts",
        Screen::Progress => "nav.progress",
        Screen::Results => "nav.results",
    }
}

/// Обрезает длинный путь для боковой панели.
fn shorten(value: &str, limit: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= limit {
        return value.to_string();
    }
    let start: String = chars[..limit.saturating_sub(1)].iter().collect();
    format!("{start}…")
}
