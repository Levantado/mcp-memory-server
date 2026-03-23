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

# 2. Подготовка директорий
echo -e "${BLUE}Preparing storage directories...${NC}"
STORAGE_DIR="$HOME/.mcp-memory"
mkdir -p "$STORAGE_DIR/storage"
mkdir -p "$STORAGE_DIR/docs"

# 3. Копирование инструкций (чтобы они были доступны серверу в WorkingDirectory)
echo -e "${BLUE}Copying guidelines to storage directory...${NC}"
cp effective_work.md "$STORAGE_DIR/"
cp docs/AGENT_GUIDELINES.md "$STORAGE_DIR/docs/"

# 4. Создание systemd user unit
echo -e "${BLUE}Creating systemd service unit...${NC}"
SERVICE_FILE="$HOME/.config/systemd/user/mcp-memory.service"
mkdir -p "$(dirname "$SERVICE_FILE")"

cat <<EOF > "$SERVICE_FILE"
[Unit]
Description=MCP Memory Rust Server
After=network.target

[Service]
Type=simple
WorkingDirectory=$STORAGE_DIR
ExecStart=$HOME/.cargo/bin/mcp-memory-server-rust --mode hybrid --port 3000 --root $STORAGE_DIR/storage
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
echo -e "Storage location: ${BLUE}$STORAGE_DIR/storage${NC}"
echo -e "Logs: ${BLUE}journalctl --user -u mcp-memory -f${NC}"
