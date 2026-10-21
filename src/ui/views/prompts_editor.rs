//! Редактор YAML-промптов с валидацией и подсказкой о параметрах генерации.

use super::{badge, card, page_header, palette, primary, secondary, soft, status_line};
use crate::app::{App, Message, Screen};
use crate::ui::theme::{self, SEMIBOLD};
use iced::widget::{column, row, text, text_editor, Space};
use iced::{Alignment, Element, Length};

/// Содержимое экрана редактора промптов.
pub fn view(app: &App) -> Element<'_, Message> {
    let p = palette(app);
    let path = app.config.prompts_path.display().to_string();

    let editor = text_editor(&app.prompts_content)
        .on_action(Message::PromptsAction)
        .padding(12)
        .size(12)
        .height(470)
        .style(move |_, status| theme::editor(p, status));

    column![
        page_header(app, "prompts.title", "prompts.hint"),
        card(
            app,
            column![
                row![
                    badge(app, app.t("prompts.path")),
                    text(path).size(11).color(p.muted),
                    Space::with_width(Length::Fill),
                    secondary(app, app.t("prompts.validate"), Message::PromptsValidate,),
                    secondary(app, app.t("prompts.reset"), Message::PromptsReload),
                    primary(app, app.t("common.save"), Message::PromptsSave),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                editor,
            ]
            .spacing(12),
        ),
        card(
            app,
            row![
                column![
                    text(app.t("prompts.parameters_title"))
                        .size(12)
                        .font(SEMIBOLD)
                        .color(p.text),
                    text(app.t("prompts.parameters_hint"))
                        .size(11)
                        .color(p.muted),
                ]
                .spacing(3),
                Space::with_width(Length::Fill),
                soft(
                    app,
                    app.t("prompts.open_ai"),
                    Message::Navigate(Screen::AiSettings),
                ),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ),
        status_line(app, app.prompts_status.as_deref()),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}
