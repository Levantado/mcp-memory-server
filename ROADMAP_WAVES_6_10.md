# Roadmap: MCP Memory Server Rust (Waves 6-10)

This roadmap defines the transition from a single-agent stdio server to a multi-agent, multi-tenant "Knowledge Grid" architecture.

## Architecture Overview
- **Multi-tenancy:** Support for multiple independent projects.
- **Hierarchical Memory:**
    - `shared`: Collective knowledge for all agents in a project.
    - `private`: Isolated memory for a specific agent.
- **Transport:** HTTP (Axum) + SSE (Server-Sent Events) for real-time synchronization.
- **Storage:** Directory-based structure (`storage/{project_id}/{scope}.json`).

---

## Wave 6: Namespace Manager & Registry
**Goal:** Enable the server to manage multiple independent memory graphs simultaneously.
- [ ] Implement `GraphRegistry` (Thread-safe `DashMap<(ProjectId, Scope), Arc<MemoryGraph>>`).
- [ ] Logic for lazy-loading graphs from disk on first request.
- [ ] Automatic directory creation for new projects/agents.
- [ ] Update `MemoryGraph` to support metadata (ProjectID, OwnerID).
- **Artifact:** `artifacts/wave6.md`

## Wave 7: HTTP Network Tier (Axum Integration)
**Goal:** Replace/Supplement stdio with a high-performance HTTP interface.
- [ ] Integrate `axum` and `tower-http`.
- [ ] Implement dynamic routing:
    - `POST /projects/:pid/shared` -> Access shared project memory.
    - `POST /projects/:pid/agents/:aid` -> Access private agent memory.
- [ ] Map JSON-RPC dispatcher to HTTP handlers.
- [ ] Support for JSON-RPC batch requests over HTTP.
- **Artifact:** `artifacts/wave7.md`

## Wave 8: Real-time Synchronization (SSE & Events)
**Goal:** Allow agents to "feel" changes in shared memory instantly.
- [ ] Implement `GET /sse` endpoint for event subscriptions.
- [ ] Add `Broadcast` system: notify all subscribers when a shared graph is updated.
- [ ] Event types: `graph_updated`, `entity_created`, `relation_added`.
- [ ] Connection management (heartbeats, automatic cleanup of stale connections).
- **Artifact:** `artifacts/wave8.md`

## Wave 9: CLI Evolution & Advanced Configuration
**Goal:** Improve server management and observability.
- [ ] Add CLI flags:
    - `--root-dir`: Base directory for all project storage.
    - `--port`: Network port for HTTP (default: 3000).
    - `--mode`: Switch between `http`, `stdio`, or `both`.
- [ ] Enhanced logging: Context-aware logs (know which agent/project is calling).
- [ ] Implement graceful shutdown for all active graphs.
- **Artifact:** `artifacts/wave9.md`

## Wave 10: Migration, Stress Test & Final Release
**Goal:** Production readiness and stability.
- [ ] Migration script: Convert old `shared-memory.json` to the new hierarchical structure.
- [ ] Concurrency Stress Test: Verify stability with 10+ agents writing simultaneously to one project.
- [ ] Final `clippy` audit and performance benchmarks.
- [ ] Documentation for multi-agent connection setup.
- **Artifact:** `artifacts/wave10.md`

---
*Created by Senior Staff Rust Engineer. Ready to execute Wave 6.*
