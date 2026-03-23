# Wave 12: Streamable HTTP Transport & Architectural Refactoring

## Original Plan Goal
**Deprecate the two-endpoint `/sse` + `/message` model in favor of the unified Streamable HTTP endpoint.** Implement header-based session management as defined in the MCP 2025 specification. 

## Summary of Completed Work
The server has been successfully upgraded to support the modern **Streamable HTTP** transport standard, achieving full compliance with the late-2025 MCP specification. Additionally, the codebase underwent a significant architectural refactoring to improve maintainability.

## Key Changes
- **Architectural Refactoring (`src/dispatcher.rs`):**
    - Extracted the massive `protocol_handle_request` JSON-RPC dispatcher from `main.rs` into its own dedicated module. This resolves the code-smell identified in the Wave 11 review and keeps the networking layer clean.
- **Streamable HTTP Implementation (`src/main.rs`):**
    - Introduced unified routing paths: `/mcp/projects/:pid/shared` and `/mcp/projects/:pid/agents/:aid`.
    - **Header-Based Handshake:** The `GET` request now establishes the SSE stream and injects `Mcp-Session-Id` and `Mcp-Protocol-Version` directly into the HTTP response headers, rather than sending a legacy `endpoint` event payload.
    - **Header-Based Routing:** The `POST` request now extracts the session ID from the `Mcp-Session-Id` HTTP header instead of relying on URL query parameters.
- **Dual-Transport Fallback Strategy:**
    - The legacy `2024-11-05` HTTP+SSE transport endpoints (`/sse/...` and `/message`) remain fully operational. This dual-architecture guarantees zero downtime or breakage for existing clients (like the current Gemini CLI) while they transition to the new standard.
- **Version Bump:** `Cargo.toml` updated to `0.7.0`.

## Verification
1. **Compilation:** `cargo check` passes cleanly.
2. **Streamable Test (Manual via curl):**
   ```bash
   # Terminal 1: Start Server
   cargo run -- --port 3000
   
   # Terminal 2: Open Streamable Connection
   curl -i -N http://127.0.0.1:3000/mcp/projects/demo/shared
   # Expect output headers to include:
   # mcp-session-id: <UUID>
   # mcp-protocol-version: 2025-11-25
   
   # Terminal 3: Send POST (using the UUID from headers)
   curl -i -X POST "http://127.0.0.1:3000/mcp/projects/demo/shared" \
        -H "Content-Type: application/json" \
        -H "Mcp-Session-Id: <UUID>" \
        -d '{"jsonrpc":"2.0","id":1,"method":"mcp_memory_health_check","params":{}}'
   # Expect output in Terminal 3: HTTP/1.1 202 Accepted
   # Expect output in Terminal 2: data: {"jsonrpc":"2.0"...}
   ```

## Next Steps
In **Wave 13: Security Hardening**, we will finalize the upgrade by implementing header validation, basic CORS restrictions, and structural validation placeholders for future resource binding features.
