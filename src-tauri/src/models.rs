use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub agent_type: AgentType,
    pub status: AgentStatus,
    pub config_path: Option<String>,
    pub last_seen: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    ClaudeCode,
    ClaudeDesktop,
    Cursor,
    Windsurf,
    OpenClaw,
    CustomMCP,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Paused,
    Offline,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub agent_id: String,
    pub action_type: ActionType,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub risk_level: RiskLevel,
    pub cost: Option<CostInfo>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    ToolCall { tool_name: String, args: serde_json::Value },
    FileAccess { path: String, operation: String },
    NetworkRequest { url: String, method: String },
    Message { content: String },
    ApiCall { endpoint: String },
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostInfo {
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub cache_write_tokens: u64,
    pub cache_read_tokens: u64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub server_name: String,
    pub scan_timestamp: DateTime<Utc>,
    pub overall_risk: RiskLevel,
    pub tools_scanned: Vec<ToolSecurityInfo>,
    pub warnings: Vec<SecurityWarning>,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env_vars: Vec<String>,
    pub source_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSecurityInfo {
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub permissions: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityWarning {
    pub severity: RiskLevel,
    pub message: String,
    pub category: WarningCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlan {
    pub id: String,
    pub file_name: String,
    pub slug: String,
    pub file_path: String,
    pub display_name: String,
    pub title: Option<String>,
    pub file_size: u64,
    pub created_at: String,
    pub modified_at: String,
    pub content: String,
    pub action_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningCategory {
    PromptInjection,
    ExcessivePermissions,
    DataExfiltration,
    SuspiciousPattern,
    ConfigIssue,
}
