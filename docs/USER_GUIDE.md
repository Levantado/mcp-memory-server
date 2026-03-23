# User Guide: MCP Memory Server Rust (Knowledge Grid)

A high-performance, multi-tenant memory infrastructure built for AI agents. This server implements the Model Context Protocol (MCP) using a hierarchical in-memory storage with atomic persistence.

## 1. Installation
Ensure you have the Rust toolchain installed.
```bash
cargo build --release
```
The binary will be located at `target/release/mcp-memory-server-rust`.

## 2. Running the Server
The server supports three execution modes (`--mode`):
- **Stdio:** Traditional single-agent connection (compatible with standard MCP clients).
- **Http:** Network-based service for multiple concurrent agents.
- **Hybrid (Default):** Both Stdio and HTTP/SSE simultaneously.

### Common Examples:
```bash
# Start a network server on port 3000 (Default mode: Hybrid)
./target/release/mcp-memory-server-rust --port 3000 --root ./storage

# Start in Stdio-only mode for a specific project
./target/release/mcp-memory-server-rust --mode stdio --project-id my_project
```

## 3. Storage Hierarchy
Data is stored in the `--root` directory (default: `./storage`) using the following structure:
```text
storage/
└── {project_id}/
    ├── shared.json         <-- Collective memory for all agents
    └── agent_{agent_id}.json <-- Private memory for a specific agent
```

## 4. Configuration (CLI / Env)
| Flag | Env Variable | Default | Description |
|------|--------------|---------|-------------|
| `--root` | `MCP_STORAGE_ROOT` | `storage` | Path to save JSON files |
| `--port` | `MCP_HTTP_PORT` | `3000` | Port for HTTP/SSE (0 to disable) |
| `--mode` | `MCP_MODE` | `hybrid` | `stdio`, `http`, or `hybrid` |
| `--interval` | `MCP_SAVE_INTERVAL` | `30` | Seconds between background saves |
| `--project-id`| `MCP_PROJECT_ID` | `default` | Project ID for stdio mode |
| `--agent-id` | `MCP_AGENT_ID` | (none) | Agent ID for stdio mode |

## 5. Network Access (MCP SSE Spec)
To connect a remote agent, use these endpoints:
- **Shared Memory:** `http://localhost:3000/sse/projects/{pid}/shared`
- **Private Memory:** `http://localhost:3000/sse/projects/{pid}/agents/{aid}`

The server follows the standard MCP SSE handshake:
1. `GET` the SSE endpoint -> Receive `event: endpoint`.
2. `POST` JSON-RPC messages to the provided `session_id` URI.
3. Receive responses/notifications via the SSE stream.

## 6. Maintenance
- **Health Check:** Send a `mcp_memory_health_check` method to any endpoint to verify graph integrity.
- **Migration:** Run `cargo run --bin migrate` to move legacy `shared-memory.json` into the new structure.
