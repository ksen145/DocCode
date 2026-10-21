//! AI-клиент для локальной Ollama.

pub mod client;
pub mod d1;
pub mod ollama;

use crate::config::AiBackendConfig;
use client::AiClient;
use ollama::OllamaClient;

/// Создание клиента Ollama по конфигурации.
pub fn make_client(config: &AiBackendConfig) -> Box<dyn AiClient> {
    let url = config
        .base_url
        .clone()
        .unwrap_or_else(|| "http://localhost:11434".to_string());
    Box::new(
        OllamaClient::new(url, config.model.clone())
            .with_embed_model(config.embedding_model.clone())
            .with_d1_model(config.d1_model.clone()),
    )
}
