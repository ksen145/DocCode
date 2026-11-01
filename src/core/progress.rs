//! Счётчики прогресса пайплайна, доступные UI без блокировки.
//!
//! Сканирование и индексация выполняются в фоновой задаче, а экран прогресса
//! обновляется по таймеру. Чтобы не передавать сообщения из глубины пайплайна,
//! стадии просто пишут счётчики в атомики, а UI читает их снимок по тику.
//! Все операции — `Relaxed`: важен только итоговый счётчик, а не порядок.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Атомарные счётчики стадий сканирования и индексации.
#[derive(Debug, Default)]
pub struct ProgressCounters {
    scan_total: AtomicUsize,
    scan_done: AtomicUsize,
    index_total: AtomicUsize,
    index_done: AtomicUsize,
}

/// Согласованный снимок счётчиков для отрисовки.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProgressSnapshot {
    /// Всего файлов найдено сканером.
    pub scan_total: usize,
    /// Сколько файлов уже разобрано.
    pub scan_done: usize,
    /// Всего фрагментов кода подготовлено к индексации.
    pub index_total: usize,
    /// Сколько фрагментов уже получило эмбеддинг.
    pub index_done: usize,
}

impl ProgressCounters {
    /// Новые обнулённые счётчики.
    pub fn new() -> Self {
        Self::default()
    }

    /// Обнуление перед новым запуском.
    pub fn reset(&self) {
        self.scan_total.store(0, Ordering::Relaxed);
        self.scan_done.store(0, Ordering::Relaxed);
        self.index_total.store(0, Ordering::Relaxed);
        self.index_done.store(0, Ordering::Relaxed);
    }

    /// Сканер закончил обход: теперь известно общее число файлов.
    pub fn set_scan_total(&self, total: usize) {
        self.scan_total.store(total, Ordering::Relaxed);
    }

    /// Один файл разобран.
    pub fn advance_scan(&self) {
        self.scan_done.fetch_add(1, Ordering::Relaxed);
    }

    /// Чанки подготовлены: известно, сколько эмбеддингов нужно получить.
    pub fn set_index_total(&self, total: usize) {
        self.index_total.store(total, Ordering::Relaxed);
    }

    /// Один чанк обработан (успешно или с ошибкой).
    pub fn advance_index(&self) {
        self.index_done.fetch_add(1, Ordering::Relaxed);
    }

    /// Текущие значения счётчиков.
    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            scan_total: self.scan_total.load(Ordering::Relaxed),
            scan_done: self.scan_done.load(Ordering::Relaxed),
            index_total: self.index_total.load(Ordering::Relaxed),
            index_done: self.index_done.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_accumulate_and_reset() {
        let counters = ProgressCounters::new();
        counters.set_scan_total(3);
        counters.advance_scan();
        counters.advance_scan();
        counters.set_index_total(10);
        counters.advance_index();

        let snapshot = counters.snapshot();
        assert_eq!(snapshot.scan_total, 3);
        assert_eq!(snapshot.scan_done, 2);
        assert_eq!(snapshot.index_total, 10);
        assert_eq!(snapshot.index_done, 1);

        counters.reset();
        assert_eq!(counters.snapshot(), ProgressSnapshot::default());
    }
}
