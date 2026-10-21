#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

//! Точка входа приложения DocCode — генератор документации для кода.

use doccode::app;
use doccode::config::AppConfig;
use doccode::utils::logging;

fn main() -> iced::Result {
    // Инициализация логирования.
    logging::init_logger();

    tracing::info!("DocCode v{}", env!("CARGO_PKG_VERSION"));

    // Загрузка конфигурации (или значений по умолчанию).
    let config = match AppConfig::load() {
        Ok(config) => {
            tracing::info!(
                "Конфигурация загружена: {}",
                AppConfig::config_path().display()
            );
            config
        }
        Err(e) => {
            tracing::warn!("{} — используются значения по умолчанию", e);
            AppConfig::default()
        }
    };

    // Запуск iced-цикла (update/view — в doccode::app).
    app::run(config)
}
