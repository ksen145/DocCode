//! Экран результатов: итог сборки, статистика, D1 и ошибки.

use super::{card, hero, page_header, palette, primary, secondary, stat, status_line};
use crate::app::{App, Message};
use crate::ui::theme::{self, BOLD, SEMIBOLD};
use iced::widget::{column, container, row, text, Space};
use iced::{Alignment, Element, Length};

/// Содержимое экрана результатов.
pub fn view(app: &App) -> Element<'_, Message> {
    let p = palette(app);
    let stats = app.site_stats.as_ref();
    let components = stats.map(|stats| stats.components).unwrap_or(0);
    let verified = stats.map(|stats| stats.verified).unwrap_or(0);
    let review = stats.map(|stats| stats.needs_review).unwrap_or(0);
    let unverified = stats.map(|stats| stats.unverified).unwrap_or(0);
    let failed = stats.map(|stats| stats.failed).unwrap_or(0);

    let success: Element<'_, Message> = if !app.written.is_empty() {
        let path = app
            .written
            .first()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        hero(
            app,
            row![
                container(text("OK").size(16).font(BOLD).color(p.success))
                    .padding([12.0, 14.0])
                    .style(move |_| theme::inset(p)),
                column![
                    text(app.t("results.success_title"))
                        .size(22)
                        .font(BOLD)
                        .color(p.text),
                    text(app.t("results.success_body")).size(12).color(p.muted),
                    text(path).size(11).color(p.muted),
                    Space::with_height(5),
                    row![
                        primary(app, app.t("results.open"), Message::OpenSite),
                        secondary(app, app.t("results.again"), Message::Generate),
                        secondary(app, app.t("results.back"), Message::BackToWelcome),
                    ]
                    .spacing(9),
                ]
                .spacing(3),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
        )
    } else if app.failures.is_empty() {
        card(
            app,
            column![
                text(app.t("results.empty_title"))
                    .size(20)
                    .font(BOLD)
                    .color(p.text),
                text(app.t("results.empty")).size(12).color(p.muted),
                secondary(app, app.t("results.back"), Message::BackToWelcome),
            ]
            .spacing(8),
        )
    } else {
        card(
            app,
            column![
                text(app.t("results.error_title"))
                    .size(20)
                    .font(BOLD)
                    .color(p.danger),
                text(app.t("results.error_body")).size(12).color(p.muted),
                secondary(app, app.t("results.again"), Message::Generate),
                secondary(app, app.t("results.back"), Message::BackToWelcome),
            ]
            .spacing(8),
        )
    };

    let mut files = column![].spacing(4);
    for path in &app.written {
        files = files.push(text(path.display().to_string()).size(11).color(p.text));
    }
    let files_block: Element<'_, Message> = if app.written.is_empty() {
        text(app.t("results.empty")).size(11).color(p.muted).into()
    } else {
        files.into()
    };

    let mut errors = column![].spacing(4);
    for failure in &app.failures {
        errors = errors.push(text(format!("• {failure}")).size(11).color(p.danger));
    }
    let errors_block: Element<'_, Message> = if app.failures.is_empty() {
        text("—").size(12).color(p.muted).into()
    } else {
        errors.into()
    };

    let quality = row![
        stat(app, "results.components", components.to_string(), false),
        stat(app, "results.verified", verified.to_string(), true),
        stat(app, "results.needs_review", review.to_string(), false),
        stat(app, "results.failures", failed.to_string(), false),
    ]
    .spacing(12);

    let mut content = column![
        page_header(app, "results.title", "results.subtitle"),
        success,
        row![
            stat(app, "results.scanned", app.files_scanned.to_string(), false),
            stat(app, "results.indexed", app.index_chunks.to_string(), false),
            stat(app, "results.written", app.written.len().to_string(), false),
        ]
        .spacing(12),
        quality,
    ]
    .spacing(16)
    .width(Length::Fill);

    content = content.push(card(
        app,
        column![
            text(app.t("results.files"))
                .size(13)
                .font(SEMIBOLD)
                .color(p.text),
            files_block,
        ]
        .spacing(7),
    ));

    content = content.push(card(
        app,
        column![
            row![
                text(app.t("results.d1_card"))
                    .size(13)
                    .font(SEMIBOLD)
                    .color(p.text),
                Space::with_width(Length::Fill),
                text(format!(
                    "{}: {}  ·  {}: {}",
                    app.t("results.verified"),
                    verified,
                    app.t("results.unverified"),
                    unverified
                ))
                .size(10)
                .color(p.muted),
            ],
            text(format!(
                "{}: ≥ {:.2}   ·   {}: ≥ {}/5   ·   {}: {}",
                app.t("project.factuality_short"),
                app.config.d1.min_factuality,
                app.t("project.completeness_short"),
                app.config.d1.min_completeness,
                app.t("project.retries_short"),
                app.config.d1.max_retries
            ))
            .size(11)
            .color(p.muted),
            text(format!(
                "{}: {}   ·   {}: {}",
                app.t("results.verified"),
                verified,
                app.t("results.needs_review"),
                review
            ))
            .size(11)
            .color(p.text),
        ]
        .spacing(7),
    ));

    if !app.failures.is_empty() {
        content = content.push(card(
            app,
            column![
                text(app.t("results.errors"))
                    .size(13)
                    .font(SEMIBOLD)
                    .color(p.danger),
                errors_block,
            ]
            .spacing(7),
        ));
    }

    content.push(status_line(app, app.status.as_deref())).into()
}
