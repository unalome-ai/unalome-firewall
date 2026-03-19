use crate::models::{RiskLevel, SecurityReport, SecurityWarning, ToolSecurityInfo, WarningCategory};
use anyhow::Result;
use chrono::Utc;
use regex::Regex;

pub struct SecurityScanner {
    prompt_injection_patterns: Vec<Regex>,
    suspicious_patterns: Vec<Regex>,
}

impl SecurityScanner {
    pub fn new() -> Self {
        Self {
            prompt_injection_patterns: vec![
                Regex::new(r"(?i)ignore previous instructions").unwrap(),
                Regex::new(r"(?i)system prompt").unwrap(),
                Regex::new(r"(?i)you are now").unwrap(),
                Regex::new(r"<[^>]*>(?i)(?:system|prompt|instruction)").unwrap(),
            ],
            suspicious_patterns: vec![
                Regex::new(r"(?i)(?:password|token|key|secret|credential)").unwrap(),
                Regex::new(r"(?i)(?:exfiltrat|send.*data|upload.*file)").unwrap(),
                Regex::new(r"(?i)(?:exec|eval|system|shell|bash|sh\s+-c)").unwrap(),
            ],
        }
    }

    pub async fn scan_mcp_server(
        &self,
        server_config: serde_json::Value,
    ) -> Result<SecurityReport> {
        let server_name = server_config
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        let mut tools = Vec::new();
        let mut warnings = Vec::new();

        // Extract command, args, env for display
        let command = server_config
            .get("command")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        let args: Vec<String> = server_config
            .get("args")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let env_vars: Vec<String> = server_config
            .get("env")
            .and_then(|e| e.as_object())
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        // Parse tools from the config (if present — some configs list them)
        if let Some(tools_array) = server_config.get("tools").and_then(|t| t.as_array()) {
            for tool in tools_array {
                let tool_info = self.analyze_tool(tool).await?;
                tools.push(tool_info);
            }
        }

        // ── Command analysis ──────────────────────────────────────────
        if let Some(cmd) = &command {
            let cmd_lower = cmd.to_lowercase();
            // Interpreted runtime warning
            if cmd_lower.contains("node") || cmd_lower.contains("python") || cmd_lower.contains("deno") || cmd_lower.contains("bun") {
                warnings.push(SecurityWarning {
                    severity: RiskLevel::Medium,
                    message: format!("Uses interpreted runtime '{}' which may have broad system access", cmd),
                    category: WarningCategory::ExcessivePermissions,
                });
            }
            // Direct shell execution
            if cmd_lower == "bash" || cmd_lower == "sh" || cmd_lower == "/bin/bash" || cmd_lower == "/bin/sh" {
                warnings.push(SecurityWarning {
                    severity: RiskLevel::High,
                    message: "Runs a raw shell — can execute arbitrary commands".to_string(),
                    category: WarningCategory::ExcessivePermissions,
                });
            }
        }

        // ── Args analysis ─────────────────────────────────────────────
        for arg in &args {
            let arg_lower = arg.to_lowercase();
            // Check for --allow-all or overly permissive flags
            if arg_lower.contains("--allow-all") || arg_lower.contains("--no-sandbox") || arg_lower.contains("--disable-security") {
                warnings.push(SecurityWarning {
                    severity: RiskLevel::High,
                    message: format!("Permissive flag '{}' disables security restrictions", arg),
                    category: WarningCategory::ExcessivePermissions,
                });
            }
            // Check for suspicious URLs in args
            if arg_lower.starts_with("http://") {
                warnings.push(SecurityWarning {
                    severity: RiskLevel::Low,
                    message: format!("Uses insecure HTTP connection: {}", arg),
                    category: WarningCategory::ConfigIssue,
                });
            }
            // Detect if running with -e / --eval (code injection vector)
            if arg == "-e" || arg == "--eval" {
                warnings.push(SecurityWarning {
                    severity: RiskLevel::High,
                    message: "Uses eval flag — can execute inline code".to_string(),
                    category: WarningCategory::ExcessivePermissions,
                });
            }
        }

        // ── Env analysis ──────────────────────────────────────────────
        for var in &env_vars {
            let var_lower = var.to_lowercase();
            if var_lower.contains("key") || var_lower.contains("token") || var_lower.contains("secret") || var_lower.contains("password") {
                warnings.push(SecurityWarning {
                    severity: RiskLevel::Low,
                    message: format!("Exposes credential-like env var '{}'", var),
                    category: WarningCategory::SuspiciousPattern,
                });
            }
        }

        // Calculate overall risk
        let overall_risk = self.calculate_overall_risk(&tools, &warnings);

        Ok(SecurityReport {
            server_name,
            scan_timestamp: Utc::now(),
            overall_risk,
            tools_scanned: tools,
            warnings,
            command,
            args,
            env_vars,
            source_agent: None, // filled by caller
        })
    }

    async fn analyze_tool(&self, tool: &serde_json::Value) -> Result<ToolSecurityInfo> {
        let name = tool
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        let description = tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let mut warnings = Vec::new();
        let mut permissions = Vec::new();

        // Check for prompt injection patterns
        for pattern in &self.prompt_injection_patterns {
            if pattern.is_match(&description) {
                warnings.push(
                    "Potential prompt injection pattern detected in description".to_string(),
                );
                break;
            }
        }

        // Check for suspicious patterns
        for pattern in &self.suspicious_patterns {
            if pattern.is_match(&description) {
                warnings.push(format!(
                    "Suspicious pattern in tool description: {}",
                    pattern.as_str()
                ));
            }
        }

        // Analyze input schema for sensitive fields
        if let Some(schema) = tool.get("inputSchema").or(tool.get("parameters")) {
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                for (prop_name, _) in props {
                    let prop_lower = prop_name.to_lowercase();
                    if prop_lower.contains("path")
                        || prop_lower.contains("file")
                        || prop_lower.contains("dir")
                    {
                        permissions.push("filesystem".to_string());
                    }
                    if prop_lower.contains("url")
                        || prop_lower.contains("endpoint")
                        || prop_lower.contains("api")
                    {
                        permissions.push("network".to_string());
                    }
                    if prop_lower.contains("command")
                        || prop_lower.contains("exec")
                        || prop_lower.contains("script")
                    {
                        permissions.push("execution".to_string());
                        warnings.push(format!("Tool '{}' accepts executable commands", name));
                    }
                }
            }
        }

        // Deduplicate permissions
        permissions.sort();
        permissions.dedup();

        // Determine risk level
        let risk_level = if warnings.is_empty() && permissions.is_empty() {
            RiskLevel::Safe
        } else if warnings.len() > 2 || permissions.contains(&"execution".to_string()) {
            RiskLevel::High
        } else if !warnings.is_empty() {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        Ok(ToolSecurityInfo {
            name,
            description,
            risk_level,
            permissions,
            warnings,
        })
    }

    fn calculate_overall_risk(
        &self,
        tools: &[ToolSecurityInfo],
        warnings: &[SecurityWarning],
    ) -> RiskLevel {
        let has_critical = tools
            .iter()
            .any(|t| matches!(t.risk_level, RiskLevel::Critical))
            || warnings
                .iter()
                .any(|w| matches!(w.severity, RiskLevel::Critical));

        let has_high = tools
            .iter()
            .any(|t| matches!(t.risk_level, RiskLevel::High))
            || warnings
                .iter()
                .any(|w| matches!(w.severity, RiskLevel::High));

        let has_medium = tools
            .iter()
            .any(|t| matches!(t.risk_level, RiskLevel::Medium))
            || warnings
                .iter()
                .any(|w| matches!(w.severity, RiskLevel::Medium));

        if has_critical {
            RiskLevel::Critical
        } else if has_high {
            RiskLevel::High
        } else if has_medium {
            RiskLevel::Medium
        } else if tools
            .iter()
            .any(|t| matches!(t.risk_level, RiskLevel::Low))
        {
            RiskLevel::Low
        } else if !warnings.is_empty() {
            RiskLevel::Low
        } else {
            RiskLevel::Safe
        }
    }
}
