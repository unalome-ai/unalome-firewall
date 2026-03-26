use crate::models::{Action, ActionType, CostInfo, RiskLevel};
use crate::parsers::pricing;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct ClaudeCodeParser {
    projects_dir: PathBuf,
    /// Byte offset per JSONL file — only read past this point on each poll.
    file_positions: HashMap<PathBuf, u64>,
    /// Set of known JSONL paths (refreshed periodically).
    known_files: Vec<PathBuf>,
    /// Counter for periodic file discovery.
    poll_count: u32,
}

impl ClaudeCodeParser {
    pub fn new(projects_dir: PathBuf) -> Self {
        Self {
            projects_dir,
            file_positions: HashMap::new(),
            known_files: Vec::new(),
            poll_count: 0,
        }
    }

    /// Discover JSONL files under ~/.claude/projects/ (max depth 4).
    fn discover_files(&mut self) {
        if !self.projects_dir.exists() {
            return;
        }
        self.known_files.clear();
        for entry in WalkDir::new(&self.projects_dir)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                self.known_files.push(path.to_path_buf());
            }
        }
    }

    /// Read new bytes from a single file, parse each line as JSON, return actions.
    fn read_new_lines(&mut self, path: &PathBuf) -> Result<Vec<Action>> {
        let metadata = fs::metadata(path)?;
        let current_size = metadata.len();
        let last_pos = self.file_positions.get(path).copied().unwrap_or(0);

        if current_size <= last_pos {
            return Ok(Vec::new());
        }

        let mut file = fs::File::open(path)?;
        file.seek(SeekFrom::Start(last_pos))?;

        let bytes_to_read = current_size - last_pos;
        let mut buf = vec![0u8; bytes_to_read as usize];
        file.read_exact(&mut buf)?;

        self.file_positions.insert(path.clone(), current_size);

        let text = String::from_utf8_lossy(&buf);
        let mut actions = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(json) = serde_json::from_str::<Value>(line) {
                if let Some(mut parsed) = self.parse_record(&json) {
                    // Use session file path as part of metadata
                    if let Some(obj) = parsed.metadata.as_object_mut() {
                        obj.insert(
                            "session_file".into(),
                            Value::String(path.to_string_lossy().to_string()),
                        );
                    }
                    actions.push(parsed);
                }
            }
        }

        Ok(actions)
    }

    /// Parse a single JSONL record into an Action (or None if not actionable).
    fn parse_record(&self, json: &Value) -> Option<Action> {
        let role = json.get("type").and_then(|v| v.as_str())?;

        match role {
            "assistant" => self.parse_assistant_record(json),
            "system" => self.parse_system_record(json),
            _ => None, // Skip "user" and other types — tool results aren't separate actions
        }
    }

    fn parse_assistant_record(&self, json: &Value) -> Option<Action> {
        // The message is at the top level or under "message"
        let message = json.get("message").unwrap_or(json);
        let content = message.get("content")?;
        let content_arr = content.as_array()?;

        // Extract cost info from message.usage
        let cost = self.extract_cost(message);

        // Find tool_use blocks in content
        for block in content_arr {
            if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                return self.parse_tool_use(block, json, cost.clone());
            }
        }

        // If no tool_use, this is just a text response — skip
        None
    }

    fn parse_tool_use(
        &self,
        block: &Value,
        record: &Value,
        cost: Option<CostInfo>,
    ) -> Option<Action> {
        let tool_name = block.get("name").and_then(|v| v.as_str())?.to_string();
        let input = block.get("input").cloned().unwrap_or(serde_json::json!({}));

        let description = self.format_tool_description(&tool_name, &input);
        let risk_level = self.assess_tool_risk(&tool_name, &input);

        let timestamp = self.extract_timestamp(record);

        Some(Action {
            id: format!("cc-{}", uuid::Uuid::new_v4()),
            agent_id: "claude-code".to_string(),
            action_type: ActionType::ToolCall {
                tool_name: tool_name.clone(),
                args: input,
            },
            timestamp,
            description,
            risk_level,
            cost,
            metadata: serde_json::json!({
                "model": record.get("message")
                    .and_then(|m| m.get("model"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown"),
                "slug": record.get("slug")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                "permission_mode": record.get("permissionMode")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                "session_id": record.get("sessionId")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            }),
        })
    }

    fn parse_system_record(&self, json: &Value) -> Option<Action> {
        let subtype = json.get("subtype").and_then(|v| v.as_str())?;

        match subtype {
            "api_error" => {
                let error_type = json
                    .get("error")
                    .and_then(|e| e.get("error"))
                    .and_then(|e| e.get("type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let status = json
                    .get("error")
                    .and_then(|e| e.get("status"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                Some(Action {
                    id: format!("cc-{}", uuid::Uuid::new_v4()),
                    agent_id: "claude-code".to_string(),
                    action_type: ActionType::Other("api_error".to_string()),
                    timestamp: self.extract_timestamp(json),
                    description: format!("API error: {} (status {})", error_type, status),
                    risk_level: RiskLevel::Low,
                    cost: None,
                    metadata: serde_json::json!({
                        "error_type": error_type,
                        "status": status,
                        "slug": json.get("slug")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                        "permission_mode": json.get("permissionMode")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                        "session_id": json.get("sessionId")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                    }),
                })
            }
            _ => None,
        }
    }

    fn extract_cost(&self, message: &Value) -> Option<CostInfo> {
        let usage = message.get("usage")?;
        let input_tokens = usage.get("input_tokens").and_then(|v| v.as_u64())?;
        let output_tokens = usage.get("output_tokens").and_then(|v| v.as_u64())?;
        let cache_creation = usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let cache_read = usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let model = message
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("claude-sonnet");

        let cost_usd =
            pricing::estimate_cost_usd(model, input_tokens, cache_creation, cache_read, output_tokens);

        Some(CostInfo {
            tokens_input: input_tokens,
            tokens_output: output_tokens,
            cache_write_tokens: cache_creation,
            cache_read_tokens: cache_read,
            estimated_cost_usd: cost_usd,
        })
    }

    fn extract_timestamp(&self, record: &Value) -> DateTime<Utc> {
        // Try "timestamp" field (ISO 8601 or epoch ms)
        if let Some(ts) = record.get("timestamp") {
            if let Some(s) = ts.as_str() {
                if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                    return dt.with_timezone(&Utc);
                }
            }
            if let Some(ms) = ts.as_u64() {
                if let Some(dt) = DateTime::from_timestamp_millis(ms as i64) {
                    return dt;
                }
            }
            if let Some(ms) = ts.as_i64() {
                if let Some(dt) = DateTime::from_timestamp_millis(ms) {
                    return dt;
                }
            }
        }
        Utc::now()
    }

    fn format_tool_description(&self, tool_name: &str, input: &Value) -> String {
        match tool_name {
            "Bash" => {
                let cmd = input
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no command)");
                let truncated = if cmd.len() > 80 {
                    format!("{}...", &cmd[..77])
                } else {
                    cmd.to_string()
                };
                format!("Bash: {}", truncated)
            }
            "Read" => {
                let path = input
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(unknown)");
                format!("Read: {}", path)
            }
            "Edit" => {
                let path = input
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(unknown)");
                format!("Edit: {}", path)
            }
            "Write" => {
                let path = input
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(unknown)");
                format!("Write: {}", path)
            }
            "Grep" => {
                let pattern = input
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(unknown)");
                format!("Grep: {}", pattern)
            }
            "Glob" => {
                let pattern = input
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(unknown)");
                format!("Glob: {}", pattern)
            }
            "Task" => {
                let desc = input
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("subagent");
                format!("Task: {}", desc)
            }
            other => format!("{}", other),
        }
    }

    fn assess_tool_risk(&self, tool_name: &str, input: &Value) -> RiskLevel {
        match tool_name {
            "Bash" => {
                let cmd = input
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if cmd.contains("rm ")
                    || cmd.contains("sudo ")
                    || cmd.contains("curl") && cmd.contains("| sh")
                    || cmd.contains("chmod")
                    || cmd.contains("mkfs")
                {
                    RiskLevel::High
                } else {
                    RiskLevel::Medium
                }
            }
            "Write" | "Edit" => RiskLevel::Medium,
            "Read" | "Glob" | "Grep" => RiskLevel::Safe,
            "Task" => RiskLevel::Low,
            _ => RiskLevel::Low,
        }
    }
}

impl super::AgentParser for ClaudeCodeParser {
    fn agent_id(&self) -> &str {
        "claude-code"
    }

    fn reset_position(&mut self) {
        self.file_positions.clear();
    }

    fn parse_new_actions(&mut self) -> Result<Vec<Action>> {
        // Re-discover files every 12 polls (~60s at 5s interval)
        if self.poll_count % 12 == 0 {
            self.discover_files();
        }
        self.poll_count = self.poll_count.wrapping_add(1);

        let mut all_actions = Vec::new();
        let paths = self.known_files.clone();

        for path in &paths {
            match self.read_new_lines(path) {
                Ok(actions) => all_actions.extend(actions),
                Err(e) => {
                    eprintln!("[ClaudeCodeParser] Error reading {:?}: {}", path, e);
                }
            }
        }

        Ok(all_actions)
    }
}
