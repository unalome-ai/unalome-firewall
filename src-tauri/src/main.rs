// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

mod data_shield;
mod database;
mod discovery;
mod firewall;
mod models;
mod parsers;
mod pii;
mod reports;
mod safety_net;
mod scanner;

use data_shield::{DataShieldEngine, DataShieldStats, DomainProfile, OutboundEvent};
use database::Database;
use discovery::AgentDiscovery;
use firewall::{DecisionType, FirewallDecision, FirewallEngine, FirewallRule, FirewallStats};
use models::{ActionType, AgentPlan};
use parsers::AgentWatcher;
use pii::{PiiFinding, PiiScanner, PiiStats};
use std::collections::{HashMap, HashSet};
use safety_net::{RestoreResult, SafetyNetEngine, SafetyNetSettings, SafetyNetStats};
use scanner::SecurityScanner;

#[tauri::command]
async fn discover_agents() -> Result<Vec<models::Agent>, String> {
    let discovery = AgentDiscovery::new();
    match discovery.discover_all().await {
        Ok(agents) => Ok(agents),
        Err(e) => Err(format!("Discovery failed: {}", e)),
    }
}

#[tauri::command]
async fn get_agent_actions(agent_id: String) -> Result<Vec<models::Action>, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.get_actions_for_agent(&agent_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_mcp_server(
    server_config: serde_json::Value,
) -> Result<models::SecurityReport, String> {
    let scanner = SecurityScanner::new();
    match scanner.scan_mcp_server(server_config).await {
        Ok(report) => Ok(report),
        Err(e) => Err(format!("Scan failed: {}", e)),
    }
}

#[tauri::command]
async fn get_all_actions() -> Result<Vec<models::Action>, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.get_all_actions(1000).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_actions_count() -> Result<i64, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.get_actions_count().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn reset_database(
    agent_watcher: tauri::State<'_, Arc<Mutex<AgentWatcher>>>,
) -> Result<(), String> {
    let db_path = Database::db_path().map_err(|e| e.to_string())?;
    // Close any connections by dropping, then delete the file
    if db_path.exists() {
        std::fs::remove_file(&db_path).map_err(|e| e.to_string())?;
    }
    // Recreate fresh database
    let _db = Database::new().await.map_err(|e| e.to_string())?;
    // Reset parser positions so they re-read log files from the beginning
    let mut watcher = agent_watcher.lock().await;
    watcher.reset();
    Ok(())
}

#[tauri::command]
async fn scan_all_mcp_configs() -> Result<Vec<models::SecurityReport>, String> {
    let discovery = AgentDiscovery::new();
    let agents = discovery.discover_all().await.map_err(|e| e.to_string())?;
    let scanner = SecurityScanner::new();
    let mut reports = Vec::new();

    for agent in &agents {
        if let Some(mcp_path) = agent.metadata.get("mcp_config_path").and_then(|v| v.as_str()) {
            let path = std::path::Path::new(mcp_path);
            if path.exists() {
                if let Ok(content) = tokio::fs::read_to_string(path).await {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        let servers = config
                            .get("mcpServers")
                            .or_else(|| config.get("servers"));
                        if let Some(servers_obj) = servers.and_then(|s| s.as_object()) {
                            for (name, server_config) in servers_obj {
                                let mut scan_config = server_config.clone();
                                if scan_config.is_object() {
                                    scan_config
                                        .as_object_mut()
                                        .unwrap()
                                        .insert("name".to_string(), serde_json::Value::String(name.clone()));
                                }
                                match scanner.scan_mcp_server(scan_config).await {
                                    Ok(mut report) => {
                                        report.source_agent = Some(agent.name.clone());
                                        reports.push(report);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to scan MCP server '{}': {}", name, e)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(reports)
}

#[tauri::command]
async fn initialize_database() -> Result<(), String> {
    Database::initialize().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn poll_new_actions(
    agent_watcher: tauri::State<'_, Arc<Mutex<AgentWatcher>>>,
) -> Result<Vec<models::Action>, String> {
    let mut watcher = agent_watcher.lock().await;
    watcher.poll().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_pii_findings(
    limit: i64,
    offset: i64,
    severity: Option<String>,
    agent_id: Option<String>,
    dismissed: Option<bool>,
) -> Result<serde_json::Value, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    let (findings, total) = db
        .get_pii_findings(limit, offset, severity, agent_id, dismissed)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "findings": findings, "total": total }))
}

#[tauri::command]
async fn dismiss_pii_finding(id: String) -> Result<(), String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.dismiss_pii_finding(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn restore_pii_finding(id: String) -> Result<(), String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.restore_pii_finding(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_pii_stats() -> Result<PiiStats, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.get_pii_stats().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_file_for_pii(
    path: String,
    pii_scanner: tauri::State<'_, Arc<Mutex<PiiScanner>>>,
) -> Result<Vec<PiiFinding>, String> {
    let scanner = pii_scanner.lock().await;
    Ok(scanner.scan_file(&path, "manual", None))
}

#[tauri::command]
async fn delete_pii_findings_by_type(finding_type: String) -> Result<i64, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.delete_pii_findings_by_type(&finding_type)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_pii_category_settings() -> Result<HashMap<String, bool>, String> {
    let all_categories = vec![
        "api_key", "private_key", "jwt", "connection_string", "ssn",
        "credit_card", "email", "phone", "ip_address", "password", "env_variable",
    ];

    let db = Database::new().await.map_err(|e| e.to_string())?;
    let saved = db.get_pii_category_settings().await.map_err(|e| e.to_string())?;

    let mut result = HashMap::new();
    for cat in all_categories {
        let enabled = saved.get(cat).copied().unwrap_or(true);
        result.insert(cat.to_string(), enabled);
    }
    Ok(result)
}

#[tauri::command]
async fn set_pii_category_enabled(
    category: String,
    enabled: bool,
    pii_scanner: tauri::State<'_, Arc<Mutex<PiiScanner>>>,
) -> Result<(), String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.set_pii_category_enabled(&category, enabled)
        .await
        .map_err(|e| e.to_string())?;

    // Reload all settings into the scanner
    let saved = db.get_pii_category_settings().await.map_err(|e| e.to_string())?;
    let disabled: HashSet<String> = saved
        .into_iter()
        .filter(|(_, v)| !v)
        .map(|(k, _)| k)
        .collect();
    let mut scanner = pii_scanner.lock().await;
    scanner.set_disabled_categories(disabled);
    Ok(())
}

// ── Safety Net Commands ──────────────────────────────────────────────

#[tauri::command]
async fn get_protected_files(
    limit: i64,
    offset: i64,
    agent_id: Option<String>,
    action_type: Option<String>,
    search: Option<String>,
) -> Result<serde_json::Value, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    let (files, total) = db
        .get_protected_files(limit, offset, agent_id, action_type, search)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "files": files, "total": total }))
}

#[tauri::command]
async fn restore_file(
    id: String,
    snapshot_path: String,
    original_path: String,
    safety_net: tauri::State<'_, Arc<Mutex<SafetyNetEngine>>>,
) -> Result<RestoreResult, String> {
    let engine = safety_net.lock().await;
    let result = engine.restore_file(&snapshot_path, &original_path);
    if result.success {
        let db = Database::new().await.map_err(|e| e.to_string())?;
        let _ = db.mark_file_restored(&id).await;
    }
    Ok(result)
}

#[tauri::command]
async fn restore_multiple(
    files: Vec<serde_json::Value>,
    safety_net: tauri::State<'_, Arc<Mutex<SafetyNetEngine>>>,
) -> Result<Vec<RestoreResult>, String> {
    let engine = safety_net.lock().await;
    let mut results = Vec::new();
    let db = Database::new().await.map_err(|e| e.to_string())?;

    for file in &files {
        let id = file.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let snapshot = file.get("snapshot_path").and_then(|v| v.as_str()).unwrap_or("");
        let original = file.get("original_path").and_then(|v| v.as_str()).unwrap_or("");
        let result = engine.restore_file(snapshot, original);
        if result.success {
            let _ = db.mark_file_restored(id).await;
        }
        results.push(result);
    }
    Ok(results)
}

#[tauri::command]
async fn preview_file(
    snapshot_path: String,
    safety_net: tauri::State<'_, Arc<Mutex<SafetyNetEngine>>>,
) -> Result<String, String> {
    let engine = safety_net.lock().await;
    Ok(engine.preview_file(&snapshot_path))
}

#[tauri::command]
async fn get_safety_net_stats(
    safety_net: tauri::State<'_, Arc<Mutex<SafetyNetEngine>>>,
) -> Result<SafetyNetStats, String> {
    let engine = safety_net.lock().await;
    let storage = engine.get_storage_size();
    let limit = engine.settings.max_storage_bytes;
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.get_safety_net_stats(storage, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_snapshot(
    id: String,
    snapshot_path: String,
    safety_net: tauri::State<'_, Arc<Mutex<SafetyNetEngine>>>,
) -> Result<(), String> {
    let mut engine = safety_net.lock().await;
    engine.delete_snapshot_file(&snapshot_path);
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.delete_protected_file(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_old_snapshots(
    before_date: String,
    safety_net: tauri::State<'_, Arc<Mutex<SafetyNetEngine>>>,
) -> Result<i64, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    // Get snapshots to delete so we can clean up files
    let old = db.get_oldest_snapshots(10000).await.map_err(|e| e.to_string())?;
    let mut engine = safety_net.lock().await;
    for pf in &old {
        if pf.created_at < before_date {
            engine.delete_snapshot_file(&pf.snapshot_path);
        }
    }
    db.delete_old_snapshots(&before_date)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_safety_net_settings(
    max_storage_bytes: u64,
    retention_days: u32,
    safety_net: tauri::State<'_, Arc<Mutex<SafetyNetEngine>>>,
) -> Result<(), String> {
    let mut engine = safety_net.lock().await;
    engine.update_settings(SafetyNetSettings {
        max_storage_bytes,
        retention_days,
    });
    Ok(())
}

// ── Data Shield Commands ────────────────────────────────────────────

#[tauri::command]
async fn get_outbound_events(
    limit: i64,
    offset: i64,
    agent_id: Option<String>,
    risk_level: Option<String>,
    destination: Option<String>,
) -> Result<serde_json::Value, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    let (events, total) = db
        .get_outbound_events(limit, offset, agent_id, risk_level, destination)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "events": events, "total": total }))
}

#[tauri::command]
async fn get_domain_profiles(
    data_shield: tauri::State<'_, Arc<Mutex<DataShieldEngine>>>,
) -> Result<Vec<DomainProfile>, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    let mut profiles = db.get_domain_profiles().await.map_err(|e| e.to_string())?;

    // Enrich profiles with proper category from the engine
    let engine = data_shield.lock().await;
    for profile in &mut profiles {
        let (risk, category) = engine.classify_domain(&profile.domain);
        profile.category = category;
        profile.risk_level = risk;
    }

    Ok(profiles)
}

#[tauri::command]
async fn get_data_shield_stats() -> Result<DataShieldStats, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.get_data_shield_stats().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn classify_domain(
    domain: String,
    classification: String,
    data_shield: tauri::State<'_, Arc<Mutex<DataShieldEngine>>>,
) -> Result<(), String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.save_domain_override(&domain, &classification)
        .await
        .map_err(|e| e.to_string())?;

    let mut engine = data_shield.lock().await;
    engine.classify_domain_manual(&domain, &classification);

    Ok(())
}

// ── Weekly Reports Commands ──────────────────────────────────────────

#[tauri::command]
async fn generate_weekly_report(
    week_start: Option<String>,
) -> Result<reports::WeeklyReport, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    let report = reports::generate_report(&db, week_start)
        .await
        .map_err(|e| e.to_string())?;

    let report_json = serde_json::to_string(&report).map_err(|e| e.to_string())?;
    db.save_weekly_report(&report.id, &report.week_start, &report.week_end, &report.generated_at, &report_json)
        .await
        .map_err(|e| e.to_string())?;

    Ok(report)
}

#[tauri::command]
async fn get_weekly_reports() -> Result<Vec<serde_json::Value>, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.get_weekly_reports().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_weekly_report(id: String) -> Result<serde_json::Value, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    match db.get_weekly_report(&id).await.map_err(|e| e.to_string())? {
        Some(json_str) => serde_json::from_str(&json_str).map_err(|e| e.to_string()),
        None => Err("Report not found".to_string()),
    }
}

#[tauri::command]
async fn export_report_html(id: String) -> Result<String, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    match db.get_weekly_report(&id).await.map_err(|e| e.to_string())? {
        Some(json_str) => {
            let report: reports::WeeklyReport =
                serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
            Ok(reports::generate_report_html(&report))
        }
        None => Err("Report not found".to_string()),
    }
}

#[tauri::command]
async fn save_report_as_file(id: String, path: Option<String>) -> Result<String, String> {
    let html = export_report_html(id.clone()).await?;

    let save_path = match path {
        Some(p) => p,
        None => {
            // Default to Desktop
            let desktop = dirs::desktop_dir()
                .or_else(|| dirs::home_dir())
                .ok_or("Cannot determine home directory")?;
            let db = Database::new().await.map_err(|e| e.to_string())?;
            let week_start = match db.get_weekly_report(&id).await.map_err(|e| e.to_string())? {
                Some(json_str) => {
                    let report: reports::WeeklyReport =
                        serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
                    report.week_start
                }
                None => "report".to_string(),
            };
            desktop
                .join(format!("unalome-report-{}.html", week_start))
                .to_string_lossy()
                .to_string()
        }
    };

    tokio::fs::write(&save_path, html)
        .await
        .map_err(|e| e.to_string())?;

    Ok(save_path)
}

#[tauri::command]
async fn rescan_mcp_configs(
    data_shield: tauri::State<'_, Arc<Mutex<DataShieldEngine>>>,
) -> Result<Vec<OutboundEvent>, String> {
    let discovery = AgentDiscovery::new();
    let agents = discovery.discover_all().await.map_err(|e| e.to_string())?;

    let engine = data_shield.lock().await;
    let events = engine.scan_mcp_configs(&agents);

    let db = Database::new().await.map_err(|e| e.to_string())?;
    for event in &events {
        let _ = db.save_outbound_event(event).await;
    }

    Ok(events)
}

// ── Firewall Commands ────────────────────────────────────────────────

#[tauri::command]
async fn get_firewall_rules() -> Result<Vec<FirewallRule>, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.get_firewall_rules().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_firewall_rule(
    rule: FirewallRule,
    firewall_engine: tauri::State<'_, Arc<Mutex<FirewallEngine>>>,
) -> Result<FirewallRule, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.save_firewall_rule(&rule).await.map_err(|e| e.to_string())?;

    // Reload rules into engine
    let rules = db.get_firewall_rules().await.map_err(|e| e.to_string())?;
    let mut engine = firewall_engine.lock().await;
    engine.load_rules(rules);

    Ok(rule)
}

#[tauri::command]
async fn update_firewall_rule(
    rule: FirewallRule,
    firewall_engine: tauri::State<'_, Arc<Mutex<FirewallEngine>>>,
) -> Result<(), String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.update_firewall_rule(&rule).await.map_err(|e| e.to_string())?;

    let rules = db.get_firewall_rules().await.map_err(|e| e.to_string())?;
    let mut engine = firewall_engine.lock().await;
    engine.load_rules(rules);

    Ok(())
}

#[tauri::command]
async fn delete_firewall_rule(
    id: String,
    firewall_engine: tauri::State<'_, Arc<Mutex<FirewallEngine>>>,
) -> Result<(), String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.delete_firewall_rule(&id).await.map_err(|e| e.to_string())?;

    let rules = db.get_firewall_rules().await.map_err(|e| e.to_string())?;
    let mut engine = firewall_engine.lock().await;
    engine.load_rules(rules);

    Ok(())
}

#[tauri::command]
async fn toggle_firewall_rule(
    id: String,
    enabled: bool,
    firewall_engine: tauri::State<'_, Arc<Mutex<FirewallEngine>>>,
) -> Result<(), String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.toggle_firewall_rule(&id, enabled).await.map_err(|e| e.to_string())?;

    let rules = db.get_firewall_rules().await.map_err(|e| e.to_string())?;
    let mut engine = firewall_engine.lock().await;
    engine.load_rules(rules);

    Ok(())
}

#[tauri::command]
async fn get_firewall_decisions(
    limit: i64,
    offset: i64,
    agent_id: Option<String>,
    decision_filter: Option<String>,
) -> Result<serde_json::Value, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    let (decisions, total) = db
        .get_firewall_decisions(
            limit,
            offset,
            agent_id.as_deref(),
            decision_filter.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "decisions": decisions, "total": total }))
}

#[tauri::command]
async fn get_firewall_stats() -> Result<FirewallStats, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.get_firewall_stats().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn test_firewall_rule(
    agent_name: String,
    tool_name: String,
    args: serde_json::Value,
    firewall_engine: tauri::State<'_, Arc<Mutex<FirewallEngine>>>,
) -> Result<FirewallDecision, String> {
    let engine = firewall_engine.lock().await;
    let decision = engine.evaluate("test", "test-agent", &agent_name, &tool_name, &args);
    Ok(decision)
}

// ── Agent Plans Commands ────────────────────────────────────────────

#[tauri::command]
async fn scan_agent_plans() -> Result<Vec<AgentPlan>, String> {
    let plans_dir = dirs::home_dir()
        .ok_or("Cannot determine home directory")?
        .join(".claude")
        .join("plans");

    if !plans_dir.exists() {
        return Ok(Vec::new());
    }

    let db = Database::new().await.map_err(|e| e.to_string())?;
    let mut active_file_names = Vec::new();

    let entries: Vec<_> = std::fs::read_dir(&plans_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .collect();

    for entry in &entries {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        let slug = file_name.trim_end_matches(".md").to_string();
        let display_name = slug
            .split('-')
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        let content = std::fs::read_to_string(&path).unwrap_or_default();

        let title = content
            .lines()
            .find(|l| l.starts_with("# "))
            .map(|l| {
                let heading = l.trim_start_matches("# ").trim();
                // Strip "Plan: " prefix if present
                heading.strip_prefix("Plan: ").unwrap_or(heading).to_string()
            });

        let created_at = meta.created()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let modified_at = meta.modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let plan = AgentPlan {
            id: format!("plan-{}", slug),
            file_name: file_name.clone(),
            slug: slug.clone(),
            file_path: path.to_string_lossy().to_string(),
            display_name,
            title,
            file_size: meta.len(),
            created_at: chrono::DateTime::<chrono::Utc>::from(created_at).to_rfc3339(),
            modified_at: chrono::DateTime::<chrono::Utc>::from(modified_at).to_rfc3339(),
            content,
            action_count: 0,
        };

        let _ = db.upsert_plan(&plan).await;
        active_file_names.push(file_name);
    }

    let _ = db.delete_stale_plans(&active_file_names).await;

    let action_counts = db.get_plan_action_counts().await.unwrap_or_default();
    let mut plans = db.get_all_plans().await.map_err(|e| e.to_string())?;
    for plan in &mut plans {
        plan.action_count = action_counts.get(&plan.slug).copied().unwrap_or(0);
    }

    Ok(plans)
}

#[tauri::command]
async fn get_agent_plans() -> Result<Vec<AgentPlan>, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    let action_counts = db.get_plan_action_counts().await.unwrap_or_default();
    let mut plans = db.get_all_plans().await.map_err(|e| e.to_string())?;
    for plan in &mut plans {
        plan.action_count = action_counts.get(&plan.slug).copied().unwrap_or(0);
    }
    Ok(plans)
}

#[tauri::command]
async fn get_plan_actions(slug: String) -> Result<Vec<models::Action>, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    db.get_actions_for_plan(&slug).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_agent_plan_content(id: String) -> Result<String, String> {
    let db = Database::new().await.map_err(|e| e.to_string())?;
    let plans = db.get_all_plans().await.map_err(|e| e.to_string())?;
    plans.into_iter()
        .find(|p| p.id == id)
        .map(|p| p.content)
        .ok_or_else(|| "Plan not found".to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            discover_agents,
            get_agent_actions,
            reset_database,
            scan_mcp_server,
            scan_all_mcp_configs,
            get_all_actions,
            get_actions_count,
            initialize_database,
            poll_new_actions,
            get_pii_findings,
            dismiss_pii_finding,
            restore_pii_finding,
            get_pii_stats,
            scan_file_for_pii,
            delete_pii_findings_by_type,
            get_pii_category_settings,
            set_pii_category_enabled,
            get_protected_files,
            restore_file,
            restore_multiple,
            preview_file,
            get_safety_net_stats,
            delete_snapshot,
            clear_old_snapshots,
            update_safety_net_settings,
            get_outbound_events,
            get_domain_profiles,
            get_data_shield_stats,
            classify_domain,
            rescan_mcp_configs,
            generate_weekly_report,
            get_weekly_reports,
            get_weekly_report,
            export_report_html,
            save_report_as_file,
            get_firewall_rules,
            create_firewall_rule,
            update_firewall_rule,
            delete_firewall_rule,
            toggle_firewall_rule,
            get_firewall_decisions,
            get_firewall_stats,
            test_firewall_rule,
            scan_agent_plans,
            get_agent_plans,
            get_plan_actions,
            get_agent_plan_content,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // Initialize SafetyNetEngine and register as managed state
            let safety_net = Arc::new(Mutex::new(SafetyNetEngine::new(SafetyNetSettings::default())));
            app_handle.manage(safety_net.clone());

            // Initialize DataShieldEngine and register as managed state
            let data_shield = Arc::new(Mutex::new(DataShieldEngine::new()));
            app_handle.manage(data_shield.clone());

            // Initialize FirewallEngine and register as managed state
            let firewall_engine = Arc::new(Mutex::new(FirewallEngine::new()));
            app_handle.manage(firewall_engine.clone());

            tauri::async_runtime::spawn(async move {
                // Initialize database
                if let Err(e) = Database::initialize().await {
                    eprintln!("Failed to initialize database: {}", e);
                    return;
                }

                // Discover agents and create parser-per-agent watcher
                let discovery = AgentDiscovery::new();
                let agents = match discovery.discover_all().await {
                    Ok(agents) => agents,
                    Err(e) => {
                        eprintln!("Agent discovery failed: {}", e);
                        Vec::new()
                    }
                };

                // Save discovered agents to DB so FK constraints on actions are satisfied
                if let Ok(db) = Database::new().await {
                    for agent in &agents {
                        if let Err(e) = db.save_agent(agent).await {
                            eprintln!("[Unalome] Failed to save agent {}: {}", agent.id, e);
                        }
                    }
                }

                // Load domain overrides into DataShieldEngine and scan MCP configs
                if let Ok(db) = Database::new().await {
                    if let Ok(overrides) = db.get_domain_overrides().await {
                        let mut engine = data_shield.lock().await;
                        engine.user_classifications = overrides;

                        let mcp_events = engine.scan_mcp_configs(&agents);
                        for event in &mcp_events {
                            let _ = db.save_outbound_event(event).await;
                            let _ = app_handle.emit("outbound-event", event);
                        }
                    }
                }

                // Load firewall rules into engine, seeding defaults if empty
                if let Ok(db) = Database::new().await {
                    if let Ok(rules) = db.get_firewall_rules().await {
                        if rules.is_empty() {
                            // Seed 3 predefined firewall rules
                            let now = chrono::Utc::now().to_rfc3339();
                            let defaults = vec![
                                FirewallRule {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    name: "Block destructive commands".to_string(),
                                    description: "Blocks rm, rmdir, and delete operations to prevent accidental data loss".to_string(),
                                    agent_pattern: "*".to_string(),
                                    allow_tools: vec![],
                                    deny_tools: vec![],
                                    conditions: vec![firewall::RuleCondition {
                                        tool_pattern: "Bash".to_string(),
                                        condition_type: firewall::ConditionType::ArgContains,
                                        value: "rm ".to_string(),
                                    }],
                                    priority: 100,
                                    enabled: true,
                                    created_at: now.clone(),
                                    updated_at: now.clone(),
                                },
                                FirewallRule {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    name: "Flag writes to system directories".to_string(),
                                    description: "Flags any file writes targeting /etc, /usr, or /System directories".to_string(),
                                    agent_pattern: "*".to_string(),
                                    allow_tools: vec![],
                                    deny_tools: vec![],
                                    conditions: vec![firewall::RuleCondition {
                                        tool_pattern: "Write".to_string(),
                                        condition_type: firewall::ConditionType::PathRestriction,
                                        value: "/etc,/usr,/System".to_string(),
                                    }],
                                    priority: 90,
                                    enabled: true,
                                    created_at: now.clone(),
                                    updated_at: now.clone(),
                                },
                                FirewallRule {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    name: "Block data exfiltration via curl/wget".to_string(),
                                    description: "Blocks shell commands that use curl or wget to prevent unauthorized data transfer".to_string(),
                                    agent_pattern: "*".to_string(),
                                    allow_tools: vec![],
                                    deny_tools: vec![],
                                    conditions: vec![firewall::RuleCondition {
                                        tool_pattern: "Bash".to_string(),
                                        condition_type: firewall::ConditionType::ArgContains,
                                        value: "curl ".to_string(),
                                    }],
                                    priority: 95,
                                    enabled: true,
                                    created_at: now.clone(),
                                    updated_at: now,
                                },
                            ];
                            for rule in &defaults {
                                let _ = db.save_firewall_rule(rule).await;
                            }
                            let mut engine = firewall_engine.lock().await;
                            engine.load_rules(defaults);
                        } else {
                            let mut engine = firewall_engine.lock().await;
                            engine.load_rules(rules);
                        }
                    }
                }

                let agent_watcher = Arc::new(Mutex::new(AgentWatcher::new(&agents)));
                let pii_scanner = Arc::new(Mutex::new(PiiScanner::new()));

                // Load disabled PII categories from DB
                if let Ok(db) = Database::new().await {
                    if let Ok(saved) = db.get_pii_category_settings().await {
                        let disabled: HashSet<String> = saved
                            .into_iter()
                            .filter(|(_, v)| !v)
                            .map(|(k, _)| k)
                            .collect();
                        if !disabled.is_empty() {
                            let mut scanner = pii_scanner.lock().await;
                            scanner.set_disabled_categories(disabled);
                        }
                    }
                }

                // Register watcher and pii_scanner as managed state
                app_handle.manage(agent_watcher.clone());
                app_handle.manage(pii_scanner.clone());

                // Start polling loop
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                    let mut watcher = agent_watcher.lock().await;
                    match watcher.poll() {
                        Ok(new_actions) => {
                            if !new_actions.is_empty() {
                                // Save to database
                                if let Ok(db) = Database::new().await {
                                    for action in &new_actions {
                                        let _ = db.save_action(action).await;
                                    }

                                    // Safety Net: protect files before agents modify them
                                    for action in &new_actions {
                                        if let ActionType::ToolCall { tool_name, args } = &action.action_type {
                                            if matches!(tool_name.as_str(), "Write" | "Edit") {
                                                if let Some(path) = args.get("file_path").and_then(|v| v.as_str()) {
                                                    let agent_name = agents.iter()
                                                        .find(|a| a.id == action.agent_id)
                                                        .map(|a| a.name.clone())
                                                        .unwrap_or_default();
                                                    let mut engine = safety_net.lock().await;
                                                    if let Some(pf) = engine.protect_file(path, &action.agent_id, &agent_name, "modified") {
                                                        let _ = db.save_protected_file(&pf).await;
                                                        let _ = app_handle.emit("file-protected", &pf);
                                                        if engine.storage_warning_needed() {
                                                            let _ = app_handle.emit("safety-net-warning", "Storage usage above 80%");
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // PII scanning
                                    {
                                        let scanner = pii_scanner.lock().await;
                                        for action in &new_actions {
                                            let findings = scanner.scan_action(action);
                                            for finding in &findings {
                                                let _ = db.save_pii_finding(finding).await;
                                                let _ = app_handle.emit("pii-finding", finding);
                                            }
                                        }
                                    }

                                    // Data Shield: detect outbound network activity
                                    for action in &new_actions {
                                        let agent_name = agents.iter()
                                            .find(|a| a.id == action.agent_id)
                                            .map(|a| a.name.clone())
                                            .unwrap_or_default();
                                        let engine = data_shield.lock().await;
                                        let events = engine.analyze_action(action, &agent_name);
                                        for event in &events {
                                            let _ = db.save_outbound_event(event).await;
                                            let _ = app_handle.emit("outbound-event", event);
                                        }
                                    }

                                    // Firewall: evaluate tool calls against rules
                                    for action in &new_actions {
                                        if let ActionType::ToolCall { tool_name, args } = &action.action_type {
                                            let agent_name = agents.iter()
                                                .find(|a| a.id == action.agent_id)
                                                .map(|a| a.name.clone())
                                                .unwrap_or_default();
                                            let engine = firewall_engine.lock().await;
                                            let decision = engine.evaluate(
                                                &action.id,
                                                &action.agent_id,
                                                &agent_name,
                                                tool_name,
                                                args,
                                            );
                                            if decision.decision != DecisionType::Allowed {
                                                let _ = db.save_firewall_decision(&decision).await;
                                                let _ = app_handle.emit("firewall-decision", &decision);
                                            }
                                        }
                                    }
                                }
                                // Detect plan file writes and emit plan-updated
                                for action in &new_actions {
                                    if let ActionType::ToolCall { tool_name, args } = &action.action_type {
                                        if matches!(tool_name.as_str(), "Write" | "Edit") {
                                            if let Some(path) = args.get("file_path").and_then(|v| v.as_str()) {
                                                if path.contains(".claude/plans/") {
                                                    let _ = app_handle.emit("plan-updated", path);
                                                }
                                            }
                                        }
                                    }
                                }

                                // Emit event to frontend
                                let _ = app_handle.emit("new_actions", ());
                            }
                        }
                        Err(e) => {
                            eprintln!("AgentWatcher poll error: {}", e);
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
