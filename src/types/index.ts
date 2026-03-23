export type AgentType =
  | "ClaudeCode"
  | "ClaudeDesktop"
  | "Cursor"
  | "Windsurf"
  | "OpenClaw"
  | "CustomMCP"
  | { Other: string };

export type AgentStatus = "Active" | "Paused" | "Offline" | "Unknown";

export interface Agent {
  id: string;
  name: string;
  agent_type: AgentType;
  status: AgentStatus;
  config_path: string | null;
  last_seen: string;
  metadata: Record<string, unknown>;
}

export type ActionType =
  | { ToolCall: { tool_name: string; args: Record<string, unknown> } }
  | { FileAccess: { path: string; operation: string } }
  | { NetworkRequest: { url: string; method: string } }
  | { Message: { content: string } }
  | { ApiCall: { endpoint: string } }
  | { Other: string };

export type RiskLevel = "Safe" | "Low" | "Medium" | "High" | "Critical";

export interface CostInfo {
  tokens_input: number;
  tokens_output: number;
  cache_write_tokens: number;
  cache_read_tokens: number;
  estimated_cost_usd: number;
}

export interface Action {
  id: string;
  agent_id: string;
  action_type: ActionType;
  timestamp: string;
  description: string;
  risk_level: RiskLevel;
  cost: CostInfo | null;
  metadata: Record<string, unknown>;
}

export interface ToolSecurityInfo {
  name: string;
  description: string;
  risk_level: RiskLevel;
  permissions: string[];
  warnings: string[];
}

export interface SecurityWarning {
  severity: RiskLevel;
  message: string;
  category: string;
}

export interface SecurityReport {
  server_name: string;
  scan_timestamp: string;
  overall_risk: RiskLevel;
  tools_scanned: ToolSecurityInfo[];
  warnings: SecurityWarning[];
  command: string | null;
  args: string[];
  env_vars: string[];
  source_agent: string | null;
}

export interface DashboardStats {
  totalAgents: number;
  activeAgents: number;
  totalActions: number;
  totalCost: number;
  highRiskEvents: number;
}

export type PiiCategory = "api_key" | "private_key" | "jwt" | "connection_string" | "ssn" | "credit_card" | "email" | "phone" | "ip_address" | "password" | "env_variable";

export interface PiiFinding {
  id: string;
  action_id: string | null;
  agent_id: string;
  finding_type: string;
  severity: string;
  description: string;
  source_file: string | null;
  source_context: string;
  redacted_value: string;
  recommended_action: string;
  timestamp: string;
  dismissed: boolean;
}

export interface PiiStats {
  total: number;
  by_severity: Record<string, number>;
  by_type: Record<string, number>;
  by_agent: Record<string, number>;
  today: number;
  this_week: number;
}

export interface ProtectedFile {
  id: string;
  original_path: string;
  snapshot_path: string;
  file_size: number;
  agent_id: string;
  agent_name: string;
  action_type: string;
  created_at: string;
  restored: boolean;
}

export interface RestoreResult {
  success: boolean;
  original_path: string;
  backup_of_current: string | null;
  message: string;
}

export interface SafetyNetStats {
  total_files: number;
  files_today: number;
  total_storage_bytes: number;
  storage_limit_bytes: number;
  oldest_snapshot: string | null;
  newest_snapshot: string | null;
  by_agent: Record<string, number>;
  restored_today: number;
}

// ── Data Shield ────────────────────────────────────────────────────

export interface OutboundEvent {
  id: string;
  agent_id: string;
  agent_name: string;
  event_type: string;
  destination: string;
  url: string | null;
  direction: string;
  description: string;
  risk_level: string;
  timestamp: string;
  blocked: boolean;
}

export interface DomainProfile {
  domain: string;
  first_seen: string;
  last_seen: string;
  total_events: number;
  risk_level: string;
  category: string;
  agents_using: string[];
}

export interface DataShieldStats {
  total_events: number;
  events_today: number;
  unique_domains: number;
  trusted_domains: number;
  unknown_domains: number;
  suspicious_domains: number;
  by_agent: Record<string, number>;
}

// ── Weekly Reports ─────────────────────────────────────────────────

export interface AgentActionSummary {
  agent_name: string;
  agent_type: string;
  action_count: number;
  cost: number;
  top_action_type: string;
}

export interface WeeklyReport {
  id: string;
  week_start: string;
  week_end: string;
  generated_at: string;
  total_actions: number;
  actions_by_agent: AgentActionSummary[];
  actions_by_type: [string, number][];
  actions_by_day: [string, number][];
  busiest_day: string;
  busiest_hour: number;
  total_cost: number;
  cost_by_agent: [string, number][];
  cost_trend: string;
  cost_by_day: [string, number][];
  files_protected: number;
  files_restored: number;
  safety_net_size_mb: number;
  pii_findings: number;
  pii_critical: number;
  new_mcp_servers: number;
  security_score: number;
  domains_contacted: number;
  unknown_domains: number;
  outbound_events: number;
  prev_week_actions: number | null;
  prev_week_cost: number | null;
}

// ── Firewall ──────────────────────────────────────────────────────

export type ConditionType = "Block" | "MaxAmount" | "ArgContains" | "PathRestriction";

export interface RuleCondition {
  tool_pattern: string;
  condition_type: ConditionType;
  value: string;
}

export interface FirewallRule {
  id: string;
  name: string;
  description: string;
  agent_pattern: string;
  allow_tools: string[];
  deny_tools: string[];
  conditions: RuleCondition[];
  priority: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export type DecisionType = "Allowed" | "Blocked" | "Flagged";

export interface FirewallDecision {
  id: string;
  action_id: string;
  timestamp: string;
  agent_id: string;
  agent_name: string;
  tool_name: string;
  mcp_server: string | null;
  arguments: Record<string, unknown>;
  decision: DecisionType;
  reason: string;
  rule_id: string | null;
  rule_name: string | null;
}

export interface FirewallStats {
  total_rules: number;
  active_rules: number;
  total_decisions: number;
  decisions_today: number;
  blocked_today: number;
  flagged_today: number;
  allowed_today: number;
  top_blocked_tools: [string, number][];
  by_agent: Record<string, number>;
}

export interface WeeklyReportSummary {
  id: string;
  week_start: string;
  week_end: string;
  generated_at: string;
  total_actions: number;
  total_cost: number;
  security_score: number;
}
