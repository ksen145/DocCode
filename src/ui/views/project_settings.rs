//! Настройки проекта: пути, папка документации и включение D1.

use super::{card, field, page_header, palette, primary, secondary, status_line};
use crate::app::{App, Message, Screen};
use crate::ui::theme::{self, SEMIBOLD};
use iced::widget::{checkbox, column, row, text, text_input};
use iced::{Element, Length};

/// Содержимое экрана настроек проекта.
pub fn view(app: &App) -> Element<'_, Message> {
    let p = palette(app);

    let path_input = text_input("D:/work/my-project", &app.project_path)
        .on_input(Message::ProjectPathChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

    let docs_input = text_input("docs/", &app.docs_path)
        .on_input(Message::DocsPathChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

    let path_picker = row![
        path_input,
        secondary(app, app.t("project.browse"), Message::PickProjectFolder,),
    ]
    .spacing(8)
    .width(Length::Fill)
    .align_y(iced::Alignment::Center);

    let docs_picker = row![
        docs_input,
        secondary(app, app.t("project.browse"), Message::PickDocsFolder),
    ]
    .spacing(8)
    .width(Length::Fill)
    .align_y(iced::Alignment::Center);

    let paths = row![
        field(app, "project.path", "project.path_hint", path_picker.into()),
        field(app, "project.docs", "project.docs_hint", docs_picker.into()),
    ]
    .spacing(16);

    let d1 = checkbox(
        app.t("project.d1_verification").to_string(),
        app.config.verify_d1,
    )
    .size(18)
    .text_size(13)
    .spacing(9)
    .style(move |_, status| theme::checkbox(p, status))
    .on_toggle(Message::ToggleD1Verification);

    let thresholds = format!(
        "{}: ≥ {:.2}   ·   {}: ≥ {}/5   ·   {}: {}",
        app.t("project.factuality_short"),
        app.config.d1.min_factuality,
        app.t("project.completeness_short"),
        app.config.d1.min_completeness,
        app.t("project.retries_short"),
        app.config.d1.max_retries
    );

    let actions = row![
        primary(app, app.t("common.save"), Message::SaveProject),
        secondary(
            app,
            app.t("project.open_ai"),
            Message::Navigate(Screen::AiSettings),
        ),
    ]
    .spacing(9);

    column![
        page_header(app, "project.title", "project.hint"),
        card(
            app,
            column![
                text(app.t("project.paths_card"))
                    .size(13)
                    .font(SEMIBOLD)
                    .color(p.text),
                paths,
            ]
            .spacing(12),
        ),
        card(
            app,
            column![
                text(app.t("project.quality_card"))
                    .size(13)
                    .font(SEMIBOLD)
                    .color(p.text),
                row![
                    d1,
                    text(app.t("project.d1_verification_hint"))
                        .size(11)
                        .color(p.muted),
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center),
                text(thresholds).size(11).color(p.muted),
                secondary(
                    app,
                    app.t("project.edit_d1"),
                    Message::Navigate(Screen::AiSettings),
                ),
            ]
            .spacing(9),
        ),
        actions,
        status_line(app, app.project_status.as_deref()),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}
