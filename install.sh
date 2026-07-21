#!/bin/bash
set -e

# Colors for terminal output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}=== Installing Ditto Clipboard Manager ===${NC}"

# 1. Platform and Architecture Check
OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" != "Linux" ]; then
  echo -e "${RED}Error: Ditto is currently only supported on Linux.${NC}"
  exit 1
fi

if [ "$ARCH" != "x86_64" ]; then
  echo -e "${RED}Error: Precompiled binaries are currently only available for x86_64 architecture.${NC}"
  echo -e "${YELLOW}To install on $ARCH, please clone the repository and build from source.${NC}"
  exit 1
fi

# 2. Dependency Check
if ! command -v curl &> /dev/null; then
  echo -e "${RED}Error: curl is required to download Ditto. Please install it first.${NC}"
  exit 1
fi

# 3. Retrieve Latest Version
echo -e "${BLUE}Retrieving the latest release information...${NC}"
LATEST_URL="https://github.com/shinymack/ditto/releases/latest"
TAG=$(curl -sI "$LATEST_URL" | grep -i location | tr -d '\r' | awk -F/ '{print $NF}')

# Fallback to default version if no releases exist yet in the repository
if [ -z "$TAG" ] || [ "$TAG" = "latest" ]; then
  TAG="v0.1.0"
  echo -e "${YELLOW}No active release found. Falling back to default tag: ${TAG}${NC}"
fi

DOWNLOAD_URL="https://github.com/shinymack/ditto/releases/download/${TAG}/ditto-linux-x86_64"

# 4. Determine Installation Directory
INSTALL_DIR="$HOME/.local/bin"
if [ ! -d "$INSTALL_DIR" ]; then
  mkdir -p "$INSTALL_DIR"
fi

# 5. Download Precompiled Binary
echo -e "${BLUE}Downloading Ditto binary (${TAG}) from GitHub...${NC}"
if ! curl -L "$DOWNLOAD_URL" -o "$INSTALL_DIR/ditto"; then
  echo -e "${RED}Error: Failed to download binary from $DOWNLOAD_URL${NC}"
  exit 1
fi

chmod +x "$INSTALL_DIR/ditto"

# 6. Verify Installation
if ! "$INSTALL_DIR/ditto" --help > /dev/null; then
  echo -e "${RED}Error: Downloaded binary is invalid or cannot be executed.${NC}"
  exit 1
fi

echo -e "${GREEN}Ditto successfully installed to $INSTALL_DIR/ditto!${NC}"
echo ""

# 7. Environment Path Check
case ":$PATH:" in
  *":$HOME/.local/bin:"*)
    ;;
  *)
    echo -e "${YELLOW}Note: $INSTALL_DIR is not in your PATH. Please add it to your shell profile (e.g., ~/.bashrc or ~/.zshrc):${NC}"
    echo -e "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    ;;
esac

echo -e "To start Ditto daemon:  ${GREEN}ditto start${NC}"
echo -e "To toggle visibility:  ${GREEN}ditto toggle${NC}"
