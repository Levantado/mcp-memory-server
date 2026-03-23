# MCP KNOWLEDGE GRAPH: AGENT OPERATING GUIDELINES

You are connected to a hierarchical **Knowledge Graph Memory System**. Your goal is to maintain a structured, cross-agent context to ensure seamless collaboration and continuity.

## 1. UNDERSTANDING YOUR SCOPE
Before you start, identify which memory level(s) you are connected to by checking your connection URL:

1.  **GLOBAL SHARED** (`/projects/global/shared`):
    - **Purpose:** Cross-project context. Facts about the User, global preferences, OS-level tools, and recurring patterns.
    - **Visibility:** Shared by ALL agents in ALL projects.
2.  **PROJECT SHARED** (`/projects/{project-name}/shared`):
    - **Purpose:** Project-specific "Team Brain". Architectural decisions, file dependencies, business logic, and project milestones.
    - **Visibility:** Shared by all agents working on THIS specific project.
3.  **PRIVATE AGENT** (`/projects/{project-name}/agents/{agent-id}`):
    - **Purpose:** Your personal scratchpad. Temporary thoughts, complex task breakdowns, or internal logs.
    - **Visibility:** Exclusive to YOU.

## 2. THE MANDATORY WORKFLOW
### Step 1: Context Discovery (First Action)
Never assume the graph is empty. Your first task is to "look around":
- `search_nodes(query: "Project Metadata")`
- `search_nodes(query: "Current Task")`
- `resources/list` (Check if there are guidelines or logs you should read).

### Step 2: Knowledge Capture (While Working)
Document as you go. One fact = one atomic update.
- **Decisions:** "Why did we do this?" -> Create a `Decision` entity.
- **Findings:** "How does this work?" -> Create an `Observation`.
- **Progress:** "What is done?" -> Link `Task` -> `Status: Done`.

### Step 3: Global Sync (Completion)
If you've learned something that applies beyond this project, record it in the **Global** scope if available.

## 3. GRAPH STANDARDS (Schema)
To keep the graph readable for other agents, use these standard types:

### Entity Types:
- `File`: Path to a source file.
- `Feature`: A specific functionality.
- `Bug`: Description of an issue.
- `Decision`: Architectural or design choice.
- `UserPreference`: Habits or tools preferred by the human.

### Relation Types:
- `implements`: `Feature` -> `Code`
- `fixes`: `Commit` -> `Bug`
- `depends_on`: `Module A` -> `Module B`
- `reasoned_by`: `Decision` -> `Requirement`

## 4. BEST PRACTICES
- **Atomic Entities:** Don't put a whole README into one node. Split it into `Features` and `Setup`.
- **Always Link:** An isolated node is invisible to search. Always connect new facts to existing nodes (e.g., to the `Project` node).
- **Search First:** Before creating "Rust Language" entity, check if it already exists.
- **Be Concise:** Observations should be clear, data-dense facts.

## 5. EXAMPLE
**Scenario:** You just updated the project to use Axum 0.8.
1. `create_entities([{ name: "Axum 0.8 Upgrade", entityType: "Decision", observations: ["Migrated from 0.7", "New {param} syntax implemented"] }])`
2. `create_relations([{ from: "src/main.rs", to: "Axum 0.8 Upgrade", relationType: "affected_by" }])`
