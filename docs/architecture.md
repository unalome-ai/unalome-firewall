# Unalome — Architecture

Unalome is a local-first desktop application — a "Personal Agent Firewall & Observatory." It monitors AI coding agents installed on the user's machine, showing their activity timeline, security posture of MCP servers, token costs, and a control panel.

## Tech stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 (Rust) |
| Frontend | React 18 + TypeScript |
| Styling | Tailwind CSS + Radix UI primitives + CVA |
| Charts | Recharts |
| Backend DB | SQLite via sqlx (app's own data) |
| Agent DB reading | rusqlite (read-only, for Cursor's SQLite) |
| Async runtime | Tokio |
| Build | Vite (frontend) + Cargo (backend) |

## High-level data flow

```mermaid
graph TB
    subgraph "User's machine"
        CC["Claude Code<br/>~/.claude/projects/**/*.jsonl"]
        CD["Claude Desktop<br/>~/Library/Logs/Claude/main.log"]
        CU["Cursor<br/>~/.cursor/ai-tracking/*.db"]
        MCP["MCP configs<br/>~/.cursor/mcp.json<br/>~/.claude/mcp.json<br/>etc."]
    end

    subgraph "Tauri Backend (Rust)"
        DISC["discovery.rs<br/>Agent Detection"]
        AW["AgentWatcher<br/>Polls every 5s"]
        SCAN["scanner.rs<br/>MCP Security Analysis"]
        DB["database.rs<br/>~/Library/Application Support/<br/>Unalome/unalome.db"]
        PR["pricing.rs<br/>Token → USD"]

        subgraph "Parsers"
            P1["ClaudeCodeParser"]
            P2["ClaudeDesktopParser"]
            P3["CursorParser"]
        end
    end

    subgraph "Frontend (React)"
        APP["App.tsx<br/>State + Navigation"]
        TL["TimelineFeed"]
        AG["AgentGrid"]
        SEC["SecurityDashboard"]
        COST["CostTracker"]
        KS["KillSwitch"]
    end

    CC --> P1
    CD --> P2
    CU --> P3
    MCP --> SCAN

    DISC -->|"discover_all()"| AW
    P1 --> AW
    P2 --> AW
    P3 --> AW
    P1 -.->|"token counts"| PR
    AW -->|"save_action()"| DB
    AW -->|"emit new_actions"| APP

    APP -->|"invoke discover_agents"| DISC
    APP -->|"invoke get_all_actions"| DB
    SEC -->|"invoke scan_all_mcp_configs"| SCAN

    APP --> TL
    APP --> AG
    APP --> SEC
    APP --> COST
    APP --> KS
```

## Startup sequence

```mermaid
sequenceDiagram
    participant FE as Frontend (React)
    participant BE as Backend (Rust)
    participant FS as Filesystem

    BE->>BE: Initialize SQLite (create tables)
    BE->>FS: Scan for installed agents
    FS-->>BE: Found: Claude Code, Cursor, etc.
    BE->>BE: Create AgentWatcher with one parser per agent
    BE->>BE: Register AgentWatcher as managed Tauri state

    FE->>BE: invoke("initialize_database")
    FE->>BE: invoke("discover_agents")
    BE-->>FE: Agent[]
    FE->>BE: invoke("get_all_actions")
    BE-->>FE: Action[] (last 100)
    FE->>FE: Render dashboard

    loop Every 5 seconds
        BE->>BE: AgentWatcher.poll()
        BE->>FS: Read new bytes / query new rows
        FS-->>BE: New data
        BE->>BE: Parse into Action[]
        BE->>BE: Save to SQLite
        BE-->>FE: emit("new_actions")
        FE->>BE: invoke("get_all_actions")
        BE-->>FE: Updated Action[]
    end
```

## Module map

### Backend (`src-tauri/src/`)

| File | Responsibility |
|---|---|
| `main.rs` | Tauri entry point, all `#[tauri::command]` handlers, background polling loop |
| `lib.rs` | Module re-exports |
| `models.rs` | All shared types: Agent, Action, ActionType, RiskLevel, CostInfo, SecurityReport |
| `database.rs` | SQLite persistence — `agents` and `actions` tables |
| `discovery.rs` | Filesystem scanning to detect installed agents and MCP configs |
| `scanner.rs` | Static analysis of MCP server configs for security risks |
| `watcher.rs` | Legacy `FileWatcher` (notify-based, kept for future use) |
| `parsers/mod.rs` | `AgentParser` trait + `AgentWatcher` orchestrator |
| `parsers/claude_code.rs` | JSONL transcript parser |
| `parsers/claude_desktop.rs` | Plain-text log parser |
| `parsers/cursor.rs` | SQLite database reader (read-only) |
| `parsers/pricing.rs` | Token-to-USD cost estimation |

### Frontend (`src/`)

| File | Responsibility |
|---|---|
| `main.tsx` | React root, mounts `<App>` |
| `App.tsx` | Root component — holds all state, sidebar nav, view switching, Tauri invoke/listen |
| `types/index.ts` | TypeScript types mirroring Rust models |
| `lib/utils.ts` | `cn()` for class merging, `formatDate()`, `formatCurrency()`, `formatNumber()` |
| `components/AgentGrid.tsx` | Grid of agent cards with status indicators |
| `components/TimelineFeed.tsx` | Filterable, searchable action timeline |
| `components/SecurityDashboard.tsx` | MCP security scan results, risk score, per-tool breakdown |
| `components/CostTracker.tsx` | Token cost charts by period, budget tracking |
| `components/KillSwitch.tsx` | Pause/resume agent controls (UI-only currently) |
| `components/OnboardingFlow.tsx` | 4-step first-run wizard |
| `components/CircularProgress.tsx` | Reusable SVG progress ring |
| `components/ui/*` | Radix + CVA primitive components (Button, Card, Badge, etc.) |

## Frontend architecture

### Navigation

No router library. A single `activeView` state variable in `App.tsx` switches between views:

| View | Component | Sidebar icon |
|---|---|---|
| `"overview"` | `AgentGrid` | Activity |
| `"timeline"` | `TimelineFeed` | Clock |
| `"security"` | `SecurityDashboard` | Shield |
| `"costs"` | `CostTracker` | DollarSign |
| `"control"` | `KillSwitch` | Power |

### State management

No global store. All state lives in `App.tsx` and flows down via props:

```
App.tsx
  ├── agents: Agent[]
  ├── actions: Action[]
  ├── loading: boolean
  ├── activeView: string
  └── showOnboarding: boolean
```

### Tauri IPC bridge

Frontend ↔ Backend communication uses two mechanisms:

1. **Commands** (`invoke()`): Request-response calls from frontend to backend
2. **Events** (`listen()`): Push notifications from backend to frontend

| Command | Direction | Purpose |
|---|---|---|
| `initialize_database` | FE → BE | Create SQLite tables |
| `discover_agents` | FE → BE | Scan for installed agents |
| `get_all_actions` | FE → BE | Load last 100 actions |
| `get_agent_actions` | FE → BE | Load actions for one agent |
| `scan_all_mcp_configs` | FE → BE | Run security scans |
| `scan_mcp_server` | FE → BE | Scan single MCP server |
| `poll_new_actions` | FE → BE | Manual poll trigger |
| `new_actions` (event) | BE → FE | Notifies frontend of new data |

### Styling

- Tailwind CSS with HSL design tokens for theming
- Per-view gradient backgrounds (`.app-bg-overview`, `.app-bg-security`, etc.)
- Glass morphism cards (`.glass-card` — frosted glass with backdrop blur)
- 72px icon sidebar with active-item glow
- Custom risk-level badges (`.status-safe` through `.status-critical`)
- Radix UI primitives + `class-variance-authority` for component variants

### Onboarding

First-run detection via `localStorage.getItem("unalome_onboarding_complete")`. The `OnboardingFlow` component walks through 4 steps before showing the main dashboard.

## Database schema

Stored at `~/Library/Application Support/Unalome/unalome.db` (macOS).

```mermaid
erDiagram
    agents {
        TEXT id PK
        TEXT name
        TEXT agent_type
        TEXT status
        TEXT config_path
        TEXT last_seen
        TEXT metadata
    }

    actions {
        TEXT id PK
        TEXT agent_id FK
        TEXT action_type
        TEXT timestamp
        TEXT description
        TEXT risk_level
        INTEGER cost_input
        INTEGER cost_output
        REAL cost_usd
        TEXT metadata
    }

    agents ||--o{ actions : "has"
```

## Security scanning

The `SecurityScanner` analyzes MCP server configs using regex pattern matching:

```mermaid
flowchart LR
    A[MCP config JSON] --> B[Extract tools array]
    B --> C{For each tool}
    C --> D[Scan description for<br/>prompt injection patterns]
    C --> E[Scan description for<br/>suspicious patterns]
    C --> F[Inspect input schema<br/>for permission types]
    D --> G[Warnings]
    E --> G
    F --> H[Permissions:<br/>filesystem / network / execution]
    G --> I[Per-tool RiskLevel]
    H --> I
    I --> J[Aggregate to<br/>overall SecurityReport]
```

**Pattern categories:**
- Prompt injection: "ignore previous instructions", "system prompt", "you are now"
- Suspicious: passwords/tokens/secrets, exfiltration keywords, exec/eval/shell
- Permission inference: file/path → filesystem, url/endpoint → network, command/exec → execution

## Known limitations

1. **action_type DB roundtrip is lossy** — `ActionType` is stored as Rust `Debug` format, loaded back as `ActionType::Other(string)`. Structured type info is lost on reload.
2. **KillSwitch is UI-only** — pause/resume updates React state but doesn't actually stop any agent process.
3. **Budget limit is hardcoded** at $50 in `CostTracker.tsx`.
4. **No URL routing** — browser back/forward don't work, no deep linking.
5. **Windsurf and OpenClaw** are discovered but have no parser implementation yet.
