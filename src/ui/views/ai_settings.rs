//! Настройки AI: подключение, модели, параметры генерации и политика D1.

use super::{badge, card, field, page_header, palette, primary, secondary, status_line};
use crate::app::{AiField, App, Message};
use crate::ui::theme::{self, Palette, SEMIBOLD};
use iced::widget::{checkbox, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length};

/// Поле ввода настроек с общим оформлением.
fn input<'a>(
    p: Palette,
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .on_input(on_input)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status))
        .into()
}

/// Содержимое экрана подключения к Ollama и параметров качества.
pub fn view(app: &App) -> Element<'_, Message> {
    let p = palette(app);

    let url_input = input(p, "http://localhost:11434", &app.ai_url, |value| {
        Message::AiFieldChanged(AiField::Url, value)
    });

    let model_input = input(p, "gemma4:e4b", &app.ai_model, |value| {
        Message::AiFieldChanged(AiField::Model, value)
    });

    let embedding_input = input(p, "nomic-embed-text", &app.ai_embedding_model, |value| {
        Message::AiFieldChanged(AiField::EmbeddingModel, value)
    });

    let d1_input = input(p, "d1", &app.ai_d1_model, |value| {
        Message::AiFieldChanged(AiField::D1Model, value)
    });

    let temperature_input = input(p, "0.15", &app.ai_temperature, |value| {
        Message::AiFieldChanged(AiField::Temperature, value)
    });

    let max_tokens_input = input(p, "4000", &app.ai_max_tokens, |value| {
        Message::AiFieldChanged(AiField::MaxTokens, value)
    });

    let thinking = checkbox(app.t("ai.thinking").to_string(), app.ai_think)
        .size(18)
        .text_size(13)
        .spacing(9)
        .style(move |_, status| theme::checkbox(p, status))
        .on_toggle(Message::ToggleAiThinking);

    let factuality_input = input(p, "0.85", &app.ai_d1_min_factuality, |value| {
        Message::AiFieldChanged(AiField::D1Factuality, value)
    });

    let completeness_input = input(p, "3", &app.ai_d1_min_completeness, |value| {
        Message::AiFieldChanged(AiField::D1Completeness, value)
    });

    let retries_input = input(p, "1", &app.ai_d1_max_retries, |value| {
        Message::AiFieldChanged(AiField::D1Retries, value)
    });

    let recommended = container(
        row![
            badge(app, app.t("ai.recommended_badge")),
            column![
                text(app.t("ai.recommended_models"))
                    .size(12)
                    .font(SEMIBOLD)
                    .color(p.text),
                text("gemma4:e4b  ·  d1")
                    .size(13)
                    .font(SEMIBOLD)
                    .color(p.accent),
            ]
            .spacing(2),
            Space::with_width(Length::Fill),
            text(app.t("ai.recommended_hint")).size(11).color(p.muted),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding(12)
    .width(Length::Fill)
    .style(move |_| theme::inset(p));

    let connection = card(
        app,
        column![
            text(app.t("ai.connection_card"))
                .size(13)
                .font(SEMIBOLD)
                .color(p.text),
            field(app, "ai.url", "ai.url_hint", url_input.into()),
        ]
        .spacing(12),
    );

    let models = card(
        app,
        column![
            text(app.t("ai.models_card"))
                .size(13)
                .font(SEMIBOLD)
                .color(p.text),
            field(app, "ai.model", "ai.model_hint", model_input.into()),
            field(
                app,
                "ai.embedding_model",
                "ai.embedding_model_hint",
                embedding_input.into(),
            ),
        ]
        .spacing(12),
    );

    let generation = card(
        app,
        column![
            text(app.t("ai.generation_card"))
                .size(13)
                .font(SEMIBOLD)
                .color(p.text),
            text(app.t("ai.generation_hint")).size(11).color(p.muted),
            row![
                field(
                    app,
                    "ai.temperature",
                    "ai.temperature_hint",
                    temperature_input.into(),
                ),
                field(
                    app,
                    "ai.max_tokens",
                    "ai.max_tokens_hint",
                    max_tokens_input.into(),
                ),
            ]
            .spacing(14),
            row![
                thinking,
                text(app.t("ai.thinking_hint")).size(11).color(p.muted),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ]
        .spacing(10),
    );

    let d1 = card(
        app,
        column![
            text(app.t("ai.d1_card"))
                .size(13)
                .font(SEMIBOLD)
                .color(p.text),
            field(app, "ai.d1_model", "ai.d1_model_hint", d1_input.into()),
            row![
                field(
                    app,
                    "ai.d1_min_factuality",
                    "ai.d1_min_factuality_hint",
                    factuality_input.into(),
                ),
                field(
                    app,
                    "ai.d1_min_completeness",
                    "ai.d1_min_completeness_hint",
                    completeness_input.into(),
                ),
                field(
                    app,
                    "ai.d1_max_retries",
                    "ai.d1_max_retries_hint",
                    retries_input.into(),
                ),
            ]
            .spacing(14),
        ]
        .spacing(10),
    );

    let actions = row![
        primary(app, app.t("common.save"), Message::SaveAi),
        secondary(app, app.t("ai.test"), Message::TestAi),
    ]
    .spacing(9);

    column![
        page_header(app, "ai.title", "ai.hint"),
        recommended,
        row![connection, models].spacing(16),
        row![generation, d1].spacing(16),
        actions,
        status_line(app, app.ai_status.as_deref()),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}
