#!/usr/bin/env sh
set -eu

REPO="${IGARA_REPO:-lucaswilliameufrasio/igara}"
BIN_NAME="igara"
INSTALL_DIR="${IGARA_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${IGARA_VERSION:-latest}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "required command not found: $1" >&2
    exit 1
  }
}

need_cmd uname
need_cmd mkdir
need_cmd tar

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux) OS_PART="unknown-linux-musl" ;;
  Darwin) OS_PART="apple-darwin" ;;
  *) echo "unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH_PART="x86_64" ;;
  aarch64|arm64) ARCH_PART="aarch64" ;;
  *) echo "unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

TARGET="${ARCH_PART}-${OS_PART}"
ARCHIVE="${BIN_NAME}-${VERSION}-${TARGET}.tar.gz"
TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t igara)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

if [ "$VERSION" = "latest" ]; then
  API_URL="https://api.github.com/repos/${REPO}/releases/latest"
  need_cmd grep
  if command -v curl >/dev/null 2>&1; then
    need_cmd curl
    VERSION="$(curl -fsSL "$API_URL" | grep '"tag_name":' | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')"
  elif command -v wget >/dev/null 2>&1; then
    need_cmd wget
    VERSION="$(wget -qO- "$API_URL" | grep '"tag_name":' | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')"
  else
    echo "curl or wget is required" >&2
    exit 1
  fi
  [ -n "$VERSION" ] || { echo "failed to resolve latest version" >&2; exit 1; }
fi

ARCHIVE="${BIN_NAME}-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

mkdir -p "$INSTALL_DIR"

if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$URL" -o "$TMP_DIR/$ARCHIVE"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$TMP_DIR/$ARCHIVE" "$URL"
else
  echo "curl or wget is required" >&2
  exit 1
fi

tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"
cp "$TMP_DIR/${BIN_NAME}-${VERSION}-${TARGET}/${BIN_NAME}" "$INSTALL_DIR/$BIN_NAME"
chmod +x "$INSTALL_DIR/$BIN_NAME"

echo "installed $BIN_NAME $VERSION to $INSTALL_DIR/$BIN_NAME"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) : ;;
  *) echo "warning: $INSTALL_DIR is not on PATH" >&2 ;;
esac
