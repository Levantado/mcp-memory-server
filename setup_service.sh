#!/bin/bash

# Цвета для вывода
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting MCP Memory Server setup...${NC}"

# 1. Сборка и установка бинарника
echo -e "${BLUE}Building and installing with cargo...${NC}"
cargo install --path .

if [ $? -ne 0 ]; then
    echo "Build failed. Make sure cargo is installed."
    exit 1
fi

# Установка mcp-init хелпера
echo -e "${BLUE}Installing mcp-init helper to ~/.cargo/bin...${NC}"
cp mcp-init.sh "$HOME/.cargo/bin/mcp-init"
chmod +x "$HOME/.cargo/bin/mcp-init"

# 2. Подготовка директорий
echo -e "${BLUE}Preparing storage and docs directories...${NC}"
STORAGE_ROOT="$HOME/.mcp-memory"
STORAGE_DIR="$STORAGE_ROOT/storage"
DOCS_DIR="$STORAGE_ROOT/docs"

mkdir -p "$STORAGE_DIR"
mkdir -p "$DOCS_DIR"

# 3. Копирование инструкций (теперь в отдельную папку docs)
echo -e "${BLUE}Copying guidelines to $DOCS_DIR...${NC}"
cp effective_work.md "$DOCS_DIR/"
cp docs/AGENT_GUIDELINES.md "$DOCS_DIR/"

# 4. Создание systemd user unit
echo -e "${BLUE}Creating systemd service unit...${NC}"
SERVICE_FILE="$HOME/.config/systemd/user/mcp-memory.service"
mkdir -p "$(dirname "$SERVICE_FILE")"

# Если API ключ уже установлен в окружении, используем его, иначе оставляем пустым
API_KEY_ENV=""
if [ ! -z "$MCP_API_KEY" ]; then
    API_KEY_ENV="Environment=MCP_API_KEY=$MCP_API_KEY"
fi

cat <<EOF > "$SERVICE_FILE"
[Unit]
Description=MCP Memory Rust Server
After=network.target

[Service]
Type=simple
WorkingDirectory=$STORAGE_ROOT
$API_KEY_ENV
ExecStart=$HOME/.cargo/bin/mcp-memory-server-rust --mode hybrid --port 3000 --root $STORAGE_DIR --docs-dir $DOCS_DIR
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
EOF

# 5. Активация сервиса
echo -e "${BLUE}Activating systemd service...${NC}"
systemctl --user daemon-reload
systemctl --user enable mcp-memory.service
systemctl --user restart mcp-memory.service

echo -e "${GREEN}Successfully installed and started!${NC}"
echo -e "Server is running at: ${BLUE}http://localhost:3000/sse${NC}"
echo -e "Storage location: ${BLUE}$STORAGE_DIR${NC}"
echo -e "Docs location: ${BLUE}$DOCS_DIR${NC}"
echo -e "Logs: ${BLUE}journalctl --user -u mcp-memory -f${NC}"

if [ -z "$MCP_API_KEY" ]; then
    echo -e "${BLUE}Hint: To enable authentication, run: export MCP_API_KEY=your_secret && ./setup_service.sh${NC}"
fi
