use crate::models::{Action, ActionType};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── Data Structures ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundEvent {
    pub id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub event_type: String,
    pub destination: String,
    pub url: Option<String>,
    pub direction: String,
    pub description: String,
    pub risk_level: String,
    pub timestamp: String,
    pub blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainProfile {
    pub domain: String,
    pub first_seen: String,
    pub last_seen: String,
    pub total_events: i64,
    pub risk_level: String,
    pub category: String,
    pub agents_using: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataShieldStats {
    pub total_events: i64,
    pub events_today: i64,
    pub unique_domains: i64,
    pub trusted_domains: i64,
    pub unknown_domains: i64,
    pub suspicious_domains: i64,
    pub by_agent: HashMap<String, i64>,
}

// ── Built-in Domain Classification ─────────────────────────────────

struct DomainEntry {
    pattern: &'static str,
    category: &'static str,
}

const SAFE_DOMAINS: &[DomainEntry] = &[
    // AI Providers
    DomainEntry { pattern: "api.openai.com", category: "ai_provider" },
    DomainEntry { pattern: "api.anthropic.com", category: "ai_provider" },
    DomainEntry { pattern: "api.cohere.com", category: "ai_provider" },
    DomainEntry { pattern: "generativelanguage.googleapis.com", category: "ai_provider" },
    DomainEntry { pattern: "api.mistral.ai", category: "ai_provider" },
    DomainEntry { pattern: "api.groq.com", category: "ai_provider" },
    DomainEntry { pattern: "api.together.xyz", category: "ai_provider" },
    DomainEntry { pattern: "api.replicate.com", category: "ai_provider" },
    // Cloud Services
    DomainEntry { pattern: ".amazonaws.com", category: "cloud_service" },
    DomainEntry { pattern: ".googleapis.com", category: "cloud_service" },
    DomainEntry { pattern: ".azure.com", category: "cloud_service" },
    DomainEntry { pattern: ".azurewebsites.net", category: "cloud_service" },
    DomainEntry { pattern: "github.com", category: "cloud_service" },
    DomainEntry { pattern: "api.github.com", category: "cloud_service" },
    DomainEntry { pattern: "gitlab.com", category: "cloud_service" },
    DomainEntry { pattern: "raw.githubusercontent.com", category: "cloud_service" },
    // Package Registries
    DomainEntry { pattern: "registry.npmjs.org", category: "package_registry" },
    DomainEntry { pattern: "pypi.org", category: "package_registry" },
    DomainEntry { pattern: "crates.io", category: "package_registry" },
    DomainEntry { pattern: "rubygems.org", category: "package_registry" },
    DomainEntry { pattern: "maven.org", category: "package_registry" },
    DomainEntry { pattern: "packagist.org", category: "package_registry" },
    // Search Engines
    DomainEntry { pattern: "google.com", category: "search_engine" },
    DomainEntry { pattern: "bing.com", category: "search_engine" },
    DomainEntry { pattern: "duckduckgo.com", category: "search_engine" },
    // Documentation
    DomainEntry { pattern: "docs.rs", category: "documentation" },
    DomainEntry { pattern: "developer.mozilla.org", category: "documentation" },
    DomainEntry { pattern: "stackoverflow.com", category: "documentation" },
    DomainEntry { pattern: "wikipedia.org", category: "documentation" },
];

// ── Engine ──────────────────────────────────────────────────────────

pub struct DataShieldEngine {
    url_regex: Regex,
    bash_network_regex: Regex,
    pub user_classifications: HashMap<String, String>,
}

impl DataShieldEngine {
    pub fn new() -> Self {
        Self {
            url_regex: Regex::new(r#"https?://[^\s"'>\]\)}{]+"#).unwrap(),
            bash_network_regex: Regex::new(
                r"\b(curl|wget|fetch|http|ssh|scp|nc|ncat|netcat|rsync)\b"
            ).unwrap(),
            user_classifications: HashMap::new(),
        }
    }

    /// Main entry point: analyze an action for outbound network indicators.
    pub fn analyze_action(&self, action: &Action, agent_name: &str) -> Vec<OutboundEvent> {
        let mut events = Vec::new();
        let now = chrono::Utc::now().to_rfc3339();

        match &action.action_type {
            ActionType::ToolCall { tool_name, args } => {
                // WebFetch / WebSearch — direct URL extraction
                if tool_name == "WebFetch" || tool_name == "WebSearch" {
                    if let Some(url) = args.get("url").and_then(|v| v.as_str()) {
                        if let Some(domain) = self.extract_domain(url) {
                            let (risk, category) = self.classify_domain(&domain);
                            events.push(OutboundEvent {
                                id: Uuid::new_v4().to_string(),
                                agent_id: action.agent_id.clone(),
                                agent_name: agent_name.to_string(),
                                event_type: "tool_web_request".to_string(),
                                destination: domain.clone(),
                                url: Some(url.to_string()),
                                direction: "outbound".to_string(),
                                description: format!(
                                    "{} {} {}",
                                    agent_name,
                                    if tool_name == "WebFetch" { "fetched" } else { "searched" },
                                    domain
                                ),
                                risk_level: risk,
                                timestamp: now.clone(),
                                blocked: false,
                            });
                            let _ = category; // used in classify_domain
                        }
                    }
                    // WebSearch may have a query but no URL — check for URLs in query
                    if tool_name == "WebSearch" {
                        if let Some(query) = args.get("query").and_then(|v| v.as_str()) {
                            for url in self.extract_urls(query) {
                                if let Some(domain) = self.extract_domain(&url) {
                                    let (risk, _) = self.classify_domain(&domain);
                                    events.push(OutboundEvent {
                                        id: Uuid::new_v4().to_string(),
                                        agent_id: action.agent_id.clone(),
                                        agent_name: agent_name.to_string(),
                                        event_type: "tool_web_request".to_string(),
                                        destination: domain.clone(),
                                        url: Some(url),
                                        direction: "outbound".to_string(),
                                        description: format!("{} searched for {}", agent_name, domain),
                                        risk_level: risk,
                                        timestamp: now.clone(),
                                        blocked: false,
                                    });
                                }
                            }
                        }
                    }
                }
                // Bash — scan command for network tools and URLs
                else if tool_name == "Bash" {
                    if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                        if self.bash_network_regex.is_match(cmd) {
                            let urls = self.extract_urls(cmd);
                            if urls.is_empty() {
                                // Network tool detected but no URL — flag generically
                                events.push(OutboundEvent {
                                    id: Uuid::new_v4().to_string(),
                                    agent_id: action.agent_id.clone(),
                                    agent_name: agent_name.to_string(),
                                    event_type: "bash_network".to_string(),
                                    destination: "unknown".to_string(),
                                    url: None,
                                    direction: "outbound".to_string(),
                                    description: format!("{} ran a network command", agent_name),
                                    risk_level: "unknown".to_string(),
                                    timestamp: now.clone(),
                                    blocked: false,
                                });
                            } else {
                                for url in urls {
                                    if let Some(domain) = self.extract_domain(&url) {
                                        let (risk, _) = self.classify_domain(&domain);
                                        events.push(OutboundEvent {
                                            id: Uuid::new_v4().to_string(),
                                            agent_id: action.agent_id.clone(),
                                            agent_name: agent_name.to_string(),
                                            event_type: "bash_network".to_string(),
                                            destination: domain.clone(),
                                            url: Some(url),
                                            direction: "outbound".to_string(),
                                            description: format!("{} contacted {}", agent_name, domain),
                                            risk_level: risk,
                                            timestamp: now.clone(),
                                            blocked: false,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                // MCP tool calls (mcp__ prefix)
                else if tool_name.starts_with("mcp__") {
                    // Extract the server name from mcp__<server>__<tool>
                    let parts: Vec<&str> = tool_name.splitn(3, "__").collect();
                    let server_name = if parts.len() >= 2 { parts[1] } else { tool_name.as_str() };

                    events.push(OutboundEvent {
                        id: Uuid::new_v4().to_string(),
                        agent_id: action.agent_id.clone(),
                        agent_name: agent_name.to_string(),
                        event_type: "mcp_remote".to_string(),
                        destination: server_name.to_string(),
                        url: None,
                        direction: "outbound".to_string(),
                        description: format!("{} used MCP server '{}'", agent_name, server_name),
                        risk_level: "unknown".to_string(),
                        timestamp: now.clone(),
                        blocked: false,
                    });
                }

                // Any tool with "url" or "endpoint" in args
                if tool_name != "WebFetch" && tool_name != "WebSearch" && tool_name != "Bash" {
                    for (key, val) in args.as_object().into_iter().flatten() {
                        if key == "url" || key == "endpoint" {
                            if let Some(url_str) = val.as_str() {
                                if let Some(domain) = self.extract_domain(url_str) {
                                    let (risk, _) = self.classify_domain(&domain);
                                    events.push(OutboundEvent {
                                        id: Uuid::new_v4().to_string(),
                                        agent_id: action.agent_id.clone(),
                                        agent_name: agent_name.to_string(),
                                        event_type: "tool_web_request".to_string(),
                                        destination: domain.clone(),
                                        url: Some(url_str.to_string()),
                                        direction: "outbound".to_string(),
                                        description: format!("{} called {} targeting {}", agent_name, tool_name, domain),
                                        risk_level: risk,
                                        timestamp: now.clone(),
                                        blocked: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            ActionType::NetworkRequest { url, .. } => {
                if let Some(domain) = self.extract_domain(url) {
                    let (risk, _) = self.classify_domain(&domain);
                    events.push(OutboundEvent {
                        id: Uuid::new_v4().to_string(),
                        agent_id: action.agent_id.clone(),
                        agent_name: agent_name.to_string(),
                        event_type: "tool_web_request".to_string(),
                        destination: domain.clone(),
                        url: Some(url.clone()),
                        direction: "outbound".to_string(),
                        description: format!("{} made a network request to {}", agent_name, domain),
                        risk_level: risk,
                        timestamp: now.clone(),
                        blocked: false,
                    });
                }
            }
            _ => {}
        }

        events
    }

    /// Extract URLs from arbitrary text.
    pub fn extract_urls(&self, text: &str) -> Vec<String> {
        self.url_regex
            .find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// Parse a URL to extract its hostname/domain.
    pub fn extract_domain(&self, url: &str) -> Option<String> {
        // Handle protocol-prefixed URLs
        if let Some(after_proto) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) {
            let host_part = after_proto.split('/').next()?;
            let host = host_part.split(':').next()?;
            if !host.is_empty() {
                return Some(host.to_lowercase());
            }
        }
        None
    }

    /// Classify a domain as (risk_level, category).
    pub fn classify_domain(&self, domain: &str) -> (String, String) {
        // User overrides take priority
        if let Some(classification) = self.user_classifications.get(domain) {
            let risk = match classification.as_str() {
                "trusted" => "safe",
                "suspicious" => "suspicious",
                _ => "unknown",
            };
            return (risk.to_string(), classification.clone());
        }

        // Check built-in list
        for entry in SAFE_DOMAINS {
            if entry.pattern.starts_with('.') {
                // Suffix match (e.g., .amazonaws.com)
                if domain.ends_with(entry.pattern) || domain == &entry.pattern[1..] {
                    return ("safe".to_string(), entry.category.to_string());
                }
            } else {
                // Exact match
                if domain == entry.pattern {
                    return ("safe".to_string(), entry.category.to_string());
                }
            }
        }

        // Localhost is always safe
        if domain == "localhost" || domain == "127.0.0.1" || domain == "0.0.0.0" || domain.ends_with(".local") {
            return ("safe".to_string(), "local".to_string());
        }

        ("unknown".to_string(), "unknown".to_string())
    }

    /// User override: mark a domain as trusted or suspicious.
    pub fn classify_domain_manual(&mut self, domain: &str, classification: &str) {
        self.user_classifications
            .insert(domain.to_string(), classification.to_string());
    }

    /// Scan MCP config files for remote server indicators.
    pub fn scan_mcp_configs(&self, agents: &[crate::models::Agent]) -> Vec<OutboundEvent> {
        let mut events = Vec::new();
        let now = chrono::Utc::now().to_rfc3339();

        for agent in agents {
            let mcp_path = agent
                .metadata
                .get("mcp_config_path")
                .and_then(|v| v.as_str());

            let Some(path_str) = mcp_path else { continue };
            let path = std::path::Path::new(path_str);
            if !path.exists() {
                continue;
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let config: serde_json::Value = match serde_json::from_str(&content) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let servers = config
                .get("mcpServers")
                .or_else(|| config.get("servers"));

            let Some(servers_obj) = servers.and_then(|s| s.as_object()) else {
                continue;
            };

            for (name, server_config) in servers_obj {
                let is_remote = self.is_remote_mcp_server(server_config);
                if is_remote {
                    let url = server_config
                        .get("url")
                        .or_else(|| server_config.get("command"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let domain = self.extract_domain(url).unwrap_or_else(|| name.clone());
                    let (risk, _) = self.classify_domain(&domain);

                    events.push(OutboundEvent {
                        id: Uuid::new_v4().to_string(),
                        agent_id: agent.id.clone(),
                        agent_name: agent.name.clone(),
                        event_type: "mcp_remote".to_string(),
                        destination: domain,
                        url: Some(url.to_string()),
                        direction: "outbound".to_string(),
                        description: format!(
                            "{} has remote MCP server '{}'",
                            agent.name, name
                        ),
                        risk_level: risk,
                        timestamp: now.clone(),
                        blocked: false,
                    });
                }
            }
        }

        events
    }

    fn is_remote_mcp_server(&self, config: &serde_json::Value) -> bool {
        // Check for URL-based servers (SSE, streamable HTTP)
        if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            if url.starts_with("http://") || url.starts_with("https://") {
                // localhost is not remote
                if let Some(domain) = self.extract_domain(url) {
                    if domain == "localhost" || domain == "127.0.0.1" || domain == "0.0.0.0" {
                        return false;
                    }
                }
                return true;
            }
        }

        // Check command args for URLs
        if let Some(args) = config.get("args").and_then(|v| v.as_array()) {
            for arg in args {
                if let Some(s) = arg.as_str() {
                    if (s.starts_with("http://") || s.starts_with("https://"))
                        && !s.contains("localhost")
                        && !s.contains("127.0.0.1")
                    {
                        return true;
                    }
                }
            }
        }

        // Check env values for API URLs
        if let Some(env) = config.get("env").and_then(|v| v.as_object()) {
            for (key, val) in env {
                if let Some(s) = val.as_str() {
                    if (key.contains("URL") || key.contains("ENDPOINT") || key.contains("HOST"))
                        && (s.starts_with("http://") || s.starts_with("https://"))
                        && !s.contains("localhost")
                        && !s.contains("127.0.0.1")
                    {
                        return true;
                    }
                }
            }
        }

        false
    }
}
