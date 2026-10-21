#!/usr/bin/env bash
#
# Собирает DocCode под Linux и пакует:
#   dist/linux/doccode_<ver>_amd64.deb  — пакет для Debian/Ubuntu
#   dist/linux/DocCode-<ver>-x86_64.AppImage — самодостаточный образ приложения
#
# Запуск:
#   * через Docker (рекомендуется, Windows/macOS/Linux):
#       powershell -File packaging/linux/docker-build.ps1
#   * либо напрямую в Ubuntu/WSL с установленным Rust:
#       bash packaging/linux/build-linux.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

TARGET="${TARGET_TRIPLE:-x86_64-unknown-linux-gnu}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
DIST="$ROOT/dist/linux"
STAGE="$DIST/stage"

command -v cargo >/dev/null 2>&1 || {
    echo "cargo не найден — установите Rust: https://rustup.rs" >&2
    exit 1
}

VERSION="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' Cargo.toml | head -n1)"
[ -n "$VERSION" ] || { echo "не удалось прочитать version из Cargo.toml" >&2; exit 1; }

echo "==> DocCode $VERSION / $TARGET"

# --- 1. иконки (png нужны для deb и AppImage) -------------------------------
if [ ! -f packaging/icon/png/doccode-512.png ]; then
    echo "==> генерация иконок"
    python3 packaging/icon/make_icon.py
fi

# --- 2. release-сборка ------------------------------------------------------
echo "==> cargo build --release --target $TARGET"
cargo build --release --target "$TARGET"

BIN="$TARGET_DIR/$TARGET/release/doccode"
[ -x "$BIN" ] || { echo "бинарь не найден: $BIN" >&2; exit 1; }

# --- 3. FHS-структура -------------------------------------------------------
PREFIX="$STAGE/usr"
rm -rf "$STAGE" "$DIST/DocCode.AppDir"
mkdir -p "$PREFIX/bin" \
         "$PREFIX/share/applications" \
         "$PREFIX/share/pixmaps" \
         "$PREFIX/share/icons/hicolor/scalable/apps" \
         "$PREFIX/share/metainfo" \
         "$PREFIX/share/doc/doccode" \
         "$DIST"

install -m755 "$BIN" "$PREFIX/bin/doccode"
install -m644 packaging/linux/doccode.desktop "$PREFIX/share/applications/doccode.desktop"
install -m644 packaging/linux/doccode.metainfo.xml "$PREFIX/share/metainfo/doccode.metainfo.xml"
install -m644 packaging/icon/icon.svg "$PREFIX/share/icons/hicolor/scalable/apps/doccode.svg"
install -m644 packaging/icon/png/doccode-512.png "$PREFIX/share/pixmaps/doccode.png"
install -m644 prompts.yaml "$PREFIX/share/doc/doccode/prompts.yaml"

for s in 16 24 32 48 64 128 256 512; do
    mkdir -p "$PREFIX/share/icons/hicolor/${s}x${s}/apps"
    install -m644 "packaging/icon/png/doccode-${s}.png" \
                  "$PREFIX/share/icons/hicolor/${s}x${s}/apps/doccode.png"
done

# --- 4. deb -----------------------------------------------------------------
echo "==> deb"
mkdir -p "$STAGE/DEBIAN"
INSTALLED_SIZE="$(du -ks "$PREFIX" | cut -f1)"
cat > "$STAGE/DEBIAN/control" <<EOF
Package: doccode
Version: $VERSION
Section: devel
Priority: optional
Architecture: amd64
Maintainer: DocCode <doccode@localhost>
Depends: libx11-6, libxcursor1, libxi6, libxrandr2, libxkbcommon0, libwayland-client0, libvulkan1, libgl1, libssl3
Installed-Size: $INSTALLED_SIZE
Homepage: https://github.com/doccode/doccode
Description: GUI documentation generator for codebases
 DocCode analyses a project and generates documentation (README,
 API references, diagrams) with the help of an LLM.
EOF
dpkg-deb --root-owner-group --build "$STAGE" "$DIST/doccode_${VERSION}_amd64.deb"
rm -rf "$STAGE/DEBIAN"

# --- 5. AppImage ------------------------------------------------------------
echo "==> AppImage"
APPDIR="$DIST/DocCode.AppDir"
mkdir -p "$APPDIR"
cp -a "$PREFIX" "$APPDIR/usr"
ln -s usr/bin/doccode "$APPDIR/AppRun"
install -m644 packaging/linux/doccode.desktop "$APPDIR/doccode.desktop"
install -m644 packaging/icon/png/doccode-256.png "$APPDIR/doccode.png"
ln -s doccode.png "$APPDIR/.DirIcon"
chmod +x "$APPDIR/AppRun"

# Инструменты (appimagetool, 15 МБ) держим вне dist/, чтобы в каталоге
# артефактов лежали только пакеты.
TOOL="$ROOT/.cache/appimagetool-x86_64"
mkdir -p "$ROOT/.cache"
if [ ! -x "$TOOL" ]; then
    echo "==> скачивание appimagetool"
    curl -fsSL -o "$TOOL" \
        https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$TOOL"
fi

# --appimage-extract-and-run: в контейнере нет FUSE.
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 \
    "$TOOL" "$APPDIR" "$DIST/DocCode-$VERSION-x86_64.AppImage"

# Промежуточная FHS-структура больше не нужна.
rm -rf "$APPDIR" "$STAGE"

# --- итог -------------------------------------------------------------------
echo
echo "Готово:"
ls -lh "$DIST"/*.deb "$DIST"/*.AppImage
