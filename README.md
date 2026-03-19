<div align="center">

# Unalome Agent Firewall

### Your personal security layer for AI coding agents.

See what your agents see. Protect what matters. Stay in control.

[Download](https://unalome.ai/download) · [Documentation](https://unalome.ai/docs) · [Mission](https://unalome.ai/mission) · [Contributing](CONTRIBUTING.md)

</div>

---

## What is Unalome Agent Firewall?

Unalome Agent Firewall is a free, open-source desktop app that gives you complete visibility into what your AI agents are doing on your machine — in plain language anyone can understand.

If you use Claude Code, Cursor, OpenClaw, Windsurf, Claude Desktop, or any AI agent with MCP tools, Unalome Agent Firewall watches them for you. It catches exposed secrets before they leak. It backs up your files before agents overwrite them. It tracks every dollar your AI spends. And it shows you exactly where your data goes.

You don't need to be technical. You just need to care about what's happening on your computer.

---

## Supported Agents

| Agent | Auto-Discovery | Activity Tracking | MCP Scanning |
|-------|:-:|:-:|:-:|
| Claude Code | Full (JSONL sessions) | Full — tool calls, text responses, session starts, costs, cache tokens | Yes |
| Cursor | Full (SQLite tracking DB) | Full — tab completions, composer edits, scored commits | Yes |
| OpenClaw | Full (JSONL sessions) | Full — tool calls, text responses, session starts, model changes, messaging channels, errors | Yes |
| Claude Desktop | Yes | Log-based — MCP events, errors, version changes | Yes |
| Windsurf | Yes | — | Yes |
| VS Code (MCP) | Yes | — | Yes |
| Custom MCP Servers | Yes | — | Yes |

---

## Features

### Action Timeline

Real-time feed of every action your AI agents take. Each action is classified by risk level (Safe, Low, Medium, High, Critical) and shows the tool used, arguments, and full token breakdown — input, output, prompt cache write, and prompt cache read. Filter by time period (1h, 24h, 7d, 30d, All), risk level, agent, or search by keyword. Compact collapsible cards expand to show full details including model info and cost.

### Security Dashboard

Scans MCP server configurations for vulnerabilities and security issues. Analyzes commands, arguments, environment variables, and tool schemas. Detects prompt injection risks, excessive permissions (raw shell execution, eval flags, permissive flags), data exfiltration patterns, suspicious configurations, credential-like env vars, and insecure HTTP connections. Produces a security score (0–100) with expandable per-server details showing command, args, env vars, tools, and warnings.

### Cost Tracker

Tracks API spending across all agents with per-action token and cost breakdowns. Supports budget limits with visual alerts when approaching thresholds. Displays input, output, cache write, and cache read tokens separately. Covers pricing for:

- **Anthropic** — Claude Opus 4.6/4.5, Sonnet 4.6/4.5, Haiku 4.5, and legacy models
- **OpenAI** — GPT-5.4, GPT-5.3 Codex, GPT-5, GPT-4.1, GPT-4o, GPT-4 Turbo, GPT-3.5, o4-mini, o3, o1
- **Google** — Gemini 3.1 Pro, Gemini 2.5 Pro/Flash, Gemini 2.0 Flash, Gemini 1.5 Pro/Flash
- **xAI** — Grok-3, Grok
- **DeepSeek** — DeepSeek R1, DeepSeek V3
- **Mistral** — Mistral Large, Mistral Small, Codestral
- **Meta** — Llama 3.3, Llama 3.1 (405b/70b)

### PII Guardian

Scans agent activity for sensitive data exposure in real time. Detects:

- **Critical** — API keys (Anthropic, OpenAI, AWS, GitHub, Stripe, Google), private keys (RSA/EC/DSA), JWTs, connection strings
- **High** — Social Security Numbers, credit cards (with Luhn validation)
- **Medium** — Email addresses, phone numbers
- **Low** — IP addresses, environment variables

Each finding includes severity, source context, and a recommended remediation action. Dismiss or restore findings with one click. Filter by time period and severity. Delete findings by type.

### Safety Net

Automatic file snapshots before any agent writes or edits a file. Protects source code, configs, and text files up to 5MB. One-click restore to any previous version — or batch-restore multiple files at once. Preview snapshot contents before restoring. Configurable storage limits (default 500MB) and retention periods (default 30 days). Filter by time period.

### Data Shield

Monitors all outbound network connections made by agents. Classifies destinations into categories: AI providers, cloud services, package registries, documentation, search engines, MCP servers, and local services. Unknown domains are flagged for your review — mark them as trusted or suspicious. Tracks connection history, frequency, and which agents use each destination. Filter by time period.

### Kill Switch

Global pause/resume for all agents, or per-agent control. Safe mode restricts agents to read-only operations — blocking writes, edits, and file creation while still allowing reads and queries.

### Weekly Reports

Generates a summary of agent activity over the past week: action counts by day and agent, cost trends with week-over-week comparison, security score, PII findings, Safety Net statistics, and Data Shield domain breakdowns. Exportable as a self-contained HTML file saved to your Desktop and opened in your browser.

### Onboarding

Guided setup wizard that auto-discovers installed agents, walks through all features, and gets you running in under a minute.

---

## How It Works

Unalome Agent Firewall runs locally on your machine. It never sends your data anywhere. There is no cloud, no account, no telemetry.

It watches the configuration files and activity logs of your AI agents, scans them for risks, and presents everything in a dashboard designed for humans — not just engineers.

---

## Tech Stack

**Frontend:** React 18, TypeScript, Tailwind CSS, Recharts, Lucide icons

**Backend:** Rust, Tauri 2, Tokio, SQLite (via sqlx), Serde, Regex, Chrono

**Storage:** Local SQLite database at `~/Library/Application Support/Unalome/`

---

## Architecture

```
src/                    # React frontend
  components/           # Feature views (Timeline, PII Guardian, etc.)
  types/                # TypeScript interfaces
  lib/                  # Utilities

src-tauri/              # Rust backend
  src/
    main.rs             # Tauri commands and app setup
    database.rs         # SQLite data layer
    models.rs           # Shared data structures
    discovery.rs        # Agent auto-detection
    parsers/            # Per-agent log parsers
      claude_code.rs    # Claude Code JSONL parser
      cursor.rs         # Cursor SQLite parser
      openclaw.rs       # OpenClaw JSONL parser
      claude_desktop.rs # Claude Desktop log parser
      pricing.rs        # Token pricing for 40+ models
    pii/                # PII pattern detection engine
    safety_net/         # File snapshot and restore engine
    data_shield/        # Network monitoring and domain classification
    scanner.rs          # MCP security scanning
    reports/            # Weekly report generation and HTML export
```

---

## Development

### Prerequisites

- Node.js 18+
- Rust 1.75+
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)

### Setup

```bash
npm install
npm run tauri dev
```

### Build (unsigned)

```bash
npm run tauri build
```

This produces an unsigned `.app` and `.dmg` in `src-tauri/target/release/bundle/`.

### Build (signed + notarized for macOS)

Signed builds are required for distribution. macOS will block unsigned apps with Gatekeeper.

**1. Set up signing credentials**

Copy the example file and fill in your values:

```bash
cp src-tauri/.env.signing.example src-tauri/.env.signing
```

Edit `src-tauri/.env.signing` with:

| Variable | Description |
|----------|-------------|
| `APPLE_SIGNING_IDENTITY` | Your Developer ID certificate. Run `security find-identity -v -p codesigning` to list available identities. |
| `APPLE_ID` | Your Apple ID email (used for notarization). |
| `APPLE_PASSWORD` | An app-specific password generated at [appleid.apple.com](https://appleid.apple.com/account/manage) under Sign-In and Security. |
| `APPLE_TEAM_ID` | Your 10-character Apple Developer Team ID. |

> **Important:** `src-tauri/.env.signing` contains secrets and is git-ignored. Never commit it.

**2. Build**

```bash
source src-tauri/.env.signing
npm run tauri build
```

This will:
1. Compile the Rust backend and React frontend
2. Bundle into `Unalome Firewall.app`
3. Code sign with your Developer ID certificate
4. Submit to Apple for notarization
5. Staple the notarization ticket to the app
6. Create a signed `.dmg` installer

---

## Contributing

We welcome contributions of all kinds — code, documentation, design, translations, and ideas.

**Good first contributions:**

- Add a new agent parser (discovery + log parsing)
- Add PII patterns for your country (IBAN, CPF, national ID formats)
- Improve the UI for accessibility
- Translate the interface to your language

---

## Why Open Source?

The unalome symbol represents the path from confusion to clarity. We believe the most important tools for understanding AI should be shared openly — not locked behind subscriptions or corporate walls.

---

## License

Apache 2.0 — free forever, for everyone.
