use crate::data_shield::{DataShieldStats, DomainProfile, OutboundEvent};
use crate::firewall::{DecisionType, FirewallDecision, FirewallRule, FirewallStats};
use crate::models::{Action, ActionType, Agent, CostInfo, RiskLevel};
use crate::pii::{PiiFinding, PiiStats};
use crate::reports::AgentActionSummary;
use crate::safety_net::{ProtectedFile, SafetyNetStats};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let db_path = Self::db_path()?;
        let options = SqliteConnectOptions::from_str(
            &format!("sqlite://{}?mode=rwc", db_path.display()),
        )?
        .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;
        Ok(Self { pool })
    }

    pub async fn initialize() -> Result<()> {
        let db = Self::new().await?;
        db.create_tables().await?;
        Ok(())
    }

    pub fn db_path() -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .context("Could not find data directory")?
            .join("Unalome");

        std::fs::create_dir_all(&data_dir)?;
        Ok(data_dir.join("unalome.db"))
    }

    async fn create_tables(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                agent_type TEXT NOT NULL,
                status TEXT NOT NULL,
                config_path TEXT,
                last_seen TEXT NOT NULL,
                metadata TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS actions (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                action_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                description TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                cost_input INTEGER,
                cost_output INTEGER,
                cache_write_tokens INTEGER,
                cache_read_tokens INTEGER,
                cost_usd REAL,
                metadata TEXT NOT NULL,
                FOREIGN KEY (agent_id) REFERENCES agents(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Migrate: add cache token columns if missing (existing DBs)
        let _ = sqlx::query("ALTER TABLE actions ADD COLUMN cache_write_tokens INTEGER")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("ALTER TABLE actions ADD COLUMN cache_read_tokens INTEGER")
            .execute(&self.pool)
            .await;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_actions_agent_id ON actions(agent_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_actions_timestamp ON actions(timestamp)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pii_findings (
                id TEXT PRIMARY KEY,
                action_id TEXT,
                agent_id TEXT NOT NULL,
                finding_type TEXT NOT NULL,
                severity TEXT NOT NULL,
                description TEXT NOT NULL,
                source_file TEXT,
                source_context TEXT NOT NULL,
                redacted_value TEXT NOT NULL,
                recommended_action TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                dismissed INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_pii_severity ON pii_findings(severity)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_pii_agent ON pii_findings(agent_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_pii_timestamp ON pii_findings(timestamp)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS protected_files (
                id TEXT PRIMARY KEY,
                original_path TEXT NOT NULL,
                snapshot_path TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                agent_id TEXT NOT NULL,
                agent_name TEXT,
                action_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                restored INTEGER DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_pf_agent ON protected_files(agent_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_pf_created ON protected_files(created_at)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_pf_path ON protected_files(original_path)")
            .execute(&self.pool)
            .await?;

        // ── Data Shield tables ──
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS outbound_events (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                agent_name TEXT,
                event_type TEXT NOT NULL,
                destination TEXT NOT NULL,
                url TEXT,
                direction TEXT DEFAULT 'outbound',
                description TEXT,
                risk_level TEXT DEFAULT 'unknown',
                timestamp TEXT NOT NULL,
                blocked INTEGER DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_outbound_dest ON outbound_events(destination)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_outbound_time ON outbound_events(timestamp)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_outbound_agent ON outbound_events(agent_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS domain_overrides (
                domain TEXT PRIMARY KEY,
                classification TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS weekly_reports (
                id TEXT PRIMARY KEY,
                week_start TEXT NOT NULL,
                week_end TEXT NOT NULL,
                generated_at TEXT NOT NULL,
                report_json TEXT NOT NULL,
                UNIQUE(week_start)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // ── Firewall tables ──
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS firewall_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                agent_pattern TEXT NOT NULL DEFAULT '*',
                allow_tools TEXT NOT NULL DEFAULT '[]',
                deny_tools TEXT NOT NULL DEFAULT '[]',
                conditions TEXT NOT NULL DEFAULT '[]',
                priority INTEGER NOT NULL DEFAULT 0,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS firewall_decisions (
                id TEXT PRIMARY KEY,
                action_id TEXT,
                timestamp TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                agent_name TEXT,
                tool_name TEXT NOT NULL,
                mcp_server TEXT,
                arguments TEXT,
                decision TEXT NOT NULL,
                reason TEXT,
                rule_id TEXT,
                rule_name TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_fw_decisions_ts ON firewall_decisions(timestamp)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_fw_decisions_agent ON firewall_decisions(agent_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_fw_decisions_decision ON firewall_decisions(decision)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pii_category_settings (
                category TEXT PRIMARY KEY,
                enabled INTEGER NOT NULL DEFAULT 1
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn save_agent(&self, agent: &Agent) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO agents (id, name, agent_type, status, config_path, last_seen, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(&agent.id)
        .bind(&agent.name)
        .bind(format!("{:?}", agent.agent_type))
        .bind(format!("{:?}", agent.status))
        .bind(&agent.config_path)
        .bind(agent.last_seen.to_rfc3339())
        .bind(serde_json::to_string(&agent.metadata)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn save_action(&self, action: &Action) -> Result<()> {
        let (cost_input, cost_output, cache_write, cache_read, cost_usd) = action
            .cost
            .as_ref()
            .map(|c| {
                (
                    Some(c.tokens_input as i64),
                    Some(c.tokens_output as i64),
                    Some(c.cache_write_tokens as i64),
                    Some(c.cache_read_tokens as i64),
                    Some(c.estimated_cost_usd),
                )
            })
            .unwrap_or((None, None, None, None, None));

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO actions (id, agent_id, action_type, timestamp, description, risk_level, cost_input, cost_output, cache_write_tokens, cache_read_tokens, cost_usd, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
        )
        .bind(&action.id)
        .bind(&action.agent_id)
        .bind(serde_json::to_string(&action.action_type).unwrap_or_else(|_| format!("{:?}", action.action_type)))
        .bind(action.timestamp.to_rfc3339())
        .bind(&action.description)
        .bind(format!("{:?}", action.risk_level))
        .bind(cost_input)
        .bind(cost_output)
        .bind(cache_write)
        .bind(cache_read)
        .bind(cost_usd)
        .bind(serde_json::to_string(&action.metadata)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_actions_for_agent(&self, agent_id: &str) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM actions WHERE agent_id = ?1 ORDER BY timestamp DESC
            "#,
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(self.row_to_action(&row)?);
        }

        Ok(actions)
    }

    pub async fn get_all_actions(&self, limit: i64) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM actions ORDER BY timestamp DESC LIMIT ?1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(self.row_to_action(&row)?);
        }

        Ok(actions)
    }

    pub async fn get_actions_count(&self) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM actions
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    #[allow(dead_code)]
    pub async fn get_total_cost(&self) -> Result<f64> {
        let cost: Option<f64> = sqlx::query_scalar(
            r#"
            SELECT SUM(cost_usd) FROM actions
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(cost.unwrap_or(0.0))
    }

    #[allow(dead_code)]
    pub async fn get_high_risk_count(&self) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM actions WHERE risk_level IN ('High', 'Critical')
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    pub async fn save_pii_finding(&self, finding: &PiiFinding) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO pii_findings (id, action_id, agent_id, finding_type, severity, description, source_file, source_context, redacted_value, recommended_action, timestamp, dismissed)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
        )
        .bind(&finding.id)
        .bind(&finding.action_id)
        .bind(&finding.agent_id)
        .bind(&finding.finding_type)
        .bind(&finding.severity)
        .bind(&finding.description)
        .bind(&finding.source_file)
        .bind(&finding.source_context)
        .bind(&finding.redacted_value)
        .bind(&finding.recommended_action)
        .bind(&finding.timestamp)
        .bind(finding.dismissed as i32)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_pii_findings(
        &self,
        limit: i64,
        offset: i64,
        severity: Option<String>,
        agent_id: Option<String>,
        dismissed: Option<bool>,
    ) -> Result<(Vec<PiiFinding>, i64)> {
        use sqlx::Row;

        let mut query = String::from("SELECT * FROM pii_findings WHERE 1=1");
        let mut count_query = String::from("SELECT COUNT(*) FROM pii_findings WHERE 1=1");

        if let Some(ref sev) = severity {
            let clause = format!(" AND severity = '{}'", sev.replace('\'', "''"));
            query.push_str(&clause);
            count_query.push_str(&clause);
        }
        if let Some(ref aid) = agent_id {
            let clause = format!(" AND agent_id = '{}'", aid.replace('\'', "''"));
            query.push_str(&clause);
            count_query.push_str(&clause);
        }
        if let Some(d) = dismissed {
            let clause = format!(" AND dismissed = {}", d as i32);
            query.push_str(&clause);
            count_query.push_str(&clause);
        }

        let total: i64 = sqlx::query_scalar(&count_query)
            .fetch_one(&self.pool)
            .await?;

        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let findings = rows
            .iter()
            .map(|row| {
                let dismissed_int: i32 = row.try_get("dismissed").unwrap_or(0);
                PiiFinding {
                    id: row.try_get("id").unwrap_or_default(),
                    action_id: row.try_get("action_id").ok(),
                    agent_id: row.try_get("agent_id").unwrap_or_default(),
                    finding_type: row.try_get("finding_type").unwrap_or_default(),
                    severity: row.try_get("severity").unwrap_or_default(),
                    description: row.try_get("description").unwrap_or_default(),
                    source_file: row.try_get("source_file").ok(),
                    source_context: row.try_get("source_context").unwrap_or_default(),
                    redacted_value: row.try_get("redacted_value").unwrap_or_default(),
                    recommended_action: row.try_get("recommended_action").unwrap_or_default(),
                    timestamp: row.try_get("timestamp").unwrap_or_default(),
                    dismissed: dismissed_int != 0,
                }
            })
            .collect();

        Ok((findings, total))
    }

    pub async fn delete_pii_findings_by_type(&self, finding_type: &str) -> Result<i64> {
        let result = sqlx::query("DELETE FROM pii_findings WHERE finding_type = ?1")
            .bind(finding_type)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() as i64)
    }

    pub async fn dismiss_pii_finding(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE pii_findings SET dismissed = 1 WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn restore_pii_finding(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE pii_findings SET dismissed = 0 WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_pii_category_settings(&self) -> Result<HashMap<String, bool>> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT category, enabled FROM pii_category_settings")
            .fetch_all(&self.pool)
            .await?;
        let mut map = HashMap::new();
        for row in rows {
            let category: String = row.get("category");
            let enabled: i32 = row.get("enabled");
            map.insert(category, enabled != 0);
        }
        Ok(map)
    }

    pub async fn set_pii_category_enabled(&self, category: &str, enabled: bool) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO pii_category_settings (category, enabled) VALUES (?1, ?2)",
        )
        .bind(category)
        .bind(if enabled { 1i32 } else { 0i32 })
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_pii_stats(&self) -> Result<PiiStats> {
        use sqlx::Row;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pii_findings WHERE dismissed = 0")
            .fetch_one(&self.pool)
            .await?;

        let severity_rows = sqlx::query("SELECT severity, COUNT(*) as cnt FROM pii_findings WHERE dismissed = 0 GROUP BY severity")
            .fetch_all(&self.pool)
            .await?;
        let mut by_severity = HashMap::new();
        for row in &severity_rows {
            let sev: String = row.try_get("severity")?;
            let cnt: i64 = row.try_get("cnt")?;
            by_severity.insert(sev, cnt);
        }

        let type_rows = sqlx::query("SELECT finding_type, COUNT(*) as cnt FROM pii_findings WHERE dismissed = 0 GROUP BY finding_type")
            .fetch_all(&self.pool)
            .await?;
        let mut by_type = HashMap::new();
        for row in &type_rows {
            let ft: String = row.try_get("finding_type")?;
            let cnt: i64 = row.try_get("cnt")?;
            by_type.insert(ft, cnt);
        }

        let agent_rows = sqlx::query("SELECT agent_id, COUNT(*) as cnt FROM pii_findings WHERE dismissed = 0 GROUP BY agent_id")
            .fetch_all(&self.pool)
            .await?;
        let mut by_agent = HashMap::new();
        for row in &agent_rows {
            let aid: String = row.try_get("agent_id")?;
            let cnt: i64 = row.try_get("cnt")?;
            by_agent.insert(aid, cnt);
        }

        let today: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pii_findings WHERE dismissed = 0 AND timestamp >= date('now')"
        )
        .fetch_one(&self.pool)
        .await?;

        let this_week: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pii_findings WHERE dismissed = 0 AND timestamp >= date('now', '-7 days')"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(PiiStats {
            total,
            by_severity,
            by_type,
            by_agent,
            today,
            this_week,
        })
    }

    fn row_to_action(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Action> {
        use sqlx::Row;

        let action_type_str: String = row.try_get("action_type")?;
        let action_type = self.parse_action_type(&action_type_str);

        let risk_level_str: String = row.try_get("risk_level")?;
        let risk_level = self.parse_risk_level(&risk_level_str);

        let cost_input: Option<i64> = row.try_get("cost_input")?;
        let cost_output: Option<i64> = row.try_get("cost_output")?;
        let cache_write: Option<i64> = row.try_get("cache_write_tokens").unwrap_or(None);
        let cache_read: Option<i64> = row.try_get("cache_read_tokens").unwrap_or(None);
        let cost_usd: Option<f64> = row.try_get("cost_usd")?;

        let cost = if cost_input.is_some() && cost_output.is_some() && cost_usd.is_some() {
            Some(CostInfo {
                tokens_input: cost_input.unwrap() as u64,
                tokens_output: cost_output.unwrap() as u64,
                cache_write_tokens: cache_write.unwrap_or(0) as u64,
                cache_read_tokens: cache_read.unwrap_or(0) as u64,
                estimated_cost_usd: cost_usd.unwrap(),
            })
        } else {
            None
        };

        let metadata_str: String = row.try_get("metadata")?;
        let metadata = serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({}));

        let timestamp_str: String = row.try_get("timestamp")?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Action {
            id: row.try_get("id")?,
            agent_id: row.try_get("agent_id")?,
            action_type,
            timestamp,
            description: row.try_get("description")?,
            risk_level,
            cost,
            metadata,
        })
    }

    fn parse_action_type(&self, s: &str) -> ActionType {
        // Try JSON format first (new format)
        if let Ok(at) = serde_json::from_str::<ActionType>(s) {
            return at;
        }

        // Fall back to parsing old Rust Debug format
        // e.g. ToolCall { tool_name: "Bash", args: Object {"command": String("ls")} }
        if s.starts_with("ToolCall { tool_name: \"") {
            let after_prefix = &s[22..]; // skip 'ToolCall { tool_name: "'
            if let Some(name_end) = after_prefix.find('"') {
                let tool_name = after_prefix[..name_end].to_string();
                // Extract args: everything between "args: " and the final " }"
                let args = if let Some(args_idx) = s.find(", args: ") {
                    let args_raw = &s[args_idx + 8..s.len().saturating_sub(2)];
                    parse_debug_value(args_raw)
                } else {
                    serde_json::json!({})
                };
                return ActionType::ToolCall { tool_name, args };
            }
        }
        if s.starts_with("FileAccess { path: \"") {
            let after = &s[20..];
            if let Some(end) = after.find('"') {
                let path = after[..end].to_string();
                let operation = s.find("operation: \"")
                    .and_then(|i| {
                        let rest = &s[i + 12..];
                        rest.find('"').map(|e| rest[..e].to_string())
                    })
                    .unwrap_or_else(|| "unknown".to_string());
                return ActionType::FileAccess { path, operation };
            }
        }
        if s.starts_with("Message { content: \"") {
            let after = &s[20..];
            if let Some(end) = after.rfind("\" }") {
                return ActionType::Message { content: after[..end].to_string() };
            }
        }
        ActionType::Other(s.to_string())
    }

    fn parse_risk_level(&self, s: &str) -> RiskLevel {
        match s {
            "Safe" => RiskLevel::Safe,
            "Low" => RiskLevel::Low,
            "Medium" => RiskLevel::Medium,
            "High" => RiskLevel::High,
            "Critical" => RiskLevel::Critical,
            _ => RiskLevel::Low,
        }
    }

    // ── Safety Net CRUD ──────────────────────────────────────────────

    pub async fn save_protected_file(&self, pf: &ProtectedFile) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO protected_files (id, original_path, snapshot_path, file_size, agent_id, agent_name, action_type, created_at, restored)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(&pf.id)
        .bind(&pf.original_path)
        .bind(&pf.snapshot_path)
        .bind(pf.file_size as i64)
        .bind(&pf.agent_id)
        .bind(&pf.agent_name)
        .bind(&pf.action_type)
        .bind(&pf.created_at)
        .bind(pf.restored as i32)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_protected_files(
        &self,
        limit: i64,
        offset: i64,
        agent_id: Option<String>,
        action_type: Option<String>,
        search: Option<String>,
    ) -> Result<(Vec<ProtectedFile>, i64)> {
        use sqlx::Row;

        let mut query = String::from("SELECT * FROM protected_files WHERE 1=1");
        let mut count_query = String::from("SELECT COUNT(*) FROM protected_files WHERE 1=1");

        if let Some(ref aid) = agent_id {
            let clause = format!(" AND agent_id = '{}'", aid.replace('\'', "''"));
            query.push_str(&clause);
            count_query.push_str(&clause);
        }
        if let Some(ref at) = action_type {
            let clause = format!(" AND action_type = '{}'", at.replace('\'', "''"));
            query.push_str(&clause);
            count_query.push_str(&clause);
        }
        if let Some(ref s) = search {
            let escaped = s.replace('\'', "''");
            let clause = format!(" AND original_path LIKE '%{}%'", escaped);
            query.push_str(&clause);
            count_query.push_str(&clause);
        }

        let total: i64 = sqlx::query_scalar(&count_query)
            .fetch_one(&self.pool)
            .await?;

        query.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let files = rows
            .iter()
            .map(|row| {
                let restored_int: i32 = row.try_get("restored").unwrap_or(0);
                ProtectedFile {
                    id: row.try_get("id").unwrap_or_default(),
                    original_path: row.try_get("original_path").unwrap_or_default(),
                    snapshot_path: row.try_get("snapshot_path").unwrap_or_default(),
                    file_size: row.try_get::<i64, _>("file_size").unwrap_or(0) as u64,
                    agent_id: row.try_get("agent_id").unwrap_or_default(),
                    agent_name: row.try_get("agent_name").unwrap_or_default(),
                    action_type: row.try_get("action_type").unwrap_or_default(),
                    created_at: row.try_get("created_at").unwrap_or_default(),
                    restored: restored_int != 0,
                }
            })
            .collect();

        Ok((files, total))
    }

    pub async fn mark_file_restored(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE protected_files SET restored = 1 WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_protected_file(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM protected_files WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_safety_net_stats(&self, storage_bytes: u64, storage_limit: u64) -> Result<SafetyNetStats> {
        use sqlx::Row;

        let total_files: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM protected_files")
                .fetch_one(&self.pool)
                .await?;

        let files_today: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM protected_files WHERE created_at >= date('now')",
        )
        .fetch_one(&self.pool)
        .await?;

        let oldest: Option<String> = sqlx::query_scalar(
            "SELECT MIN(created_at) FROM protected_files",
        )
        .fetch_one(&self.pool)
        .await?;

        let newest: Option<String> = sqlx::query_scalar(
            "SELECT MAX(created_at) FROM protected_files",
        )
        .fetch_one(&self.pool)
        .await?;

        let agent_rows = sqlx::query(
            "SELECT agent_name, COUNT(*) as cnt FROM protected_files GROUP BY agent_name",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut by_agent = HashMap::new();
        for row in &agent_rows {
            let name: String = row.try_get("agent_name").unwrap_or_default();
            let cnt: i64 = row.try_get("cnt").unwrap_or(0);
            if !name.is_empty() {
                by_agent.insert(name, cnt);
            }
        }

        let restored_today: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM protected_files WHERE restored = 1 AND created_at >= date('now')",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(SafetyNetStats {
            total_files,
            files_today,
            total_storage_bytes: storage_bytes,
            storage_limit_bytes: storage_limit,
            oldest_snapshot: oldest,
            newest_snapshot: newest,
            by_agent,
            restored_today,
        })
    }

    pub async fn get_oldest_snapshots(&self, limit: i64) -> Result<Vec<ProtectedFile>> {
        use sqlx::Row;

        let rows = sqlx::query("SELECT * FROM protected_files ORDER BY created_at ASC LIMIT ?1")
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        let files = rows
            .iter()
            .map(|row| {
                let restored_int: i32 = row.try_get("restored").unwrap_or(0);
                ProtectedFile {
                    id: row.try_get("id").unwrap_or_default(),
                    original_path: row.try_get("original_path").unwrap_or_default(),
                    snapshot_path: row.try_get("snapshot_path").unwrap_or_default(),
                    file_size: row.try_get::<i64, _>("file_size").unwrap_or(0) as u64,
                    agent_id: row.try_get("agent_id").unwrap_or_default(),
                    agent_name: row.try_get("agent_name").unwrap_or_default(),
                    action_type: row.try_get("action_type").unwrap_or_default(),
                    created_at: row.try_get("created_at").unwrap_or_default(),
                    restored: restored_int != 0,
                }
            })
            .collect();

        Ok(files)
    }

    pub async fn delete_old_snapshots(&self, before_date: &str) -> Result<i64> {
        let result = sqlx::query("DELETE FROM protected_files WHERE created_at < ?1")
            .bind(before_date)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() as i64)
    }

    // ── Data Shield CRUD ───────────────────────────────────────────

    pub async fn save_outbound_event(&self, event: &OutboundEvent) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO outbound_events (id, agent_id, agent_name, event_type, destination, url, direction, description, risk_level, timestamp, blocked)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(&event.id)
        .bind(&event.agent_id)
        .bind(&event.agent_name)
        .bind(&event.event_type)
        .bind(&event.destination)
        .bind(&event.url)
        .bind(&event.direction)
        .bind(&event.description)
        .bind(&event.risk_level)
        .bind(&event.timestamp)
        .bind(event.blocked as i32)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_outbound_events(
        &self,
        limit: i64,
        offset: i64,
        agent_id: Option<String>,
        risk_level: Option<String>,
        destination: Option<String>,
    ) -> Result<(Vec<OutboundEvent>, i64)> {
        use sqlx::Row;

        let mut query = String::from("SELECT * FROM outbound_events WHERE 1=1");
        let mut count_query = String::from("SELECT COUNT(*) FROM outbound_events WHERE 1=1");

        if let Some(ref aid) = agent_id {
            let clause = format!(" AND agent_id = '{}'", aid.replace('\'', "''"));
            query.push_str(&clause);
            count_query.push_str(&clause);
        }
        if let Some(ref rl) = risk_level {
            let clause = format!(" AND risk_level = '{}'", rl.replace('\'', "''"));
            query.push_str(&clause);
            count_query.push_str(&clause);
        }
        if let Some(ref dest) = destination {
            let escaped = dest.replace('\'', "''");
            let clause = format!(" AND destination LIKE '%{}%'", escaped);
            query.push_str(&clause);
            count_query.push_str(&clause);
        }

        let total: i64 = sqlx::query_scalar(&count_query)
            .fetch_one(&self.pool)
            .await?;

        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let events = rows
            .iter()
            .map(|row| {
                let blocked_int: i32 = row.try_get("blocked").unwrap_or(0);
                OutboundEvent {
                    id: row.try_get("id").unwrap_or_default(),
                    agent_id: row.try_get("agent_id").unwrap_or_default(),
                    agent_name: row.try_get("agent_name").unwrap_or_default(),
                    event_type: row.try_get("event_type").unwrap_or_default(),
                    destination: row.try_get("destination").unwrap_or_default(),
                    url: row.try_get("url").ok(),
                    direction: row.try_get("direction").unwrap_or_default(),
                    description: row.try_get("description").unwrap_or_default(),
                    risk_level: row.try_get("risk_level").unwrap_or_default(),
                    timestamp: row.try_get("timestamp").unwrap_or_default(),
                    blocked: blocked_int != 0,
                }
            })
            .collect();

        Ok((events, total))
    }

    pub async fn get_domain_profiles(&self) -> Result<Vec<DomainProfile>> {
        use sqlx::Row;

        let rows = sqlx::query(
            r#"
            SELECT
                destination as domain,
                MIN(timestamp) as first_seen,
                MAX(timestamp) as last_seen,
                COUNT(*) as total_events,
                risk_level,
                GROUP_CONCAT(DISTINCT agent_name) as agents_csv
            FROM outbound_events
            GROUP BY destination
            ORDER BY total_events DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let profiles = rows
            .iter()
            .map(|row| {
                let agents_csv: String = row.try_get("agents_csv").unwrap_or_default();
                let agents_using: Vec<String> = agents_csv
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();

                DomainProfile {
                    domain: row.try_get("domain").unwrap_or_default(),
                    first_seen: row.try_get("first_seen").unwrap_or_default(),
                    last_seen: row.try_get("last_seen").unwrap_or_default(),
                    total_events: row.try_get("total_events").unwrap_or(0),
                    risk_level: row.try_get("risk_level").unwrap_or_default(),
                    category: "unknown".to_string(),
                    agents_using,
                }
            })
            .collect();

        Ok(profiles)
    }

    pub async fn get_data_shield_stats(&self) -> Result<DataShieldStats> {
        use sqlx::Row;

        let total_events: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM outbound_events")
                .fetch_one(&self.pool)
                .await?;

        let events_today: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM outbound_events WHERE timestamp >= date('now')",
        )
        .fetch_one(&self.pool)
        .await?;

        let unique_domains: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT destination) FROM outbound_events",
        )
        .fetch_one(&self.pool)
        .await?;

        let trusted_domains: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT destination) FROM outbound_events WHERE risk_level = 'safe'",
        )
        .fetch_one(&self.pool)
        .await?;

        let unknown_domains: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT destination) FROM outbound_events WHERE risk_level = 'unknown'",
        )
        .fetch_one(&self.pool)
        .await?;

        let suspicious_domains: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT destination) FROM outbound_events WHERE risk_level = 'suspicious'",
        )
        .fetch_one(&self.pool)
        .await?;

        let agent_rows = sqlx::query(
            "SELECT agent_name, COUNT(*) as cnt FROM outbound_events GROUP BY agent_name",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut by_agent = HashMap::new();
        for row in &agent_rows {
            let name: String = row.try_get("agent_name").unwrap_or_default();
            let cnt: i64 = row.try_get("cnt").unwrap_or(0);
            if !name.is_empty() {
                by_agent.insert(name, cnt);
            }
        }

        Ok(DataShieldStats {
            total_events,
            events_today,
            unique_domains,
            trusted_domains,
            unknown_domains,
            suspicious_domains,
            by_agent,
        })
    }

    pub async fn save_domain_override(&self, domain: &str, classification: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO domain_overrides (domain, classification) VALUES (?1, ?2)",
        )
        .bind(domain)
        .bind(classification)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_domain_overrides(&self) -> Result<HashMap<String, String>> {
        use sqlx::Row;

        let rows = sqlx::query("SELECT domain, classification FROM domain_overrides")
            .fetch_all(&self.pool)
            .await?;

        let mut overrides = HashMap::new();
        for row in &rows {
            let domain: String = row.try_get("domain").unwrap_or_default();
            let classification: String = row.try_get("classification").unwrap_or_default();
            overrides.insert(domain, classification);
        }

        Ok(overrides)
    }

    // ── Weekly Reports CRUD ───────────────────────────────────────

    pub async fn save_weekly_report(
        &self,
        id: &str,
        week_start: &str,
        week_end: &str,
        generated_at: &str,
        report_json: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO weekly_reports (id, week_start, week_end, generated_at, report_json) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(id)
        .bind(week_start)
        .bind(week_end)
        .bind(generated_at)
        .bind(report_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_weekly_reports(&self) -> Result<Vec<serde_json::Value>> {
        use sqlx::Row;

        let rows = sqlx::query(
            "SELECT id, week_start, week_end, generated_at, report_json FROM weekly_reports ORDER BY week_start DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut reports = Vec::new();
        for row in &rows {
            let id: String = row.try_get("id").unwrap_or_default();
            let week_start: String = row.try_get("week_start").unwrap_or_default();
            let week_end: String = row.try_get("week_end").unwrap_or_default();
            let generated_at: String = row.try_get("generated_at").unwrap_or_default();
            let json_str: String = row.try_get("report_json").unwrap_or_default();

            // Extract summary fields from the full JSON
            let parsed: serde_json::Value =
                serde_json::from_str(&json_str).unwrap_or(serde_json::json!({}));
            let total_actions = parsed.get("total_actions").and_then(|v| v.as_u64()).unwrap_or(0);
            let total_cost = parsed.get("total_cost").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let security_score = parsed.get("security_score").and_then(|v| v.as_u64()).unwrap_or(0);

            reports.push(serde_json::json!({
                "id": id,
                "week_start": week_start,
                "week_end": week_end,
                "generated_at": generated_at,
                "total_actions": total_actions,
                "total_cost": total_cost,
                "security_score": security_score,
            }));
        }

        Ok(reports)
    }

    pub async fn get_weekly_report(&self, id: &str) -> Result<Option<String>> {
        let result: Option<String> = sqlx::query_scalar(
            "SELECT report_json FROM weekly_reports WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(result)
    }

    // ── Report Aggregation Queries ────────────────────────────────

    pub async fn count_actions_in_range(&self, start: &str, end: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM actions WHERE timestamp >= ?1 AND timestamp < date(?2, '+1 day')",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn actions_by_agent_in_range(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<AgentActionSummary>> {
        use sqlx::Row;

        let rows = sqlx::query(
            r#"
            SELECT a.agent_id, ag.name as agent_name, ag.agent_type,
                   COUNT(*) as action_count,
                   COALESCE(SUM(a.cost_usd), 0.0) as total_cost,
                   a.action_type as top_action
            FROM actions a
            LEFT JOIN agents ag ON a.agent_id = ag.id
            WHERE a.timestamp >= ?1 AND a.timestamp < date(?2, '+1 day')
            GROUP BY a.agent_id
            ORDER BY action_count DESC
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        let summaries = rows
            .iter()
            .map(|row| AgentActionSummary {
                agent_name: row.try_get("agent_name").unwrap_or_else(|_| {
                    row.try_get::<String, _>("agent_id").unwrap_or_default()
                }),
                agent_type: row.try_get("agent_type").unwrap_or_default(),
                action_count: row.try_get::<i64, _>("action_count").unwrap_or(0) as u64,
                cost: row.try_get("total_cost").unwrap_or(0.0),
                top_action_type: row.try_get("top_action").unwrap_or_default(),
            })
            .collect();

        Ok(summaries)
    }

    pub async fn actions_by_type_in_range(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<(String, u64)>> {
        use sqlx::Row;

        let rows = sqlx::query(
            "SELECT action_type, COUNT(*) as cnt FROM actions WHERE timestamp >= ?1 AND timestamp < date(?2, '+1 day') GROUP BY action_type ORDER BY cnt DESC",
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| {
                let t: String = r.try_get("action_type").unwrap_or_default();
                let c: i64 = r.try_get("cnt").unwrap_or(0);
                (t, c as u64)
            })
            .collect())
    }

    pub async fn actions_by_day_in_range(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<(String, u64)>> {
        use sqlx::Row;

        let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let rows = sqlx::query(
            "SELECT CAST(strftime('%w', timestamp) AS INTEGER) as dow, COUNT(*) as cnt FROM actions WHERE timestamp >= ?1 AND timestamp < date(?2, '+1 day') GROUP BY dow ORDER BY dow",
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        let mut counts = HashMap::new();
        for row in &rows {
            let dow: i32 = row.try_get("dow").unwrap_or(0);
            let cnt: i64 = row.try_get("cnt").unwrap_or(0);
            // SQLite %w: 0=Sunday, 1=Monday... convert to Mon-Sun order
            let idx = if dow == 0 { 6 } else { (dow - 1) as usize };
            counts.insert(idx, cnt as u64);
        }

        Ok(day_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.to_string(), *counts.get(&i).unwrap_or(&0)))
            .collect())
    }

    pub async fn busiest_day_hour_in_range(
        &self,
        start: &str,
        end: &str,
    ) -> Result<(String, u8)> {
        use sqlx::Row;

        let day_names = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

        let day_row = sqlx::query(
            "SELECT CAST(strftime('%w', timestamp) AS INTEGER) as dow, COUNT(*) as cnt FROM actions WHERE timestamp >= ?1 AND timestamp < date(?2, '+1 day') GROUP BY dow ORDER BY cnt DESC LIMIT 1",
        )
        .bind(start)
        .bind(end)
        .fetch_optional(&self.pool)
        .await?;

        let busiest_day = day_row
            .as_ref()
            .and_then(|r| r.try_get::<i32, _>("dow").ok())
            .map(|d| day_names[d as usize % 7].to_string())
            .unwrap_or_else(|| "Monday".to_string());

        let hour_row = sqlx::query(
            "SELECT CAST(strftime('%H', timestamp) AS INTEGER) as hr, COUNT(*) as cnt FROM actions WHERE timestamp >= ?1 AND timestamp < date(?2, '+1 day') GROUP BY hr ORDER BY cnt DESC LIMIT 1",
        )
        .bind(start)
        .bind(end)
        .fetch_optional(&self.pool)
        .await?;

        let busiest_hour = hour_row
            .as_ref()
            .and_then(|r| r.try_get::<i32, _>("hr").ok())
            .unwrap_or(0) as u8;

        Ok((busiest_day, busiest_hour))
    }

    pub async fn total_cost_in_range(&self, start: &str, end: &str) -> Result<f64> {
        let cost: Option<f64> = sqlx::query_scalar(
            "SELECT SUM(cost_usd) FROM actions WHERE timestamp >= ?1 AND timestamp < date(?2, '+1 day')",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        Ok(cost.unwrap_or(0.0))
    }

    pub async fn cost_by_agent_in_range(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<(String, f64)>> {
        use sqlx::Row;

        let rows = sqlx::query(
            "SELECT ag.name as agent_name, COALESCE(SUM(a.cost_usd), 0.0) as total FROM actions a LEFT JOIN agents ag ON a.agent_id = ag.id WHERE a.timestamp >= ?1 AND a.timestamp < date(?2, '+1 day') GROUP BY a.agent_id ORDER BY total DESC",
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| {
                let name: String = r.try_get("agent_name").unwrap_or_default();
                let cost: f64 = r.try_get("total").unwrap_or(0.0);
                (name, cost)
            })
            .collect())
    }

    pub async fn cost_by_day_in_range(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<(String, f64)>> {
        use sqlx::Row;

        let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let rows = sqlx::query(
            "SELECT CAST(strftime('%w', timestamp) AS INTEGER) as dow, COALESCE(SUM(cost_usd), 0.0) as total FROM actions WHERE timestamp >= ?1 AND timestamp < date(?2, '+1 day') GROUP BY dow ORDER BY dow",
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        let mut costs: HashMap<usize, f64> = HashMap::new();
        for row in &rows {
            let dow: i32 = row.try_get("dow").unwrap_or(0);
            let total: f64 = row.try_get("total").unwrap_or(0.0);
            let idx = if dow == 0 { 6 } else { (dow - 1) as usize };
            costs.insert(idx, total);
        }

        Ok(day_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.to_string(), *costs.get(&i).unwrap_or(&0.0)))
            .collect())
    }

    pub async fn count_protected_in_range(&self, start: &str, end: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM protected_files WHERE created_at >= ?1 AND created_at < date(?2, '+1 day')",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_restored_in_range(&self, start: &str, end: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM protected_files WHERE restored = 1 AND created_at >= ?1 AND created_at < date(?2, '+1 day')",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn safety_net_total_size_mb(&self) -> Result<f64> {
        let bytes: Option<i64> = sqlx::query_scalar(
            "SELECT SUM(file_size) FROM protected_files",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(bytes.unwrap_or(0) as f64 / 1_048_576.0)
    }

    pub async fn count_pii_in_range(&self, start: &str, end: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pii_findings WHERE dismissed = 0 AND timestamp >= ?1 AND timestamp < date(?2, '+1 day')",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_pii_critical_in_range(&self, start: &str, end: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pii_findings WHERE dismissed = 0 AND severity = 'critical' AND timestamp >= ?1 AND timestamp < date(?2, '+1 day')",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_pii_high_in_range(&self, start: &str, end: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pii_findings WHERE dismissed = 0 AND severity = 'high' AND timestamp >= ?1 AND timestamp < date(?2, '+1 day')",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_domains_in_range(&self, start: &str, end: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT destination) FROM outbound_events WHERE timestamp >= ?1 AND timestamp < date(?2, '+1 day')",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_unknown_domains_in_range(&self, start: &str, end: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT destination) FROM outbound_events WHERE risk_level = 'unknown' AND timestamp >= ?1 AND timestamp < date(?2, '+1 day')",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_outbound_in_range(&self, start: &str, end: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM outbound_events WHERE timestamp >= ?1 AND timestamp < date(?2, '+1 day')",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    // ── Firewall Rule CRUD ───────────────────────────────────────────

    pub async fn save_firewall_rule(&self, rule: &FirewallRule) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO firewall_rules (id, name, description, agent_pattern, allow_tools, deny_tools, conditions, priority, enabled, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(&rule.agent_pattern)
        .bind(serde_json::to_string(&rule.allow_tools)?)
        .bind(serde_json::to_string(&rule.deny_tools)?)
        .bind(serde_json::to_string(&rule.conditions)?)
        .bind(rule.priority)
        .bind(rule.enabled as i32)
        .bind(&rule.created_at)
        .bind(&rule.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_firewall_rules(&self) -> Result<Vec<FirewallRule>> {
        let rows = sqlx::query("SELECT * FROM firewall_rules ORDER BY priority DESC")
            .fetch_all(&self.pool)
            .await?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(self.row_to_firewall_rule(&row)?);
        }
        Ok(rules)
    }

    pub async fn update_firewall_rule(&self, rule: &FirewallRule) -> Result<()> {
        self.save_firewall_rule(rule).await
    }

    pub async fn delete_firewall_rule(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM firewall_rules WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn toggle_firewall_rule(&self, id: &str, enabled: bool) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE firewall_rules SET enabled = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(enabled as i32)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    fn row_to_firewall_rule(&self, row: &sqlx::sqlite::SqliteRow) -> Result<FirewallRule> {
        use sqlx::Row;
        let allow_str: String = row.try_get("allow_tools").unwrap_or_else(|_| "[]".to_string());
        let deny_str: String = row.try_get("deny_tools").unwrap_or_else(|_| "[]".to_string());
        let cond_str: String = row.try_get("conditions").unwrap_or_else(|_| "[]".to_string());

        Ok(FirewallRule {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get::<String, _>("description").unwrap_or_default(),
            agent_pattern: row.try_get::<String, _>("agent_pattern").unwrap_or_else(|_| "*".to_string()),
            allow_tools: serde_json::from_str(&allow_str).unwrap_or_default(),
            deny_tools: serde_json::from_str(&deny_str).unwrap_or_default(),
            conditions: serde_json::from_str(&cond_str).unwrap_or_default(),
            priority: row.try_get("priority").unwrap_or(0),
            enabled: row.try_get::<i32, _>("enabled").unwrap_or(1) != 0,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    // ── Firewall Decision CRUD ──────────────────────────────────────

    pub async fn save_firewall_decision(&self, decision: &FirewallDecision) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO firewall_decisions (id, action_id, timestamp, agent_id, agent_name, tool_name, mcp_server, arguments, decision, reason, rule_id, rule_name)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
        )
        .bind(&decision.id)
        .bind(&decision.action_id)
        .bind(&decision.timestamp)
        .bind(&decision.agent_id)
        .bind(&decision.agent_name)
        .bind(&decision.tool_name)
        .bind(&decision.mcp_server)
        .bind(serde_json::to_string(&decision.arguments)?)
        .bind(decision.decision.to_string())
        .bind(&decision.reason)
        .bind(&decision.rule_id)
        .bind(&decision.rule_name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_firewall_decisions(
        &self,
        limit: i64,
        offset: i64,
        agent_id: Option<&str>,
        decision_filter: Option<&str>,
    ) -> Result<(Vec<FirewallDecision>, i64)> {
        let total: i64;
        let rows;

        match (agent_id, decision_filter) {
            (Some(aid), Some(df)) => {
                total = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM firewall_decisions WHERE agent_id = ?1 AND decision = ?2",
                )
                .bind(aid)
                .bind(df)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

                rows = sqlx::query(
                    "SELECT * FROM firewall_decisions WHERE agent_id = ?1 AND decision = ?2 ORDER BY timestamp DESC LIMIT ?3 OFFSET ?4",
                )
                .bind(aid)
                .bind(df)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;
            }
            (Some(aid), None) => {
                total = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM firewall_decisions WHERE agent_id = ?1",
                )
                .bind(aid)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

                rows = sqlx::query(
                    "SELECT * FROM firewall_decisions WHERE agent_id = ?1 ORDER BY timestamp DESC LIMIT ?2 OFFSET ?3",
                )
                .bind(aid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;
            }
            (None, Some(df)) => {
                total = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM firewall_decisions WHERE decision = ?1",
                )
                .bind(df)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

                rows = sqlx::query(
                    "SELECT * FROM firewall_decisions WHERE decision = ?1 ORDER BY timestamp DESC LIMIT ?2 OFFSET ?3",
                )
                .bind(df)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;
            }
            (None, None) => {
                total = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM firewall_decisions",
                )
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

                rows = sqlx::query(
                    "SELECT * FROM firewall_decisions ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;
            }
        }

        let mut decisions = Vec::new();
        for row in rows {
            decisions.push(self.row_to_firewall_decision(&row)?);
        }

        Ok((decisions, total))
    }

    fn row_to_firewall_decision(&self, row: &sqlx::sqlite::SqliteRow) -> Result<FirewallDecision> {
        use sqlx::Row;
        let args_str: String = row.try_get("arguments").unwrap_or_else(|_| "{}".to_string());
        let decision_str: String = row.try_get("decision")?;
        let decision = match decision_str.as_str() {
            "Blocked" => DecisionType::Blocked,
            "Flagged" => DecisionType::Flagged,
            _ => DecisionType::Allowed,
        };

        Ok(FirewallDecision {
            id: row.try_get("id")?,
            action_id: row.try_get("action_id").unwrap_or_default(),
            timestamp: row.try_get("timestamp")?,
            agent_id: row.try_get("agent_id")?,
            agent_name: row.try_get::<String, _>("agent_name").unwrap_or_default(),
            tool_name: row.try_get("tool_name")?,
            mcp_server: row.try_get("mcp_server").ok(),
            arguments: serde_json::from_str(&args_str).unwrap_or(serde_json::json!({})),
            decision,
            reason: row.try_get("reason").unwrap_or_default(),
            rule_id: row.try_get("rule_id").ok(),
            rule_name: row.try_get("rule_name").ok(),
        })
    }

    pub async fn get_firewall_stats(&self) -> Result<FirewallStats> {
        let total_rules: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM firewall_rules")
                .fetch_one(&self.pool)
                .await?;

        let active_rules: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM firewall_rules WHERE enabled = 1")
                .fetch_one(&self.pool)
                .await?;

        let total_decisions: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM firewall_decisions")
                .fetch_one(&self.pool)
                .await?;

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let today_start = format!("{}T00:00:00", today);

        let decisions_today: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM firewall_decisions WHERE timestamp >= ?1",
        )
        .bind(&today_start)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let blocked_today: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM firewall_decisions WHERE decision = 'Blocked' AND timestamp >= ?1",
        )
        .bind(&today_start)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let flagged_today: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM firewall_decisions WHERE decision = 'Flagged' AND timestamp >= ?1",
        )
        .bind(&today_start)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let allowed_today = decisions_today - blocked_today - flagged_today;

        // Top blocked tools
        let top_rows = sqlx::query(
            "SELECT tool_name, COUNT(*) as cnt FROM firewall_decisions WHERE decision = 'Blocked' GROUP BY tool_name ORDER BY cnt DESC LIMIT 5",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut top_blocked_tools = Vec::new();
        for row in &top_rows {
            use sqlx::Row;
            let name: String = row.try_get("tool_name")?;
            let cnt: i64 = row.try_get("cnt")?;
            top_blocked_tools.push((name, cnt));
        }

        // By agent
        let agent_rows = sqlx::query(
            "SELECT agent_name, COUNT(*) as cnt FROM firewall_decisions WHERE decision != 'Allowed' GROUP BY agent_name ORDER BY cnt DESC LIMIT 10",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut by_agent = HashMap::new();
        for row in &agent_rows {
            use sqlx::Row;
            let name: String = row.try_get("agent_name").unwrap_or_default();
            let cnt: i64 = row.try_get("cnt")?;
            by_agent.insert(name, cnt);
        }

        Ok(FirewallStats {
            total_rules,
            active_rules,
            total_decisions,
            decisions_today,
            blocked_today,
            flagged_today,
            allowed_today,
            top_blocked_tools,
            by_agent,
        })
    }
}

/// Parse a Rust Debug-formatted serde_json::Value back into actual JSON.
/// Handles: Object {"k": String("v"), "k2": Number(123)}, String("x"), Number(x), Bool(x), Null
fn parse_debug_value(s: &str) -> serde_json::Value {
    let s = s.trim();

    if s.starts_with("Object {") || s.starts_with('{') {
        // Strip "Object " prefix
        let inner = if s.starts_with("Object ") { &s[7..] } else { s };
        // Try to reconstruct valid JSON by replacing Debug tokens
        let json_str = convert_debug_to_json(inner);
        serde_json::from_str(&json_str).unwrap_or(serde_json::json!({}))
    } else if s.starts_with("String(\"") && s.ends_with("\")") {
        let inner = &s[8..s.len() - 2];
        serde_json::Value::String(inner.to_string())
    } else if s.starts_with("Number(") && s.ends_with(')') {
        let inner = &s[7..s.len() - 1];
        if let Ok(n) = inner.parse::<i64>() {
            serde_json::json!(n)
        } else if let Ok(n) = inner.parse::<f64>() {
            serde_json::json!(n)
        } else {
            serde_json::json!(null)
        }
    } else if s == "Bool(true)" {
        serde_json::json!(true)
    } else if s == "Bool(false)" {
        serde_json::json!(false)
    } else if s == "Null" {
        serde_json::json!(null)
    } else {
        serde_json::json!({})
    }
}

/// Convert Rust Debug object format to valid JSON string.
/// Replaces String("..."), Number(...), Bool(...), Null tokens with JSON equivalents.
fn convert_debug_to_json(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Check for String(" — match char by char to avoid byte indexing issues
        if i + 8 <= len && chars_match(&chars[i..], &['S','t','r','i','n','g','(','\"']) {
            result.push('"');
            i += 8;
            // Copy until closing ")
            while i < len {
                if chars[i] == '\\' && i + 1 < len {
                    // Escape: emit both chars but JSON-escape the inner content
                    let next = chars[i + 1];
                    if next == '"' || next == '\\' || next == 'n' || next == 'r' || next == 't' {
                        result.push('\\');
                        result.push(next);
                    } else {
                        result.push('\\');
                        result.push(next);
                    }
                    i += 2;
                    continue;
                }
                if chars[i] == '"' && i + 1 < len && chars[i + 1] == ')' {
                    result.push('"');
                    i += 2; // skip ")
                    break;
                }
                // Escape chars that are invalid in JSON strings
                match chars[i] {
                    '"' => result.push_str("\\\""),
                    '\n' => result.push_str("\\n"),
                    '\r' => result.push_str("\\r"),
                    '\t' => result.push_str("\\t"),
                    c => result.push(c),
                }
                i += 1;
            }
            continue;
        }
        // Check for Number(
        if i + 7 <= len && chars_match(&chars[i..], &['N','u','m','b','e','r','(']) {
            i += 7;
            while i < len && chars[i] != ')' {
                result.push(chars[i]);
                i += 1;
            }
            if i < len { i += 1; } // skip )
            continue;
        }
        // Check for Bool(true)
        if i + 10 <= len && chars_match(&chars[i..], &['B','o','o','l','(','t','r','u','e',')']) {
            result.push_str("true");
            i += 10;
            continue;
        }
        // Check for Bool(false)
        if i + 11 <= len && chars_match(&chars[i..], &['B','o','o','l','(','f','a','l','s','e',')']) {
            result.push_str("false");
            i += 11;
            continue;
        }
        // Check for Null
        if i + 4 <= len && chars_match(&chars[i..], &['N','u','l','l']) {
            result.push_str("null");
            i += 4;
            continue;
        }
        // Skip "Array " prefix
        if i + 6 <= len && chars_match(&chars[i..], &['A','r','r','a','y',' ']) {
            i += 6;
            continue;
        }
        // Skip "Object " prefix
        if i + 7 <= len && chars_match(&chars[i..], &['O','b','j','e','c','t',' ']) {
            i += 7;
            continue;
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Helper: check if a char slice starts with the given pattern.
fn chars_match(slice: &[char], pattern: &[char]) -> bool {
    slice.len() >= pattern.len() && slice[..pattern.len()] == *pattern
}
