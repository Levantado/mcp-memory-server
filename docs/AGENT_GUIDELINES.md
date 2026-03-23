# MCP KNOWLEDGE GRAPH MEMORY: THE COLLECTIVE BRAIN

You are connected to a high-performance **Shared Knowledge Graph** (v2025-11-25). Your mission is to build, maintain, and query the project's "Collective Brain" to ensure all agents (present and future) share the same context.

## CONNECTION & CONTEXT
- **Server:** `http://localhost:3000/mcp`
- **Project ID:** (Determined by root folder name or environment)
- **Shared Scope:** `projects/{pid}/shared` — **DEFAULT for project knowledge.**
- **Private Scope:** `projects/{pid}/agents/{aid}` — Only for your personal scratchpad.

## MANDATORY WORKFLOW
### 1. Discovery (First 5 mins)
Before adding anything, **search** for existing knowledge to avoid duplication:
`search_nodes(query: "Project Metadata")`
`search_nodes(query: "Current Architecture")`

### 2. Knowledge Capture (As you work)
- **When a decision is made:** Create a `Decision` entity.
- **When a bug is fixed:** Update the `Bug` entity or create an `Observation` on the file.
- **When a feature is added:** Link `Function` -> `Feature` -> `Requirements`.

### 3. Verification (Periodic)
Run `read_graph` to see how your changes fit into the global structure.

## SCHEMA STANDARDS (Recommended)
### Entity Types:
- `File`: Path to a file (e.g., `src/main.rs`).
- `Function`: Method or function name.
- `Decision`: Architectural or logic choice.
- `Task`: Current goal or user request.
- `Pattern`: Used design patterns (e.g., `Arc`, `Cow`, `SSE`).

### Relation Types:
- `implements`: `Function` -> `Task`
- `fixes`: `Commit` -> `Bug`
- `depends_on`: `File` -> `Library`
- `reasoned_by`: `Decision` -> `Observation`

## BEST PRACTICES FOR AGENTS
- **Atomic Facts:** Instead of "Refactored the whole project", use "Refactored `src/graph.rs` to use `Arc<str>` for memory efficiency."
- **Contextual Linking:** Always link new entities to existing ones. An isolated node is a lost node.
- **Search-Driven Actions:** If you're unsure how a part of the system works, **ask the graph first**.

## EXAMPLES
### Creating a new decision:
`create_entities([{ name: "Switch to Axum 0.8", entityType: "Decision", observations: ["Required by v2025-11-25 protocol", "Introduces {param} route syntax"] }])`

### Linking files:
`create_relations([{ from: "src/main.rs", to: "src/graph.rs", relationType: "uses_logic" }])`
