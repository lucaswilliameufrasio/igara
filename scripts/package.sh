#!/usr/bin/env bash
set -euo pipefail

BIN_NAME="${BIN_NAME:-igara}"
TARGET="${1:?target triple required}"
VERSION="${VERSION:-$(git describe --tags --always --dirty 2>/dev/null || echo dev)}"
OUT_DIR="${OUT_DIR:-dist}"

mkdir -p "$OUT_DIR"

case "$TARGET" in
  *windows*)
    EXT=".exe"
    ARCHIVE_EXT="zip"
    ;;
  *)
    EXT=""
    ARCHIVE_EXT="tar.gz"
    ;;
esac

BIN_PATH="target/${TARGET}/release/${BIN_NAME}${EXT}"
if [[ ! -f "$BIN_PATH" ]]; then
  echo "binary not found at $BIN_PATH" >&2
  exit 1
fi

PKG_DIR="${OUT_DIR}/${BIN_NAME}-${VERSION}-${TARGET}"
rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR"
cp "$BIN_PATH" "$PKG_DIR/"
cp README.md "$PKG_DIR/"
cp LICENSE "$PKG_DIR/" 2>/dev/null || true

if [[ "$ARCHIVE_EXT" == "zip" ]]; then
  (cd "$OUT_DIR" && zip -r "${BIN_NAME}-${VERSION}-${TARGET}.zip" "$(basename "$PKG_DIR")")
else
  tar -C "$OUT_DIR" -czf "${OUT_DIR}/${BIN_NAME}-${VERSION}-${TARGET}.tar.gz" "$(basename "$PKG_DIR")"
fi

echo "packaged $TARGET"
