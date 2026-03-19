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

pub struct OpenClawParser {
    sessions_dir: PathBuf,
    file_positions: HashMap<PathBuf, u64>,
    known_files: Vec<PathBuf>,
    poll_count: u32,
}

impl OpenClawParser {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        let sessions_dir = home.join(".openclaw/agents/main/sessions");

        Self {
            sessions_dir,
            file_positions: HashMap::new(),
            known_files: Vec::new(),
            poll_count: 0,
        }
    }

    fn discover_files(&mut self) {
        if !self.sessions_dir.exists() {
            return;
        }
        self.known_files.clear();
        for entry in WalkDir::new(&self.sessions_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                self.known_files.push(path.to_path_buf());
            }
        }
    }

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
                for action in self.parse_record(&json) {
                    let mut action = action;
                    if let Some(obj) = action.metadata.as_object_mut() {
                        obj.insert(
                            "session_file".into(),
                            Value::String(path.to_string_lossy().to_string()),
                        );
                    }
                    actions.push(action);
                }
            }
        }

        Ok(actions)
    }

    /// Parse a single JSONL record into zero or more Actions.
    fn parse_record(&self, json: &Value) -> Vec<Action> {
        let record_type = match json.get("type").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return vec![],
        };

        match record_type {
            "session" => self.parse_session(json).into_iter().collect(),
            "model_change" => self.parse_model_change(json).into_iter().collect(),
            "thinking_level_change" => vec![], // too noisy, skip
            "custom" => self.parse_custom(json).into_iter().collect(),
            "message" => self.parse_message(json),
            _ => vec![],
        }
    }

    // ── Session start ────────────────────────────────────────────

    fn parse_session(&self, json: &Value) -> Option<Action> {
        let session_id = json.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let cwd = json.get("cwd").and_then(|v| v.as_str()).unwrap_or("");
        let ts = self.extract_timestamp(json);

        Some(Action {
            id: format!("oc-{}", uuid::Uuid::new_v4()),
            agent_id: "openclaw".to_string(),
            action_type: ActionType::Other("session_start".to_string()),
            timestamp: ts,
            description: format!("Session started: {}", truncate(session_id, 36)),
            risk_level: RiskLevel::Safe,
            cost: None,
            metadata: serde_json::json!({
                "session_id": session_id,
                "cwd": cwd,
            }),
        })
    }

    // ── Model change ─────────────────────────────────────────────

    fn parse_model_change(&self, json: &Value) -> Option<Action> {
        let provider = json.get("provider").and_then(|v| v.as_str()).unwrap_or("unknown");
        let model_id = json.get("modelId").and_then(|v| v.as_str()).unwrap_or("unknown");
        let ts = self.extract_timestamp(json);

        Some(Action {
            id: format!("oc-{}", uuid::Uuid::new_v4()),
            agent_id: "openclaw".to_string(),
            action_type: ActionType::Other("model_change".to_string()),
            timestamp: ts,
            description: format!("Model: {}/{}", provider, model_id),
            risk_level: RiskLevel::Safe,
            cost: None,
            metadata: serde_json::json!({
                "provider": provider,
                "model_id": model_id,
            }),
        })
    }

    // ── Custom records (errors, model snapshots) ─────────────────

    fn parse_custom(&self, json: &Value) -> Option<Action> {
        let custom_type = json.get("customType").and_then(|v| v.as_str())?;
        let data = json.get("data").cloned().unwrap_or(serde_json::json!({}));
        let ts = self.extract_timestamp_from_data(&data)
            .unwrap_or_else(|| self.extract_timestamp(json));

        match custom_type {
            "openclaw:prompt-error" => {
                let provider = data.get("provider").and_then(|v| v.as_str()).unwrap_or("unknown");
                let model = data.get("model").and_then(|v| v.as_str()).unwrap_or("unknown");
                let error = data.get("error").and_then(|v| v.as_str()).unwrap_or("unknown error");

                Some(Action {
                    id: format!("oc-{}", uuid::Uuid::new_v4()),
                    agent_id: "openclaw".to_string(),
                    action_type: ActionType::Other("api_error".to_string()),
                    timestamp: ts,
                    description: format!("API error: {}/{} — {}", provider, model, truncate(error, 80)),
                    risk_level: RiskLevel::Low,
                    cost: None,
                    metadata: data,
                })
            }
            // Skip model-snapshot — too noisy and redundant with model_change
            _ => None,
        }
    }

    // ── Messages (assistant, user, toolResult) ───────────────────

    fn parse_message(&self, json: &Value) -> Vec<Action> {
        let message = match json.get("message") {
            Some(m) => m,
            None => return vec![],
        };
        let role = match message.get("role").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return vec![],
        };

        match role {
            "assistant" => self.parse_assistant_message(json, message),
            "user" => self.parse_user_message(json, message).into_iter().collect(),
            _ => vec![], // toolResult — skip, the tool call itself is tracked
        }
    }

    fn parse_assistant_message(&self, record: &Value, message: &Value) -> Vec<Action> {
        let content = match message.get("content").and_then(|v| v.as_array()) {
            Some(c) => c,
            None => return vec![],
        };

        let cost = self.extract_cost(message);
        let ts = self.extract_timestamp(record);
        let model = message
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let mut actions = Vec::new();
        let mut has_tool_call = false;

        // Parse tool calls
        for block in content {
            if block.get("type").and_then(|v| v.as_str()) == Some("toolCall") {
                has_tool_call = true;
                if let Some(action) = self.parse_tool_call(block, ts, cost.clone(), model) {
                    actions.push(action);
                }
            }
        }

        // If no tool calls, capture the text response (with cost)
        if !has_tool_call {
            if let Some(text_block) = content.iter().find(|b| {
                b.get("type").and_then(|v| v.as_str()) == Some("text")
            }) {
                let text = text_block
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Skip empty responses
                if text.is_empty() {
                    return actions;
                }

                // Skip if zero tokens (empty response)
                if let Some(ref c) = cost {
                    if c.tokens_input == 0 && c.tokens_output == 0 {
                        return actions;
                    }
                }

                actions.push(Action {
                    id: format!("oc-{}", uuid::Uuid::new_v4()),
                    agent_id: "openclaw".to_string(),
                    action_type: ActionType::Other("response".to_string()),
                    timestamp: ts,
                    description: format!("Response: {}", truncate(text, 100)),
                    risk_level: RiskLevel::Safe,
                    cost,
                    metadata: serde_json::json!({
                        "model": model,
                    }),
                });
            }
        }

        actions
    }

    fn parse_user_message(&self, record: &Value, message: &Value) -> Option<Action> {
        let content = message.get("content")?.as_array()?;
        let ts = self.extract_timestamp(record);

        let text_block = content.iter().find(|b| {
            b.get("type").and_then(|v| v.as_str()) == Some("text")
        })?;
        let text = text_block.get("text").and_then(|v| v.as_str()).unwrap_or("");

        if text.is_empty() {
            return None;
        }

        // Detect messaging channel from user message metadata
        let (channel, description) = if text.contains("\"sender_id\"") || text.contains("Conversation info") {
            // Message from an external channel (WhatsApp, Signal, etc.)
            let channel = self.detect_channel(text);
            let sender = self.extract_sender_id(text);
            let desc = match (channel.as_str(), sender.as_str()) {
                (ch, s) if !s.is_empty() => format!("Incoming: {} from {}", ch, s),
                (ch, _) => format!("Incoming: {}", ch),
            };
            (channel, desc)
        } else if text.contains("gateway-client") || text.contains("openclaw-tui") {
            ("tui".to_string(), format!("User (TUI): {}", truncate(text.lines().last().unwrap_or(text), 80)))
        } else if text.contains("[cron:") {
            let cron_name = text
                .split(']')
                .next()
                .unwrap_or("")
                .trim_start_matches("[cron:")
                .trim();
            ("cron".to_string(), format!("Cron: {}", truncate(cron_name, 80)))
        } else {
            return None; // Skip generic user messages to avoid noise
        };

        Some(Action {
            id: format!("oc-{}", uuid::Uuid::new_v4()),
            agent_id: "openclaw".to_string(),
            action_type: ActionType::Other(format!("channel:{}", channel)),
            timestamp: ts,
            description,
            risk_level: RiskLevel::Safe,
            cost: None,
            metadata: serde_json::json!({
                "channel": channel,
            }),
        })
    }

    // ── Tool call parsing ────────────────────────────────────────

    fn parse_tool_call(
        &self,
        block: &Value,
        timestamp: DateTime<Utc>,
        cost: Option<CostInfo>,
        model: &str,
    ) -> Option<Action> {
        let tool_name = block.get("name").and_then(|v| v.as_str())?.to_string();
        let args = block
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let description = self.format_tool_description(&tool_name, &args);
        let risk_level = self.assess_tool_risk(&tool_name, &args);

        Some(Action {
            id: format!("oc-{}", uuid::Uuid::new_v4()),
            agent_id: "openclaw".to_string(),
            action_type: ActionType::ToolCall {
                tool_name: tool_name.clone(),
                args,
            },
            timestamp,
            description,
            risk_level,
            cost,
            metadata: serde_json::json!({
                "model": model,
            }),
        })
    }

    // ── Cost extraction ──────────────────────────────────────────

    fn extract_cost(&self, message: &Value) -> Option<CostInfo> {
        let usage = message.get("usage")?;
        let input_tokens = usage.get("input").and_then(|v| v.as_u64())?;
        let output_tokens = usage.get("output").and_then(|v| v.as_u64())?;
        let cache_write = usage.get("cacheWrite").and_then(|v| v.as_u64()).unwrap_or(0);
        let cache_read = usage.get("cacheRead").and_then(|v| v.as_u64()).unwrap_or(0);

        // Use pre-calculated cost if available
        let pre_cost = usage
            .get("cost")
            .and_then(|c| c.get("total"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let model = message
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let cost_usd = if pre_cost > 0.0 {
            pre_cost
        } else {
            pricing::estimate_cost_usd(model, input_tokens, cache_write, cache_read, output_tokens)
        };

        // Skip zero-token entries
        if input_tokens == 0 && output_tokens == 0 {
            return None;
        }

        Some(CostInfo {
            tokens_input: input_tokens,
            tokens_output: output_tokens,
            cache_write_tokens: cache_write,
            cache_read_tokens: cache_read,
            estimated_cost_usd: cost_usd,
        })
    }

    // ── Timestamp helpers ────────────────────────────────────────

    fn extract_timestamp(&self, record: &Value) -> DateTime<Utc> {
        if let Some(ts) = record.get("timestamp") {
            if let Some(s) = ts.as_str() {
                if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                    return dt.with_timezone(&Utc);
                }
                if let Ok(dt) = s.parse::<DateTime<Utc>>() {
                    return dt;
                }
            }
            if let Some(ms) = ts.as_i64() {
                if let Some(dt) = DateTime::from_timestamp_millis(ms) {
                    return dt;
                }
            }
            if let Some(ms) = ts.as_u64() {
                if let Some(dt) = DateTime::from_timestamp_millis(ms as i64) {
                    return dt;
                }
            }
        }
        Utc::now()
    }

    fn extract_timestamp_from_data(&self, data: &Value) -> Option<DateTime<Utc>> {
        let ts = data.get("timestamp")?;
        if let Some(ms) = ts.as_i64() {
            return DateTime::from_timestamp_millis(ms);
        }
        if let Some(ms) = ts.as_u64() {
            return DateTime::from_timestamp_millis(ms as i64);
        }
        None
    }

    // ── Channel detection helpers ────────────────────────────────

    fn detect_channel(&self, text: &str) -> String {
        let lower = text.to_lowercase();
        if lower.contains("whatsapp") { return "whatsapp".to_string(); }
        if lower.contains("signal") { return "signal".to_string(); }
        if lower.contains("telegram") { return "telegram".to_string(); }
        if lower.contains("discord") { return "discord".to_string(); }
        if lower.contains("slack") { return "slack".to_string(); }
        if lower.contains("imessage") { return "imessage".to_string(); }
        if lower.contains("irc") { return "irc".to_string(); }
        if lower.contains("google_chat") || lower.contains("google chat") { return "google_chat".to_string(); }
        if lower.contains("webchat") { return "webchat".to_string(); }
        "unknown".to_string()
    }

    fn extract_sender_id(&self, text: &str) -> String {
        // Try to find "sender_id": "+..."
        if let Some(idx) = text.find("\"sender_id\"") {
            let after = &text[idx..];
            if let Some(start) = after.find('"').and_then(|i| {
                after[i + 1..].find('"').map(|j| i + 1 + j + 1)
            }) {
                // Find the value after the colon
                if let Some(colon) = after.find(':') {
                    let value_part = after[colon + 1..].trim();
                    if let Some(q1) = value_part.find('"') {
                        if let Some(q2) = value_part[q1 + 1..].find('"') {
                            let sender = &value_part[q1 + 1..q1 + 1 + q2];
                            if !sender.is_empty() {
                                return sender.to_string();
                            }
                        }
                    }
                }
                let _ = start; // suppress unused warning
            }
        }
        String::new()
    }

    // ── Tool description formatting ──────────────────────────────

    fn format_tool_description(&self, tool_name: &str, args: &Value) -> String {
        match tool_name {
            "message" => {
                let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("send");
                let channel = args.get("channel").and_then(|v| v.as_str()).unwrap_or("unknown");
                if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                    format!("Command ({}): {}", channel, truncate(cmd, 80))
                } else if let Some(msg) = args.get("message").and_then(|v| v.as_str()) {
                    format!("Message via {}: {}", channel, truncate(msg, 60))
                } else {
                    format!("Message: {} via {}", action, channel)
                }
            }
            name if name.contains("web_fetch") => {
                let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("(unknown)");
                format!("Web Fetch: {}", truncate(url, 80))
            }
            name if name.contains("web_search") => {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("(unknown)");
                format!("Web Search: {}", query)
            }
            "sessions_list" => "Sessions: list".to_string(),
            "sessions_send" => {
                let msg = args.get("message").and_then(|v| v.as_str()).unwrap_or("(message)");
                format!("Session Send: {}", truncate(msg, 60))
            }
            "sessions_history" => "Sessions: history".to_string(),
            other => other.to_string(),
        }
    }

    fn assess_tool_risk(&self, tool_name: &str, args: &Value) -> RiskLevel {
        match tool_name {
            "message" => {
                if args.get("command").is_some() {
                    let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                    if cmd.contains("rm ") || cmd.contains("sudo ") || cmd.contains("chmod") {
                        return RiskLevel::High;
                    }
                    return RiskLevel::Medium;
                }
                // Sending messages to external channels
                let channel = args.get("channel").and_then(|v| v.as_str()).unwrap_or("");
                match channel {
                    "whatsapp" | "signal" | "telegram" | "discord" | "slack" | "imessage" => RiskLevel::Medium,
                    _ => RiskLevel::Low,
                }
            }
            name if name.contains("web_fetch") || name.contains("web_search") => RiskLevel::Low,
            "sessions_list" | "sessions_history" => RiskLevel::Safe,
            "sessions_send" => RiskLevel::Low,
            _ => RiskLevel::Low,
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    // Get first line only
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() > max {
        format!("{}...", &first_line[..max.saturating_sub(3)])
    } else {
        first_line.to_string()
    }
}

impl super::AgentParser for OpenClawParser {
    fn agent_id(&self) -> &str {
        "openclaw"
    }

    fn reset_position(&mut self) {
        self.file_positions.clear();
    }

    fn parse_new_actions(&mut self) -> Result<Vec<Action>> {
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
                    eprintln!("[OpenClawParser] Error reading {:?}: {}", path, e);
                }
            }
        }

        Ok(all_actions)
    }
}
