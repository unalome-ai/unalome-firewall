# Changelog

All notable changes to Unalome Agent Firewall will be documented in this file.

## [0.3.0] - 2026-03-26

### Added

- **Agent Plans Tab** — Browse implementation plans written by Claude Code during plan mode (`~/.claude/plans/*.md`). Plans are scanned from disk, cached in SQLite, and displayed with full markdown rendering (tables, code blocks, headings).
- **Plan-Action Linking** — Every captured action now carries a `slug` field from its JSONL record. Actions are linked back to the plan they belong to, with action counts shown on each plan card and a collapsible "Linked Actions" section in the detail view.
- **Plan Detail View** — Click any plan to see its full markdown content rendered with `react-markdown` + `remark-gfm`, along with metadata (slug, file size, last modified, action count) and a timeline of all tool calls executed during that plan.
- **Real-Time Plan Updates** — When Claude Code writes to `~/.claude/plans/`, a `plan-updated` event is emitted and the Plans tab refreshes automatically.

### Technical

- `ClaudeCodeParser` now extracts `slug`, `permissionMode`, and `sessionId` into action metadata for both tool calls and system records.
- New `AgentPlan` Rust model and `agent_plans` SQLite table with indexes on `slug` and `modified_at`.
- Four new Tauri commands: `scan_agent_plans`, `get_agent_plans`, `get_plan_actions`, `get_agent_plan_content`.
- New `AgentPlan` TypeScript interface.
- Added `react-markdown`, `remark-gfm`, and `@tailwindcss/typography` dependencies.

## [0.2.0] - 2026-03-23

### Added

- **PII Guardian: Category Toggles** — Enable or disable detection for individual PII categories (API keys, emails, passwords, etc.). Settings persist across sessions.
- **PII Guardian: Color-Coded Annotations** — Detected values in the source context are now highlighted with category-specific colors for instant visual identification.
- **Firewall: Predefined Rules** — Three default firewall rules are seeded on first launch:
  - Block destructive commands (`rm` in Bash)
  - Block data exfiltration via `curl`/`wget`
  - Flag writes to system directories (`/etc`, `/usr`, `/System`)
- **Custom DMG Installer** — macOS installer now features a branded background image.
- **App Branding** — "Unalome Agent Firewall v0.2.0" displayed in the window title and dashboard overview.

### Changed

- Dashboard overview reduced from 9 to 8 cards (removed Reports card; Reports still accessible from sidebar).
- PiiScanner is now shared state (`Arc<Mutex<PiiScanner>>`) for runtime category toggling.

### Technical

- New `pii_category_settings` SQLite table for persisting category enable/disable preferences.
- `PiiScanner` gains `disabled_categories: HashSet<String>` field with `set_disabled_categories()` method.
- Two new Tauri commands: `get_pii_category_settings`, `set_pii_category_enabled`.
- New TypeScript type: `PiiCategory`.

## [0.1.0] - 2026-03-16

### Added

- Initial release
- Agent discovery (Claude Code, Cursor, Windsurf, and more)
- Real-time action timeline with filtering
- Security dashboard with MCP server scanning
- Cost tracker with token usage breakdown
- PII Guardian — scans for 13 categories of sensitive data
- Safety Net — auto-backup before agent file modifications
- Data Shield — outbound connection monitoring
- Kill Switch — pause any agent instantly
- Weekly reports with HTML export
- Firewall — rule-based tool call filtering
