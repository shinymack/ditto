#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}=== Installing Ditto Clipboard Manager ===${NC}"

# Check dependencies
for cmd in cargo bun; do
  if ! command -v $cmd &> /dev/null; then
    echo -e "${RED}Error: $cmd is not installed. Please install it first.${NC}"
    exit 1
  fi
done

echo -e "${BLUE}Building release binary...${NC}"
bun run build

# Determine installation directory
INSTALL_DIR="$HOME/.local/bin"
if [ ! -d "$INSTALL_DIR" ]; then
  mkdir -p "$INSTALL_DIR"
fi

echo -e "${BLUE}Installing binary to $INSTALL_DIR/ditto...${NC}"
cp target/release/ditto "$INSTALL_DIR/ditto"
chmod +x "$INSTALL_DIR/ditto"

echo -e "${BLUE}Running initial configuration...${NC}"
# Run once to set up autostart & icons
"$INSTALL_DIR/ditto" --help > /dev/null

echo -e "${GREEN}Ditto installed successfully to $INSTALL_DIR/ditto!${NC}"
echo ""
echo -e "${YELLOW}Please make sure $INSTALL_DIR is in your PATH. If not, add this to your shell profile (.bashrc/.zshrc):${NC}"
echo -e "  export PATH=\"\$HOME/.local/bin:\$PATH\""
echo ""
echo -e "To start Ditto daemon:  ${GREEN}ditto start${NC}"
echo -e "To toggle visibility:  ${GREEN}ditto toggle${NC}"
