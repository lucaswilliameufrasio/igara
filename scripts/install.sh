#!/usr/bin/env sh
set -eu

REPO="${IGARA_REPO:-lucaswilliameufrasio/igara}"
VERSION="${IGARA_VERSION:-latest}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "required command not found: $1" >&2
    exit 1
  }
}

need_cmd mktemp
need_cmd rm
need_cmd sh

case "$REPO" in
  */*) ;;
  *) echo "repository must be in OWNER/REPO format" >&2; exit 1 ;;
esac

if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/${REPO}/releases/latest/download/igara-installer.sh"
else
  case "$VERSION" in
    v*) TAG="$VERSION" ;;
    *) TAG="v$VERSION" ;;
  esac
  URL="https://github.com/${REPO}/releases/download/${TAG}/igara-installer.sh"
fi

TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t igara)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

INSTALLER="$TMP_DIR/igara-installer.sh"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$URL" -o "$INSTALLER"
elif command -v wget >/dev/null 2>&1; then
  wget -q "$URL" -O "$INSTALLER"
else
  echo "curl or wget is required" >&2
  exit 1
fi

sh "$INSTALLER"
