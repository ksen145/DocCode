//! Экран прогресса: шаги пайплайна, прогресс-бар и живые счётчики.
//!
//! Прогресс стадии строится по реальным счётчикам (файлы, фрагменты,
//! компоненты), рядом идёт таймер с начала запуска. Значения обновляются
//! подпиской на таймер (см. `app::run`).

use super::{card, page_header, palette, status_text};
use crate::app::{format_duration, App, Message, Stage};
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
    let stage_percent = app.stage_percent().clamp(0.0, 100.0);

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
                    text(format!("{}%", stage_percent.round() as u32))
                        .size(13)
                        .font(SEMIBOLD)
                        .color(p.accent),
                ],
                text(app.t(app.stage.label_key()))
                    .size(24)
                    .font(BOLD)
                    .color(p.text),
                progress_bar(0.0..=100.0, stage_percent)
                    .height(9)
                    .width(Length::Fill)
                    .style(move |_| theme::progress(p)),
                text(stage_detail(app)).size(11).color(p.muted),
                Space::with_height(4),
                steps,
                Space::with_height(6),
                stat("progress.elapsed", format_duration(app.elapsed())),
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

/// Подпись под баром стадии: реальные единицы работы или пояснение фазы.
fn stage_detail(app: &App) -> String {
    let snapshot = app.progress_snapshot();
    match app.stage {
        Stage::Scan if snapshot.scan_total > 0 => format!(
            "{}: {} / {}",
            app.t("progress.files"),
            snapshot.scan_done,
            snapshot.scan_total
        ),
        Stage::Scan => app.t("progress.collecting").to_string(),
        Stage::Index if snapshot.index_total > 0 => format!(
            "{}: {} / {}",
            app.t("progress.chunks"),
            snapshot.index_done,
            snapshot.index_total
        ),
        Stage::Index => app.t("progress.preparing_index").to_string(),
        Stage::Generate => format!(
            "{}: {} / {}",
            app.t("progress.components"),
            app.progress_done,
            app.progress_total
        ),
        Stage::Plan => app.t("progress.waiting_model").to_string(),
        Stage::Finalize => app.t("progress.writing").to_string(),
        Stage::Idle | Stage::Done => String::new(),
    }
}
