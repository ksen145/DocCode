//! Локальный in-memory векторный индекс.
//!
//! Используется как fallback без фичи `qdrant` (например, в лёгких тестах).
//! Основной бэкенд — Qdrant Edge в [`crate::index::qdrant_store`].

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

/// Простейший векторный стор с косинусным поиском.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorStore {
    points: Vec<VectorPoint>,
    /// Ожидаемая размерность векторов (0 — любая).
    dimension: usize,
}

impl VectorStore {
    pub fn new(dimension: usize) -> Self {
        Self {
            points: Vec::new(),
            dimension,
        }
    }

    /// Размерность векторов.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Количество точек.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Добавление точки. При несовпадении размерности — ошибка.
    pub fn add(&mut self, point: VectorPoint) -> Result<(), String> {
        if self.dimension == 0 {
            self.dimension = point.vector.len();
        }
        if point.vector.len() != self.dimension {
            return Err(format!(
                "Размерность {} не совпадает с ожидаемой {}",
                point.vector.len(),
                self.dimension
            ));
        }
        // Замена существующего id
        if let Some(existing) = self.points.iter_mut().find(|p| p.id == point.id) {
            *existing = point;
        } else {
            self.points.push(point);
        }
        Ok(())
    }

    /// Добавление пачки точек (одна проверка размерности на пачку).
    pub fn add_batch(&mut self, points: Vec<VectorPoint>) -> Result<(), String> {
        for point in points {
            self.add(point)?;
        }
        Ok(())
    }

    /// Поиск top_k ближайших точек по косинусной близости.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(VectorPoint, f32)> {
        let mut scored: Vec<(VectorPoint, f32)> = self
            .points
            .iter()
            .map(|p| (p.clone(), cosine_similarity(query, &p.vector)))
            .filter(|(_, s)| s.is_finite())
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// Все точки (для сериализации).
    pub fn points(&self) -> &[VectorPoint] {
        &self.points
    }

    /// Сохранение индекса в JSON.
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Не удалось создать {}: {}", parent.display(), e))?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())
    }

    /// Загрузка индекса из JSON (при ошибке — пустой стор).
    pub fn load(path: impl AsRef<std::path::Path>) -> Self {
        std::fs::read_to_string(path.as_ref())
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }
}

/// Косинусная близость двух векторов.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return f32::NAN;
    }
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_orders_by_similarity() {
        let mut store = VectorStore::new(2);
        store
            .add(VectorPoint {
                id: "a".into(),
                vector: vec![1.0, 0.0],
                payload: "a".into(),
            })
            .unwrap();
        store
            .add(VectorPoint {
                id: "b".into(),
                vector: vec![0.0, 1.0],
                payload: "b".into(),
            })
            .unwrap();

        let results = store.search(&[1.0, 0.1], 2);
        assert_eq!(results[0].0.id, "a");
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn dimension_mismatch_rejected() {
        let mut store = VectorStore::new(3);
        let res = store.add(VectorPoint {
            id: "x".into(),
            vector: vec![1.0, 2.0],
            payload: String::new(),
        });
        assert!(res.is_err());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("doccode-vs-test");
        let path = dir.join("index.json");

        let mut store = VectorStore::new(2);
        store
            .add(VectorPoint {
                id: "p".into(),
                vector: vec![0.5, 0.5],
                payload: "payload".into(),
            })
            .unwrap();
        store.save(&path).unwrap();

        let loaded = VectorStore::load(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.points()[0].id, "p");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
