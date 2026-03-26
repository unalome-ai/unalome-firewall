import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  FileText,
  ArrowLeft,
  Search,
  Activity,
  Clock,
  HardDrive,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { AgentPlan, Action } from "@/types";

function formatRelativeDate(dateStr: string): string {
  const now = new Date();
  const date = new Date(dateStr);
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 30) return `${diffDays}d ago`;
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function riskColor(risk: string): string {
  switch (risk) {
    case "Critical": return "text-red-400";
    case "High": return "text-orange-400";
    case "Medium": return "text-yellow-400";
    case "Low": return "text-blue-400";
    case "Safe": return "text-emerald-400";
    default: return "text-white/50";
  }
}

function riskBg(risk: string): string {
  switch (risk) {
    case "Critical": return "bg-red-500/15 border-red-500/30";
    case "High": return "bg-orange-500/15 border-orange-500/30";
    case "Medium": return "bg-yellow-500/15 border-yellow-500/30";
    case "Low": return "bg-blue-500/15 border-blue-500/30";
    case "Safe": return "bg-emerald-500/15 border-emerald-500/30";
    default: return "bg-white/5 border-white/10";
  }
}

function getToolName(action: Action): string {
  if ("ToolCall" in action.action_type) return action.action_type.ToolCall.tool_name;
  if ("Other" in action.action_type) return action.action_type.Other;
  return "unknown";
}

export function AgentPlans() {
  const [plans, setPlans] = useState<AgentPlan[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedPlan, setSelectedPlan] = useState<AgentPlan | null>(null);
  const [planActions, setPlanActions] = useState<Action[]>([]);
  const [actionsExpanded, setActionsExpanded] = useState(false);
  const [actionsLoading, setActionsLoading] = useState(false);
  const [search, setSearch] = useState("");

  const loadPlans = useCallback(async () => {
    try {
      const result = await invoke<AgentPlan[]>("scan_agent_plans");
      setPlans(result);
    } catch (e) {
      console.error("Failed to load plans:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadPlans();

    const unlistenPlan = listen("plan-updated", () => {
      loadPlans();
    });
    const unlistenActions = listen("new_actions", () => {
      // Refresh action counts
      invoke<AgentPlan[]>("get_agent_plans").then(setPlans).catch(console.error);
    });

    return () => {
      unlistenPlan.then((fn) => fn());
      unlistenActions.then((fn) => fn());
    };
  }, [loadPlans]);

  const loadPlanActions = useCallback(async (slug: string) => {
    setActionsLoading(true);
    try {
      const result = await invoke<Action[]>("get_plan_actions", { slug });
      setPlanActions(result);
    } catch (e) {
      console.error("Failed to load plan actions:", e);
      setPlanActions([]);
    } finally {
      setActionsLoading(false);
    }
  }, []);

  const handleSelectPlan = (plan: AgentPlan) => {
    setSelectedPlan(plan);
    setActionsExpanded(false);
    setPlanActions([]);
  };

  const handleBack = () => {
    setSelectedPlan(null);
    setPlanActions([]);
    setActionsExpanded(false);
  };

  const filteredPlans = plans.filter((p) => {
    if (!search) return true;
    const q = search.toLowerCase();
    return (
      (p.title || p.display_name).toLowerCase().includes(q) ||
      p.slug.toLowerCase().includes(q)
    );
  });

  // ── Detail View ────────────────────────────────────────────────
  if (selectedPlan) {
    return (
      <div className="space-y-6">
        {/* Back button + header */}
        <div className="flex items-center gap-4">
          <button
            onClick={handleBack}
            className="p-2 rounded-xl hover:bg-white/5 transition-colors"
          >
            <ArrowLeft className="w-5 h-5 text-white/50" />
          </button>
          <div className="flex-1">
            <h1 className="text-2xl font-bold">
              {selectedPlan.title || selectedPlan.display_name}
            </h1>
            <div className="flex items-center gap-4 mt-1 text-sm text-white/40">
              <span className="font-mono text-xs">{selectedPlan.slug}</span>
              <span className="flex items-center gap-1">
                <HardDrive className="w-3 h-3" />
                {formatFileSize(selectedPlan.file_size)}
              </span>
              <span className="flex items-center gap-1">
                <Clock className="w-3 h-3" />
                {formatRelativeDate(selectedPlan.modified_at)}
              </span>
              {selectedPlan.action_count > 0 && (
                <span className="flex items-center gap-1">
                  <Activity className="w-3 h-3" />
                  {selectedPlan.action_count} action{selectedPlan.action_count !== 1 ? "s" : ""}
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Plan Content */}
        <div className="glass-card p-6">
          <div className="prose prose-invert max-w-none
            prose-headings:text-white prose-headings:font-bold
            prose-h1:text-2xl prose-h1:mb-4 prose-h1:mt-6
            prose-h2:text-xl prose-h2:mb-3 prose-h2:mt-5
            prose-h3:text-lg prose-h3:mb-2 prose-h3:mt-4
            prose-p:text-white/70 prose-p:leading-relaxed
            prose-a:text-purple-400 prose-a:no-underline hover:prose-a:underline
            prose-strong:text-white
            prose-code:text-purple-300 prose-code:bg-white/5 prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded
            prose-pre:bg-black/30 prose-pre:border prose-pre:border-white/5 prose-pre:rounded-xl
            prose-li:text-white/60
            prose-table:border-collapse
            prose-th:text-left prose-th:text-white/80 prose-th:border-b prose-th:border-white/10 prose-th:pb-2 prose-th:pr-4
            prose-td:text-white/60 prose-td:border-b prose-td:border-white/5 prose-td:py-2 prose-td:pr-4
            prose-hr:border-white/10
            prose-blockquote:border-l-purple-500/50 prose-blockquote:text-white/50
          ">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>
              {selectedPlan.content}
            </ReactMarkdown>
          </div>
        </div>

        {/* Linked Actions */}
        {selectedPlan.action_count > 0 && (
          <div className="glass-card overflow-hidden">
            <button
              onClick={() => {
                const willExpand = !actionsExpanded;
                setActionsExpanded(willExpand);
                if (willExpand && planActions.length === 0) {
                  loadPlanActions(selectedPlan.slug);
                }
              }}
              className="w-full flex items-center justify-between p-4 hover:bg-white/[0.02] transition-colors"
            >
              <div className="flex items-center gap-3">
                <Activity className="w-4 h-4 text-purple-400" />
                <span className="font-medium">
                  Linked Actions
                </span>
                <span className="text-xs px-2 py-0.5 rounded-full bg-purple-500/15 text-purple-300 border border-purple-500/20">
                  {selectedPlan.action_count}
                </span>
              </div>
              {actionsExpanded ? (
                <ChevronDown className="w-4 h-4 text-white/40" />
              ) : (
                <ChevronRight className="w-4 h-4 text-white/40" />
              )}
            </button>

            {actionsExpanded && (
              <div className="border-t border-white/5">
                {actionsLoading ? (
                  <div className="p-6 flex items-center justify-center">
                    <div className="w-5 h-5 rounded-full border-2 border-purple-500 border-t-transparent animate-spin" />
                  </div>
                ) : planActions.length === 0 ? (
                  <div className="p-6 text-center text-white/30 text-sm">
                    No linked actions found.
                  </div>
                ) : (
                  <div className="max-h-[400px] overflow-y-auto">
                    {planActions.map((action) => (
                      <div
                        key={action.id}
                        className="flex items-center gap-3 px-4 py-3 border-b border-white/[0.03] last:border-0 hover:bg-white/[0.02]"
                      >
                        <div className="w-1.5 h-1.5 rounded-full bg-purple-500/60 shrink-0" />
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="text-xs font-mono px-1.5 py-0.5 rounded bg-white/5 text-white/70">
                              {getToolName(action)}
                            </span>
                            <span className={`text-[10px] px-1.5 py-0.5 rounded-full border ${riskBg(action.risk_level)} ${riskColor(action.risk_level)}`}>
                              {action.risk_level}
                            </span>
                          </div>
                          <p className="text-xs text-white/40 mt-1 truncate">
                            {action.description}
                          </p>
                        </div>
                        <span className="text-[10px] text-white/25 shrink-0">
                          {formatRelativeDate(action.timestamp)}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    );
  }

  // ── List View ──────────────────────────────────────────────────
  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-purple-500/15 flex items-center justify-center">
            <FileText className="w-5 h-5 text-purple-400" />
          </div>
          <div>
            <h1 className="text-2xl font-bold">Agent Plans</h1>
            <p className="text-sm text-white/40">
              Implementation plans from Claude Code sessions
            </p>
          </div>
          {plans.length > 0 && (
            <span className="text-xs px-2 py-0.5 rounded-full bg-purple-500/15 text-purple-300 border border-purple-500/20 ml-2">
              {plans.length}
            </span>
          )}
        </div>
      </div>

      {/* Search */}
      {plans.length > 0 && (
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-white/30" />
          <input
            type="text"
            placeholder="Search plans..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full pl-10 pr-4 py-2.5 rounded-xl bg-white/5 border border-white/10 text-sm placeholder:text-white/25 focus:outline-none focus:border-purple-500/30"
          />
        </div>
      )}

      {/* Plan cards */}
      {loading ? (
        <div className="flex items-center justify-center py-16">
          <div className="w-8 h-8 rounded-full border-2 border-purple-500 border-t-transparent animate-spin" />
        </div>
      ) : filteredPlans.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <FileText className="w-12 h-12 text-white/10 mb-4" />
          <p className="text-white/40 text-lg font-medium">
            {search ? "No plans match your search" : "No plans found"}
          </p>
          <p className="text-white/25 text-sm mt-1">
            {search
              ? "Try a different search term"
              : "Plans will appear here when Claude Code creates implementation plans"}
          </p>
        </div>
      ) : (
        <div className="grid gap-3">
          {filteredPlans.map((plan) => (
            <div
              key={plan.id}
              onClick={() => handleSelectPlan(plan)}
              className="glass-card p-5 cursor-pointer card-lift group"
            >
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1 min-w-0">
                  <h3 className="font-semibold text-white group-hover:text-purple-300 transition-colors truncate">
                    {plan.title || plan.display_name}
                  </h3>
                  <p className="text-xs font-mono text-white/25 mt-1">
                    {plan.slug}
                  </p>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  {plan.action_count > 0 && (
                    <span className="text-xs px-2 py-0.5 rounded-full bg-purple-500/15 text-purple-300 border border-purple-500/20">
                      {plan.action_count} action{plan.action_count !== 1 ? "s" : ""}
                    </span>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-4 mt-3 text-xs text-white/30">
                <span className="flex items-center gap-1">
                  <Clock className="w-3 h-3" />
                  {formatRelativeDate(plan.modified_at)}
                </span>
                <span className="flex items-center gap-1">
                  <HardDrive className="w-3 h-3" />
                  {formatFileSize(plan.file_size)}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
