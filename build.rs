//! rustc не отслеживает чтение файлов из proc-macro, а `i18n!` вшивает
//! `locales/*.yml` на этапе компиляции — без этой build-скрипта правка перевода
//! не пересобрала бы крейт. Хеш содержимого уходит в `rustc-env`: изменился —
//! cargo пересоберёт либу, макрос перечитает YAML.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    embed_windows_resources();

    // Иконка окна вшивается через include_bytes!: без файла сборка упадёт,
    // поэтому предупреждаем заранее и с готовой командой.
    const WINDOW_ICON: &str = "packaging/icon/doccode-128.rgba";
    println!("cargo:rerun-if-changed={WINDOW_ICON}");
    if !std::path::Path::new(WINDOW_ICON).exists() {
        println!("cargo:warning={WINDOW_ICON} отсутствует — выполните `python packaging/icon/make_icon.py`");
    }

    let dir = "locales";
    println!("cargo:rerun-if-changed={dir}");
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    paths.sort();

    // FNV-1a по содержимому файлов (детерминирован, без зависимостей).
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for path in &paths {
        println!("cargo:rerun-if-changed={}", path.display());
        for byte in std::fs::read(path).unwrap_or_default() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        // Разделитель между файлами, чтобы перестановка не совпала.
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    println!("cargo:rustc-env=DOC_LOCALES_FINGERPRINT={hash:016x}");
}

/// Иконка и данные версии в PE-ресурсах Windows (Проводник, Пуск, «Свойства файла»).
///
/// На других целевых ОС — no-op. Если `rc.exe` или сам .ico недоступны, сборка
/// не роняется: печатается `cargo:warning`, перекрёстная сборка остаётся рабочей.
fn embed_windows_resources() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let icon = "packaging/icon/doccode.ico";
    println!("cargo:rerun-if-changed={icon}");
    if !std::path::Path::new(icon).exists() {
        println!("cargo:warning={icon} не найден — выполните `python packaging/icon/make_icon.py`");
        return;
    }

    let mut res = winresource::WindowsResource::new();
    res.set_icon(icon);
    res.set("ProductName", "DocCode");
    res.set(
        "FileDescription",
        "DocCode — генератор документации для кода",
    );
    res.set("OriginalFilename", "doccode.exe");
    res.set("InternalName", "doccode");
    res.set("LegalCopyright", "Copyright (c) DocCode");
    res.set_language(0x0409); // en-US, чтобы ресурс читался без сюрпризов в любой локали

    if let Err(err) = res.compile() {
        println!("cargo:warning=Ресурсы Windows не встроены: {err}");
    }
}
