use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Models ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub agent_pattern: String,
    pub allow_tools: Vec<String>,
    pub deny_tools: Vec<String>,
    pub conditions: Vec<RuleCondition>,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub tool_pattern: String,
    pub condition_type: ConditionType,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    Block,
    MaxAmount,
    ArgContains,
    PathRestriction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallDecision {
    pub id: String,
    pub action_id: String,
    pub timestamp: String,
    pub agent_id: String,
    pub agent_name: String,
    pub tool_name: String,
    pub mcp_server: Option<String>,
    pub arguments: serde_json::Value,
    pub decision: DecisionType,
    pub reason: String,
    pub rule_id: Option<String>,
    pub rule_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DecisionType {
    Allowed,
    Blocked,
    Flagged,
}

impl std::fmt::Display for DecisionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionType::Allowed => write!(f, "Allowed"),
            DecisionType::Blocked => write!(f, "Blocked"),
            DecisionType::Flagged => write!(f, "Flagged"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallStats {
    pub total_rules: i64,
    pub active_rules: i64,
    pub total_decisions: i64,
    pub decisions_today: i64,
    pub blocked_today: i64,
    pub flagged_today: i64,
    pub allowed_today: i64,
    pub top_blocked_tools: Vec<(String, i64)>,
    pub by_agent: HashMap<String, i64>,
}

// ── Compiled rule for fast matching ─────────────────────────────────

struct CompiledRule {
    rule: FirewallRule,
    agent_regex: Option<Regex>,
    deny_regexes: Vec<Regex>,
    allow_regexes: Vec<Regex>,
    condition_regexes: Vec<(Regex, ConditionType, String)>,
}

// ── Engine ──────────────────────────────────────────────────────────

pub struct FirewallEngine {
    compiled_rules: Vec<CompiledRule>,
}

impl FirewallEngine {
    pub fn new() -> Self {
        Self {
            compiled_rules: Vec::new(),
        }
    }

    /// Load rules (sorted by priority descending — higher priority first)
    pub fn load_rules(&mut self, rules: Vec<FirewallRule>) {
        let mut compiled = Vec::new();
        let mut sorted_rules = rules;
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in sorted_rules {
            if !rule.enabled {
                continue;
            }

            let agent_regex = if rule.agent_pattern.is_empty() || rule.agent_pattern == "*" {
                None // matches all
            } else {
                Regex::new(&rule.agent_pattern).ok()
            };

            let deny_regexes = rule
                .deny_tools
                .iter()
                .filter_map(|p| Regex::new(p).ok())
                .collect();

            let allow_regexes = rule
                .allow_tools
                .iter()
                .filter_map(|p| Regex::new(p).ok())
                .collect();

            let condition_regexes = rule
                .conditions
                .iter()
                .filter_map(|c| {
                    Regex::new(&c.tool_pattern)
                        .ok()
                        .map(|r| (r, c.condition_type.clone(), c.value.clone()))
                })
                .collect();

            compiled.push(CompiledRule {
                rule,
                agent_regex,
                deny_regexes,
                allow_regexes,
                condition_regexes,
            });
        }

        self.compiled_rules = compiled;
    }

    /// Evaluate an action against all rules, returning a decision
    pub fn evaluate(
        &self,
        action_id: &str,
        agent_id: &str,
        agent_name: &str,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> FirewallDecision {
        let now = Utc::now().to_rfc3339();

        for compiled in &self.compiled_rules {
            // Check agent pattern match
            if let Some(ref regex) = compiled.agent_regex {
                if !regex.is_match(agent_name) && !regex.is_match(agent_id) {
                    continue;
                }
            }

            // Check deny list
            for deny_re in &compiled.deny_regexes {
                if deny_re.is_match(tool_name) {
                    return FirewallDecision {
                        id: uuid::Uuid::new_v4().to_string(),
                        action_id: action_id.to_string(),
                        timestamp: now,
                        agent_id: agent_id.to_string(),
                        agent_name: agent_name.to_string(),
                        tool_name: tool_name.to_string(),
                        mcp_server: None,
                        arguments: args.clone(),
                        decision: DecisionType::Blocked,
                        reason: format!(
                            "Tool '{}' denied by rule '{}'",
                            tool_name, compiled.rule.name
                        ),
                        rule_id: Some(compiled.rule.id.clone()),
                        rule_name: Some(compiled.rule.name.clone()),
                    };
                }
            }

            // Check conditions
            for (tool_re, cond_type, value) in &compiled.condition_regexes {
                if !tool_re.is_match(tool_name) {
                    continue;
                }

                match cond_type {
                    ConditionType::Block => {
                        return FirewallDecision {
                            id: uuid::Uuid::new_v4().to_string(),
                            action_id: action_id.to_string(),
                            timestamp: now,
                            agent_id: agent_id.to_string(),
                            agent_name: agent_name.to_string(),
                            tool_name: tool_name.to_string(),
                            mcp_server: None,
                            arguments: args.clone(),
                            decision: DecisionType::Blocked,
                            reason: format!(
                                "Tool '{}' blocked by condition in rule '{}'",
                                tool_name, compiled.rule.name
                            ),
                            rule_id: Some(compiled.rule.id.clone()),
                            rule_name: Some(compiled.rule.name.clone()),
                        };
                    }
                    ConditionType::ArgContains => {
                        let args_str = serde_json::to_string(args).unwrap_or_default();
                        if args_str.contains(value.as_str()) {
                            return FirewallDecision {
                                id: uuid::Uuid::new_v4().to_string(),
                                action_id: action_id.to_string(),
                                timestamp: now,
                                agent_id: agent_id.to_string(),
                                agent_name: agent_name.to_string(),
                                tool_name: tool_name.to_string(),
                                mcp_server: None,
                                arguments: args.clone(),
                                decision: DecisionType::Flagged,
                                reason: format!(
                                    "Arguments contain '{}' (rule '{}')",
                                    value, compiled.rule.name
                                ),
                                rule_id: Some(compiled.rule.id.clone()),
                                rule_name: Some(compiled.rule.name.clone()),
                            };
                        }
                    }
                    ConditionType::PathRestriction => {
                        if let Some(path) = args
                            .get("file_path")
                            .or_else(|| args.get("path"))
                            .and_then(|v| v.as_str())
                        {
                            if path.starts_with(value.as_str()) || path.contains(value.as_str()) {
                                return FirewallDecision {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    action_id: action_id.to_string(),
                                    timestamp: now,
                                    agent_id: agent_id.to_string(),
                                    agent_name: agent_name.to_string(),
                                    tool_name: tool_name.to_string(),
                                    mcp_server: None,
                                    arguments: args.clone(),
                                    decision: DecisionType::Blocked,
                                    reason: format!(
                                        "Path '{}' restricted by rule '{}'",
                                        path, compiled.rule.name
                                    ),
                                    rule_id: Some(compiled.rule.id.clone()),
                                    rule_name: Some(compiled.rule.name.clone()),
                                };
                            }
                        }
                    }
                    ConditionType::MaxAmount => {
                        // MaxAmount: flag but don't block (actual enforcement needs counter state)
                        return FirewallDecision {
                            id: uuid::Uuid::new_v4().to_string(),
                            action_id: action_id.to_string(),
                            timestamp: now,
                            agent_id: agent_id.to_string(),
                            agent_name: agent_name.to_string(),
                            tool_name: tool_name.to_string(),
                            mcp_server: None,
                            arguments: args.clone(),
                            decision: DecisionType::Flagged,
                            reason: format!(
                                "Tool '{}' flagged for rate limit check (rule '{}')",
                                tool_name, compiled.rule.name
                            ),
                            rule_id: Some(compiled.rule.id.clone()),
                            rule_name: Some(compiled.rule.name.clone()),
                        };
                    }
                }
            }

            // Check allow list — if rule has explicit allows and tool not in them, flag it
            if !compiled.allow_regexes.is_empty() {
                let is_allowed = compiled.allow_regexes.iter().any(|r| r.is_match(tool_name));
                if !is_allowed {
                    return FirewallDecision {
                        id: uuid::Uuid::new_v4().to_string(),
                        action_id: action_id.to_string(),
                        timestamp: now,
                        agent_id: agent_id.to_string(),
                        agent_name: agent_name.to_string(),
                        tool_name: tool_name.to_string(),
                        mcp_server: None,
                        arguments: args.clone(),
                        decision: DecisionType::Flagged,
                        reason: format!(
                            "Tool '{}' not in allow list for rule '{}'",
                            tool_name, compiled.rule.name
                        ),
                        rule_id: Some(compiled.rule.id.clone()),
                        rule_name: Some(compiled.rule.name.clone()),
                    };
                }
            }
        }

        // No rule matched — allowed by default
        FirewallDecision {
            id: uuid::Uuid::new_v4().to_string(),
            action_id: action_id.to_string(),
            timestamp: now,
            agent_id: agent_id.to_string(),
            agent_name: agent_name.to_string(),
            tool_name: tool_name.to_string(),
            mcp_server: None,
            arguments: args.clone(),
            decision: DecisionType::Allowed,
            reason: "No matching deny rule".to_string(),
            rule_id: None,
            rule_name: None,
        }
    }

    /// Add a rule to the engine (recompiles)
    #[allow(dead_code)]
    pub fn add_rule(&mut self, rule: FirewallRule, all_rules: &mut Vec<FirewallRule>) {
        all_rules.push(rule);
        self.load_rules(all_rules.clone());
    }

    /// Remove a rule by ID (recompiles)
    #[allow(dead_code)]
    pub fn remove_rule(&mut self, rule_id: &str, all_rules: &mut Vec<FirewallRule>) {
        all_rules.retain(|r| r.id != rule_id);
        self.load_rules(all_rules.clone());
    }

    /// Update a rule (recompiles)
    #[allow(dead_code)]
    pub fn update_rule(&mut self, updated: FirewallRule, all_rules: &mut Vec<FirewallRule>) {
        if let Some(existing) = all_rules.iter_mut().find(|r| r.id == updated.id) {
            *existing = updated;
        }
        self.load_rules(all_rules.clone());
    }
}
