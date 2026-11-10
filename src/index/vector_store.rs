//! Точка векторного индекса (общая для индексации и поиска).

use serde::{Deserialize, Serialize};

/// Точка векторного индекса.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPoint {
    /// Идентификатор (путь к файлу/сущность).
    pub id: String,
    /// Вектор.
    pub vector: Vec<f32>,
    /// Полезная нагрузка (например, путь к сущности).
    pub payload: String,
}
