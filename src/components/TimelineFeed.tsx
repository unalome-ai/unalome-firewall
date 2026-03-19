import {
  Activity,
  Clock,
  FileText,
  Globe,
  MessageSquare,
  Zap,
  Search,
  ShieldCheck,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import { useState, useMemo } from "react";
import type { Action, Agent, RiskLevel } from "@/types";
import { cn } from "@/lib/utils";

interface TimelineFeedProps {
  actions: Action[];
  agents: Agent[];
  actionsCount?: number;
  initialAgentFilter?: string;
}

const TIME_PERIODS = [
  { label: "1h", ms: 60 * 60 * 1000 },
  { label: "24h", ms: 24 * 60 * 60 * 1000 },
  { label: "7d", ms: 7 * 24 * 60 * 60 * 1000 },
  { label: "30d", ms: 30 * 24 * 60 * 60 * 1000 },
  { label: "All", ms: 0 },
];

const RISK_CONFIG: Record<string, { color: string; bg: string; border: string }> = {
  Safe: { color: "text-emerald-400", bg: "bg-emerald-500/15", border: "border-emerald-500/30" },
  Low: { color: "text-blue-400", bg: "bg-blue-500/15", border: "border-blue-500/30" },
  Medium: { color: "text-amber-400", bg: "bg-amber-500/15", border: "border-amber-500/30" },
  High: { color: "text-orange-400", bg: "bg-orange-500/15", border: "border-orange-500/30" },
  Critical: { color: "text-rose-400", bg: "bg-rose-500/15", border: "border-rose-500/30" },
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

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

export function TimelineFeed({ actions, agents, actionsCount, initialAgentFilter }: TimelineFeedProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [riskFilter, setRiskFilter] = useState<RiskLevel | "all">("all");
  const [agentFilter, setAgentFilter] = useState<string>(initialAgentFilter || "all");
  const [timePeriod, setTimePeriod] = useState("All");

  const filteredActions = useMemo(() => {
    const period = TIME_PERIODS.find((p) => p.label === timePeriod);
    const cutoff = period && period.ms > 0 ? Date.now() - period.ms : 0;

    return actions
      .filter((action) => {
        const ts = new Date(action.timestamp).getTime();
        if (cutoff > 0 && ts < cutoff) return false;

        const matchesSearch =
          searchQuery === "" ||
          action.description.toLowerCase().includes(searchQuery.toLowerCase());

        const matchesRisk =
          riskFilter === "all" || action.risk_level === riskFilter;

        const matchesAgent =
          agentFilter === "all" || action.agent_id === agentFilter;

        return matchesSearch && matchesRisk && matchesAgent;
      })
      .sort(
        (a, b) =>
          new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
      );
  }, [actions, searchQuery, riskFilter, agentFilter, timePeriod]);

  const hasFilters = agentFilter !== "all" || riskFilter !== "all" || searchQuery !== "";

  return (
    <div className="space-y-6">
      {/* Hero Status Banner */}
      <div className="glass-card p-5 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 rounded-xl bg-fuchsia-500/15 flex items-center justify-center">
            <Clock className="w-6 h-6 text-fuchsia-400" />
          </div>
          <div>
            <h2 className="text-lg font-bold flex items-center gap-2">
              Action Timeline
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
            </h2>
            <p className="text-sm text-white/50">
              {actionsCount != null && actionsCount > actions.length
                ? `${actionsCount.toLocaleString()} total · listing ${filteredActions.length.toLocaleString()}`
                : `${filteredActions.length.toLocaleString()} action${filteredActions.length !== 1 ? "s" : ""} recorded`}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-3">
          {(() => {
            const risks = filteredActions.reduce<Record<string, number>>((acc, a) => {
              acc[a.risk_level] = (acc[a.risk_level] || 0) + 1;
              return acc;
            }, {});
            return (
              <>
                {(risks["High"] || 0) + (risks["Critical"] || 0) > 0 && (
                  <span className="px-2.5 py-1 rounded-full bg-orange-500/15 text-orange-400 text-xs font-semibold">
                    {(risks["High"] || 0) + (risks["Critical"] || 0)} high risk
                  </span>
                )}
                {(risks["Medium"] || 0) > 0 && (
                  <span className="px-2.5 py-1 rounded-full bg-amber-500/15 text-amber-400 text-xs font-semibold">
                    {risks["Medium"]} medium
                  </span>
                )}
              </>
            );
          })()}
        </div>
      </div>

      {/* Time Period Filter */}
      <div className="flex items-center gap-1 p-1 rounded-xl glass w-fit">
        {TIME_PERIODS.map((p) => (
          <button
            key={p.label}
            onClick={() => setTimePeriod(p.label)}
            className={cn(
              "px-4 py-1.5 rounded-lg text-xs font-medium transition-all",
              timePeriod === p.label
                ? "bg-white/15 text-white shadow-sm"
                : "text-white/40 hover:text-white/60"
            )}
          >
            {p.label}
          </button>
        ))}
      </div>

      {/* Toolbar */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-white/30" />
          <input
            type="text"
            placeholder="Search actions..."
            className="w-full pl-9 pr-4 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-white placeholder-white/30 outline-none focus:ring-1 focus:ring-fuchsia-500/50"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>

        <div className="flex items-center rounded-lg bg-white/5 p-0.5">
          {(["all", "Safe", "Low", "Medium", "High", "Critical"] as const).map((r) => (
            <button
              key={r}
              onClick={() => setRiskFilter(r)}
              className={cn(
                "px-3 py-1.5 rounded-md text-xs font-medium transition-all",
                riskFilter === r
                  ? "bg-white/10 text-white shadow-sm"
                  : "text-white/40 hover:text-white/60"
              )}
            >
              {r === "all" ? "All" : r}
            </button>
          ))}
        </div>

        {agents.length > 0 && (
          <select
            value={agentFilter}
            onChange={(e) => setAgentFilter(e.target.value)}
            className="px-3 py-2 rounded-lg glass text-xs text-white/70 outline-none bg-transparent"
          >
            <option value="all">All Agents</option>
            {agents.map((agent) => (
              <option key={agent.id} value={agent.id}>{agent.name}</option>
            ))}
          </select>
        )}

        {hasFilters && (
          <button
            onClick={() => { setAgentFilter("all"); setRiskFilter("all"); setSearchQuery(""); }}
            className="text-xs text-fuchsia-400 hover:text-fuchsia-300 transition-colors shrink-0"
          >
            Clear
          </button>
        )}
      </div>

      {/* Empty State */}
      {filteredActions.length === 0 && actions.length > 0 && (
        <div className="glass-card p-12 text-center">
          <div className="w-16 h-16 rounded-2xl bg-amber-500/15 flex items-center justify-center mx-auto mb-4">
            <Search className="w-8 h-8 text-amber-400" />
          </div>
          <h3 className="text-xl font-bold mb-2">No matching actions</h3>
          <p className="text-white/50 max-w-md mx-auto">
            No actions found for the selected filters. Try a wider time range or clear filters.
          </p>
        </div>
      )}
      {actions.length === 0 && (
        <div className="glass-card p-12 text-center">
          <div className="w-16 h-16 rounded-2xl bg-fuchsia-500/15 flex items-center justify-center mx-auto mb-4">
            <Clock className="w-8 h-8 text-fuchsia-400" />
          </div>
          <h3 className="text-xl font-bold mb-2">No activity yet</h3>
          <p className="text-white/50 max-w-md mx-auto">
            Actions will appear here as your AI agents work. Start using an agent to see its activity.
          </p>
        </div>
      )}

      {/* Action Cards */}
      <div className="space-y-2">
        {filteredActions.map((action) => (
          <ActionCard
            key={action.id}
            action={action}
            agent={agents.find((a) => a.id === action.agent_id)}
          />
        ))}
      </div>
    </div>
  );
}

function ActionCard({ action, agent }: { action: Action; agent?: Agent }) {
  const [expanded, setExpanded] = useState(false);
  const Icon = getActionIcon(action.action_type);
  const risk = RISK_CONFIG[action.risk_level] ?? RISK_CONFIG.Safe;

  return (
    <div className={cn("glass-card border transition-all duration-200", risk.border)}>
      {/* Compact row */}
      <div
        className="flex items-center gap-3 p-3 cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        <div className={cn("w-7 h-7 rounded-lg flex items-center justify-center shrink-0", risk.bg)}>
          <Icon className={cn("w-3.5 h-3.5", risk.color)} />
        </div>

        <span className={cn("text-xs font-semibold px-2 py-0.5 rounded-full shrink-0", risk.bg, risk.color)}>
          {action.risk_level}
        </span>

        <span className="text-sm text-white/70 truncate flex-1">{action.description}</span>

        {"ToolCall" in action.action_type &&
          (action.action_type.ToolCall.tool_name === "Write" || action.action_type.ToolCall.tool_name === "Edit") && (
          <span title="Protected by Safety Net"><ShieldCheck className="w-3.5 h-3.5 text-emerald-400 shrink-0" /></span>
        )}

        {agent && (
          <span className="px-2 py-0.5 rounded-full glass text-[10px] text-white/40 shrink-0">
            {agent.name}
          </span>
        )}

        <span className="text-xs text-white/30 shrink-0">{timeAgo(action.timestamp)}</span>

        {action.cost && (
          <span className="text-xs text-fuchsia-400 shrink-0">
            ${action.cost.estimated_cost_usd.toFixed(4)}
          </span>
        )}

        {expanded ? (
          <ChevronDown className="w-4 h-4 text-white/30 shrink-0" />
        ) : (
          <ChevronRight className="w-4 h-4 text-white/30 shrink-0" />
        )}
      </div>

      {/* Expanded details */}
      {expanded && (
        <div className="px-3 pb-3 pt-0 ml-10 space-y-2">
          {"ToolCall" in action.action_type && (
            <div className="text-xs space-y-1">
              <p className="text-white/60">
                <span className="text-white/40">Tool:</span>{" "}
                {action.action_type.ToolCall.tool_name}
              </p>
              {Object.keys(action.action_type.ToolCall.args).length > 0 && (
                <pre className="text-white/50 bg-black/30 rounded-lg p-2 overflow-x-auto max-h-40 text-[11px] font-mono">
                  {JSON.stringify(action.action_type.ToolCall.args, null, 2)}
                </pre>
              )}
            </div>
          )}
          {"FileAccess" in action.action_type && (
            <p className="text-xs text-white/60">
              <span className="text-white/40">Path:</span>{" "}
              {action.action_type.FileAccess.path} ({action.action_type.FileAccess.operation})
            </p>
          )}
          {"Other" in action.action_type && (
            <p className="text-xs text-white/60">
              <span className="text-white/40">Type:</span> {action.action_type.Other}
            </p>
          )}

          {/* Cost breakdown with cache tokens */}
          {action.cost != null && (
            <div className="bg-black/30 rounded-lg px-3 py-2">
              <div className="flex flex-wrap gap-x-5 gap-y-1 text-xs">
                <span className="text-white/50">
                  <span className="text-white/30">Input:</span> {formatTokens(action.cost.tokens_input)}
                </span>
                <span className="text-white/50">
                  <span className="text-white/30">Output:</span> {formatTokens(action.cost.tokens_output)}
                </span>
                {action.cost.cache_write_tokens > 0 && (
                  <span className="text-purple-400/70">
                    <span className="text-purple-400/40">Cache write:</span> {formatTokens(action.cost.cache_write_tokens)}
                  </span>
                )}
                {action.cost.cache_read_tokens > 0 && (
                  <span className="text-cyan-400/70">
                    <span className="text-cyan-400/40">Cache read:</span> {formatTokens(action.cost.cache_read_tokens)}
                  </span>
                )}
                <span className="text-fuchsia-400">
                  <span className="text-fuchsia-400/50">Cost:</span> ${action.cost.estimated_cost_usd.toFixed(4)}
                </span>
              </div>
            </div>
          )}

          {/* Model info */}
          {typeof action.metadata?.model === "string" && action.metadata.model !== "unknown" && (
            <p className="text-xs text-white/40">Model: {action.metadata.model}</p>
          )}
        </div>
      )}
    </div>
  );
}

function getActionIcon(actionType: Action["action_type"]) {
  if ("ToolCall" in actionType) return Zap;
  if ("FileAccess" in actionType) return FileText;
  if ("NetworkRequest" in actionType) return Globe;
  if ("Message" in actionType) return MessageSquare;
  return Activity;
}
