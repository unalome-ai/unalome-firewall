use crate::database::Database;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyReport {
    pub id: String,
    pub week_start: String,
    pub week_end: String,
    pub generated_at: String,
    pub total_actions: u64,
    pub actions_by_agent: Vec<AgentActionSummary>,
    pub actions_by_type: Vec<(String, u64)>,
    pub actions_by_day: Vec<(String, u64)>,
    pub busiest_day: String,
    pub busiest_hour: u8,
    pub total_cost: f64,
    pub cost_by_agent: Vec<(String, f64)>,
    pub cost_trend: String,
    pub cost_by_day: Vec<(String, f64)>,
    pub files_protected: u64,
    pub files_restored: u64,
    pub safety_net_size_mb: f64,
    pub pii_findings: u64,
    pub pii_critical: u64,
    pub new_mcp_servers: u64,
    pub security_score: u8,
    pub domains_contacted: u64,
    pub unknown_domains: u64,
    pub outbound_events: u64,
    pub prev_week_actions: Option<u64>,
    pub prev_week_cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActionSummary {
    pub agent_name: String,
    pub agent_type: String,
    pub action_count: u64,
    pub cost: f64,
    pub top_action_type: String,
}

/// Compute the Monday (start) of the week containing `date`.
fn week_start_for(date: NaiveDate) -> NaiveDate {
    let days_since_monday = date.weekday().num_days_from_monday();
    date - chrono::Duration::days(days_since_monday as i64)
}

pub async fn generate_report(
    db: &Database,
    week_start_opt: Option<String>,
) -> anyhow::Result<WeeklyReport> {
    let today = chrono::Local::now().date_naive();
    let week_start = match week_start_opt {
        Some(ref s) => NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap_or_else(|_| week_start_for(today)),
        None => week_start_for(today),
    };
    let week_end = week_start + chrono::Duration::days(6);

    let ws = week_start.format("%Y-%m-%d").to_string();
    let we = week_end.format("%Y-%m-%d").to_string();

    // Previous week bounds
    let prev_start = week_start - chrono::Duration::days(7);
    let prev_end = prev_start + chrono::Duration::days(6);
    let pws = prev_start.format("%Y-%m-%d").to_string();
    let pwe = prev_end.format("%Y-%m-%d").to_string();

    // Aggregate current week
    let total_actions = db.count_actions_in_range(&ws, &we).await.unwrap_or(0) as u64;
    let actions_by_agent = db.actions_by_agent_in_range(&ws, &we).await.unwrap_or_default();
    let actions_by_type = db.actions_by_type_in_range(&ws, &we).await.unwrap_or_default();
    let actions_by_day = db.actions_by_day_in_range(&ws, &we).await.unwrap_or_default();
    let (busiest_day, busiest_hour) = db.busiest_day_hour_in_range(&ws, &we).await.unwrap_or(("Monday".into(), 0));
    let total_cost = db.total_cost_in_range(&ws, &we).await.unwrap_or(0.0);
    let cost_by_agent = db.cost_by_agent_in_range(&ws, &we).await.unwrap_or_default();
    let cost_by_day = db.cost_by_day_in_range(&ws, &we).await.unwrap_or_default();

    // Safety net
    let files_protected = db.count_protected_in_range(&ws, &we).await.unwrap_or(0) as u64;
    let files_restored = db.count_restored_in_range(&ws, &we).await.unwrap_or(0) as u64;
    let safety_net_size_mb = db.safety_net_total_size_mb().await.unwrap_or(0.0);

    // PII
    let pii_findings = db.count_pii_in_range(&ws, &we).await.unwrap_or(0) as u64;
    let pii_critical = db.count_pii_critical_in_range(&ws, &we).await.unwrap_or(0) as u64;

    // Data Shield
    let domains_contacted = db.count_domains_in_range(&ws, &we).await.unwrap_or(0) as u64;
    let unknown_domains = db.count_unknown_domains_in_range(&ws, &we).await.unwrap_or(0) as u64;
    let outbound_events = db.count_outbound_in_range(&ws, &we).await.unwrap_or(0) as u64;

    // Previous week for comparison
    let prev_week_actions = db.count_actions_in_range(&pws, &pwe).await.ok().map(|v| v as u64);
    let prev_week_cost = db.total_cost_in_range(&pws, &pwe).await.ok();

    // Security score heuristic
    let mut score: i32 = 100;
    score -= (pii_critical as i32) * 20;
    let pii_high = db.count_pii_high_in_range(&ws, &we).await.unwrap_or(0);
    score -= (pii_high as i32) * 10;
    score -= (unknown_domains as i32) * 5;
    if files_protected > 0 {
        score += 5;
    }
    let security_score = score.clamp(0, 100) as u8;

    // Cost trend
    let cost_trend = match prev_week_cost {
        Some(prev) if prev > 0.0 => {
            let pct = ((total_cost - prev) / prev * 100.0).round();
            if pct > 5.0 {
                format!("up {}%", pct as i64)
            } else if pct < -5.0 {
                format!("down {}%", (-pct) as i64)
            } else {
                "stable".to_string()
            }
        }
        _ => "stable".to_string(),
    };

    let id = uuid::Uuid::new_v4().to_string();
    let generated_at = chrono::Utc::now().to_rfc3339();

    Ok(WeeklyReport {
        id,
        week_start: ws,
        week_end: we,
        generated_at,
        total_actions,
        actions_by_agent,
        actions_by_type,
        actions_by_day,
        busiest_day,
        busiest_hour,
        total_cost,
        cost_by_agent,
        cost_trend,
        cost_by_day,
        files_protected,
        files_restored,
        safety_net_size_mb,
        pii_findings,
        pii_critical,
        new_mcp_servers: 0,
        security_score,
        domains_contacted,
        unknown_domains,
        outbound_events,
        prev_week_actions,
        prev_week_cost,
    })
}

pub fn generate_report_html(report: &WeeklyReport) -> String {
    let max_actions_day = report.actions_by_day.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1);
    let max_cost_day = report.cost_by_day.iter().map(|(_, v)| *v as u64).max().unwrap_or(1).max(1) as f64;

    let score_color = if report.security_score >= 80 {
        "#10B981"
    } else if report.security_score >= 50 {
        "#F59E0B"
    } else {
        "#EF4444"
    };

    let trend_arrow = if report.cost_trend.starts_with("up") {
        r#"<span style="color:#EF4444">&#x25B2;</span>"#
    } else if report.cost_trend.starts_with("down") {
        r#"<span style="color:#10B981">&#x25BC;</span>"#
    } else {
        r#"<span style="color:#6B7280">&#x25CF;</span>"#
    };

    // Actions by day bars
    let mut actions_bars = String::new();
    for (day, count) in &report.actions_by_day {
        let pct = (*count as f64 / max_actions_day as f64 * 100.0).round();
        let short_day = &day[..3.min(day.len())];
        actions_bars.push_str(&format!(
            r#"<div style="flex:1;text-align:center"><div style="height:120px;display:flex;align-items:flex-end;justify-content:center"><div style="width:24px;height:{pct}%;background:linear-gradient(180deg,#A855F7,#6366F1);border-radius:4px 4px 0 0;min-height:4px"></div></div><div style="font-size:11px;color:#9CA3AF;margin-top:6px">{short_day}</div><div style="font-size:10px;color:#6B7280">{count}</div></div>"#,
        ));
    }

    // Cost by day bars
    let mut cost_bars = String::new();
    for (day, cost) in &report.cost_by_day {
        let pct = (*cost / max_cost_day * 100.0).round();
        let short_day = &day[..3.min(day.len())];
        cost_bars.push_str(&format!(
            r#"<div style="flex:1;text-align:center"><div style="height:80px;display:flex;align-items:flex-end;justify-content:center"><div style="width:24px;height:{pct}%;background:linear-gradient(180deg,#10B981,#059669);border-radius:4px 4px 0 0;min-height:4px"></div></div><div style="font-size:11px;color:#9CA3AF;margin-top:6px">{short_day}</div><div style="font-size:10px;color:#6B7280">${cost:.3}</div></div>"#,
        ));
    }

    // Agent ranking
    let mut agent_rows = String::new();
    for agent in &report.actions_by_agent {
        agent_rows.push_str(&format!(
            r#"<div style="display:flex;justify-content:space-between;align-items:center;padding:8px 0;border-bottom:1px solid rgba(255,255,255,0.06)"><div><div style="font-size:14px;color:#E5E7EB">{}</div><div style="font-size:11px;color:#6B7280">{}</div></div><div style="text-align:right"><div style="font-size:14px;color:#E5E7EB">{} actions</div><div style="font-size:11px;color:#6B7280">${:.4}</div></div></div>"#,
            agent.agent_name, agent.agent_type, agent.action_count, agent.cost
        ));
    }

    // SVG circular gauge
    let circumference = 2.0 * std::f64::consts::PI * 45.0;
    let offset = circumference * (1.0 - report.security_score as f64 / 100.0);

    let prev_actions_html = match report.prev_week_actions {
        Some(prev) if prev > 0 => {
            let diff = report.total_actions as i64 - prev as i64;
            let arrow = if diff > 0 { "&#x25B2;" } else if diff < 0 { "&#x25BC;" } else { "&#x25CF;" };
            let color = if diff > 0 { "#F59E0B" } else { "#10B981" };
            format!(r#"<span style="font-size:14px;color:{color}">{arrow} {diff:+} vs last week</span>"#)
        }
        _ => String::new(),
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=1200">
<title>Unalome Weekly Report — {week_start} to {week_end}</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{background:#0a0a0f;color:#E5E7EB;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;
background-image:radial-gradient(rgba(255,255,255,0.03) 1px,transparent 1px);background-size:20px 20px}}
.container{{max-width:1200px;margin:0 auto;padding:40px 32px}}
.header{{display:flex;justify-content:space-between;align-items:center;margin-bottom:40px}}
.header-left h1{{font-size:28px;font-weight:700;background:linear-gradient(135deg,#fff,rgba(255,255,255,0.7));-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text}}
.header-left p{{font-size:14px;color:#9CA3AF;margin-top:4px}}
.gauge{{position:relative;width:120px;height:120px}}
.gauge svg{{transform:rotate(-90deg)}}
.gauge .score{{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;flex-direction:column}}
.gauge .score .num{{font-size:32px;font-weight:700;color:{score_color}}}
.gauge .score .label{{font-size:10px;color:#9CA3AF;text-transform:uppercase;letter-spacing:1px}}
.section{{background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.06);border-radius:16px;padding:24px;margin-bottom:24px}}
.section-title{{font-size:16px;font-weight:600;color:#D1D5DB;margin-bottom:16px;display:flex;align-items:center;gap:8px}}
.big-num{{font-size:48px;font-weight:700;background:linear-gradient(135deg,#fff,rgba(255,255,255,0.7));-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text}}
.grid-2{{display:grid;grid-template-columns:1fr 1fr;gap:24px}}
.grid-3{{display:grid;grid-template-columns:1fr 1fr 1fr;gap:16px}}
.pill{{display:inline-block;padding:4px 12px;border-radius:999px;font-size:12px;font-weight:500}}
.pill-purple{{background:rgba(168,85,247,0.2);color:#C084FC}}
.pill-green{{background:rgba(16,185,129,0.2);color:#6EE7B7}}
.pill-red{{background:rgba(239,68,68,0.2);color:#FCA5A5}}
.pill-amber{{background:rgba(245,158,11,0.2);color:#FCD34D}}
.stat-box{{background:rgba(255,255,255,0.02);border:1px solid rgba(255,255,255,0.05);border-radius:12px;padding:16px;text-align:center}}
.stat-box .val{{font-size:24px;font-weight:700;color:#F3F4F6}}
.stat-box .lbl{{font-size:11px;color:#6B7280;margin-top:4px}}
.footer{{text-align:center;padding:32px 0 16px;color:#4B5563;font-size:12px}}
.footer a{{color:#A855F7;text-decoration:none}}
@media (max-width:700px){{.grid-2{{grid-template-columns:1fr}}.grid-3{{grid-template-columns:1fr 1fr}}.container{{padding:20px 16px}}}}
</style>
</head>
<body>
<div class="container">
  <div class="header">
    <div class="header-left">
      <h1>Weekly Report</h1>
      <p>{week_start} — {week_end}</p>
      <p style="font-size:11px;color:#6B7280;margin-top:2px">Generated {generated_at}</p>
    </div>
    <div class="gauge">
      <svg width="120" height="120" viewBox="0 0 100 100">
        <circle cx="50" cy="50" r="45" fill="none" stroke="rgba(255,255,255,0.08)" stroke-width="8"/>
        <circle cx="50" cy="50" r="45" fill="none" stroke="{score_color}" stroke-width="8" stroke-linecap="round"
          stroke-dasharray="{circumference}" stroke-dashoffset="{offset}"/>
      </svg>
      <div class="score"><span class="num">{security_score}</span><span class="label">Security</span></div>
    </div>
  </div>

  <!-- Activity -->
  <div class="section">
    <div class="section-title">&#x26A1; Activity</div>
    <div class="grid-2">
      <div>
        <div class="big-num">{total_actions}</div>
        <div style="color:#9CA3AF;font-size:14px;margin-top:4px">total actions this week</div>
        <div style="margin-top:8px">{prev_actions_html}</div>
        <div style="margin-top:12px;display:flex;gap:8px">
          <span class="pill pill-purple">Busiest: {busiest_day}</span>
          <span class="pill pill-purple">Peak hour: {busiest_hour}:00</span>
        </div>
      </div>
      <div>
        <div style="display:flex;gap:4px;align-items:flex-end">{actions_bars}</div>
      </div>
    </div>
    {agent_section}
  </div>

  <!-- Cost -->
  <div class="section">
    <div class="section-title">&#x1F4B0; Cost</div>
    <div class="grid-2">
      <div>
        <div class="big-num">${total_cost:.4}</div>
        <div style="color:#9CA3AF;font-size:14px;margin-top:4px">total cost {trend_arrow} {cost_trend}</div>
      </div>
      <div>
        <div style="display:flex;gap:4px;align-items:flex-end">{cost_bars}</div>
      </div>
    </div>
  </div>

  <!-- Safety & PII -->
  <div class="grid-2">
    <div class="section">
      <div class="section-title">&#x1F6E1; Safety Net</div>
      <div class="grid-3">
        <div class="stat-box"><div class="val">{files_protected}</div><div class="lbl">Files Protected</div></div>
        <div class="stat-box"><div class="val">{files_restored}</div><div class="lbl">Files Restored</div></div>
        <div class="stat-box"><div class="val">{safety_net_size_mb:.1} MB</div><div class="lbl">Storage Used</div></div>
      </div>
    </div>
    <div class="section">
      <div class="section-title">&#x1F50D; PII Guard</div>
      <div class="grid-3">
        <div class="stat-box"><div class="val">{pii_findings}</div><div class="lbl">Findings</div></div>
        <div class="stat-box"><div class="val">{pii_critical}</div><div class="lbl">Critical</div></div>
        <div class="stat-box"><div class="val">{security_score}</div><div class="lbl">Score</div></div>
      </div>
    </div>
  </div>

  <!-- Data Shield -->
  <div class="section">
    <div class="section-title">&#x1F310; Data Shield</div>
    <div class="grid-3">
      <div class="stat-box"><div class="val">{domains_contacted}</div><div class="lbl">Domains Contacted</div></div>
      <div class="stat-box"><div class="val">{unknown_domains}</div><div class="lbl">Unknown Domains</div></div>
      <div class="stat-box"><div class="val">{outbound_events}</div><div class="lbl">Outbound Events</div></div>
    </div>
  </div>

  <div class="footer">Generated by <a href="https://unalome.ai">Unalome Agent Firewall</a> &middot; unalome.ai</div>
</div>
</body>
</html>"##,
        week_start = report.week_start,
        week_end = report.week_end,
        generated_at = &report.generated_at[..10.min(report.generated_at.len())],
        score_color = score_color,
        circumference = circumference,
        offset = offset,
        security_score = report.security_score,
        total_actions = report.total_actions,
        prev_actions_html = prev_actions_html,
        busiest_day = report.busiest_day,
        busiest_hour = report.busiest_hour,
        actions_bars = actions_bars,
        total_cost = report.total_cost,
        trend_arrow = trend_arrow,
        cost_trend = report.cost_trend,
        cost_bars = cost_bars,
        files_protected = report.files_protected,
        files_restored = report.files_restored,
        safety_net_size_mb = report.safety_net_size_mb,
        pii_findings = report.pii_findings,
        pii_critical = report.pii_critical,
        domains_contacted = report.domains_contacted,
        unknown_domains = report.unknown_domains,
        outbound_events = report.outbound_events,
        agent_section = if report.actions_by_agent.is_empty() {
            String::new()
        } else {
            format!(r#"<div style="margin-top:20px;border-top:1px solid rgba(255,255,255,0.06);padding-top:16px"><div style="font-size:13px;color:#9CA3AF;margin-bottom:8px">Agent Breakdown</div>{}</div>"#, agent_rows)
        },
    )
}
