import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  AlertTriangle,
  Scan,
  Lock,
  Shield,
  Loader2,
  ChevronDown,
  ChevronRight,
  Terminal,
  Key,
  Package,
  Check,
} from "lucide-react";
import type { Agent, SecurityReport } from "@/types";
import { cn } from "@/lib/utils";

interface SecurityDashboardProps {
  agents: Agent[];
}

const RISK_CONFIG: Record<string, { color: string; bg: string; border: string }> = {
  Safe: { color: "text-emerald-400", bg: "bg-emerald-500/15", border: "border-emerald-500/30" },
  Low: { color: "text-blue-400", bg: "bg-blue-500/15", border: "border-blue-500/30" },
  Medium: { color: "text-amber-400", bg: "bg-amber-500/15", border: "border-amber-500/30" },
  High: { color: "text-orange-400", bg: "bg-orange-500/15", border: "border-orange-500/30" },
  Critical: { color: "text-rose-400", bg: "bg-rose-500/15", border: "border-rose-500/30" },
};

export function SecurityDashboard({ agents: _agents }: SecurityDashboardProps) {
  const [reports, setReports] = useState<SecurityReport[]>([]);
  const [scanning, setScanning] = useState(false);
  const [hasScanned, setHasScanned] = useState(false);

  const runScan = async () => {
    setScanning(true);
    try {
      const results = await invoke<SecurityReport[]>("scan_all_mcp_configs");
      setReports(results);
      setHasScanned(true);
    } catch (e) {
      console.error("MCP scan failed:", e);
    } finally {
      setScanning(false);
    }
  };

  useEffect(() => {
    runScan();
  }, []);

  const totalTools = reports.reduce((sum, r) => sum + r.tools_scanned.length, 0);
  const totalWarnings = reports.reduce((sum, r) => sum + r.warnings.length, 0);
  const highRiskCount = reports.filter(
    (r) => r.overall_risk === "High" || r.overall_risk === "Critical"
  ).length;
  const securityScore = hasScanned
    ? Math.max(0, Math.min(100, 100 - highRiskCount * 20 - totalWarnings * 5))
    : 0;

  return (
    <div className="space-y-6">
      {/* Hero Score Card */}
      <div className="glass-card p-6">
        <div className="flex items-center gap-6">
          <div className="relative">
            <CircularScore value={securityScore} size={120} />
          </div>

          <div className="flex-1">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-12 h-12 rounded-xl bg-fuchsia-500/15 flex items-center justify-center">
                <Shield className="w-6 h-6 text-fuchsia-400" />
              </div>
              <div>
                <h2 className="text-lg font-bold">Security Score</h2>
                <p className="text-sm text-white/50">
                  {hasScanned
                    ? `${reports.length} MCP server${reports.length !== 1 ? "s" : ""} · ${totalTools} tool${totalTools !== 1 ? "s" : ""} · ${totalWarnings} warning${totalWarnings !== 1 ? "s" : ""}`
                    : "Scanning your agent configurations..."}
                </p>
              </div>
            </div>

            <div className="flex items-center gap-3 mt-3">
              {highRiskCount > 0 && (
                <span className="px-2.5 py-1 rounded-full bg-orange-500/15 text-orange-400 text-xs font-semibold">
                  {highRiskCount} high risk
                </span>
              )}
              {totalWarnings > 0 && (
                <span className="px-2.5 py-1 rounded-full bg-amber-500/15 text-amber-400 text-xs font-semibold">
                  {totalWarnings} warning{totalWarnings !== 1 ? "s" : ""}
                </span>
              )}

              <button
                onClick={runScan}
                disabled={scanning}
                className="ml-auto px-4 py-2 rounded-xl text-xs font-medium glass hover:bg-white/10 transition-colors flex items-center gap-2"
              >
                {scanning ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <Scan className="w-3.5 h-3.5" />
                )}
                {scanning ? "Scanning..." : "Rescan All"}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* MCP Server Cards */}
      {reports.length > 0 ? (
        <div className="space-y-2">
          {reports.map((report) => (
            <ServerCard key={report.server_name} report={report} />
          ))}
        </div>
      ) : hasScanned ? (
        <div className="glass-card p-12 text-center">
          <div className="w-16 h-16 rounded-2xl bg-fuchsia-500/15 flex items-center justify-center mx-auto mb-4">
            <Lock className="w-8 h-8 text-fuchsia-400" />
          </div>
          <h3 className="text-xl font-bold mb-2">No MCP servers found</h3>
          <p className="text-white/50 max-w-md mx-auto">
            Servers will appear here when your agents have MCP configurations.
          </p>
        </div>
      ) : null}
    </div>
  );
}

function CircularScore({ value, size }: { value: number; size: number }) {
  const r = (size - 12) / 2;
  const circumference = 2 * Math.PI * r;
  const strokeDashoffset = circumference - (value / 100) * circumference;

  const getColor = () => {
    if (value >= 80) return "#10B981";
    if (value >= 60) return "#F59E0B";
    return "#F43F5E";
  };

  return (
    <div className="relative" style={{ width: size, height: size }}>
      <svg className="w-full h-full -rotate-90" viewBox={`0 0 ${size} ${size}`}>
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="rgba(255,255,255,0.06)"
          strokeWidth={10}
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={getColor()}
          strokeWidth={10}
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={strokeDashoffset}
          style={{ transition: "stroke-dashoffset 0.8s ease" }}
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <span className="text-3xl font-bold">{value}</span>
        <span className="text-[10px] text-white/40">Secure</span>
      </div>
    </div>
  );
}

function ServerCard({ report }: { report: SecurityReport }) {
  const [expanded, setExpanded] = useState(false);
  const risk = RISK_CONFIG[report.overall_risk] ?? RISK_CONFIG.Safe;

  return (
    <div className={cn("glass-card border transition-all duration-200", risk.border)}>
      {/* Compact row */}
      <div
        className="flex items-center gap-3 p-3 cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        <div className={cn("w-8 h-8 rounded-lg flex items-center justify-center shrink-0", risk.bg)}>
          <Lock className={cn("w-4 h-4", risk.color)} />
        </div>

        <span className={cn("text-xs font-semibold px-2 py-0.5 rounded-full shrink-0", risk.bg, risk.color)}>
          {report.overall_risk}
        </span>

        <span className="text-sm font-medium text-white/90 truncate flex-1">
          {report.server_name}
        </span>

        {report.source_agent && (
          <span className="px-2 py-0.5 rounded-full glass text-[10px] text-white/40 shrink-0">
            {report.source_agent}
          </span>
        )}

        <span className="text-xs text-white/30 shrink-0">
          {report.tools_scanned.length > 0
            ? `${report.tools_scanned.length} tool${report.tools_scanned.length !== 1 ? "s" : ""}`
            : "config only"}
        </span>

        {report.warnings.length > 0 && (
          <span className="text-xs text-amber-400/70 shrink-0">
            {report.warnings.length} warning{report.warnings.length !== 1 ? "s" : ""}
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
        <div className="px-3 pb-3 pt-0 ml-11 space-y-3">
          {/* Command info */}
          {report.command && (
            <div className="bg-black/30 rounded-lg px-3 py-2 space-y-1.5">
              <div className="flex items-center gap-2 text-xs text-white/50">
                <Terminal className="w-3 h-3" />
                <span className="text-white/30">Command:</span>
                <code className="text-white/70 font-mono">{report.command}</code>
              </div>
              {report.args.length > 0 && (
                <div className="flex items-start gap-2 text-xs">
                  <span className="text-white/30 shrink-0">Args:</span>
                  <code className="text-white/50 font-mono break-all">
                    {report.args.join(" ")}
                  </code>
                </div>
              )}
            </div>
          )}

          {/* Env vars */}
          {report.env_vars.length > 0 && (
            <div className="bg-black/30 rounded-lg px-3 py-2">
              <div className="flex items-center gap-2 text-xs text-white/50 mb-1.5">
                <Key className="w-3 h-3" />
                <span className="text-white/30">Environment Variables</span>
              </div>
              <div className="flex flex-wrap gap-1.5">
                {report.env_vars.map((v) => (
                  <span
                    key={v}
                    className="px-2 py-0.5 rounded-md bg-white/5 text-[11px] font-mono text-white/50"
                  >
                    {v}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Tools */}
          {report.tools_scanned.length > 0 && (
            <div className="space-y-1.5">
              <div className="flex items-center gap-2 text-xs text-white/40">
                <Package className="w-3 h-3" />
                Tools
              </div>
              {report.tools_scanned.map((tool) => {
                const toolRisk = RISK_CONFIG[tool.risk_level] ?? RISK_CONFIG.Safe;
                return (
                  <div
                    key={tool.name}
                    className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-white/3 text-xs"
                  >
                    <span className={cn("w-1.5 h-1.5 rounded-full shrink-0", toolRisk.bg.replace("/15", ""))} />
                    <span className="text-white/70 font-medium">{tool.name}</span>
                    {tool.permissions.length > 0 && (
                      <span className="text-white/30">
                        ({tool.permissions.join(", ")})
                      </span>
                    )}
                    {tool.warnings.length > 0 && (
                      <span className="text-amber-400/60 ml-auto">
                        {tool.warnings.length} issue{tool.warnings.length !== 1 ? "s" : ""}
                      </span>
                    )}
                  </div>
                );
              })}
            </div>
          )}

          {/* Warnings */}
          {report.warnings.length > 0 && (
            <div className="space-y-1.5">
              <div className="flex items-center gap-2 text-xs text-white/40">
                <AlertTriangle className="w-3 h-3" />
                Warnings
              </div>
              {report.warnings.map((w, i) => {
                const wRisk = RISK_CONFIG[w.severity] ?? RISK_CONFIG.Medium;
                return (
                  <div
                    key={i}
                    className={cn("flex items-start gap-2 px-3 py-2 rounded-lg border text-xs", wRisk.border, wRisk.bg)}
                  >
                    <AlertTriangle className={cn("w-3 h-3 shrink-0 mt-0.5", wRisk.color)} />
                    <div className="flex-1 min-w-0">
                      <span className={cn("font-medium", wRisk.color)}>{w.severity}</span>
                      <span className="text-white/50"> · {w.category}</span>
                      <p className="text-white/60 mt-0.5">{w.message}</p>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {/* Clean state */}
          {report.warnings.length === 0 && report.tools_scanned.length === 0 && (
            <div className="flex items-center gap-2 text-xs text-emerald-400/70 px-3 py-2 rounded-lg bg-emerald-500/10">
              <Check className="w-3.5 h-3.5" />
              No issues detected in static config analysis
            </div>
          )}
        </div>
      )}
    </div>
  );
}
