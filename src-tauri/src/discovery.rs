use crate::models::{Agent, AgentStatus, AgentType};
use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use std::path::PathBuf;

pub struct AgentDiscovery {
    home_dir: PathBuf,
}

impl AgentDiscovery {
    pub fn new() -> Self {
        Self {
            home_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from("~")),
        }
    }

    pub async fn discover_all(&self) -> Result<Vec<Agent>> {
        let mut agents = Vec::new();

        // Discover Claude Code
        if let Some(agent) = self.discover_claude_code().await? {
            agents.push(agent);
        }

        // Discover Claude Desktop
        if let Some(agent) = self.discover_claude_desktop().await? {
            agents.push(agent);
        }

        // Discover Cursor
        if let Some(agent) = self.discover_cursor().await? {
            agents.push(agent);
        }

        // Discover Windsurf
        if let Some(agent) = self.discover_windsurf().await? {
            agents.push(agent);
        }

        // Discover OpenClaw
        if let Some(agent) = self.discover_openclaw().await? {
            agents.push(agent);
        }

        // Discover MCP configs
        let mcp_agents = self.discover_mcp_configs().await?;
        agents.extend(mcp_agents);

        Ok(agents)
    }

    async fn discover_claude_code(&self) -> Result<Option<Agent>> {
        let claude_dir = self.home_dir.join(".claude");
        let projects_dir = claude_dir.join("projects");

        if !claude_dir.exists() {
            return Ok(None);
        }

        // Check for recent activity by looking at project directories
        let has_activity = if projects_dir.exists() {
            projects_dir.read_dir()?.next().is_some()
        } else {
            false
        };

        Ok(Some(Agent {
            id: "claude-code".to_string(),
            name: "Claude Code".to_string(),
            agent_type: AgentType::ClaudeCode,
            status: if has_activity {
                AgentStatus::Active
            } else {
                AgentStatus::Offline
            },
            config_path: Some(claude_dir.to_string_lossy().to_string()),
            last_seen: Utc::now(),
            metadata: json!({
                "projects_dir": projects_dir.to_string_lossy().to_string(),
                "has_projects": has_activity,
            }),
        }))
    }

    async fn discover_claude_desktop(&self) -> Result<Option<Agent>> {
        let config_paths = [
            self.home_dir
                .join("Library/Application Support/Claude"),
            self.home_dir.join(".config/Claude"),
        ];

        for config_path in &config_paths {
            if config_path.exists() {
                let mcp_config = config_path.join("claude_desktop_config.json");
                let has_mcp = mcp_config.exists();

                return Ok(Some(Agent {
                    id: "claude-desktop".to_string(),
                    name: "Claude Desktop".to_string(),
                    agent_type: AgentType::ClaudeDesktop,
                    status: AgentStatus::Active,
                    config_path: Some(config_path.to_string_lossy().to_string()),
                    last_seen: Utc::now(),
                    metadata: json!({
                        "mcp_config_path": mcp_config.to_string_lossy().to_string(),
                        "has_mcp_servers": has_mcp,
                    }),
                }));
            }
        }

        Ok(None)
    }

    async fn discover_cursor(&self) -> Result<Option<Agent>> {
        let cursor_dir = self.home_dir.join(".cursor");
        let mcp_config = cursor_dir.join("mcp.json");

        if !cursor_dir.exists() {
            return Ok(None);
        }

        Ok(Some(Agent {
            id: "cursor".to_string(),
            name: "Cursor".to_string(),
            agent_type: AgentType::Cursor,
            status: AgentStatus::Active,
            config_path: Some(cursor_dir.to_string_lossy().to_string()),
            last_seen: Utc::now(),
            metadata: json!({
                "mcp_config_path": mcp_config.to_string_lossy().to_string(),
                "has_mcp_servers": mcp_config.exists(),
            }),
        }))
    }

    async fn discover_windsurf(&self) -> Result<Option<Agent>> {
        let windsurf_dir = self.home_dir.join(".windsurf");
        let mcp_config = windsurf_dir.join("mcp.json");

        if !windsurf_dir.exists() {
            return Ok(None);
        }

        Ok(Some(Agent {
            id: "windsurf".to_string(),
            name: "Windsurf".to_string(),
            agent_type: AgentType::Windsurf,
            status: AgentStatus::Active,
            config_path: Some(windsurf_dir.to_string_lossy().to_string()),
            last_seen: Utc::now(),
            metadata: json!({
                "mcp_config_path": mcp_config.to_string_lossy().to_string(),
                "has_mcp_servers": mcp_config.exists(),
            }),
        }))
    }

    async fn discover_openclaw(&self) -> Result<Option<Agent>> {
        let openclaw_dir = self.home_dir.join(".openclaw");
        let config_file = openclaw_dir.join("openclaw.json");

        if !openclaw_dir.exists() {
            return Ok(None);
        }

        Ok(Some(Agent {
            id: "openclaw".to_string(),
            name: "OpenClaw".to_string(),
            agent_type: AgentType::OpenClaw,
            status: AgentStatus::Active,
            config_path: Some(config_file.to_string_lossy().to_string()),
            last_seen: Utc::now(),
            metadata: json!({
                "config_exists": config_file.exists(),
            }),
        }))
    }

    async fn discover_mcp_configs(&self) -> Result<Vec<Agent>> {
        let mut agents = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        // Common MCP config locations
        let search_paths = [
            self.home_dir.join(".vscode/mcp.json"),
            self.home_dir.join(".cursor/mcp.json"),
            self.home_dir.join(".windsurf/mcp.json"),
            self.home_dir.join(".claude/mcp.json"),
        ];

        for path in &search_paths {
            if path.exists() && seen_paths.insert(path.clone()) {
                let content = tokio::fs::read_to_string(path).await?;
                let config: serde_json::Value = serde_json::from_str(&content)
                    .unwrap_or_else(|_| json!({}));

                if let Some(servers) = config.get("mcpServers").or(config.get("servers")) {
                    if let Some(obj) = servers.as_object() {
                        for (name, server_config) in obj {
                            agents.push(Agent {
                                id: format!("mcp-{}", name),
                                name: name.clone(),
                                agent_type: AgentType::CustomMCP,
                                status: AgentStatus::Active,
                                config_path: Some(path.to_string_lossy().to_string()),
                                last_seen: Utc::now(),
                                metadata: json!({
                                    "server_config": server_config,
                                    "source": path.to_string_lossy().to_string(),
                                }),
                            });
                        }
                    }
                }
            }
        }

        Ok(agents)
    }

}
