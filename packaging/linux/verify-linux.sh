#!/usr/bin/env bash
#
# Проверяет собранные Linux-артефакты:
#   * control и состав doccode_<ver>_amd64.deb
#   * валидность packaging/linux/doccode.desktop
#   * динамические ссылки бинарника (ldd)
#   * наличие AppImage
#
# Запуск (после build-linux.sh), внутри образа сборки:
#   powershell -File packaging/linux/verify-linux.sh   # см. docker-build.ps1 -Verify
#   bash packaging/linux/verify-linux.sh               # в Ubuntu/WSL
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DIST="$ROOT/dist/linux"
VERSION="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' "$ROOT/Cargo.toml" | head -n1)"
DEB="$DIST/doccode_${VERSION}_amd64.deb"
APPIMAGE="$DIST/DocCode-${VERSION}-x86_64.AppImage"

[ -f "$DEB" ] || { echo "нет пакета: $DEB — сначала build-linux.sh" >&2; exit 1; }
[ -f "$APPIMAGE" ] || { echo "нет AppImage: $APPIMAGE — сначала build-linux.sh" >&2; exit 1; }

echo "== control пакета =="
dpkg-deb -I "$DEB"

echo
echo "== состав пакета =="
dpkg-deb -c "$DEB"

echo
echo "== валидация .desktop =="
desktop-file-validate "$ROOT/packaging/linux/doccode.desktop"
echo "doccode.desktop: OK"

echo
echo "== валидация AppStream-метаданных =="
python3 -c "import xml.dom.minidom,sys; xml.dom.minidom.parse(sys.argv[1]); print('metainfo: XML OK')" \
    "$ROOT/packaging/linux/doccode.metainfo.xml"

echo
echo "== динамические ссылки бинарника =="
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
dpkg-deb -x "$DEB" "$TMP"
ldd "$TMP/usr/bin/doccode"

echo
echo "OK: $DEB"
echo "OK: $APPIMAGE"
