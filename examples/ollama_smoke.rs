//! Проверка доступности Ollama: ping → generate → embed.
//!
//! Быстрый способ убедиться, что сервер запущен, а выбранные модели
//! установлены, не запуская полную генерацию документации.
//!
//! Запуск:
//! ```text
//! cargo run --example ollama_smoke -- [url] [модель]
//! cargo run --example ollama_smoke -- http://localhost:11434 gemma4:e4b
//! ```

use doccode::ai::client::GenerateOptions;
use doccode::ai::make_client;
use doccode::config::AiBackendConfig;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "http://localhost:11434".to_string());
    let model = args.next().unwrap_or_else(|| "gemma4:e4b".to_string());

    println!("=== DocCode / Ollama smoke test ===");
    println!("url:   {url}");
    println!("model: {model}\n");

    let config = AiBackendConfig {
        base_url: Some(url),
        model,
        ..Default::default()
    };

    let client = make_client(&config);
    println!("[1/3] make_client: OK");

    let mut failures = 0;

    match client.ping().await {
        Ok(()) => println!("[2/3] ping: OK"),
        Err(error) => {
            println!("[2/3] ping: FAIL — {error}");
            failures += 1;
        }
    }

    let started = Instant::now();
    match client
        .generate("Ответь одним словом: готово", &GenerateOptions::default())
        .await
    {
        Ok(result) => {
            let usage = result.usage.map(|usage| {
                format!(
                    "prompt={} completion={}",
                    usage.prompt_tokens, usage.completion_tokens
                )
            });
            println!(
                "[3/3] generate: OK ({} мс) — {:?} ({})",
                started.elapsed().as_millis(),
                result.content.trim(),
                usage.unwrap_or_default()
            );
        }
        Err(error) => {
            println!("[3/3] generate: FAIL — {error}");
            failures += 1;
        }
    }

    match client.embed("привет, мир").await {
        Ok(vector) => println!(
            "      embed: OK (dim={}, first={:?})",
            vector.len(),
            vector
                .iter()
                .take(3)
                .map(|value| format!("{value:.4}"))
                .collect::<Vec<_>>()
        ),
        Err(error) => {
            println!("      embed: FAIL — {error}");
            failures += 1;
        }
    }

    println!();
    if failures == 0 {
        println!("ИТОГ: Ollama доступен, генерация и эмбеддинги работают");
    } else {
        println!("ИТОГ: проверок с ошибкой — {failures}");
        std::process::exit(1);
    }
}
