import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  BarChart3,
  ArrowLeft,
  Download,
  Plus,
  Calendar,
  TrendingUp,
  TrendingDown,
  Minus,
  Activity,
  DollarSign,
  ShieldCheck,
  Fingerprint,
  Wifi,
} from "lucide-react";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from "recharts";
import type { WeeklyReport, WeeklyReportSummary } from "@/types";


function formatDate(dateStr: string): string {
  const d = new Date(dateStr + "T00:00:00");
  return d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function SecurityGauge({ score, size = 100 }: { score: number; size?: number }) {
  const r = size * 0.4;
  const circumference = 2 * Math.PI * r;
  const offset = circumference * (1 - score / 100);
  const color =
    score >= 80 ? "#10B981" : score >= 50 ? "#F59E0B" : "#EF4444";

  return (
    <div className="relative" style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="rgba(255,255,255,0.08)"
          strokeWidth={size * 0.07}
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={color}
          strokeWidth={size * 0.07}
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          style={{ transform: "rotate(-90deg)", transformOrigin: "center" }}
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <span className="font-bold" style={{ fontSize: size * 0.28, color }}>
          {score}
        </span>
        <span
          className="text-white/40 uppercase tracking-wider"
          style={{ fontSize: size * 0.09 }}
        >
          Security
        </span>
      </div>
    </div>
  );
}

function TrendBadge({ trend }: { trend: string }) {
  if (trend.startsWith("up")) {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-red-500/15 text-red-400">
        <TrendingUp className="w-3 h-3" /> {trend}
      </span>
    );
  }
  if (trend.startsWith("down")) {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-emerald-500/15 text-emerald-400">
        <TrendingDown className="w-3 h-3" /> {trend}
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-white/10 text-white/50">
      <Minus className="w-3 h-3" /> stable
    </span>
  );
}

// ────────────────────────────────────────────────────────

export function WeeklyReportPage() {
  const [reports, setReports] = useState<WeeklyReportSummary[]>([]);
  const [selectedReport, setSelectedReport] = useState<WeeklyReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);

  const loadReports = useCallback(async () => {
    try {
      const list = await invoke<WeeklyReportSummary[]>("get_weekly_reports");
      setReports(list);
    } catch (e) {
      console.error("Failed to load reports:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadReports();
  }, [loadReports]);

  async function handleGenerate() {
    setGenerating(true);
    try {
      const report = await invoke<WeeklyReport>("generate_weekly_report", {
        weekStart: null,
      });
      setSelectedReport(report);
      await loadReports();
    } catch (e) {
      console.error("Failed to generate report:", e);
    } finally {
      setGenerating(false);
    }
  }

  async function handleViewReport(id: string) {
    try {
      const report = await invoke<WeeklyReport>("get_weekly_report", { id });
      setSelectedReport(report);
    } catch (e) {
      console.error("Failed to load report:", e);
    }
  }

  const [saveStatus, setSaveStatus] = useState<string | null>(null);

  async function handleSaveHtml() {
    if (!selectedReport) return;
    try {
      const savedPath = await invoke<string>("save_report_as_file", {
        id: selectedReport.id,
        path: null,
      });
      setSaveStatus(savedPath);
      // Auto-clear status after 5 seconds
      setTimeout(() => setSaveStatus(null), 5000);
      // Open the file in default browser
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(savedPath);
    } catch (e) {
      console.error("Failed to save report:", e);
      setSaveStatus("error");
      setTimeout(() => setSaveStatus(null), 3000);
    }
  }

  // ── Detail View ──

  if (selectedReport) {
    const r = selectedReport;
    const actionsData = r.actions_by_day.map(([name, value]) => ({
      name,
      value,
    }));
    const costData = r.cost_by_day.map(([name, value]) => ({
      name,
      value,
    }));

    return (
      <div className="space-y-6">
        {/* Top bar */}
        <div className="flex items-center justify-between">
          <button
            onClick={() => setSelectedReport(null)}
            className="flex items-center gap-2 text-sm text-white/50 hover:text-white transition-colors"
          >
            <ArrowLeft className="w-4 h-4" /> Back to Reports
          </button>
          <div className="flex items-center gap-3">
            {saveStatus && saveStatus !== "error" && (
              <span className="text-xs text-emerald-400">Saved to Desktop</span>
            )}
            {saveStatus === "error" && (
              <span className="text-xs text-rose-400">Failed to save</span>
            )}
            <button
              onClick={handleSaveHtml}
              className="flex items-center gap-2 px-4 py-2 rounded-xl bg-white/5 border border-white/10 text-sm hover:bg-white/10 transition-colors"
            >
              <Download className="w-4 h-4" /> Save as HTML
            </button>
          </div>
        </div>

        {/* Header */}
        <div className="glass-card p-6 flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold">
              {formatDate(r.week_start)} — {formatDate(r.week_end)}
            </h2>
            <p className="text-sm text-white/40 mt-1">
              Generated {new Date(r.generated_at).toLocaleString()}
            </p>
          </div>
          <SecurityGauge score={r.security_score} size={100} />
        </div>

        {/* Activity */}
        <div className="glass-card p-6">
          <div className="flex items-center gap-2 mb-4">
            <div className="icon-container-purple w-8 h-8 rounded-lg flex items-center justify-center">
              <Activity className="w-4 h-4" />
            </div>
            <h3 className="text-lg font-semibold">Activity</h3>
          </div>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <div>
              <p className="stat-number">{r.total_actions}</p>
              <p className="text-white/40 text-sm mt-1">total actions this week</p>
              {r.prev_week_actions != null && r.prev_week_actions > 0 && (
                <p className="text-sm mt-2">
                  <span
                    className={
                      r.total_actions > r.prev_week_actions
                        ? "text-amber-400"
                        : "text-emerald-400"
                    }
                  >
                    {r.total_actions > r.prev_week_actions ? "+" : ""}
                    {r.total_actions - r.prev_week_actions} vs last week
                  </span>
                </p>
              )}
              <div className="flex gap-2 mt-3 flex-wrap">
                <span className="px-3 py-1 rounded-full text-xs bg-purple-500/15 text-purple-300">
                  Busiest: {r.busiest_day}
                </span>
                <span className="px-3 py-1 rounded-full text-xs bg-purple-500/15 text-purple-300">
                  Peak: {r.busiest_hour}:00
                </span>
              </div>
            </div>
            <div className="h-[160px]">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={actionsData}>
                  <XAxis
                    dataKey="name"
                    tick={{ fill: "rgba(255,255,255,0.4)", fontSize: 11 }}
                    axisLine={false}
                    tickLine={false}
                  />
                  <YAxis hide />
                  <Tooltip
                    contentStyle={{
                      background: "rgba(0,0,0,0.8)",
                      border: "1px solid rgba(255,255,255,0.1)",
                      borderRadius: 8,
                      color: "#fff",
                      fontSize: 12,
                    }}
                  />
                  <Bar dataKey="value" radius={[4, 4, 0, 0]}>
                    {actionsData.map((_, i) => (
                      <Cell
                        key={i}
                        fill={`hsl(${270 - i * 5}, 70%, ${55 + i * 3}%)`}
                      />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* Agent breakdown */}
          {r.actions_by_agent.length > 0 && (
            <div className="mt-4 pt-4 border-t border-white/5">
              <p className="text-xs text-white/40 mb-2">Agent Breakdown</p>
              <div className="space-y-2">
                {r.actions_by_agent.map((agent) => (
                  <div
                    key={agent.agent_name}
                    className="flex justify-between items-center text-sm"
                  >
                    <div>
                      <span className="text-white/80">{agent.agent_name}</span>
                      <span className="text-white/30 ml-2 text-xs">
                        {agent.agent_type}
                      </span>
                    </div>
                    <div className="text-right">
                      <span className="text-white/70">
                        {agent.action_count} actions
                      </span>
                      <span className="text-white/30 ml-3 text-xs">
                        ${agent.cost.toFixed(4)}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Cost */}
        <div className="glass-card p-6">
          <div className="flex items-center gap-2 mb-4">
            <div className="icon-container-emerald w-8 h-8 rounded-lg flex items-center justify-center">
              <DollarSign className="w-4 h-4" />
            </div>
            <h3 className="text-lg font-semibold">Cost</h3>
          </div>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <div>
              <p className="stat-number">${r.total_cost.toFixed(4)}</p>
              <div className="mt-2">
                <TrendBadge trend={r.cost_trend} />
              </div>
            </div>
            <div className="h-[120px]">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={costData}>
                  <XAxis
                    dataKey="name"
                    tick={{ fill: "rgba(255,255,255,0.4)", fontSize: 11 }}
                    axisLine={false}
                    tickLine={false}
                  />
                  <YAxis hide />
                  <Tooltip
                    contentStyle={{
                      background: "rgba(0,0,0,0.8)",
                      border: "1px solid rgba(255,255,255,0.1)",
                      borderRadius: 8,
                      color: "#fff",
                      fontSize: 12,
                    }}
                    formatter={(value: number) => [`$${value.toFixed(4)}`, "Cost"]}
                  />
                  <Bar dataKey="value" fill="#10B981" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>
        </div>

        {/* Safety & PII */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div className="glass-card p-6">
            <div className="flex items-center gap-2 mb-4">
              <div className="icon-container-cyan w-8 h-8 rounded-lg flex items-center justify-center">
                <ShieldCheck className="w-4 h-4" />
              </div>
              <h3 className="text-lg font-semibold">Safety Net</h3>
            </div>
            <div className="grid grid-cols-3 gap-3">
              <div className="text-center">
                <p className="text-2xl font-bold">{r.files_protected}</p>
                <p className="text-xs text-white/40">Protected</p>
              </div>
              <div className="text-center">
                <p className="text-2xl font-bold">{r.files_restored}</p>
                <p className="text-xs text-white/40">Restored</p>
              </div>
              <div className="text-center">
                <p className="text-2xl font-bold">
                  {r.safety_net_size_mb.toFixed(1)}
                </p>
                <p className="text-xs text-white/40">MB Used</p>
              </div>
            </div>
          </div>
          <div className="glass-card p-6">
            <div className="flex items-center gap-2 mb-4">
              <div className="icon-container-rose w-8 h-8 rounded-lg flex items-center justify-center">
                <Fingerprint className="w-4 h-4" />
              </div>
              <h3 className="text-lg font-semibold">PII Guard</h3>
            </div>
            <div className="grid grid-cols-3 gap-3">
              <div className="text-center">
                <p className="text-2xl font-bold">{r.pii_findings}</p>
                <p className="text-xs text-white/40">Findings</p>
              </div>
              <div className="text-center">
                <p className="text-2xl font-bold text-red-400">
                  {r.pii_critical}
                </p>
                <p className="text-xs text-white/40">Critical</p>
              </div>
              <div className="text-center">
                <SecurityGauge score={r.security_score} size={56} />
              </div>
            </div>
          </div>
        </div>

        {/* Data Shield */}
        <div className="glass-card p-6">
          <div className="flex items-center gap-2 mb-4">
            <div className="icon-container-emerald w-8 h-8 rounded-lg flex items-center justify-center">
              <Wifi className="w-4 h-4" />
            </div>
            <h3 className="text-lg font-semibold">Data Shield</h3>
          </div>
          <div className="grid grid-cols-3 gap-4">
            <div className="text-center">
              <p className="text-2xl font-bold">{r.domains_contacted}</p>
              <p className="text-xs text-white/40">Domains Contacted</p>
            </div>
            <div className="text-center">
              <p className="text-2xl font-bold text-amber-400">
                {r.unknown_domains}
              </p>
              <p className="text-xs text-white/40">Unknown</p>
            </div>
            <div className="text-center">
              <p className="text-2xl font-bold">{r.outbound_events}</p>
              <p className="text-xs text-white/40">Events</p>
            </div>
          </div>
        </div>

        {/* Footer */}
        <p className="text-center text-xs text-white/20 pb-4">
          Generated by Unalome Agent Firewall &middot; unalome.ai
        </p>
      </div>
    );
  }

  // ── List View ──

  return (
    <div className="space-y-8">
      {/* Hero */}
      <div className="text-center pt-4 pb-2">
        <div className="flex items-center justify-center gap-3 mb-2">
          <BarChart3 className="w-8 h-8 text-purple-400" />
          <h1 className="text-3xl font-bold">Weekly Reports</h1>
          <span className="w-2 h-2 rounded-full bg-purple-400 pulse-live" />
        </div>
        <p className="text-white/50 text-lg">
          Generate shareable infographic summaries of AI agent activity
        </p>
      </div>

      {/* Generate button */}
      <div className="flex justify-center">
        <button
          onClick={handleGenerate}
          disabled={generating}
          className="action-button flex items-center gap-2 disabled:opacity-50"
        >
          {generating ? (
            <>
              <div className="w-5 h-5 rounded-full border-2 border-white border-t-transparent animate-spin" />
              Generating...
            </>
          ) : (
            <>
              <Plus className="w-5 h-5" /> Generate This Week's Report
            </>
          )}
        </button>
      </div>

      {/* Report list */}
      {loading ? (
        <div className="flex justify-center py-12">
          <div className="w-8 h-8 rounded-full border-2 border-purple-500 border-t-transparent animate-spin" />
        </div>
      ) : reports.length === 0 ? (
        <div className="text-center py-16">
          <BarChart3 className="w-16 h-16 text-white/10 mx-auto mb-4" />
          <p className="text-white/30 text-lg">No reports yet</p>
          <p className="text-white/20 text-sm mt-1">
            Generate your first weekly report above
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {reports.map((report) => (
            <div
              key={report.id}
              onClick={() => handleViewReport(report.id)}
              className="glass-card p-5 cursor-pointer card-lift"
            >
              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-2">
                  <Calendar className="w-4 h-4 text-white/40" />
                  <span className="text-sm font-medium">
                    {formatDate(report.week_start)} —{" "}
                    {formatDate(report.week_end)}
                  </span>
                </div>
                <SecurityGauge score={report.security_score} size={40} />
              </div>
              <div className="grid grid-cols-2 gap-4 mt-2">
                <div>
                  <p className="text-xl font-bold">{report.total_actions}</p>
                  <p className="text-xs text-white/40">actions</p>
                </div>
                <div>
                  <p className="text-xl font-bold">
                    ${report.total_cost.toFixed(2)}
                  </p>
                  <p className="text-xs text-white/40">cost</p>
                </div>
              </div>
              <p className="text-xs text-white/20 mt-3">
                {new Date(report.generated_at).toLocaleDateString()}
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
