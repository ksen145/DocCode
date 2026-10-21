//! Клиент для Ollama API (http://localhost:11434).

use crate::ai::client::{strip_reasoning, AiClient, GenerateOptions, GenerationResult, Usage};
use crate::ai::d1::{D1Request, D1Response};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

/// Таймаут соединения с Ollama (сервер может грузить модель).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Таймаут одного запроса генерации/эмбеддинга.
///
/// Защищает от зависшей модели: без него `reqwest` ждёт вечно (проверено
/// на живой qwen3.5:4b — один вызов «висел» без ответа десятки минут).
/// 10 минут — с запасом на загрузку модели и длинные промпты на слабом железе.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);

/// Ollama-клиент.
pub struct OllamaClient {
    /// Базовый URL Ollama (по умолчанию http://localhost:11434).
    url: String,
    /// Имя модели для генерации (например, qwen2.5-coder).
    model: String,
    /// Имя модели для эмбеддингов (например, nomic-embed-text).
    embed_model: String,
    /// Имя модели D1 для контроля качества документации.
    d1_model: String,
    /// Переиспользуемый HTTP-клиент (соединения живут между запросами).
    http: reqwest::Client,
}

impl OllamaClient {
    pub fn new(url: impl Into<String>, model: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("reqwest-клиент с таймаутами создаётся");
        Self {
            url: url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            embed_model: "nomic-embed-text".to_string(),
            d1_model: "d1".to_string(),
            http,
        }
    }

    /// Конструктор для локального Ollama с моделью по умолчанию.
    pub fn local() -> Self {
        Self::new("http://localhost:11434", "qwen3.5:4b")
    }

    /// Указать модель для эмбеддингов.
    pub fn with_embed_model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        if !model.trim().is_empty() {
            self.embed_model = model;
        }
        self
    }

    /// Указать модель D1 для проверки документации.
    pub fn with_d1_model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        if !model.trim().is_empty() {
            self.d1_model = model;
        }
        self
    }

    /// Тело запроса генерации: параметры уходят в блок `options` Ollama
    /// (`max_tokens` → `num_predict` — так Ollama называет лимит ответа).
    fn generate_body(&self, prompt: &str, options: &GenerateOptions) -> serde_json::Value {
        let mut opts = serde_json::Map::new();
        if let Some(temperature) = options.temperature {
            opts.insert("temperature".to_string(), serde_json::json!(temperature));
        }
        if let Some(max_tokens) = options.max_tokens {
            opts.insert("num_predict".to_string(), serde_json::json!(max_tokens));
        }

        serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            // Не просим reasoning: для документации нужно только готовое описание.
            // Кастомные модели могут игнорировать флаг — остаток вычищается ниже.
            "think": false,
            "options": serde_json::Value::Object(opts),
        })
    }

    async fn post_json(
        &self,
        endpoint: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let response = self
            .http
            .post(format!("{}{}", self.url, endpoint))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP-ошибка Ollama: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Ollama вернул {}: {}", status, text));
        }

        response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Не удалось разобрать ответ Ollama: {}", e))
    }
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    #[serde(default)]
    response: String,
    #[serde(default)]
    prompt_eval_count: u32,
    #[serde(default)]
    eval_count: u32,
}

#[derive(Debug, Deserialize)]
struct EmbeddingsResponse {
    #[serde(default)]
    embedding: Vec<f32>,
}

#[async_trait]
impl AiClient for OllamaClient {
    async fn generate(
        &self,
        prompt: &str,
        options: &GenerateOptions,
    ) -> Result<GenerationResult, String> {
        let body = self.generate_body(prompt, options);

        let value = self.post_json("/api/generate", body).await?;
        let parsed: GenerateResponse =
            serde_json::from_value(value).map_err(|e| format!("Схема ответа generate: {}", e))?;

        Ok(GenerationResult {
            content: strip_reasoning(&parsed.response),
            usage: Some(Usage {
                prompt_tokens: parsed.prompt_eval_count,
                completion_tokens: parsed.eval_count,
            }),
        })
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let body = serde_json::json!({
            "model": self.embed_model,
            "prompt": text,
        });

        let value = self.post_json("/api/embeddings", body).await?;
        let parsed: EmbeddingsResponse =
            serde_json::from_value(value).map_err(|e| format!("Схема ответа embeddings: {}", e))?;

        if parsed.embedding.is_empty() {
            return Err("Ollama вернул пустой эмбеддинг".to_string());
        }
        Ok(parsed.embedding)
    }

    async fn verify(&self, request: &D1Request) -> Result<D1Response, String> {
        // D1 живёт на отдельном эндпоинте Ollama; модель можно переопределить
        // в конфигурации, но по умолчанию используется `d1`.
        let mut body = request.clone();
        if body.model.trim().is_empty() {
            body.model = self.d1_model.clone();
        }

        let value = self
            .post_json(
                "/v1/systemone",
                serde_json::to_value(body).map_err(|e| e.to_string())?,
            )
            .await?;
        serde_json::from_value(value).map_err(|e| format!("Схема ответа D1: {}", e))
    }

    async fn ping(&self) -> Result<(), String> {
        let response = self
            .http
            .get(format!("{}/api/tags", self.url))
            .send()
            .await
            .map_err(|e| format!("Ollama недоступна: {}", e))?;

        if !response.status().is_success() {
            return Err(response.text().await.unwrap_or_default());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_body_maps_defaults_to_ollama_options() {
        let client = OllamaClient::new("http://localhost:11434", "test-model");
        let body = client.generate_body("привет", &GenerateOptions::new(0.2, 800));

        assert_eq!(body["model"], "test-model");
        assert_eq!(body["prompt"], "привет");
        assert_eq!(body["stream"], false);
        assert_eq!(body["think"], false);
        let temperature = body["options"]["temperature"]
            .as_f64()
            .expect("temperature — число");
        assert!(
            (temperature - 0.2).abs() < 1e-6,
            "temperature={temperature}"
        );
        assert_eq!(body["options"]["num_predict"], 800);
    }

    #[test]
    fn generate_body_keeps_backend_defaults_when_options_absent() {
        let client = OllamaClient::local();
        let body = client.generate_body("prompt", &GenerateOptions::default());

        let opts = body["options"].as_object().expect("options — объект");
        assert!(
            opts.is_empty(),
            "пустые options не должны добавлять параметров"
        );
    }

    #[test]
    fn url_trailing_slash_is_trimmed() {
        let client = OllamaClient::new("http://localhost:11434/", "m");
        assert_eq!(client.url, "http://localhost:11434");
    }
}
