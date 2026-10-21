//! Контракт с моделью D1 (Ollama `/v1/systemone`) для контроля качества
//! сгенерированной документации.
//!
//! D1 не генерирует текст: она отвечает на типизированные вопросы о состоянии
//! (`state`). Мы спрашиваем два вопроса:
//! - `is_factual` (`noul`, вероятность 0..1) — подтверждается ли документация
//!   исходным кодом;
//! - `completeness` (`score`, 1..5) — достаточно ли документации разработчику.
//!
//! Пороги принятия по умолчанию — [`MIN_FACTUALITY`] и [`MIN_COMPLETENESS`];
//! в приложении они настраиваются через [`D1Policy`].

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Минимальная вероятность фактической точности (значение по умолчанию).
pub const MIN_FACTUALITY: f32 = 0.85;
/// Минимальная оценка полноты по шкале 1..5 (значение по умолчанию).
pub const MIN_COMPLETENESS: u8 = 3;

/// Пороги приёмки документа. Задаются в настройках приложения.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct D1Policy {
    /// Минимальная вероятность фактической точности (0.0–1.0).
    pub min_factuality: f32,
    /// Минимальная оценка полноты (1–5).
    pub min_completeness: u8,
}

impl Default for D1Policy {
    fn default() -> Self {
        Self {
            min_factuality: MIN_FACTUALITY,
            min_completeness: MIN_COMPLETENESS,
        }
    }
}

/// Запрос к D1.
#[derive(Debug, Clone, Serialize)]
pub struct D1Request {
    /// Имя модели D1 (обычно `d1`).
    pub model: String,
    /// Документация + исходный код, на который она должна опираться.
    pub state: String,
    /// Типизированные вопросы (см. формат Ollama systemone).
    pub questions: serde_json::Value,
}

impl D1Request {
    /// Запрос с двумя стандартными вопросами проверки документации.
    ///
    /// `factuality_instructions` и `completeness_instructions` берутся из
    /// `prompts.yaml`, чтобы правила проверки можно было менять без пересборки.
    pub fn for_documentation(
        model: impl Into<String>,
        state: impl Into<String>,
        factuality_instructions: &str,
        completeness_instructions: &str,
        completeness_criteria: &[String],
    ) -> Self {
        Self {
            model: model.into(),
            state: state.into(),
            questions: serde_json::json!({
                "is_factual": {
                    "type": "noul",
                    "instructions": factuality_instructions,
                },
                "completeness": {
                    "type": "score",
                    "instructions": completeness_instructions,
                    "criteria": completeness_criteria,
                },
            }),
        }
    }
}

/// Ответ D1: произвольные именованные ответы.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct D1Response {
    #[serde(default)]
    pub answers: HashMap<String, serde_json::Value>,
}

impl D1Response {
    /// Вероятность фактической точности (`answers.is_factual.noul`).
    pub fn factuality(&self) -> Option<f32> {
        let value = self.answers.get("is_factual")?;
        extract_number(value, "noul").map(|v| v as f32)
    }

    /// Оценка полноты (`answers.completeness.score`).
    ///
    /// D1 нумерует критерии с нуля (при пяти критериях возвращает 0..4),
    /// поэтому оценка приводится к человеческой шкале 1..5.
    pub fn completeness(&self) -> Option<u8> {
        let value = self.answers.get("completeness")?;
        extract_number(value, "score").map(|v| (v.round() as i32 + 1).clamp(1, 5) as u8)
    }

    /// Итоговая проверка по порогам по умолчанию.
    pub fn evaluate(&self) -> D1Evaluation {
        self.evaluate_with(D1Policy::default())
    }

    /// Итоговая проверка по настраиваемым порогам.
    pub fn evaluate_with(&self, policy: D1Policy) -> D1Evaluation {
        let factual = self.factuality();
        let completeness = self.completeness();
        let accepted = matches!(
            (factual, completeness),
            (Some(f), Some(c)) if f >= policy.min_factuality && c >= policy.min_completeness
        );
        D1Evaluation {
            factual,
            completeness,
            accepted,
        }
    }
}

/// Результат применения политики приёмки.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct D1Evaluation {
    /// Вероятность точности (если D1 ответила).
    pub factual: Option<f32>,
    /// Оценка полноты (если D1 ответила).
    pub completeness: Option<u8>,
    /// Документ прошёл пороги.
    pub accepted: bool,
}

impl D1Evaluation {
    /// Короткое пояснение для HTML/логов.
    pub fn label(&self) -> String {
        match (self.factual, self.completeness) {
            (Some(f), Some(c)) => format!(
                "D1: точность {:.2}, полнота {}/5 — {}",
                f,
                c,
                if self.accepted {
                    "принято"
                } else {
                    "требует проверки"
                }
            ),
            (Some(f), None) => format!("D1: точность {:.2}, полнота —", f),
            (None, Some(c)) => format!("D1: точность —, полнота {}/5", c),
            (None, None) => "D1: ответ без оценок".to_string(),
        }
    }
}

/// Достаёт число из `{"noul": 0.9}` или из голого числа.
fn extract_number(value: &serde_json::Value, key: &str) -> Option<f64> {
    if let Some(n) = value.get(key).and_then(serde_json::Value::as_f64) {
        return Some(n);
    }
    if let Some(n) = value.as_f64() {
        return Some(n);
    }
    // Некоторые сборки D1 отдают массив с единственным значением.
    value
        .as_array()
        .and_then(|values| values.first())
        .and_then(serde_json::Value::as_f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_d1_answers() {
        let raw = r#"{
            "answers": {
                "is_factual": {"noul": 0.93},
                "completeness": {"score": 4}
            }
        }"#;
        let response: D1Response = serde_json::from_str(raw).unwrap();
        assert_eq!(response.factuality(), Some(0.93));
        assert_eq!(response.completeness(), Some(5));
        let evaluation = response.evaluate();
        assert!(evaluation.accepted);
        assert!(evaluation.label().contains("принято"));
    }

    #[test]
    fn rejects_low_factuality_or_completeness() {
        let low_fact = D1Response {
            answers: HashMap::from([
                ("is_factual".to_string(), serde_json::json!({"noul": 0.5})),
                ("completeness".to_string(), serde_json::json!({"score": 5})),
            ]),
        };
        assert!(!low_fact.evaluate().accepted);

        let low_completeness = D1Response {
            answers: HashMap::from([
                ("is_factual".to_string(), serde_json::json!({"noul": 0.99})),
                ("completeness".to_string(), serde_json::json!({"score": 1})),
            ]),
        };
        assert!(!low_completeness.evaluate().accepted);
    }

    #[test]
    fn custom_policy_changes_acceptance() {
        let response = D1Response {
            answers: HashMap::from([
                ("is_factual".to_string(), serde_json::json!({"noul": 0.72})),
                ("completeness".to_string(), serde_json::json!({"score": 3})),
            ]),
        };
        assert!(!response.evaluate().accepted);
        assert!(
            response
                .evaluate_with(D1Policy {
                    min_factuality: 0.70,
                    min_completeness: 4,
                })
                .accepted
        );
        assert!(
            !response
                .evaluate_with(D1Policy {
                    min_factuality: 0.70,
                    min_completeness: 5,
                })
                .accepted
        );
    }

    #[test]
    fn tolerates_plain_numbers_and_missing_answers() {
        let raw = r#"{"answers":{"is_factual":0.9,"completeness":4}}"#;
        let response: D1Response = serde_json::from_str(raw).unwrap();
        assert_eq!(response.factuality(), Some(0.9));
        assert_eq!(response.completeness(), Some(5));

        let empty: D1Response = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.factuality(), None);
        assert!(!empty.evaluate().accepted);
    }

    #[test]
    fn request_contains_two_typed_questions() {
        let request = D1Request::for_documentation(
            "d1",
            "doc + code",
            "проверь факты",
            "оцени полноту",
            &["мало".into(), "много".into()],
        );
        assert_eq!(request.model, "d1");
        assert_eq!(request.questions["is_factual"]["type"], "noul");
        assert_eq!(request.questions["completeness"]["type"], "score");
        assert_eq!(
            request.questions["completeness"]["criteria"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}
