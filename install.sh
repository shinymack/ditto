#!/bin/sh
set -eu

# POSIX-compliant color functions using printf
info() {
  printf "\033[0;34m%s\033[0m\n" "$*"
}

success() {
  printf "\033[0;32m%s\033[0m\n" "$*"
}

warn() {
  printf "\033[1;33m%s\033[0m\n" "$*"
}

error() {
  printf "\033[0;31mError: %s\033[0m\n" "$*" >&2
  exit 1
}

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

# 1. Dependency Check
if ! has_cmd curl && ! has_cmd wget; then
  error "Either 'curl' or 'wget' is required to download Ditto. Please install one of them first."
fi

# 2. OS & Architecture Detection
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux*)
    OS="linux"
    ;;
  *)
    error "Ditto is currently only supported on Linux."
    ;;
esac

case "$ARCH" in
  x86_64|amd64)
    ASSET_NAMES="ditto-linux-x86_64 ditto_linux_x64 ditto"
    ;;
  aarch64|arm64)
    ASSET_NAMES="ditto-linux-aarch64 ditto_linux_arm64 ditto"
    ;;
  *)
    error "Unsupported architecture: $ARCH"
    ;;
esac

# 3. Retrieve Latest Tag
get_latest_tag() {
  _effective_url=""
  if has_cmd curl; then
    _effective_url="$(curl -sIL -o /dev/null -w "%{url_effective}" "https://github.com/shinymack/ditto/releases/latest" 2>/dev/null || true)"
  elif has_cmd wget; then
    _effective_url="$(wget --max-redirect=5 --spider -S "https://github.com/shinymack/ditto/releases/latest" 2>&1 | grep -i "Location:" | tail -n 1 | awk '{print $2}' || true)"
  fi

  _tag=""
  if [ -n "$_effective_url" ]; then
    _tag="$(basename "$_effective_url")"
  fi

  if [ -z "$_tag" ] || [ "$_tag" = "latest" ] || [ "$_tag" = "releases" ]; then
    _tag="v0.1.0"
  fi

  printf "%s" "$_tag"
}

download_file() {
  _url="$1"
  _dest="$2"
  if has_cmd curl; then
    curl -fsSL "$_url" -o "$_dest" 2>/dev/null
  elif has_cmd wget; then
    wget -qO "$_dest" "$_url" 2>/dev/null
  fi
}

info "=== Installing Ditto Clipboard Manager ==="

TAG="$(get_latest_tag)"
info "Resolving target release: ${TAG}"

INSTALL_DIR="$HOME/.local/bin"
if [ ! -d "$INSTALL_DIR" ]; then
  mkdir -p "$INSTALL_DIR"
fi

DOWNLOAD_SUCCESS=0
for ASSET in $ASSET_NAMES; do
  DOWNLOAD_URL="https://github.com/shinymack/ditto/releases/download/${TAG}/${ASSET}"
  info "Trying to download binary (${ASSET})..."
  if download_file "$DOWNLOAD_URL" "$INSTALL_DIR/ditto"; then
    if [ -s "$INSTALL_DIR/ditto" ]; then
      DOWNLOAD_SUCCESS=1
      break
    fi
  fi
done

if [ "$DOWNLOAD_SUCCESS" -ne 1 ]; then
  error "Failed to download a valid binary from release ${TAG}. Checked assets: ${ASSET_NAMES}"
fi

chmod +x "$INSTALL_DIR/ditto"

info "Verifying binary execution..."
if ! "$INSTALL_DIR/ditto" --help >/dev/null 2>&1; then
  error "Downloaded binary is invalid or cannot be executed on this system."
fi

success "Ditto successfully installed to $INSTALL_DIR/ditto!"
printf "\n"

case ":$PATH:" in
  *":$HOME/.local/bin:"*)
    ;;
  *)
    warn "Note: $INSTALL_DIR is not in your PATH."
    warn "Add this line to your shell profile (e.g., ~/.bashrc or ~/.zshrc):"
    printf "  export PATH=\"\$HOME/.local/bin:\$PATH\"\n\n"
    ;;
esac

info "To start Ditto daemon:  ditto start"
info "To toggle visibility:  ditto toggle"
