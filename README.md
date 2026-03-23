# MCP Memory Server (Rust)

A high-performance implementation of the **Model Context Protocol (MCP)** Knowledge Graph Memory Server, written in Rust. Designed for speed, security, and seamless integration with AI agents like Claude, Gemini, and others.

## 🚀 Key Features

- **Blazing Fast:** Built with Rust, Axum, and `dashmap` for concurrent, low-latency operations (~5000+ RPS).
- **Hierarchical Memory:** Support for three distinct memory scopes:
  - **Global Shared:** Context that follows you across all projects.
  - **Project Shared:** A "Team Brain" for specific project context.
  - **Private Agent:** Personal scratchpads for individual agents.
- **Modern Protocol:** Fully supports **MCP v2025-11-25** (Streamable SSE) and legacy protocols.
- **Production Ready:** 
  - **Security:** Bearer Token (API Key) authentication.
  - **Reliability:** Background session cleanup and atomic JSON storage.
  - **Resources:** Built-in support for sharing guidelines and project lists as MCP Resources.
- **Easy Deployment:** Automated setup script for `systemd --user` background service.

## 🛠 Installation

### Prerequisites
- [Rust](https://rustup.rs/) (latest stable)
- [uv](https://github.com/astral-sh/uv) (optional, for stress tests)

### Quick Start (Linux/macOS)
1. Clone the repository:
   ```bash
   git clone https://github.com/youruser/mcp-memory-server.git
   cd mcp-memory-server
   ```
2. Run the automated setup script:
   ```bash
   chmod +x setup_service.sh
   ./setup_service.sh
   ```
   *To enable authentication:* `export MCP_API_KEY=your_secret && ./setup_service.sh`

## ⚙️ Configuration

### Global Config (`~/.gemini/settings.json`)
```json
{
  "mcpServers": {
    "mcp-memory": {
      "url": "http://127.0.0.1:3000/mcp/projects/global/shared",
      "type": "sse",
      "headers": { "Authorization": "Bearer your_secret" },
      "trust": true
    }
  }
}
```

### Modes
- **Hybrid (Default):** Runs both Stdio and HTTP/SSE server.
- **Http:** Network-only mode for remote agents.
- **Stdio:** Standard mode for local integration.

## 📊 Performance
The server includes a comprehensive stress test suite. In our benchmarks, it handles **4000+ RPS** with **sub-5ms** average latency on standard hardware.

Run benchmarks:
```bash
uv run --with aiohttp python stress_test.py
```

## 📜 Documentation
Refer to the `docs/` folder for:
- [Agent Operating Guidelines](./docs/AGENT_GUIDELINES.md)
- [Effective Work Policy](./effective_work.md)

## ⚖️ License
MIT / Apache 2.0
