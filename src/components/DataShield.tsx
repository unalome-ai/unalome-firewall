import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Wifi,
  Search,
  AlertTriangle,
  CheckCircle2,
  HelpCircle,
  ExternalLink,
  ChevronDown,
} from "lucide-react";
import type { OutboundEvent, DomainProfile, DataShieldStats } from "@/types";
import { cn } from "@/lib/utils";

const CATEGORY_CONFIG: Record<string, { label: string; color: string; bg: string }> = {
  ai_provider: { label: "AI Provider", color: "text-blue-400", bg: "bg-blue-500/15" },
  cloud_service: { label: "Cloud", color: "text-slate-400", bg: "bg-slate-500/15" },
  mcp_server: { label: "MCP Server", color: "text-purple-400", bg: "bg-purple-500/15" },
  package_registry: { label: "Package Registry", color: "text-teal-400", bg: "bg-teal-500/15" },
  search_engine: { label: "Search", color: "text-cyan-400", bg: "bg-cyan-500/15" },
  documentation: { label: "Docs", color: "text-indigo-400", bg: "bg-indigo-500/15" },
  local: { label: "Local", color: "text-emerald-400", bg: "bg-emerald-500/15" },
  unknown: { label: "Unknown", color: "text-yellow-400", bg: "bg-yellow-500/15" },
  trusted: { label: "Trusted", color: "text-emerald-400", bg: "bg-emerald-500/15" },
  suspicious: { label: "Suspicious", color: "text-red-400", bg: "bg-red-500/15" },
};

const RISK_CONFIG: Record<string, { color: string; bg: string; icon: React.ElementType }> = {
  safe: { color: "text-emerald-400", bg: "bg-emerald-500/15", icon: CheckCircle2 },
  unknown: { color: "text-yellow-400", bg: "bg-yellow-500/15", icon: HelpCircle },
  suspicious: { color: "text-red-400", bg: "bg-red-500/15", icon: AlertTriangle },
};

type SortMode = "count" | "recent" | "risk";

const TIME_PERIODS = [
  { label: "1h", ms: 60 * 60 * 1000 },
  { label: "24h", ms: 24 * 60 * 60 * 1000 },
  { label: "7d", ms: 7 * 24 * 60 * 60 * 1000 },
  { label: "30d", ms: 30 * 24 * 60 * 60 * 1000 },
  { label: "All", ms: 0 },
];

function timeAgo(timestamp: string): string {
  const now = Date.now();
  const then = new Date(timestamp).getTime();
  const diff = Math.floor((now - then) / 1000);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

export function DataShield() {
  const [stats, setStats] = useState<DataShieldStats | null>(null);
  const [domains, setDomains] = useState<DomainProfile[]>([]);
  const [events, setEvents] = useState<OutboundEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [sortMode, setSortMode] = useState<SortMode>("count");
  const [timePeriod, setTimePeriod] = useState("All");

  const loadData = useCallback(async () => {
    try {
      const [shieldStats, domainProfiles, eventsResult] = await Promise.all([
        invoke<DataShieldStats>("get_data_shield_stats"),
        invoke<DomainProfile[]>("get_domain_profiles"),
        invoke<{ events: OutboundEvent[]; total: number }>("get_outbound_events", {
          limit: 50,
          offset: 0,
          agentId: null,
          riskLevel: null,
          destination: null,
        }),
      ]);
      setStats(shieldStats);
      setDomains(domainProfiles);
      setEvents(eventsResult.events);
    } catch (e) {
      console.error("Failed to load Data Shield data:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();

    const unlisten = listen<OutboundEvent>("outbound-event", (event) => {
      setEvents((prev) => [event.payload, ...prev.slice(0, 99)]);
      // Refresh stats and domains when new events arrive
      loadData();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadData]);

  const handleClassify = async (domain: string, classification: string) => {
    try {
      await invoke("classify_domain", { domain, classification });
      await loadData();
    } catch (e) {
      console.error("Failed to classify domain:", e);
    }
  };

  // Time filter helper
  const isInTimePeriod = (timestamp: string) => {
    const period = TIME_PERIODS.find((p) => p.label === timePeriod);
    if (!period || period.ms === 0) return true;
    return new Date(timestamp).getTime() >= Date.now() - period.ms;
  };

  // Filter and sort domains
  const filteredDomains = domains
    .filter((d) => isInTimePeriod(d.last_seen))
    .filter((d) => !search || d.domain.toLowerCase().includes(search.toLowerCase()))
    .sort((a, b) => {
      if (sortMode === "count") return b.total_events - a.total_events;
      if (sortMode === "recent") return b.last_seen.localeCompare(a.last_seen);
      // risk: suspicious > unknown > safe
      const riskOrder: Record<string, number> = { suspicious: 0, unknown: 1, safe: 2 };
      return (riskOrder[a.risk_level] ?? 1) - (riskOrder[b.risk_level] ?? 1);
    });

  // Filter events by time
  const filteredEvents = events.filter((e) => isInTimePeriod(e.timestamp));

  // Unknown domains that need attention
  const unknownDomains = domains.filter((d) => d.risk_level === "unknown" && isInTimePeriod(d.last_seen));

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <div className="w-8 h-8 rounded-full border-2 border-cyan-500 border-t-transparent animate-spin" />
      </div>
    );
  }

  const isEmpty = !stats || stats.total_events === 0;

  return (
    <div className="space-y-6">
      {/* Hero Status Banner */}
      <div className="glass-card p-5 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 rounded-xl bg-cyan-500/15 flex items-center justify-center">
            <Wifi className="w-6 h-6 text-cyan-400" />
          </div>
          <div>
            <h2 className="text-lg font-bold flex items-center gap-2">
              Data Shield
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
            </h2>
            <p className="text-sm text-white/50">
              {isEmpty
                ? "Monitoring agent network activity"
                : `Your agents contacted ${stats!.unique_domains} destination${stats!.unique_domains !== 1 ? "s" : ""} today`}
            </p>
          </div>
        </div>

        {stats && !isEmpty && (
          <div className="flex items-center gap-3">
            {stats.trusted_domains > 0 && (
              <span className="px-2.5 py-1 rounded-full bg-emerald-500/15 text-emerald-400 text-xs font-semibold">
                {stats.trusted_domains} trusted
              </span>
            )}
            {stats.unknown_domains > 0 && (
              <span className="px-2.5 py-1 rounded-full bg-yellow-500/15 text-yellow-400 text-xs font-semibold">
                {stats.unknown_domains} unknown
              </span>
            )}
            {stats.suspicious_domains > 0 && (
              <span className="px-2.5 py-1 rounded-full bg-red-500/15 text-red-400 text-xs font-semibold">
                {stats.suspicious_domains} suspicious
              </span>
            )}
          </div>
        )}
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

      {/* Suspicious Warning */}
      {stats && stats.suspicious_domains > 0 && (
        <div className="glass-card p-4 border border-red-500/30 flex items-center gap-3">
          <AlertTriangle className="w-5 h-5 text-red-400 shrink-0" />
          <p className="text-sm text-red-300">
            {stats.suspicious_domains} suspicious destination{stats.suspicious_domains !== 1 ? "s" : ""} detected. Review below.
          </p>
        </div>
      )}

      {/* Empty State */}
      {isEmpty && (
        <div className="glass-card p-12 text-center">
          <div className="w-16 h-16 rounded-2xl bg-cyan-500/15 flex items-center justify-center mx-auto mb-4">
            <Wifi className="w-8 h-8 text-cyan-400" />
          </div>
          <h3 className="text-xl font-bold mb-2">All quiet</h3>
          <p className="text-white/50 max-w-md mx-auto">
            Data Shield is monitoring your agents' connections. You'll see destinations here when agents make network requests.
          </p>
        </div>
      )}

      {/* Unknown Domain Alerts */}
      {unknownDomains.length > 0 && (
        <div className="space-y-2">
          {unknownDomains.slice(0, 3).map((d) => (
            <div
              key={d.domain}
              className="glass-card p-4 border border-yellow-500/20 flex items-center justify-between"
            >
              <div className="flex items-center gap-3 min-w-0">
                <HelpCircle className="w-4 h-4 text-yellow-400 shrink-0" />
                <p className="text-sm text-white/70 truncate">
                  {d.agents_using[0] ? `${d.agents_using[0]} connected to` : "Agent connected to"}{" "}
                  <span className="font-mono text-white/90">{d.domain}</span>.{" "}
                  <span className="text-white/40">Is this expected?</span>
                </p>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <button
                  onClick={() => handleClassify(d.domain, "trusted")}
                  className="px-3 py-1.5 rounded-lg bg-emerald-500/15 text-emerald-400 text-xs font-medium hover:bg-emerald-500/25 transition-colors"
                >
                  Trust
                </button>
                <button
                  onClick={() => handleClassify(d.domain, "suspicious")}
                  className="px-3 py-1.5 rounded-lg bg-red-500/15 text-red-400 text-xs font-medium hover:bg-red-500/25 transition-colors"
                >
                  Suspicious
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Domain List */}
      {!isEmpty && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold text-white/70 uppercase tracking-wider">
              Destinations ({filteredDomains.length})
            </h3>
            <div className="flex items-center gap-3">
              {/* Search */}
              <div className="relative">
                <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-white/30" />
                <input
                  type="text"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="Filter domains..."
                  className="pl-8 pr-3 py-1.5 rounded-lg glass text-xs text-white/80 placeholder:text-white/30 w-48 focus:outline-none focus:ring-1 focus:ring-white/20"
                />
              </div>
              {/* Sort */}
              <div className="relative">
                <select
                  value={sortMode}
                  onChange={(e) => setSortMode(e.target.value as SortMode)}
                  className="appearance-none pl-3 pr-7 py-1.5 rounded-lg glass text-xs text-white/60 focus:outline-none cursor-pointer"
                >
                  <option value="count">Most contacted</option>
                  <option value="recent">Most recent</option>
                  <option value="risk">Risk level</option>
                </select>
                <ChevronDown className="w-3 h-3 absolute right-2 top-1/2 -translate-y-1/2 text-white/30 pointer-events-none" />
              </div>
            </div>
          </div>

          {filteredDomains.map((domain) => (
            <DomainRow
              key={domain.domain}
              domain={domain}
              onClassify={handleClassify}
            />
          ))}
        </div>
      )}

      {/* Recent Activity Feed */}
      {filteredEvents.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-sm font-semibold text-white/70 uppercase tracking-wider">
            Recent Activity ({filteredEvents.length})
          </h3>
          <div className="glass-card divide-y divide-white/5">
            {filteredEvents.slice(0, 20).map((event) => {
              const risk = RISK_CONFIG[event.risk_level] ?? RISK_CONFIG.unknown;
              return (
                <div
                  key={event.id}
                  className="px-4 py-3 flex items-center gap-3"
                >
                  <span className="text-xs text-white/30 w-16 shrink-0">
                    {timeAgo(event.timestamp)}
                  </span>
                  {event.agent_name && (
                    <span className="px-2 py-0.5 rounded-full glass text-xs text-white/50 shrink-0">
                      {event.agent_name}
                    </span>
                  )}
                  <span className="text-sm text-white/70 truncate flex-1">
                    {event.description}
                  </span>
                  <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium", risk.bg, risk.color)}>
                    {event.risk_level}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

function DomainRow({
  domain,
  onClassify,
}: {
  domain: DomainProfile;
  onClassify: (domain: string, classification: string) => void;
}) {
  const [hovered, setHovered] = useState(false);
  const categoryConfig = CATEGORY_CONFIG[domain.category] ?? CATEGORY_CONFIG.unknown;
  const riskConfig = RISK_CONFIG[domain.risk_level] ?? RISK_CONFIG.unknown;
  const RiskIcon = riskConfig.icon;

  return (
    <div
      className="glass-card p-4 flex items-center gap-4 transition-all duration-200"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {/* Risk icon */}
      <div className={cn("w-8 h-8 rounded-lg flex items-center justify-center shrink-0", riskConfig.bg)}>
        <RiskIcon className={cn("w-4 h-4", riskConfig.color)} />
      </div>

      {/* Domain name */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-mono text-sm font-semibold text-white/90 truncate">
            {domain.domain}
          </span>
          <span className={cn("px-2 py-0.5 rounded-full text-[10px] font-semibold", categoryConfig.bg, categoryConfig.color)}>
            {categoryConfig.label}
          </span>
        </div>
        <div className="flex items-center gap-3 mt-1">
          <span className="text-xs text-white/40">
            {domain.total_events} event{domain.total_events !== 1 ? "s" : ""}
          </span>
          <span className="text-xs text-white/30">{timeAgo(domain.last_seen)}</span>
          {domain.agents_using.map((agent) => (
            <span key={agent} className="px-1.5 py-0.5 rounded text-[10px] glass text-white/40">
              {agent}
            </span>
          ))}
        </div>
      </div>

      {/* Classify buttons for unknown domains */}
      {domain.risk_level === "unknown" && hovered && (
        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={() => onClassify(domain.domain, "trusted")}
            className="px-2.5 py-1 rounded-lg bg-emerald-500/15 text-emerald-400 text-xs font-medium hover:bg-emerald-500/25 transition-colors"
          >
            Trust
          </button>
          <button
            onClick={() => onClassify(domain.domain, "suspicious")}
            className="px-2.5 py-1 rounded-lg bg-red-500/15 text-red-400 text-xs font-medium hover:bg-red-500/25 transition-colors"
          >
            Suspicious
          </button>
        </div>
      )}

      {/* External link for non-unknown */}
      {domain.risk_level !== "unknown" && domain.domain !== "unknown" && (
        <ExternalLink className="w-3.5 h-3.5 text-white/20 shrink-0" />
      )}
    </div>
  );
}
