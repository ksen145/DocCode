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
