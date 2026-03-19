use crate::models::{Action, ActionType, RiskLevel};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;

pub struct CursorParser {
    db_path: PathBuf,
    last_code_hash_timestamp: i64,
    last_scored_commit_timestamp: i64,
}

impl CursorParser {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        let db_path = home.join(".cursor/ai-tracking/ai-code-tracking.db");

        Self {
            db_path,
            last_code_hash_timestamp: 0,
            last_scored_commit_timestamp: 0,
        }
    }

    fn read_code_hashes(&mut self, conn: &Connection) -> Result<Vec<Action>> {
        let mut stmt = conn.prepare(
            "SELECT hash, source, fileExtension, fileName, model, timestamp, requestId, conversationId
             FROM ai_code_hashes WHERE timestamp > ?1 ORDER BY timestamp ASC",
        )?;

        let mut actions = Vec::new();
        let mut max_ts = self.last_code_hash_timestamp;

        let rows = stmt.query_map([self.last_code_hash_timestamp], |row| {
            let hash: String = row.get(0)?;
            let source: String = row.get(1)?;
            let file_ext: Option<String> = row.get(2)?;
            let file_name: Option<String> = row.get(3)?;
            let model: Option<String> = row.get(4)?;
            let timestamp: i64 = row.get(5)?;
            let request_id: Option<String> = row.get(6)?;
            let conversation_id: Option<String> = row.get(7)?;
            Ok((
                hash,
                source,
                file_ext,
                file_name,
                model,
                timestamp,
                request_id,
                conversation_id,
            ))
        })?;

        for row in rows {
            let (hash, source, file_ext, file_name, model, timestamp, request_id, conversation_id) =
                row?;

            if timestamp > max_ts {
                max_ts = timestamp;
            }

            let (tool_name, risk, desc) = match source.as_str() {
                "tab" => (
                    "tab_completion".to_string(),
                    RiskLevel::Safe,
                    format!(
                        "Tab completion: {}",
                        file_name.as_deref().unwrap_or("unknown file")
                    ),
                ),
                "composer" => (
                    "composer_edit".to_string(),
                    RiskLevel::Low,
                    format!(
                        "Composer edit: {}",
                        file_name.as_deref().unwrap_or("unknown file")
                    ),
                ),
                other => (
                    other.to_string(),
                    RiskLevel::Low,
                    format!(
                        "{}: {}",
                        other,
                        file_name.as_deref().unwrap_or("unknown file")
                    ),
                ),
            };

            let ts = DateTime::from_timestamp_millis(timestamp).unwrap_or_else(|| Utc::now());

            actions.push(Action {
                id: format!("cur-{}", uuid::Uuid::new_v4()),
                agent_id: "cursor".to_string(),
                action_type: ActionType::ToolCall {
                    tool_name,
                    args: serde_json::json!({
                        "file_name": file_name,
                        "file_extension": file_ext,
                        "model": model,
                        "request_id": request_id,
                        "conversation_id": conversation_id,
                    }),
                },
                timestamp: ts,
                description: desc,
                risk_level: risk,
                cost: None,
                metadata: serde_json::json!({
                    "hash": hash,
                    "source": source,
                    "model": model,
                }),
            });
        }

        self.last_code_hash_timestamp = max_ts;
        Ok(actions)
    }

    fn read_scored_commits(&mut self, conn: &Connection) -> Result<Vec<Action>> {
        // Check if scored_commits table exists
        let table_exists: bool = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='scored_commits'")?
            .exists([])?;

        if !table_exists {
            return Ok(Vec::new());
        }

        let mut stmt = conn.prepare(
            "SELECT commitHash, commitMessage, commitDate, tabLinesAdded, composerLinesAdded, humanLinesAdded, scoredAt
             FROM scored_commits WHERE scoredAt > ?1 ORDER BY scoredAt ASC",
        )?;

        let mut actions = Vec::new();
        let mut max_ts = self.last_scored_commit_timestamp;

        let rows = stmt.query_map([self.last_scored_commit_timestamp], |row| {
            let commit_hash: String = row.get(0)?;
            let commit_msg: Option<String> = row.get(1)?;
            let commit_date: Option<String> = row.get(2)?;
            let tab_lines: i64 = row.get(3)?;
            let composer_lines: i64 = row.get(4)?;
            let human_lines: i64 = row.get(5)?;
            let scored_at: i64 = row.get(6)?;
            Ok((
                commit_hash,
                commit_msg,
                commit_date,
                tab_lines,
                composer_lines,
                human_lines,
                scored_at,
            ))
        })?;

        for row in rows {
            let (commit_hash, commit_msg, _commit_date, tab_lines, composer_lines, human_lines, scored_at) =
                row?;

            if scored_at > max_ts {
                max_ts = scored_at;
            }

            let total_ai = tab_lines + composer_lines;
            let total = total_ai + human_lines;
            let ai_pct = if total > 0 {
                (total_ai as f64 / total as f64 * 100.0).round()
            } else {
                0.0
            };

            let msg = commit_msg.as_deref().unwrap_or("(no message)");
            let short_msg = if msg.len() > 60 {
                format!("{}...", &msg[..57])
            } else {
                msg.to_string()
            };

            let ts =
                DateTime::from_timestamp_millis(scored_at).unwrap_or_else(|| Utc::now());

            actions.push(Action {
                id: format!("cur-{}", uuid::Uuid::new_v4()),
                agent_id: "cursor".to_string(),
                action_type: ActionType::Other("scored_commit".to_string()),
                timestamp: ts,
                description: format!("Commit: {} ({}% AI)", short_msg, ai_pct),
                risk_level: RiskLevel::Safe,
                cost: None,
                metadata: serde_json::json!({
                    "commit_hash": commit_hash,
                    "tab_lines": tab_lines,
                    "composer_lines": composer_lines,
                    "human_lines": human_lines,
                    "ai_percentage": ai_pct,
                }),
            });
        }

        self.last_scored_commit_timestamp = max_ts;
        Ok(actions)
    }
}

impl super::AgentParser for CursorParser {
    fn agent_id(&self) -> &str {
        "cursor"
    }

    fn reset_position(&mut self) {
        self.last_code_hash_timestamp = 0;
        self.last_scored_commit_timestamp = 0;
    }

    fn parse_new_actions(&mut self) -> Result<Vec<Action>> {
        if !self.db_path.exists() {
            return Ok(Vec::new());
        }

        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        let mut actions = self.read_code_hashes(&conn)?;
        actions.extend(self.read_scored_commits(&conn)?);

        Ok(actions)
    }
}
