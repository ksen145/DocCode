//! Экран-обзор: быстрый старт, текущие настройки и рекомендованные модели.

use super::{badge, card, hero, palette, secondary, status_line};
use crate::app::{App, Message, Screen};
use crate::ui::theme::{self, BOLD, SEMIBOLD};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};

/// Содержимое экрана приветствия.
pub fn view(app: &App) -> Element<'_, Message> {
    let p = palette(app);
    let project = app.project_path.trim().to_string();
    let project = if project.is_empty() {
        app.config
            .project_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| app.t("welcome.not_set").to_string())
    } else {
        project
    };
    let d1_state = if app.config.verify_d1 {
        app.t("common.enabled")
    } else {
        app.t("common.disabled")
    };

    let start = button(
        text(app.t("welcome.start"))
            .size(14)
            .font(SEMIBOLD)
            .color(p.on_accent),
    )
    .padding([11.0, 18.0])
    .style(move |_, status| theme::primary_button(p, status))
    .on_press(Message::Generate);

    let recommended = column![
        text(app.t("welcome.recommended"))
            .size(10)
            .font(SEMIBOLD)
            .color(p.accent),
        Space::with_height(8),
        text("gemma4:e4b").size(20).font(BOLD).color(p.text),
        text(app.t("welcome.recommended_generation"))
            .size(10)
            .color(p.muted),
        Space::with_height(12),
        text("d1").size(20).font(BOLD).color(p.text),
        text(app.t("welcome.recommended_quality"))
            .size(10)
            .color(p.muted),
    ]
    .spacing(2)
    .width(Length::Fill);

    let setup_card = column![
        text(app.t("welcome.project"))
            .size(10)
            .font(SEMIBOLD)
            .color(p.muted),
        text(shorten(&project, 34))
            .size(14)
            .font(SEMIBOLD)
            .color(p.text),
        text(format!(
            "{}: {}",
            app.t("welcome.model"),
            short(&app.ai_model, 22)
        ))
        .size(10)
        .color(p.muted),
    ]
    .spacing(3);
    let ai_card = column![
        text(app.t("welcome.ai"))
            .size(10)
            .font(SEMIBOLD)
            .color(p.muted),
        text(short(&app.ai_model, 26))
            .size(14)
            .font(SEMIBOLD)
            .color(p.text),
        text(format!("D1: {}", short(&app.ai_d1_model, 22)))
            .size(10)
            .color(p.muted),
    ]
    .spacing(3);
    let quality_card = column![
        text(app.t("welcome.quality"))
            .size(10)
            .font(SEMIBOLD)
            .color(p.muted),
        text(d1_state).size(14).font(SEMIBOLD).color(p.text),
        text(format!(
            "≥ {:.2} / ≥ {}/5",
            app.config.d1.min_factuality, app.config.d1.min_completeness
        ))
        .size(10)
        .color(p.muted),
    ]
    .spacing(3);

    let info = row![
        container(setup_card)
            .width(Length::FillPortion(1))
            .style(move |_| theme::inset(p)),
        container(ai_card)
            .width(Length::FillPortion(1))
            .style(move |_| theme::inset(p)),
        container(quality_card)
            .width(Length::FillPortion(1))
            .style(move |_| theme::inset(p)),
    ]
    .spacing(12);

    let step = |number: &'static str, title: &'static str, body: &'static str| {
        column![
            text(number).size(10).font(SEMIBOLD).color(p.accent),
            text(app.t(title)).size(13).font(SEMIBOLD).color(p.text),
            text(app.t(body)).size(11).color(p.muted),
        ]
        .spacing(3)
    };

    column![
        hero(
            app,
            row![
                column![
                    badge(app, app.t("welcome.eyebrow")),
                    Space::with_height(6),
                    text(app.t("welcome.title"))
                        .size(34)
                        .font(BOLD)
                        .color(p.text),
                    text(app.t("welcome.subtitle")).size(13).color(p.muted),
                    Space::with_height(6),
                    row![
                        start,
                        secondary(
                            app,
                            app.t("welcome.configure"),
                            Message::Navigate(Screen::ProjectSettings)
                        ),
                    ]
                    .spacing(9),
                ]
                .spacing(6)
                .width(Length::FillPortion(2)),
                container(recommended)
                    .padding(16)
                    .width(Length::FillPortion(1))
                    .style(move |_| theme::inset(p)),
            ]
            .spacing(22)
            .align_y(Alignment::Center),
        ),
        status_line(app, app.status.as_deref()),
        info,
        card(
            app,
            column![
                text(app.t("welcome.how"))
                    .size(14)
                    .font(SEMIBOLD)
                    .color(p.text),
                Space::with_height(4),
                row![
                    step("01", "welcome.step1.title", "welcome.step1.body"),
                    step("02", "welcome.step2.title", "welcome.step2.body"),
                    step("03", "welcome.step3.title", "welcome.step3.body"),
                ]
                .spacing(18),
            ]
            .spacing(10),
        ),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

fn short(value: &str, limit: usize) -> String {
    let value = value.trim();
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= limit {
        return value.to_string();
    }
    let mut out: String = chars[..limit.saturating_sub(1)].iter().collect();
    out.push('…');
    out
}

fn shorten(value: &str, limit: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= limit {
        return value.to_string();
    }
    let start: String = chars[..limit.saturating_sub(1)].iter().collect();
    format!("{start}…")
}
