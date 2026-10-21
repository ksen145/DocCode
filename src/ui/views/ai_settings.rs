//! Настройки AI: подключение, модели, параметры генерации и политика D1.

use super::{badge, card, field, page_header, palette, primary, secondary, status_line};
use crate::app::{App, Message};
use crate::ui::theme::{self, SEMIBOLD};
use iced::widget::{column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length};

/// Содержимое экрана подключения к Ollama и параметров качества.
pub fn view(app: &App) -> Element<'_, Message> {
    let p = palette(app);

    let url_input = text_input("http://localhost:11434", &app.ai_url)
        .on_input(Message::AiUrlChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

    let model_input = text_input("gemma4:e4b", &app.ai_model)
        .on_input(Message::AiModelChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

    let embedding_input = text_input("nomic-embed-text", &app.ai_embedding_model)
        .on_input(Message::AiEmbeddingModelChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

    let d1_input = text_input("d1", &app.ai_d1_model)
        .on_input(Message::AiD1ModelChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

    let temperature_input = text_input("0.15", &app.ai_temperature)
        .on_input(Message::AiTemperatureChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

    let max_tokens_input = text_input("4000", &app.ai_max_tokens)
        .on_input(Message::AiMaxTokensChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

    let factuality_input = text_input("0.85", &app.ai_d1_min_factuality)
        .on_input(Message::AiD1FactualityChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

    let completeness_input = text_input("3", &app.ai_d1_min_completeness)
        .on_input(Message::AiD1CompletenessChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

    let retries_input = text_input("1", &app.ai_d1_max_retries)
        .on_input(Message::AiD1RetriesChanged)
        .padding([10.0, 11.0])
        .size(13)
        .width(Length::Fill)
        .style(move |_, status| theme::input(p, status));

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
