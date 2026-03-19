use crate::models::{Action, ActionType, RiskLevel};
use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
use regex::Regex;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub struct ClaudeDesktopParser {
    log_path: PathBuf,
    file_position: u64,
    line_regex: Regex,
}

impl ClaudeDesktopParser {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        let log_path = home.join("Library/Logs/Claude/main.log");

        Self {
            log_path,
            file_position: 0,
            line_regex: Regex::new(
                r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})\s+\[(\w+)\]\s+(.*)$",
            )
            .unwrap(),
        }
    }

    fn parse_line(&self, line: &str) -> Option<Action> {
        let caps = self.line_regex.captures(line)?;
        let time_str = caps.get(1)?.as_str();
        let level = caps.get(2)?.as_str();
        let message = caps.get(3)?.as_str();

        let timestamp = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|naive| naive.and_utc())
            .unwrap_or_else(|| Utc::now());

        // MCP events
        if message.to_lowercase().contains("mcp") {
            return Some(Action {
                id: format!("cd-{}", uuid::Uuid::new_v4()),
                agent_id: "claude-desktop".to_string(),
                action_type: ActionType::Other("mcp_event".to_string()),
                timestamp,
                description: truncate_msg("MCP: ", message, 120),
                risk_level: RiskLevel::Safe,
                cost: None,
                metadata: serde_json::json!({ "level": level, "raw": message }),
            });
        }

        // Skip noisy repetitive log lines
        let msg_lower = message.to_lowercase();
        if msg_lower.contains("health check fetch failed")
            || msg_lower.contains("healthcheck")
            || msg_lower.contains("ping timeout")
            || msg_lower.contains("keepalive")
        {
            return None;
        }

        // Errors
        if level == "error" {
            return Some(Action {
                id: format!("cd-{}", uuid::Uuid::new_v4()),
                agent_id: "claude-desktop".to_string(),
                action_type: ActionType::Other("error".to_string()),
                timestamp,
                description: truncate_msg("Error: ", message, 120),
                risk_level: RiskLevel::Medium,
                cost: None,
                metadata: serde_json::json!({ "level": level, "raw": message }),
            });
        }

        // App start / version lines
        if message.contains("version") || message.contains("starting") || message.contains("Started") {
            return Some(Action {
                id: format!("cd-{}", uuid::Uuid::new_v4()),
                agent_id: "claude-desktop".to_string(),
                action_type: ActionType::Other("app_start".to_string()),
                timestamp,
                description: truncate_msg("", message, 120),
                risk_level: RiskLevel::Safe,
                cost: None,
                metadata: serde_json::json!({ "level": level }),
            });
        }

        None
    }
}

fn truncate_msg(prefix: &str, msg: &str, max: usize) -> String {
    let available = max.saturating_sub(prefix.len());
    if msg.len() > available {
        format!("{}{}...", prefix, &msg[..available.saturating_sub(3)])
    } else {
        format!("{}{}", prefix, msg)
    }
}

impl super::AgentParser for ClaudeDesktopParser {
    fn agent_id(&self) -> &str {
        "claude-desktop"
    }

    fn reset_position(&mut self) {
        self.file_position = 0;
    }

    fn parse_new_actions(&mut self) -> Result<Vec<Action>> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let metadata = fs::metadata(&self.log_path)?;
        let current_size = metadata.len();

        if current_size <= self.file_position {
            // File may have been rotated (smaller than last position)
            if current_size < self.file_position {
                self.file_position = 0;
            } else {
                return Ok(Vec::new());
            }
        }

        let mut file = fs::File::open(&self.log_path)?;
        file.seek(SeekFrom::Start(self.file_position))?;

        let bytes_to_read = current_size - self.file_position;
        let mut buf = vec![0u8; bytes_to_read as usize];
        file.read_exact(&mut buf)?;

        self.file_position = current_size;

        let text = String::from_utf8_lossy(&buf);
        let mut actions = Vec::new();

        for line in text.lines() {
            if let Some(action) = self.parse_line(line) {
                actions.push(action);
            }
        }

        Ok(actions)
    }
}
