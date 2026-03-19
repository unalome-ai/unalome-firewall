import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  ShieldCheck,
  Key,
  Mail,
  CreditCard,
  Phone,
  Globe,
  FileKey,
  Lock,
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  Eye,
  Undo2,
  Fingerprint,
} from "lucide-react";
import type { Agent, PiiFinding, PiiStats } from "@/types";
import { cn } from "@/lib/utils";

interface PiiGuardianProps {
  agents: Agent[];
}

const SEVERITY_CONFIG: Record<string, { color: string; bg: string; border: string; label: string }> = {
  critical: { color: "text-red-400", bg: "bg-red-500/15", border: "border-red-500/30", label: "Critical" },
  high: { color: "text-orange-400", bg: "bg-orange-500/15", border: "border-orange-500/30", label: "High" },
  medium: { color: "text-yellow-400", bg: "bg-yellow-500/15", border: "border-yellow-500/30", label: "Medium" },
  low: { color: "text-blue-400", bg: "bg-blue-500/15", border: "border-blue-500/30", label: "Low" },
};

const TYPE_ICONS: Record<string, React.ElementType> = {
  api_key: Key,
  private_key: FileKey,
  jwt: Lock,
  connection_string: Globe,
  ssn: Fingerprint,
  credit_card: CreditCard,
  password: Lock,
  env_variable: FileKey,
  email: Mail,
  phone: Phone,
  ip_address: Globe,
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

const TIME_PERIODS = [
  { label: "1h", ms: 60 * 60 * 1000 },
  { label: "24h", ms: 24 * 60 * 60 * 1000 },
  { label: "7d", ms: 7 * 24 * 60 * 60 * 1000 },
  { label: "30d", ms: 30 * 24 * 60 * 60 * 1000 },
  { label: "All", ms: 0 },
];

export function PiiGuardian({ agents: _agents }: PiiGuardianProps) {
  const [findings, setFindings] = useState<PiiFinding[]>([]);
  const [dismissedFindings, setDismissedFindings] = useState<PiiFinding[]>([]);
  const [stats, setStats] = useState<PiiStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [showDismissed, setShowDismissed] = useState(false);
  const [total, setTotal] = useState(0);
  const [timePeriod, setTimePeriod] = useState("All");
  const [expandedSeverities, setExpandedSeverities] = useState<Set<string>>(new Set());

  const loadFindings = useCallback(async () => {
    try {
      const result = await invoke<{ findings: PiiFinding[]; total: number }>("get_pii_findings", {
        limit: 100,
        offset: 0,
        severity: null,
        agentId: null,
        dismissed: false,
      });
      setFindings(result.findings);
      setTotal(result.total);

      const dismissedResult = await invoke<{ findings: PiiFinding[]; total: number }>("get_pii_findings", {
        limit: 50,
        offset: 0,
        severity: null,
        agentId: null,
        dismissed: true,
      });
      setDismissedFindings(dismissedResult.findings);

      const piiStats = await invoke<PiiStats>("get_pii_stats");
      setStats(piiStats);
    } catch (e) {
      console.error("Failed to load PII findings:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadFindings();

    const unlisten = listen<PiiFinding>("pii-finding", (event) => {
      setFindings((prev) => [event.payload, ...prev]);
      setTotal((prev) => prev + 1);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadFindings]);

  const handleDismiss = async (id: string) => {
    try {
      await invoke("dismiss_pii_finding", { id });
      const finding = findings.find((f) => f.id === id);
      setFindings((prev) => prev.filter((f) => f.id !== id));
      setTotal((prev) => prev - 1);
      if (finding) {
        setDismissedFindings((prev) => [{ ...finding, dismissed: true }, ...prev]);
      }
    } catch (e) {
      console.error("Failed to dismiss finding:", e);
    }
  };

  const handleRestore = async (id: string) => {
    try {
      await invoke("restore_pii_finding", { id });
      const finding = dismissedFindings.find((f) => f.id === id);
      setDismissedFindings((prev) => prev.filter((f) => f.id !== id));
      if (finding) {
        setFindings((prev) => [{ ...finding, dismissed: false }, ...prev]);
        setTotal((prev) => prev + 1);
      }
    } catch (e) {
      console.error("Failed to restore finding:", e);
    }
  };

  const toggleSeverity = (sev: string) => {
    setExpandedSeverities((prev) => {
      const next = new Set(prev);
      if (next.has(sev)) next.delete(sev);
      else next.add(sev);
      return next;
    });
  };

  // Filter findings by time period
  const filteredFindings = findings.filter((f) => {
    const period = TIME_PERIODS.find((p) => p.label === timePeriod);
    if (!period || period.ms === 0) return true;
    const cutoff = Date.now() - period.ms;
    return new Date(f.timestamp).getTime() >= cutoff;
  });

  // Group findings by severity
  const grouped = filteredFindings.reduce<Record<string, PiiFinding[]>>((acc, f) => {
    const sev = f.severity;
    if (!acc[sev]) acc[sev] = [];
    acc[sev].push(f);
    return acc;
  }, {});

  const severityOrder = ["critical", "high", "medium", "low"];

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <div className="w-8 h-8 rounded-full border-2 border-rose-500 border-t-transparent animate-spin" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Status Banner */}
      <div className="glass-card p-5 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 rounded-xl bg-rose-500/15 flex items-center justify-center">
            <ShieldCheck className="w-6 h-6 text-rose-400" />
          </div>
          <div>
            <h2 className="text-lg font-bold flex items-center gap-2">
              PII Guardian
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
            </h2>
            <p className="text-sm text-white/50">Scanning agent activity for sensitive data</p>
          </div>
        </div>

        <div className="flex items-center gap-3">
          {stats && (
            <>
              {(stats.by_severity["critical"] ?? 0) > 0 && (
                <span className="px-2.5 py-1 rounded-full bg-red-500/15 text-red-400 text-xs font-semibold">
                  {stats.by_severity["critical"]} critical
                </span>
              )}
              {(stats.by_severity["high"] ?? 0) > 0 && (
                <span className="px-2.5 py-1 rounded-full bg-orange-500/15 text-orange-400 text-xs font-semibold">
                  {stats.by_severity["high"]} high
                </span>
              )}
              {(stats.by_severity["medium"] ?? 0) > 0 && (
                <span className="px-2.5 py-1 rounded-full bg-yellow-500/15 text-yellow-400 text-xs font-semibold">
                  {stats.by_severity["medium"]} medium
                </span>
              )}
            </>
          )}
          <span className="px-3 py-1.5 rounded-full glass text-xs font-medium text-white/60">
            {total} finding{total !== 1 ? "s" : ""}
          </span>
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

      {/* Stats Row */}
      {stats && stats.total > 0 && (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="glass-card p-4">
            <p className="text-xs text-white/50 font-medium">Total Active</p>
            <p className="text-2xl font-bold mt-1">{stats.total}</p>
          </div>
          <div className="glass-card p-4">
            <p className="text-xs text-white/50 font-medium">Today</p>
            <p className="text-2xl font-bold mt-1">{stats.today}</p>
          </div>
          <div className="glass-card p-4">
            <p className="text-xs text-white/50 font-medium">This Week</p>
            <p className="text-2xl font-bold mt-1">{stats.this_week}</p>
          </div>
          <div className="glass-card p-4">
            <p className="text-xs text-white/50 font-medium">Types Found</p>
            <p className="text-2xl font-bold mt-1">{Object.keys(stats.by_type).length}</p>
          </div>
        </div>
      )}

      {/* Empty State */}
      {filteredFindings.length === 0 && findings.length > 0 && (
        <div className="glass-card p-12 text-center">
          <div className="w-16 h-16 rounded-2xl bg-yellow-500/15 flex items-center justify-center mx-auto mb-4">
            <AlertTriangle className="w-8 h-8 text-yellow-400" />
          </div>
          <h3 className="text-xl font-bold mb-2">No findings in this period</h3>
          <p className="text-white/50 max-w-md mx-auto">
            No PII detected in the selected time range. Try a wider period.
          </p>
        </div>
      )}
      {filteredFindings.length === 0 && findings.length === 0 && (
        <div className="glass-card p-12 text-center">
          <div className="w-16 h-16 rounded-2xl bg-emerald-500/15 flex items-center justify-center mx-auto mb-4">
            <ShieldCheck className="w-8 h-8 text-emerald-400" />
          </div>
          <h3 className="text-xl font-bold mb-2">No PII detected</h3>
          <p className="text-white/50 max-w-md mx-auto">
            PII Guardian is actively monitoring agent activity. You'll be alerted when sensitive data like API keys, emails, or passwords are detected.
          </p>
        </div>
      )}

      {/* Findings grouped by severity */}
      {severityOrder.map((sev) => {
        const items = grouped[sev];
        if (!items || items.length === 0) return null;
        const config = SEVERITY_CONFIG[sev];
        const isExpanded = expandedSeverities.has(sev);

        return (
          <div key={sev} className="space-y-3">
            <button
              onClick={() => toggleSeverity(sev)}
              className="flex items-center gap-2 hover:opacity-80 transition-opacity"
            >
              {isExpanded ? (
                <ChevronDown className={cn("w-4 h-4", config.color)} />
              ) : (
                <ChevronRight className={cn("w-4 h-4", config.color)} />
              )}
              <AlertTriangle className={cn("w-4 h-4", config.color)} />
              <h3 className="text-sm font-semibold text-white/70 uppercase tracking-wider">
                {config.label} ({items.length})
              </h3>
            </button>

            {isExpanded &&
              items.map((finding) => (
                <FindingCard
                  key={finding.id}
                  finding={finding}
                  onDismiss={handleDismiss}
                />
              ))}
          </div>
        );
      })}

      {/* Dismissed Section */}
      {dismissedFindings.length > 0 && (
        <div className="mt-8">
          <button
            onClick={() => setShowDismissed(!showDismissed)}
            className="flex items-center gap-2 text-sm text-white/40 hover:text-white/60 transition-colors"
          >
            {showDismissed ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
            Dismissed ({dismissedFindings.length})
          </button>

          {showDismissed && (
            <div className="mt-3 space-y-2">
              {dismissedFindings.map((finding) => (
                <div
                  key={finding.id}
                  className="glass-card p-4 opacity-50 flex items-center justify-between"
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <span className={cn("text-xs font-semibold px-2 py-0.5 rounded-full", SEVERITY_CONFIG[finding.severity]?.bg, SEVERITY_CONFIG[finding.severity]?.color)}>
                      {finding.finding_type}
                    </span>
                    <span className="text-sm text-white/50 truncate">{finding.description}</span>
                  </div>
                  <button
                    onClick={() => handleRestore(finding.id)}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg glass glass-hover text-xs text-white/60 shrink-0"
                  >
                    <Undo2 className="w-3 h-3" />
                    Restore
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function FindingCard({
  finding,
  onDismiss,
}: {
  finding: PiiFinding;
  onDismiss: (id: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const config = SEVERITY_CONFIG[finding.severity] ?? SEVERITY_CONFIG.low;
  const Icon = TYPE_ICONS[finding.finding_type] ?? AlertTriangle;

  return (
    <div className={cn("glass-card border transition-all duration-200", config.border)}>
      {/* Compact row */}
      <div
        className="flex items-center justify-between gap-3 p-3 cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-3 min-w-0">
          <div className={cn("w-7 h-7 rounded-lg flex items-center justify-center shrink-0", config.bg)}>
            <Icon className={cn("w-3.5 h-3.5", config.color)} />
          </div>
          <span className={cn("text-xs font-semibold px-2 py-0.5 rounded-full shrink-0", config.bg, config.color)}>
            {finding.finding_type}
          </span>
          <span className="text-sm text-white/70 truncate">{finding.description}</span>
          <span className="text-xs text-white/30 shrink-0">{timeAgo(finding.timestamp)}</span>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={(e) => {
              e.stopPropagation();
              onDismiss(finding.id);
            }}
            className="px-2.5 py-1 rounded-lg glass glass-hover text-xs text-white/40 hover:text-white/60"
          >
            Dismiss
          </button>
          {expanded ? (
            <ChevronDown className="w-4 h-4 text-white/30" />
          ) : (
            <ChevronRight className="w-4 h-4 text-white/30" />
          )}
        </div>
      </div>

      {/* Expanded details */}
      {expanded && (
        <div className="px-3 pb-3 pt-0 ml-10 space-y-2">
          <div className="bg-black/30 rounded-lg px-3 py-2">
            <code className="text-xs text-white/60 font-mono break-all">
              {finding.source_context || finding.redacted_value}
            </code>
          </div>

          {finding.source_file && (
            <p className="text-xs text-white/40 flex items-center gap-1">
              <Eye className="w-3 h-3" />
              {finding.source_file}
            </p>
          )}

          <div className="rounded-lg bg-teal-500/10 border border-teal-500/20 px-3 py-2">
            <p className="text-xs text-teal-300">{finding.recommended_action}</p>
          </div>
        </div>
      )}
    </div>
  );
}
