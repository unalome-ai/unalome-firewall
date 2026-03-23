import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Shield,
  ShieldAlert,
  ShieldCheck,
  Plus,
  Trash2,
  Pencil,
  X,
  ChevronDown,
  ChevronRight,
  FlaskConical,
} from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import type {
  Agent,
  FirewallRule,
  FirewallDecision,
  FirewallStats,
  RuleCondition,
} from "@/types";

interface FirewallRulesProps {
  agents: Agent[];
}

const DECISION_CONFIG: Record<string, { color: string; bg: string; border: string; label: string }> = {
  Blocked: { color: "text-rose-400", bg: "bg-rose-500/15", border: "border-rose-500/30", label: "BLOCKED" },
  Flagged: { color: "text-amber-400", bg: "bg-amber-500/15", border: "border-amber-500/30", label: "FLAGGED" },
  Allowed: { color: "text-emerald-400", bg: "bg-emerald-500/15", border: "border-emerald-500/30", label: "ALLOWED" },
};

function timeAgo(timestamp: string): string {
  const now = Date.now();
  const then = new Date(timestamp).getTime();
  const diff = Math.floor((now - then) / 1000);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

export function FirewallRules({ agents: _agents }: FirewallRulesProps) {
  const [rules, setRules] = useState<FirewallRule[]>([]);
  const [decisions, setDecisions] = useState<FirewallDecision[]>([]);
  const [stats, setStats] = useState<FirewallStats | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [editingRule, setEditingRule] = useState<FirewallRule | null>(null);
  const [expandedRule, setExpandedRule] = useState<string | null>(null);
  const [decisionFilter, setDecisionFilter] = useState<string>("all");

  const loadData = useCallback(async () => {
    try {
      const [r, s, d] = await Promise.all([
        invoke<FirewallRule[]>("get_firewall_rules"),
        invoke<FirewallStats>("get_firewall_stats"),
        invoke<{ decisions: FirewallDecision[]; total: number }>("get_firewall_decisions", {
          limit: 50,
          offset: 0,
          decisionFilter: decisionFilter === "all" ? null : decisionFilter,
        }),
      ]);
      setRules(r);
      setStats(s);
      setDecisions(d.decisions);
    } catch (e) {
      console.error("Failed to load firewall data:", e);
    }
  }, [decisionFilter]);

  useEffect(() => {
    loadData();
    const unlisten = listen("firewall-decision", () => loadData());
    return () => { unlisten.then((fn) => fn()); };
  }, [loadData]);

  async function handleToggle(id: string, enabled: boolean) {
    try {
      await invoke("toggle_firewall_rule", { id, enabled });
      loadData();
    } catch (e) {
      console.error("Failed to toggle rule:", e);
    }
  }

  async function handleDelete(id: string) {
    try {
      await invoke("delete_firewall_rule", { id });
      loadData();
    } catch (e) {
      console.error("Failed to delete rule:", e);
    }
  }

  function handleEdit(rule: FirewallRule) {
    setEditingRule(rule);
    setShowForm(true);
  }

  function handleCreate() {
    setEditingRule(null);
    setShowForm(true);
  }

  return (
    <div className="space-y-6">
      {/* Hero Banner */}
      <div className="glass-card p-5 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 rounded-xl bg-rose-500/15 flex items-center justify-center">
            <ShieldAlert className="w-6 h-6 text-rose-400" />
          </div>
          <div>
            <h2 className="text-lg font-bold">Firewall Rules</h2>
            <p className="text-sm text-white/50">
              Define rules to monitor and control agent tool usage
            </p>
          </div>
        </div>
        <button
          onClick={handleCreate}
          className="flex items-center gap-2 px-4 py-2 rounded-xl bg-fuchsia-500/20 text-fuchsia-400 hover:bg-fuchsia-500/30 border border-fuchsia-500/30 text-sm font-semibold transition-colors"
        >
          <Plus className="w-4 h-4" />
          New Rule
        </button>
      </div>

      {/* Stats Cards */}
      {stats && (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="glass-card p-4">
            <div className="flex items-center gap-2 mb-2">
              <Shield className="w-4 h-4 text-fuchsia-400" />
              <span className="text-xs text-white/50">Active Rules</span>
            </div>
            <p className="text-2xl font-bold">{stats.active_rules}</p>
            <p className="text-xs text-white/40">{stats.total_rules} total</p>
          </div>
          <div className="glass-card p-4">
            <div className="flex items-center gap-2 mb-2">
              <ShieldAlert className="w-4 h-4 text-rose-400" />
              <span className="text-xs text-white/50">Blocked Today</span>
            </div>
            <p className="text-2xl font-bold text-rose-400">{stats.blocked_today}</p>
          </div>
          <div className="glass-card p-4">
            <div className="flex items-center gap-2 mb-2">
              <ShieldAlert className="w-4 h-4 text-amber-400" />
              <span className="text-xs text-white/50">Flagged Today</span>
            </div>
            <p className="text-2xl font-bold text-amber-400">{stats.flagged_today}</p>
          </div>
          <div className="glass-card p-4">
            <div className="flex items-center gap-2 mb-2">
              <ShieldCheck className="w-4 h-4 text-emerald-400" />
              <span className="text-xs text-white/50">Decisions Today</span>
            </div>
            <p className="text-2xl font-bold">{stats.decisions_today}</p>
          </div>
        </div>
      )}

      {/* Rules List */}
      <div>
        <div className="flex items-center gap-4 mb-4">
          <div className="flex-1 h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />
          <span className="text-xs text-white/20 uppercase tracking-widest">Rules</span>
          <div className="flex-1 h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />
        </div>

        {rules.length === 0 ? (
          <div className="glass-card p-12 text-center">
            <div className="w-16 h-16 rounded-2xl bg-fuchsia-500/15 flex items-center justify-center mx-auto mb-4">
              <Shield className="w-8 h-8 text-fuchsia-400" />
            </div>
            <h3 className="text-xl font-bold mb-2">No firewall rules yet</h3>
            <p className="text-white/50 max-w-md mx-auto mb-4">
              Create your first rule to start monitoring agent tool usage.
            </p>
            <button
              onClick={handleCreate}
              className="px-6 py-2 rounded-xl bg-fuchsia-500/20 text-fuchsia-400 hover:bg-fuchsia-500/30 border border-fuchsia-500/30 text-sm font-semibold transition-colors"
            >
              <Plus className="w-4 h-4 inline mr-2" />
              Create Rule
            </button>
          </div>
        ) : (
          <div className="space-y-2">
            {rules.map((rule) => (
              <RuleCard
                key={rule.id}
                rule={rule}
                expanded={expandedRule === rule.id}
                onToggleExpand={() => setExpandedRule(expandedRule === rule.id ? null : rule.id)}
                onToggle={(enabled) => handleToggle(rule.id, enabled)}
                onEdit={() => handleEdit(rule)}
                onDelete={() => handleDelete(rule.id)}
              />
            ))}
          </div>
        )}
      </div>

      {/* Recent Decisions */}
      <div>
        <div className="flex items-center gap-4 mb-4">
          <div className="flex-1 h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />
          <span className="text-xs text-white/20 uppercase tracking-widest">Recent Decisions</span>
          <div className="flex-1 h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />
        </div>

        <div className="flex items-center gap-1 p-1 rounded-xl glass w-fit mb-4">
          {["all", "Blocked", "Flagged"].map((f) => (
            <button
              key={f}
              onClick={() => setDecisionFilter(f)}
              className={cn(
                "px-4 py-1.5 rounded-lg text-xs font-medium transition-all",
                decisionFilter === f
                  ? "bg-white/15 text-white shadow-sm"
                  : "text-white/40 hover:text-white/60"
              )}
            >
              {f === "all" ? "All" : f}
            </button>
          ))}
        </div>

        {decisions.length === 0 ? (
          <div className="glass-card p-8 text-center">
            <p className="text-white/40 text-sm">No decisions recorded yet</p>
          </div>
        ) : (
          <div className="space-y-1">
            {decisions.map((d) => {
              const cfg = DECISION_CONFIG[d.decision] ?? DECISION_CONFIG.Allowed;
              return (
                <div key={d.id} className={cn("glass-card p-3 flex items-center gap-3 border", cfg.border)}>
                  <span className={cn("text-xs font-semibold px-2 py-0.5 rounded-full shrink-0", cfg.bg, cfg.color)}>
                    {cfg.label}
                  </span>
                  <span className="text-sm text-white/70 truncate flex-1">
                    <span className="text-white/40">{d.agent_name || d.agent_id}</span>
                    {" \u2192 "}
                    <span className="font-mono text-xs">{d.tool_name}</span>
                  </span>
                  {d.rule_name && (
                    <span className="text-[10px] text-white/30 shrink-0">
                      rule: {d.rule_name}
                    </span>
                  )}
                  <span className="text-xs text-white/30 shrink-0">{timeAgo(d.timestamp)}</span>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Rule Form Modal */}
      {showForm && (
        <RuleFormModal
          rule={editingRule}
          onClose={() => { setShowForm(false); setEditingRule(null); }}
          onSave={() => { setShowForm(false); setEditingRule(null); loadData(); }}
        />
      )}
    </div>
  );
}

// ── Rule Card ──────────────────────────────────────────────────────

function RuleCard({
  rule,
  expanded,
  onToggleExpand,
  onToggle,
  onEdit,
  onDelete,
}: {
  rule: FirewallRule;
  expanded: boolean;
  onToggleExpand: () => void;
  onToggle: (enabled: boolean) => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  return (
    <div className={cn("glass-card border transition-all", rule.enabled ? "border-white/10" : "border-white/5 opacity-60")}>
      <div className="flex items-center gap-3 p-3 cursor-pointer" onClick={onToggleExpand}>
        {expanded ? (
          <ChevronDown className="w-4 h-4 text-white/30 shrink-0" />
        ) : (
          <ChevronRight className="w-4 h-4 text-white/30 shrink-0" />
        )}

        <div className={cn("w-7 h-7 rounded-lg flex items-center justify-center shrink-0", rule.enabled ? "bg-rose-500/15" : "bg-white/5")}>
          <Shield className={cn("w-3.5 h-3.5", rule.enabled ? "text-rose-400" : "text-white/30")} />
        </div>

        <div className="flex-1 min-w-0">
          <p className="text-sm font-semibold truncate">{rule.name}</p>
          {rule.description && (
            <p className="text-xs text-white/40 truncate">{rule.description}</p>
          )}
        </div>

        {rule.deny_tools.length > 0 && (
          <div className="flex gap-1 shrink-0">
            {rule.deny_tools.slice(0, 3).map((t) => (
              <span key={t} className="px-1.5 py-0.5 rounded bg-rose-500/15 text-rose-400 text-[10px] font-mono">
                {t}
              </span>
            ))}
            {rule.deny_tools.length > 3 && (
              <span className="text-[10px] text-white/30">+{rule.deny_tools.length - 3}</span>
            )}
          </div>
        )}

        <span className="text-[10px] text-white/20 shrink-0">P{rule.priority}</span>

        <div onClick={(e) => e.stopPropagation()}>
          <Switch
            checked={rule.enabled}
            onCheckedChange={(checked) => onToggle(checked)}
          />
        </div>
      </div>

      {expanded && (
        <div className="px-3 pb-3 ml-10 space-y-2">
          <div className="text-xs space-y-1">
            <p className="text-white/60">
              <span className="text-white/40">Agent pattern:</span> {rule.agent_pattern}
            </p>
            {rule.allow_tools.length > 0 && (
              <p className="text-white/60">
                <span className="text-white/40">Allow:</span>{" "}
                {rule.allow_tools.map((t) => (
                  <span key={t} className="px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-400 text-[10px] font-mono mr-1">
                    {t}
                  </span>
                ))}
              </p>
            )}
            {rule.deny_tools.length > 0 && (
              <p className="text-white/60">
                <span className="text-white/40">Deny:</span>{" "}
                {rule.deny_tools.map((t) => (
                  <span key={t} className="px-1.5 py-0.5 rounded bg-rose-500/15 text-rose-400 text-[10px] font-mono mr-1">
                    {t}
                  </span>
                ))}
              </p>
            )}
            {rule.conditions.length > 0 && (
              <div>
                <span className="text-white/40">Conditions:</span>
                {rule.conditions.map((c, i) => (
                  <p key={i} className="text-white/50 ml-2">
                    {c.tool_pattern} &rarr; {c.condition_type}: {c.value}
                  </p>
                ))}
              </div>
            )}
          </div>

          <div className="flex gap-2 pt-1">
            <button onClick={onEdit} className="flex items-center gap-1 px-3 py-1 rounded-lg bg-white/5 hover:bg-white/10 text-white/60 text-xs transition-colors">
              <Pencil className="w-3 h-3" /> Edit
            </button>
            <button onClick={onDelete} className="flex items-center gap-1 px-3 py-1 rounded-lg bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 text-xs transition-colors">
              <Trash2 className="w-3 h-3" /> Delete
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Rule Form Modal ────────────────────────────────────────────────

function RuleFormModal({
  rule,
  onClose,
  onSave,
}: {
  rule: FirewallRule | null;
  onClose: () => void;
  onSave: () => void;
}) {
  const [name, setName] = useState(rule?.name ?? "");
  const [description, setDescription] = useState(rule?.description ?? "");
  const [agentPattern, setAgentPattern] = useState(rule?.agent_pattern ?? "*");
  const [denyTools, setDenyTools] = useState(rule?.deny_tools.join(", ") ?? "");
  const [allowTools, setAllowTools] = useState(rule?.allow_tools.join(", ") ?? "");
  const [priority, setPriority] = useState(rule?.priority ?? 0);
  const [conditions, setConditions] = useState<RuleCondition[]>(rule?.conditions ?? []);
  const [saving, setSaving] = useState(false);
  const [testResult, setTestResult] = useState<string | null>(null);

  function parseToolList(s: string): string[] {
    return s
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0);
  }

  async function handleSave() {
    if (!name.trim()) return;
    setSaving(true);
    try {
      const now = new Date().toISOString();
      const ruleData: FirewallRule = {
        id: rule?.id ?? crypto.randomUUID(),
        name: name.trim(),
        description: description.trim(),
        agent_pattern: agentPattern.trim() || "*",
        allow_tools: parseToolList(allowTools),
        deny_tools: parseToolList(denyTools),
        conditions,
        priority,
        enabled: rule?.enabled ?? true,
        created_at: rule?.created_at ?? now,
        updated_at: now,
      };

      if (rule) {
        await invoke("update_firewall_rule", { rule: ruleData });
      } else {
        await invoke("create_firewall_rule", { rule: ruleData });
      }
      onSave();
    } catch (e) {
      console.error("Failed to save rule:", e);
    } finally {
      setSaving(false);
    }
  }

  async function handleTest() {
    try {
      const result = await invoke<{ decision: string; reason: string }>("test_firewall_rule", {
        agentName: "Test Agent",
        toolName: parseToolList(denyTools)[0] || "Bash",
        args: {},
      });
      setTestResult(`${result.decision}: ${result.reason}`);
    } catch (e) {
      setTestResult(`Error: ${e}`);
    }
  }

  function addCondition() {
    setConditions([...conditions, { tool_pattern: ".*", condition_type: "Block", value: "" }]);
  }

  function removeCondition(index: number) {
    setConditions(conditions.filter((_, i) => i !== index));
  }

  function updateCondition(index: number, field: keyof RuleCondition, value: string) {
    const updated = [...conditions];
    updated[index] = { ...updated[index], [field]: value };
    setConditions(updated);
  }

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="glass-card w-full max-w-lg p-6 mx-4 space-y-4 max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-bold">{rule ? "Edit Rule" : "New Firewall Rule"}</h2>
          <button onClick={onClose} className="text-white/40 hover:text-white/70 transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Name */}
        <div>
          <label className="text-xs text-white/50 block mb-1">Rule Name *</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g., Block dangerous tools"
            className="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-white placeholder-white/30 outline-none focus:ring-1 focus:ring-fuchsia-500/50"
          />
        </div>

        {/* Description */}
        <div>
          <label className="text-xs text-white/50 block mb-1">Description</label>
          <input
            type="text"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Optional description"
            className="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-white placeholder-white/30 outline-none focus:ring-1 focus:ring-fuchsia-500/50"
          />
        </div>

        {/* Agent Pattern */}
        <div>
          <label className="text-xs text-white/50 block mb-1">Agent Pattern (regex, * = all)</label>
          <input
            type="text"
            value={agentPattern}
            onChange={(e) => setAgentPattern(e.target.value)}
            placeholder="*"
            className="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-white placeholder-white/30 outline-none focus:ring-1 focus:ring-fuchsia-500/50 font-mono"
          />
        </div>

        {/* Deny/Allow Tools */}
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="text-xs text-white/50 block mb-1">Deny Tools (comma-separated)</label>
            <input
              type="text"
              value={denyTools}
              onChange={(e) => setDenyTools(e.target.value)}
              placeholder="Bash, Write"
              className="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-white placeholder-white/30 outline-none focus:ring-1 focus:ring-rose-500/50 font-mono"
            />
          </div>
          <div>
            <label className="text-xs text-white/50 block mb-1">Allow Tools (comma-separated)</label>
            <input
              type="text"
              value={allowTools}
              onChange={(e) => setAllowTools(e.target.value)}
              placeholder="Read, Glob"
              className="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-white placeholder-white/30 outline-none focus:ring-1 focus:ring-emerald-500/50 font-mono"
            />
          </div>
        </div>

        {/* Priority */}
        <div>
          <label className="text-xs text-white/50 block mb-1">Priority (higher = checked first)</label>
          <input
            type="number"
            value={priority}
            onChange={(e) => setPriority(parseInt(e.target.value) || 0)}
            className="w-24 px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-white outline-none focus:ring-1 focus:ring-fuchsia-500/50"
          />
        </div>

        {/* Conditions */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <label className="text-xs text-white/50">Conditions</label>
            <button onClick={addCondition} className="text-xs text-fuchsia-400 hover:text-fuchsia-300">
              + Add Condition
            </button>
          </div>
          {conditions.map((cond, i) => (
            <div key={i} className="flex items-center gap-2 mb-2">
              <input
                type="text"
                value={cond.tool_pattern}
                onChange={(e) => updateCondition(i, "tool_pattern", e.target.value)}
                placeholder="Tool pattern"
                className="flex-1 px-2 py-1.5 rounded-lg bg-white/5 border border-white/10 text-xs text-white placeholder-white/30 outline-none font-mono"
              />
              <select
                value={cond.condition_type}
                onChange={(e) => updateCondition(i, "condition_type", e.target.value)}
                className="px-2 py-1.5 rounded-lg bg-white/5 border border-white/10 text-xs text-white/70 outline-none bg-transparent"
              >
                <option value="Block">Block</option>
                <option value="ArgContains">ArgContains</option>
                <option value="PathRestriction">PathRestriction</option>
                <option value="MaxAmount">MaxAmount</option>
              </select>
              <input
                type="text"
                value={cond.value}
                onChange={(e) => updateCondition(i, "value", e.target.value)}
                placeholder="Value"
                className="flex-1 px-2 py-1.5 rounded-lg bg-white/5 border border-white/10 text-xs text-white placeholder-white/30 outline-none font-mono"
              />
              <button onClick={() => removeCondition(i)} className="text-rose-400/60 hover:text-rose-400">
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
          ))}
        </div>

        {/* Test Result */}
        {testResult && (
          <div className="px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-xs text-white/60">
            {testResult}
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-between pt-2">
          <button
            onClick={handleTest}
            className="flex items-center gap-1 px-3 py-2 rounded-xl text-sm text-white/50 hover:text-white/70 transition-colors"
          >
            <FlaskConical className="w-4 h-4" />
            Test
          </button>
          <div className="flex gap-3">
            <button
              onClick={onClose}
              className="px-4 py-2 rounded-xl text-sm font-medium text-white/60 hover:text-white transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              disabled={!name.trim() || saving}
              className="px-4 py-2 rounded-xl text-sm font-semibold bg-fuchsia-500/20 text-fuchsia-400 hover:bg-fuchsia-500/30 border border-fuchsia-500/30 transition-colors disabled:opacity-50"
            >
              {saving ? "Saving..." : rule ? "Update Rule" : "Create Rule"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
