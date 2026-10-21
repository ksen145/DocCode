//! Модель проекта: файлы и извлечённые из них сущности.

use crate::core::parser::Entity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Интерактивная модель всего проекта.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectModel {
    /// Имя проекта.
    pub name: String,
    /// Сущности по файлам (ключ — путь к файлу относительно корня).
    pub entities: HashMap<String, Vec<Entity>>,
    /// Все поддерживаемые файлы проекта (относительные пути, `/` в качестве разделителя).
    ///
    /// В отличие от `entities`, включает и файлы, из которых не извлечено ни
    /// одной сущности — они нужны для дерева проекта и векторного индекса.
    #[serde(default)]
    pub files: Vec<String>,
}

impl ProjectModel {
    /// Все сущности плоским списком (для генерации документации).
    pub fn all_entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values().flatten()
    }
}
