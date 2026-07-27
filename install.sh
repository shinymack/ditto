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
RAW_OS="$(uname -s)"
RAW_ARCH="$(uname -m)"

case "$RAW_OS" in
  Linux*)
    SYSTEM_OS="linux"
    ;;
  Darwin*)
    SYSTEM_OS="darwin"
    ;;
  *)
    error "Ditto is currently only supported on Linux and macOS."
    ;;
esac

case "$RAW_ARCH" in
  x86_64|amd64)
    if [ "$SYSTEM_OS" = "darwin" ]; then
      ASSET_NAMES="ditto_darwin_x86_64 ditto-darwin-x86_64 ditto_darwin_aarch64 ditto"
    else
      ASSET_NAMES="ditto-linux-x86_64 ditto_linux_x64 ditto"
    fi
    ;;
  aarch64|arm64)
    if [ "$SYSTEM_OS" = "darwin" ]; then
      ASSET_NAMES="ditto_darwin_aarch64 ditto-darwin-aarch64 ditto"
    else
      ASSET_NAMES="ditto-linux-aarch64 ditto_linux_aarch64 ditto"
    fi
    ;;
  *)
    error "Unsupported architecture: $RAW_ARCH"
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
  set +e
  _url="$1"
  _dest="$2"
  _tmp="${_dest}.tmp"
  _res=1
  if has_cmd curl; then
    curl -fsSL "$_url" -o "$_tmp" 2>/dev/null
    _res=$?
  elif has_cmd wget; then
    wget -qO "$_tmp" "$_url" 2>/dev/null
    _res=$?
  fi
  if [ "$_res" -eq 0 ]; then
    mv -f "$_tmp" "$_dest"
  else
    rm -f "$_tmp"
  fi
  set -e
  return $_res
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

info "Starting Ditto background daemon..."
if "$INSTALL_DIR/ditto" start >/dev/null 2>&1; then
  success "Ditto background daemon started and autostart registered!"
else
  warn "Could not start daemon automatically. You can start it manually with: ditto start"
fi
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

info "================================================================"
info "                     Keybinding Setup                           "
info "================================================================"
if [ "$SYSTEM_OS" = "darwin" ]; then
  info "To open Ditto with a keyboard shortcut, configure a hotkey      "
  info "(e.g., Cmd+Shift+V) in macOS System Settings / Shortcuts to:    "
else
  info "To open Ditto with a keyboard shortcut, bind a global hotkey    "
  info "(e.g., Super+V or Ctrl+Alt+V) in your system settings to:       "
fi
printf "  \033[1;32mditto toggle\033[0m\n"
info "================================================================"
