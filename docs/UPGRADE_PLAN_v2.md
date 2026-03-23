# MCP Knowledge Grid: Version 2.0 Upgrade Plan
**Target Specification:** Model Context Protocol v2025-11-25

## 1. Executive Summary
This document outlines the strategic upgrade of the Rust MCP Memory Server from the legacy `2024-11-05` standard to the latest `2025-11-25` enterprise specification. The primary focus is transitioning to **Streamable HTTP**, implementing **Structured Content**, and adding **JSON-RPC Batching**.

---

## 2. Phase 1: Protocol Negotiation & Structured Content
**Goal:** Allow the server to communicate effectively with modern LLMs without unnecessary string parsing, while declaring the new protocol version.

### Technical Implementation:
- **Initialize Handshake:** Update `protocolVersion` to `2025-11-25`.
- **Tool Outputs:** Modify `protocol_handle_request` in `src/main.rs`. Instead of returning serialized JSON inside a `text` block, use the new `json` or `structuredContent` type block.
  ```json
  // Legacy: { "content": [{ "type": "text", "text": "{\"entities\": ...}" }] }
  // New:    { "content": [{ "type": "json", "value": {"entities": ...} }] }
  ```

### Risks & Mitigations:
- **Risk:** Older clients (like current Gemini CLI internal SDK) might crash if they receive a `json` type block instead of `text`.
- **Mitigation:** Implement **Protocol Version Negotiation**. During the `initialize` call, read the client's requested `protocolVersion`. If it is `2024-*`, fallback to returning stringified text. If it is `2025-*`, use structured JSON.

---

## 3. Phase 2: Streamable HTTP Transport
**Goal:** Deprecate the two-endpoint `/sse` + `/message` model in favor of the unified Streamable HTTP endpoint.

### Technical Implementation:
- **Unified Endpoint:** Create a new generic route: `ANY /mcp/projects/:pid/:scope`.
- **GET Request (Stream Init):** 
  - If `Accept: text/event-stream` is present, generate a UUID and return an SSE stream.
  - Inject the header: `Mcp-Session-Id: <UUID>`.
  - Do *not* send the legacy `event: endpoint` payload.
- **POST Request (Message Delivery):**
  - Read the `Mcp-Session-Id` header from the request to route to the correct `mpsc::Sender`.
  - Process the JSON-RPC body.

### Risks & Mitigations:
- **Risk:** Complete breakage of existing integrations.
- **Mitigation:** **Dual-Transport Architecture**. We will retain `handle_sse_shared` and `handle_message_post` under their current paths. The new Streamable HTTP will live under a distinct `/mcp/...` prefix. We will mark the old paths as `#[deprecated]` in code comments but keep them active indefinitely.

---

## 4. Phase 3: JSON-RPC Batching
**Goal:** Handle high-throughput scenarios where agents send dozens of memory updates simultaneously to reduce network round-trips.

### Technical Implementation:
- Update the Axum JSON extractor in `handle_message_post` to accept an `enum`:
  ```rust
  #[derive(Deserialize)]
  #[serde(untagged)]
  enum RpcPayload {
      Single(JsonRpcRequest),
      Batch(Vec<JsonRpcRequest>),
  }
  ```
- If a `Batch` is received, iterate over the requests.
- **Concurrency execution:** Spawn a `tokio::task` for each request in the batch, `join_all` of them, and return a `Vec<JsonRpcResponse>` back through the SSE stream.

### Risks & Mitigations:
- **Risk:** Deadlocks in `DashMap` if multiple requests in a batch attempt to modify interconnected relations concurrently in a non-deterministic order.
- **Mitigation:** `DashMap` handles shard-level locking safely. However, we must ensure that our `add_relations` logic does not acquire multiple locks simultaneously across different shards (e.g., locking Entity A, then locking Entity B). We will enforce single-entity lock granularity during batch processing.

---

## 5. Execution Roadmap (Waves 11-13)
*   **Wave 11:** Implement Protocol Negotiation and Structured JSON Outputs (Phase 1 & 3).
*   **Wave 12:** Implement Streamable HTTP Transport with header-based session management (Phase 2).
*   **Wave 13:** Security Hardening (Header validation, CORS restrictions) and final testing against modern SDKs.
