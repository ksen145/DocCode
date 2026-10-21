//! Экран прогресса: шаги пайплайна, прогресс-бар и живые счётчики.

use super::{card, page_header, palette, status_text};
use crate::app::{App, Message, Stage};
use crate::ui::theme::{self, BOLD, SEMIBOLD};
use iced::widget::{column, container, progress_bar, row, text, Space};
use iced::{Element, Length};

const STEPS: [Stage; 5] = [
    Stage::Scan,
    Stage::Index,
    Stage::Plan,
    Stage::Generate,
    Stage::Finalize,
];

/// Содержимое экрана прогресса.
pub fn view(app: &App) -> Element<'_, Message> {
    let p = palette(app);
    let total = app.progress_total.max(1) as f32;
    let done = (app.progress_done as f32).min(total);
    let percent = (done / total * 100.0).round() as u32;

    let mut steps = row![].spacing(7).width(Length::Fill);
    for (index, stage) in STEPS.iter().enumerate() {
        let active = app.stage == *stage;
        let done_step = app.stage == Stage::Done
            || STEPS
                .iter()
                .position(|candidate| *candidate == app.stage)
                .map(|current| current > index)
                .unwrap_or(false);
        let chip = container(
            column![
                text(format!("{:02}", index + 1)).size(9).font(SEMIBOLD),
                text(app.t(stage.label_key())).size(10),
            ]
            .spacing(2),
        )
        .padding([8.0, 9.0])
        .width(Length::FillPortion(1))
        .style(move |_| theme::step(p, active, done_step));
        steps = steps.push(chip);
    }

    let stat = |label: &'static str, value: String| {
        container(
            column![
                text(app.t(label)).size(9).font(SEMIBOLD).color(p.muted),
                text(value).size(18).font(BOLD).color(p.text),
            ]
            .spacing(2),
        )
        .padding(10)
        .width(Length::FillPortion(1))
        .style(move |_| theme::inset(p))
    };

    column![
        page_header(app, "progress.title", "progress.subtitle"),
        card(
            app,
            column![
                row![
                    text(app.t("progress.current"))
                        .size(10)
                        .font(SEMIBOLD)
                        .color(p.muted),
                    Space::with_width(Length::Fill),
                    text(format!("{percent}%"))
                        .size(13)
                        .font(SEMIBOLD)
                        .color(p.accent),
                ],
                text(app.t(app.stage.label_key()))
                    .size(24)
                    .font(BOLD)
                    .color(p.text),
                progress_bar(0.0..=total, done)
                    .height(9)
                    .width(Length::Fill)
                    .style(move |_| theme::progress(p)),
                text(format!(
                    "{}: {} / {}",
                    app.t("progress.working"),
                    app.progress_done,
                    app.progress_total
                ))
                .size(11)
                .color(p.muted),
                Space::with_height(4),
                steps,
            ]
            .spacing(11),
        ),
        row![
            stat("results.scanned", app.files_scanned.to_string()),
            stat("results.indexed", app.index_chunks.to_string()),
            stat(
                "results.components",
                app.site_stats
                    .as_ref()
                    .map(|stats| stats.components)
                    .unwrap_or(0)
                    .to_string(),
            ),
        ]
        .spacing(12),
        status_text(app, app.t("progress.hint")),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}
