//! Trait AI-клиента (генерация текста, эмбеддинги, ping).

use crate::ai::d1::{D1Request, D1Response};
use async_trait::async_trait;
use serde::Deserialize;

/// Теги, в которые локальные модели иногда оборачивают цепочку рассуждений.
const REASONING_TAGS: [(&str, &str); 4] = [
    ("<thinking>", "</thinking>"),
    ("<think>", "</think>"),
    ("<reasoning>", "</reasoning>"),
    ("<thought>", "</thought>"),
];

/// Удаляет из ответа модели блоки рассуждений.
///
/// Некоторые модели (в частности, кастомные Qwen) игнорируют `think: false`
/// и возвращают reasoning прямо в поле `response`. Если оставить его как есть,
/// текст попадает в Markdown и затем в итоговый HTML. Функция также понимает
/// случай, когда открывающий ` thinking` добавлен шаблоном Ollama: тогда в ответе
/// остаётся только закрывающий тег, а всё до него — рассуждения.
pub fn strip_reasoning(text: &str) -> String {
    let without_blocks = remove_reasoning_blocks(text);
    remove_leaked_reasoning_prefix(&without_blocks)
        .trim()
        .to_string()
}

/// Удаляет парные блоки `<think>…</think>` (и аналоги) вне fenced-кода.
fn remove_reasoning_blocks(text: &str) -> String {
    let fences = fenced_ranges(text);
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;

    while let Some((open_start, open_end, close_tag)) = find_opening(&lower, cursor, &fences) {
        // Закрывающий тег ищем без учёта fence: внутри рассуждений модель
        // может использовать Markdown-код, и ``` не должен прятать `</think>`.
        let Some(close_start) = lower[open_end..].find(close_tag).map(|p| open_end + p) else {
            // Блок не закрыт — всё после начала рассуждений не является ответом.
            out.push_str(&text[cursor..open_start]);
            return out;
        };

        out.push_str(&text[cursor..open_start]);
        cursor = close_start + close_tag.len();
    }

    out.push_str(&text[cursor..]);
    out
}

/// Удаляет «хвост» рассуждений, если открывающий тег был в промпте/шаблоне.
fn remove_leaked_reasoning_prefix(text: &str) -> String {
    let mut result = text.to_string();

    loop {
        let fences = fenced_ranges(&result);
        let lower = result.to_ascii_lowercase();
        let Some((close_start, close_end)) = find_earliest_closing(&lower, &fences) else {
            break;
        };

        let before_has_heading = first_h1_start(&result[..close_start], &fences).is_some();
        let after_has_heading = {
            let after = &result[close_end..];
            let after_fences = fenced_ranges(after);
            first_h1_start(after, &after_fences).is_some()
        };

        if after_has_heading || !before_has_heading {
            // Всё до закрывающего тега — рассуждения; ответ начинается после.
            result = result[close_end..].to_string();
        } else {
            // Ответ уже начался до тега — убираем только сам тег.
            result.replace_range(close_start..close_end, "");
        }
    }

    result
}

fn find_opening(
    lower: &str,
    from: usize,
    fences: &[(usize, usize)],
) -> Option<(usize, usize, &'static str)> {
    REASONING_TAGS
        .iter()
        .filter_map(|(open, close)| {
            find_outside(lower, open, from, fences).map(|start| (start, start + open.len(), *close))
        })
        .min_by_key(|(start, _, _)| *start)
}

fn find_earliest_closing(lower: &str, fences: &[(usize, usize)]) -> Option<(usize, usize)> {
    REASONING_TAGS
        .iter()
        .filter_map(|(_, close)| {
            find_outside(lower, close, 0, fences).map(|start| (start, start + close.len()))
        })
        .min_by_key(|(start, _)| *start)
}

fn find_outside(text: &str, needle: &str, from: usize, fences: &[(usize, usize)]) -> Option<usize> {
    let mut cursor = from;
    while let Some(relative) = text[cursor..].find(needle) {
        let position = cursor + relative;
        let inside_fence = fences
            .iter()
            .any(|(start, end)| position >= *start && position < *end);
        if !inside_fence {
            return Some(position);
        }
        cursor = position + needle.len();
    }
    None
}

/// Диапазоны fenced-кода (включая строки с открывающими/закрывающими ```).
fn fenced_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut fence_start = None;
    let mut offset = 0usize;

    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with("```") {
            match fence_start {
                Some(start) => {
                    ranges.push((start, offset + line.len()));
                    fence_start = None;
                }
                None => fence_start = Some(offset),
            }
        }
        offset += line.len();
    }

    if let Some(start) = fence_start {
        ranges.push((start, text.len()));
    }
    ranges
}

/// Позиция первого заголовка первого уровня (`# `) вне fenced-кода.
fn first_h1_start(text: &str, fences: &[(usize, usize)]) -> Option<usize> {
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let indentation = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        let inside_fence = fences
            .iter()
            .any(|(start, end)| offset >= *start && offset < *end);
        if !inside_fence && trimmed.starts_with("# ") {
            return Some(offset + indentation);
        }
        offset += line.len();
    }
    None
}

/// Результат генерации текста.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerationResult {
    /// Сгенерированный текст.
    pub content: String,
    /// Статистика использования (если бэкенд её отдаёт).
    pub usage: Option<Usage>,
}

/// Статистика токенов.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Параметры генерации (из настроек приложения → запрос к LLM).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GenerateOptions {
    /// Температура сэмплирования (0.0–2.0); `None` — значение бэкенда.
    pub temperature: Option<f32>,
    /// Ограничение длины ответа в токенах; `None` — значение бэкенда.
    pub max_tokens: Option<u32>,
    /// Разрешить reasoning (`think` в Ollama); `None` — значение модели.
    pub think: Option<bool>,
}

impl GenerateOptions {
    /// Параметры сэмплирования; `think` остаётся на усмотрение модели.
    pub fn new(temperature: f32, max_tokens: u32) -> Self {
        Self {
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
            think: None,
        }
    }
}

/// Trait AI-клиента (объектно-безопасный — можно `Box<dyn AiClient>`).
#[async_trait]
pub trait AiClient: Send + Sync {
    /// Генерация текста по промпту с параметрами (temperature/max_tokens).
    async fn generate(
        &self,
        prompt: &str,
        options: &GenerateOptions,
    ) -> Result<GenerationResult, String>;

    /// Получение эмбеддинга текста (по умолчанию — не поддерживается).
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let _ = text;
        Err("Эмбеддинги не поддерживаются этим бэкендом".to_string())
    }

    /// Проверка сгенерированного текста моделью D1 (Ollama `/v1/systemone`).
    ///
    /// По умолчанию не поддерживается: документация всё равно генерируется,
    /// но помечается как непроверенная.
    async fn verify(&self, request: &D1Request) -> Result<D1Response, String> {
        let _ = request;
        Err("Модель D1 не поддерживается этим бэкендом".to_string())
    }

    /// Проверка доступности сервиса.
    async fn ping(&self) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::strip_reasoning;

    #[test]
    fn removes_complete_thinking_block() {
        let text = "<think>план и рассуждения</think>\n\n# Monitoring\n\nТекст.";
        assert_eq!(strip_reasoning(text), "# Monitoring\n\nТекст.");
    }

    #[test]
    fn removes_prefilled_reasoning_before_answer_heading() {
        let text = "Пользователь просит описать компонент Monitoring.\n\
                    Нужно следовать структуре.\n\n</think>\n\n# Monitoring\n\nТекст.";
        assert_eq!(strip_reasoning(text), "# Monitoring\n\nТекст.");
    }

    #[test]
    fn removes_reasoning_with_code_fences_inside() {
        let text = "<think>\n```rust\nfn main() {}\n```\n</think>\n# Monitoring\n\nТекст.";
        assert_eq!(strip_reasoning(text), "# Monitoring\n\nТекст.");
    }

    #[test]
    fn keeps_thinking_tags_inside_documented_code() {
        let text = "```html\n<think>пример</think>\n```\n\n# Monitoring\n\nТекст.";
        assert_eq!(strip_reasoning(text), text);
    }

    #[test]
    fn drops_unclosed_reasoning_block() {
        let text = "Начало ответа.\n<think>модель не успела закончить";
        assert_eq!(strip_reasoning(text), "Начало ответа.");
    }

    #[test]
    fn leaves_plain_markdown_untouched() {
        let text = "# Monitoring\n\n## Назначение\n\nТекст.";
        assert_eq!(strip_reasoning(text), text);
    }
}
