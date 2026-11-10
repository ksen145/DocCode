//! Логирование на основе tracing.

/// Инициализация глобального логгера (fmt + env-filter).
/// Повторный вызов безопасен (ошибка "already set" подавляется).
pub fn init_logger() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent() {
        init_logger();
        init_logger(); // не должно паниковать
        tracing::info!("test message");
    }
}
