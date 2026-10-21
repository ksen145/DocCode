# Упаковка DocCode

Всё, что нужно, чтобы получить иконку приложения и установщики под Windows и Linux.

```powershell
# всё сразу: иконки + Windows + Linux (Linux собирается в Docker)
powershell -File packaging\build-release.ps1

# только Windows
powershell -File packaging\build-release.ps1 -Target windows

# только Linux
powershell -File packaging\build-release.ps1 -Target linux
```

Готовые артефакты:

| Платформа | Артефакт |
|---|---|
| Windows | `dist/windows/DocCode-<ver>-x64-setup.exe` (NSIS) |
| Linux | `dist/linux/doccode_<ver>_amd64.deb` (Debian/Ubuntu) |
| Linux | `dist/linux/DocCode-<ver>-x86_64.AppImage` |

> **macOS** из-под Windows не собирается: нужен macOS SDK и подпись приложения.
> Если понадобится — собирайте на Mac через `cargo build --release` и `cargo-bundle`
> (иконка-мастер `packaging/icon/icon.svg` уже готова, `.icns` собирается из PNG).

---

## Иконка

| Файл | Назначение |
|---|---|
| `packaging/icon/make_icon.py` | генератор: рисует мастер 1024×1024 и раскладывает во все форматы |
| `packaging/icon/icon.svg` | векторный мастер (hicolor/scalable, документация, веб) |
| `packaging/icon/png/doccode-*.png` | 16…512 px — для Linux (hicolor), AppImage, обложек |
| `packaging/icon/doccode.ico` | Windows: ресурсы exe (`build.rs`) и иконка установщика |
| `packaging/icon/doccode-128.rgba` | сырые пиксели для иконки окна iced (вшита в бинарь) |

```powershell
python packaging\icon\make_icon.py      # требует Pillow: pip install pillow
```

Композиция меняется по размеру (всё рисуется в 1024 и уменьшается LANCZOS):

* **16, 24 px** — синий скруглённый квадрат и белый `</>`: читается даже в списке файлов;
* **32, 48 px** — тот же квадрат, белый лист и синий `</>`;
* **64 px и больше** — лист, `</>` и строки «текста».

**Важно:** `doccode-128.rgba` нужен на этапе компиляции (`include_bytes!`), поэтому
сгенерированные иконки лежат в дереве и их нужно коммитить. Если файла нет, `build.rs`
напечатает предупреждение с командой генерации.

Где иконка используется:

* **окно/панель задач** — `src/app/mod.rs` (`iced::window::icon::from_rgba`);
* **Проводник, «Свойства файла», ярлыки** — PE-ресурсы exe, их встраивает `build.rs`
  через `winresource` (`rc.exe` из Windows SDK);
* **«Пуск», установщик, «Удаление программ»** — `installer.nsi`;
* **Linux** — `hicolor` 16…512 + `scalable`, `pixmaps`, `.desktop`.

---

## Windows

```powershell
# обычно достаточно:
powershell -File packaging\windows\build-installer.ps1

# без пересборки exe:
powershell -File packaging\windows\build-installer.ps1 -SkipBuild
```

Требования: Rust (MSVC), Windows SDK (`rc.exe` — уже есть, если проект собирался),
NSIS:

```powershell
scoop install nsis
```

Что делает `installer.nsi`:

* ставит в `C:\Program Files\DocCode` (`doccode.exe`, `prompts.yaml`, `doccode.ico`);
* ярлыки в «Пуск» и (опционально) на рабочий стол;
* запись в «Установка и удаление программ» (иконка, версия, `EstimatedSize`,
  тихая деинсталляция `uninstall.exe /S`);
* проверка 64-битной Windows, сжатие LZMA, деинсталлятор.

Версия и размер берутся из `Cargo.toml` автоматически; вручную:

```powershell
cd packaging\windows
makensis /DPRODUCT_VERSION=0.1.0 /DESTIMATED_SIZE=19245 installer.nsi
```

---

## Linux

```powershell
powershell -File packaging\linux\docker-build.ps1          # обычный запуск
powershell -File packaging\linux\docker-build.ps1 -Verify   # + проверка пакетов
powershell -File packaging\linux\docker-build.ps1 -RebuildImage   # образ с нуля
```

Образ — `packaging/linux/Dockerfile`: **Ubuntu 22.04 + Rust + зависимости
winit/wgpu/OpenSSL**. Собирать в контейнере, а не в WSL, имеет смысл по двум
причинам: не нужно ставить тулчейн в систему, а glibc 2.35 даёт пакеты,
запускающиеся на Ubuntu 22.04+, Debian 12+, Fedora 36+ и большинстве свежих
дистрибутивов.

Кеш (`cargo` registry и `target`) живёт в именованном томе `doccode-build-cache`,
поэтому повторные сборки инкрементальные. `appimagetool` (15 МБ) скачивается один
раз в `.cache/appimagetool-x86_64` — в `dist/` попадают только пакеты.
`.gitignore` уже покрывает `/dist/`, `/.cache/` и `/target/`, но **не**
`packaging/icon/**` — иконки лежат в дереве намеренно (см. выше).

Без Docker — в Ubuntu/WSL с установленным Rust:

```bash
bash packaging/linux/build-linux.sh
```

Что в пакетах:

* `/usr/bin/doccode`
* `/usr/share/applications/doccode.desktop` (+ `Comment[ru]`, `Keywords[ru]`)
* `/usr/share/icons/hicolor/{16,24,32,48,64,128,256,512}x*/apps/doccode.png`
* `/usr/share/icons/hicolor/scalable/apps/doccode.svg`
* `/usr/share/metainfo/doccode.metainfo.xml` (AppStream — для GNOME Software/KDE Discover)
* `/usr/share/pixmaps/doccode.png`, `/usr/share/doc/doccode/prompts.yaml`

Зависимости deb: `libx11-6, libxcursor1, libxi6, libxrandr2, libxkbcommon0,
libwayland-client0, libvulkan1, libgl1, libssl3` (X11/Wayland/GL подгружаются
dlopen-ом, поэтому их нет в `ldd`, но в `Depends` они нужны).

Проверка пакетов (контролы deb, `desktop-file-validate`, `ldd`, metainfo):

```powershell
powershell -File packaging\linux\docker-build.ps1 -Verify
# либо отдельно, в Ubuntu/WSL:
bash packaging/linux/verify-linux.sh
```

AppImage самодостаточен:

```bash
chmod +x dist/linux/DocCode-0.1.0-x86_64.AppImage
./dist/linux/DocCode-0.1.0-x86_64.AppImage
```

---

## Структура

```
packaging/
├── build-release.ps1        # единая точка сборки дистрибутива
├── README.md
├── icon/
│   ├── make_icon.py         # генератор PNG/ICO/RGBA/SVG
│   ├── icon.svg
│   ├── doccode.ico
│   ├── doccode-128.rgba
│   └── png/doccode-{16..512}.png
├── windows/
│   ├── installer.nsi        # NSIS-скрипт (ASCII-only, локализация — MUI)
│   └── build-installer.ps1
└── linux/
    ├── Dockerfile           # Ubuntu 22.04 + Rust + зависимости GUI
    ├── docker-build.ps1     # запуск сборки в контейнере (-Verify — с проверкой)
    ├── build-linux.sh       # сама сборка: deb + AppImage
    ├── verify-linux.sh      # проверка собранных пакетов
    ├── doccode.desktop      # запись меню (локализованы ru/en)
    └── doccode.metainfo.xml # AppStream-метаданные
```
