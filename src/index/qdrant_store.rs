//! Qdrant Edge — локальный векторный магазин на диске (embedded, без сервера).
//!
//! Включается фичей `qdrant` (`cargo run --features qdrant`). Без фичи проект
//! собирается с in-memory `VectorStore` из [`crate::index::vector_store`].
//!
//! Особенности API qdrant-edge 0.8:
//! - Всё публичное экспортируется из корня крейта (`qdrant_edge::*`).
//! - [`EdgeShard::load`] создаёт новый шард, если каталог пуст.
//! - `flush` обязателен: без него обновления видны только в WAL и не видны
//!   последующему поиску.
//! - Поиск возвращает [`ScoredPoint`] (id + score + payload), а не `Record`.
//! - Для Cosine в edge-версии score — это уже косинусное сходство
//!   (`Distance::distance_order() == LargeBetter`, postprocess — идентичность),
//!   т.е. он совпадает с `cosine_similarity` in-memory стора: больше = ближе.
//!
//! Id точек в Qdrant — только u64/UUID, поэтому строковый id чанка хэшируется
//! (blake3 → u64), а исходный id и payload хранятся в payload точки.

use std::path::{Path, PathBuf};

use qdrant_edge::{
    Distance, EdgeConfig, EdgeShard, EdgeVectorParamsBuilder, NamedQuery, PointInsertOperations,
    PointOperations, PointStruct, PointStructPersisted, QueryEnum, QueryRequestBuilder,
    ScoringQuery, UpdateOperation, VectorInternal, Vectors, WithPayloadInterface,
};

use crate::index::vector_store::VectorPoint;

/// Имя вектора в конфигурации шарда (один dense-вектор).
const VECTOR_NAME: &str = "doc";

/// Держит оригинальный строковый id чанка.
const PAYLOAD_ID: &str = "id";
/// Держит payload чанка (путь к сущности).
const PAYLOAD_TEXT: &str = "payload";

/// Векторное хранилище на Qdrant Edge: один шард на диске.
///
/// API намеренно зеркалит [`crate::index::vector_store::VectorStore`]
/// (`add`/`search`/`len`), чтобы стора можно было выбирать взаимозаменяемо.
pub struct QdrantVectorStore {
    path: PathBuf,
    shard: EdgeShard,
    dimension: usize,
}

impl QdrantVectorStore {
    /// Открыть (или создать, если каталог пуст) шард по пути `path`.
    ///
    /// Размерность векторов фиксируется при создании шарда и не может быть 0.
    pub fn open(path: impl AsRef<Path>, dimension: usize) -> Result<Self, String> {
        if dimension == 0 {
            return Err("Размерность векторов должна быть > 0".to_string());
        }
        let path = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Не удалось создать {}: {e}", path.display()))?;

        let vector = EdgeVectorParamsBuilder::new(dimension, Distance::Cosine).build();
        let config = EdgeConfig::builder().vector(VECTOR_NAME, vector).build();
        // `load` создаёт новый шард в пустом каталоге, иначе открывает существующий.
        let shard = EdgeShard::load(&path, Some(config))
            .map_err(|e| format!("Qdrant шард не открыт ({}): {e}", path.display()))?;

        Ok(Self {
            path,
            shard,
            dimension,
        })
    }

    /// Путь каталога шарда (для логов).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Ожидаемая размерность векторов.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Добавление/замена точки. При несовпадении размерности — ошибка.
    ///
    /// Как и в in-memory сторе, точка с существующим id заменяется (id
    /// детерминированно хэшируется в u64, поэтому замена работает).
    pub fn add(&mut self, point: VectorPoint) -> Result<(), String> {
        self.add_batch(vec![point])
    }

    /// Добавление пачки точек одним upsert + один flush.
    ///
    /// Индексация проекта отправляет тысячи чанков; flush на каждую точку
    /// делает это недопустимо медленным.
    pub fn add_batch(&mut self, points: Vec<VectorPoint>) -> Result<(), String> {
        if points.is_empty() {
            return Ok(());
        }
        let mut qpoints: Vec<PointStructPersisted> = Vec::with_capacity(points.len());
        for point in points {
            if point.vector.len() != self.dimension {
                return Err(format!(
                    "Размерность {} не совпадает с ожидаемой {}",
                    point.vector.len(),
                    self.dimension
                ));
            }
            let numeric_id = Self::numeric_id(&point.id);
            let payload = serde_json::json!({
                PAYLOAD_ID: point.id,
                PAYLOAD_TEXT: point.payload,
            });
            qpoints.push(
                PointStruct::new(
                    numeric_id,
                    // Шард сконфигурирован с именованным вектором `doc` — точка
                    // должна использовать то же имя, иначе upsert падает с
                    // "Not existing vector name".
                    Vectors::new_named([(VECTOR_NAME, point.vector)]),
                    payload,
                )
                .into(),
            );
        }

        let op = UpdateOperation::PointOperation(PointOperations::UpsertPoints(
            PointInsertOperations::PointsList(qpoints),
        ));
        self.shard
            .update(op)
            .map_err(|e| format!("Qdrant upsert не выполнен: {e}"))?;
        // Без flush обновление остаётся в WAL и не видно поиску.
        self.shard
            .flush()
            .map_err(|e| format!("Qdrant flush не выполнен: {e}"))?;
        Ok(())
    }

    /// Количество точек в шарде.
    pub fn len(&self) -> Result<usize, String> {
        use qdrant_edge::CountRequest;
        self.shard
            .count(CountRequest::default())
            .map_err(|e| format!("Qdrant count не выполнен: {e}"))
    }

    pub fn is_empty(&self) -> Result<bool, String> {
        Ok(self.len()? == 0)
    }

    /// Поиск top_k ближайших точек.
    ///
    /// Возвращает `(VectorPoint, близость)`, где близость — косинусное
    /// сходство (как у in-memory стора; для Cosine score edge-шарда уже
    /// является сходством).
    pub fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(VectorPoint, f32)>, String> {
        if query.len() != self.dimension {
            return Err(format!(
                "Размерность {} не совпадает с ожидаемой {}",
                query.len(),
                self.dimension
            ));
        }
        let request = QueryRequestBuilder::new(top_k)
            .query(ScoringQuery::Vector(QueryEnum::Nearest(NamedQuery::new(
                VectorInternal::Dense(query.to_vec()),
                VECTOR_NAME,
            ))))
            .with_payload(WithPayloadInterface::Bool(true))
            .build();
        let found = self
            .shard
            .query(request)
            .map_err(|e| format!("Qdrant search не выполнен: {e}"))?;

        let mut results = Vec::with_capacity(found.len());
        for sp in found {
            let payload = sp
                .payload
                .as_ref()
                .ok_or_else(|| format!("Qdrant точка {} без payload", sp.id))?;
            let id = payload
                .0
                .get(PAYLOAD_ID)
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Qdrant точка {} без id в payload", sp.id))?
                .to_string();
            let text = payload
                .0
                .get(PAYLOAD_TEXT)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            // Для Cosine в edge score уже косинусное сходство (больше = ближе).
            results.push((
                VectorPoint {
                    id,
                    // Вектор не возвращаем: `with_vector` выключен по умолчанию.
                    vector: Vec::new(),
                    payload: text,
                },
                sp.score,
            ));
        }
        Ok(results)
    }

    /// Строчный id → u64 (первые 8 байт blake3): детерминированно и равномерно.
    fn numeric_id(id: &str) -> u64 {
        let hash = blake3::hash(id.as_bytes());
        u64::from_le_bytes(hash.as_bytes()[..8].try_into().expect("8 байт blake3"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("doccode_qdrant_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn add_and_search_roundtrip() {
        let dir = tmp_dir("roundtrip");
        let mut store = QdrantVectorStore::open(&dir, 4).unwrap();
        store
            .add(VectorPoint {
                id: "a".into(),
                vector: vec![1.0, 0.0, 0.0, 0.0],
                payload: "a-entity".into(),
            })
            .unwrap();
        store
            .add(VectorPoint {
                id: "b".into(),
                vector: vec![0.0, 1.0, 0.0, 0.0],
                payload: "b-entity".into(),
            })
            .unwrap();

        let hits = store.search(&[1.0, 0.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0.id, "a");
        assert_eq!(hits[0].0.payload, "a-entity");
        assert!(hits[0].1 >= hits[1].1, "ближайший должен быть первым");
        assert!(
            hits[0].1 > 0.99,
            "идентичные векторы дают близость ~1, got {}",
            hits[0].1
        );
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_wrong_dim() {
        let dir = tmp_dir("dim");
        let mut store = QdrantVectorStore::open(&dir, 3).unwrap();
        let bad = store.add(VectorPoint {
            id: "x".into(),
            vector: vec![1.0, 0.0],
            payload: String::new(),
        });
        assert!(bad.is_err());
        let bad_search = store.search(&[1.0, 0.0], 1);
        assert!(bad_search.is_err());
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zero_dim_rejected() {
        let dir = tmp_dir("zerodim");
        assert!(QdrantVectorStore::open(&dir, 0).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reopen_keeps_data() {
        let dir = tmp_dir("reopen");
        {
            let mut store = QdrantVectorStore::open(&dir, 2).unwrap();
            store
                .add(VectorPoint {
                    id: "p".into(),
                    vector: vec![0.6, 0.8],
                    payload: "payload".into(),
                })
                .unwrap();
        }
        let store = QdrantVectorStore::open(&dir, 2).unwrap();
        assert_eq!(store.len().unwrap(), 1);
        let hits = store.search(&[0.6, 0.8], 1).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0.id, "p");
        assert_eq!(hits[0].0.payload, "payload");
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
