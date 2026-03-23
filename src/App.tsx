import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Shield,
  ShieldAlert,
  Activity,
  Lock,
  DollarSign,
  Power,
  LayoutGrid,
  Clock,
  RefreshCw,
  Fingerprint,
  ShieldCheck,
  Wifi,
  BarChart3,
  RotateCcw,
} from "lucide-react";
import { AgentGrid } from "@/components/AgentGrid";
import { TimelineFeed } from "@/components/TimelineFeed";
import { SecurityDashboard } from "@/components/SecurityDashboard";
import { CostTracker } from "@/components/CostTracker";
import { KillSwitch } from "@/components/KillSwitch";
import { PiiGuardian } from "@/components/PiiGuardian";
import { SafetyNet } from "@/components/SafetyNet";
import { DataShield } from "@/components/DataShield";
import { WeeklyReportPage } from "@/components/WeeklyReportPage";
import { FirewallRules } from "@/components/FirewallRules";
import { OnboardingFlow } from "@/components/OnboardingFlow";
import type { Agent, Action, PiiStats, SafetyNetStats, DataShieldStats, FirewallStats } from "@/types";
import { cn } from "@/lib/utils";

const ONBOARDING_KEY = "unalome_onboarding_complete";

type NavItem = {
  id: string;
  label: string;
  icon: React.ElementType;
  bgClass: string;
};

const NAV_ITEMS: NavItem[] = [
  { id: "overview", label: "Overview", icon: LayoutGrid, bgClass: "app-bg-overview" },
  { id: "timeline", label: "Timeline", icon: Clock, bgClass: "app-bg-timeline" },
  { id: "security", label: "Security", icon: Shield, bgClass: "app-bg-security" },
  { id: "costs", label: "Costs", icon: DollarSign, bgClass: "app-bg-costs" },
  { id: "pii", label: "PII Guard", icon: Fingerprint, bgClass: "app-bg-pii" },
  { id: "safety", label: "Safety Net", icon: ShieldCheck, bgClass: "app-bg-safety" },
  { id: "datashield", label: "Data Shield", icon: Wifi, bgClass: "app-bg-datashield" },
  { id: "firewall", label: "Firewall", icon: ShieldAlert, bgClass: "app-bg-firewall" },
  { id: "reports", label: "Reports", icon: BarChart3, bgClass: "app-bg-reports" },
  { id: "control", label: "Control", icon: Power, bgClass: "app-bg-control" },
];

function App() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [actions, setActions] = useState<Action[]>([]);
  const [actionsCount, setActionsCount] = useState(0);
  const [loading, setLoading] = useState(true);
  const [activeView, setActiveView] = useState("overview");
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [selectedAgentId, setSelectedAgentId] = useState<string | undefined>();
  const [showResetConfirm, setShowResetConfirm] = useState(false);

  useEffect(() => {
    initializeApp();

    // Listen for new actions from the backend watcher
    const unlisten = listen("new_actions", () => {
      refreshData();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function initializeApp() {
    try {
      await invoke("initialize_database");
      await refreshData();

      const completed = localStorage.getItem(ONBOARDING_KEY);
      if (!completed) {
        setShowOnboarding(true);
      }
    } catch (error) {
      console.error("Failed to initialize app:", error);
    } finally {
      setLoading(false);
    }
  }

  async function refreshData() {
    try {
      const discoveredAgents = await invoke<Agent[]>("discover_agents");
      setAgents(discoveredAgents);

      const allActions = await invoke<Action[]>("get_all_actions");
      setActions(allActions);

      const count = await invoke<number>("get_actions_count");
      setActionsCount(count);
    } catch (error) {
      console.error("Failed to refresh data:", error);
    }
  }

  function handleOnboardingComplete() {
    localStorage.setItem(ONBOARDING_KEY, "true");
    setShowOnboarding(false);
    refreshData();
  }

  async function handleReset() {
    try {
      await invoke("reset_database");
      setActions([]);
      setActionsCount(0);
      setAgents([]);
      setShowResetConfirm(false);
      await refreshData();
    } catch (error) {
      console.error("Failed to reset database:", error);
    }
  }

  const currentNav = NAV_ITEMS.find((n) => n.id === activeView) || NAV_ITEMS[0];

  if (loading) {
    return (
      <div className="min-h-screen app-bg flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <div className="w-12 h-12 rounded-full border-2 border-fuchsia-500 border-t-transparent animate-spin" />
          <p className="text-muted-foreground">Initializing Unalome...</p>
        </div>
      </div>
    );
  }

  if (showOnboarding) {
    return <OnboardingFlow onComplete={handleOnboardingComplete} />;
  }

  return (
    <div className={cn("min-h-screen flex transition-all duration-700", currentNav.bgClass)}>
      {/* Sidebar — CleanMyMac style */}
      <aside className="sidebar w-[72px] flex flex-col items-center py-6 gap-1 shrink-0 sticky top-0 h-screen">
        {/* Logo */}
        <div className="w-10 h-10 rounded-2xl glow-magenta mb-6 overflow-hidden">
          <img src="/unalome.svg" alt="Unalome" className="w-full h-full" />
        </div>

        {/* Nav Items */}
        <nav className="flex flex-col gap-1 flex-1">
          {NAV_ITEMS.map((item) => {
            const Icon = item.icon;
            const isActive = activeView === item.id;
            return (
              <button
                key={item.id}
                onClick={() => {
                  setActiveView(item.id);
                  if (item.id === "timeline") setSelectedAgentId(undefined);
                }}
                className={cn("sidebar-item", isActive && "active")}
                title={item.label}
              >
                <Icon className="sidebar-icon w-5 h-5" />
                <span className="text-[10px] font-medium leading-none">{item.label}</span>
              </button>
            );
          })}
        </nav>

        {/* Bottom actions */}
        <div className="mt-auto flex flex-col gap-1">
          <button
            onClick={refreshData}
            className="sidebar-item"
            title="Refresh"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
          <button
            onClick={() => setShowResetConfirm(true)}
            className="sidebar-item text-rose-400/60 hover:text-rose-400"
            title="Reset Database"
          >
            <RotateCcw className="w-4 h-4" />
          </button>
        </div>
      </aside>

      {/* Reset Confirmation Modal */}
      {showResetConfirm && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div className="glass-card w-full max-w-md p-6 mx-4 space-y-5">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-rose-500/15 flex items-center justify-center">
                <RotateCcw className="w-5 h-5 text-rose-400" />
              </div>
              <h2 className="text-xl font-bold">Reset Database</h2>
            </div>

            <p className="text-white/60 leading-relaxed">
              This will permanently delete all stored data and start fresh. This action cannot be undone.
            </p>

            <div className="space-y-2 text-sm text-white/50">
              <p className="font-medium text-white/70">You will lose:</p>
              <ul className="space-y-1 ml-4 list-disc">
                <li>All recorded agent actions and timeline history</li>
                <li>PII Guardian findings</li>
                <li>Safety Net file snapshots and backups</li>
                <li>Data Shield domain profiles and events</li>
                <li>Cost tracking data</li>
                <li>Weekly reports</li>
              </ul>
            </div>

            <div className="flex justify-end gap-3 pt-2">
              <button
                onClick={() => setShowResetConfirm(false)}
                className="px-4 py-2 rounded-xl text-sm font-medium text-white/60 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleReset}
                className="px-4 py-2 rounded-xl text-sm font-semibold bg-rose-500/20 text-rose-400 hover:bg-rose-500/30 border border-rose-500/30 transition-colors"
              >
                Reset Everything
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Main Content */}
      <main className="flex-1 min-h-screen overflow-y-auto">
        {/* Top bar — slim title */}
        <header className="titlebar-drag sticky top-0 z-40 flex items-center justify-center py-3">
          <span className="text-sm font-medium text-white/50 titlebar-no-drag">
            {currentNav.label}
          </span>
        </header>

        <div className="px-8 pb-10 max-w-6xl mx-auto">
          {activeView === "overview" && (
            <OverviewView
              agents={agents}
              actions={actions}
              actionsCount={actionsCount}
              onAgentClick={(agent) => {
                setSelectedAgentId(agent.id);
                setActiveView("timeline");
              }}
              onNavigate={setActiveView}
            />
          )}
          {activeView === "timeline" && (
            <TimelineFeed actions={actions} agents={agents} actionsCount={actionsCount} initialAgentFilter={selectedAgentId} />
          )}
          {activeView === "security" && <SecurityDashboard agents={agents} />}
          {activeView === "costs" && <CostTracker actions={actions} />}
          {activeView === "pii" && <PiiGuardian agents={agents} />}
          {activeView === "safety" && <SafetyNet />}
          {activeView === "datashield" && <DataShield />}
          {activeView === "firewall" && <FirewallRules agents={agents} />}
          {activeView === "reports" && <WeeklyReportPage />}
          {activeView === "control" && (
            <KillSwitch agents={agents} onStatusChange={refreshData} />
          )}
        </div>
      </main>
    </div>
  );
}

function OverviewView({
  agents,
  actions,
  actionsCount,
  onAgentClick,
  onNavigate,
}: {
  agents: Agent[];
  actions: Action[];
  actionsCount: number;
  onAgentClick: (agent: Agent) => void;
  onNavigate: (view: string) => void;
}) {
  const [piiStats, setPiiStats] = useState<PiiStats | null>(null);
  const [safetyStats, setSafetyStats] = useState<SafetyNetStats | null>(null);
  const [shieldStats, setShieldStats] = useState<DataShieldStats | null>(null);
  const [firewallStats, setFirewallStats] = useState<FirewallStats | null>(null);

  const loadStats = useCallback(async () => {
    try {
      const [pii, safety, shield, fw] = await Promise.all([
        invoke<PiiStats>("get_pii_stats"),
        invoke<SafetyNetStats>("get_safety_net_stats"),
        invoke<DataShieldStats>("get_data_shield_stats"),
        invoke<FirewallStats>("get_firewall_stats"),
      ]);
      setPiiStats(pii);
      setSafetyStats(safety);
      setShieldStats(shield);
      setFirewallStats(fw);
    } catch (e) {
      console.error("Failed to load overview stats:", e);
    }
  }, []);

  useEffect(() => {
    loadStats();
  }, [loadStats, actions]);

  const activeAgents = agents.filter((a) => a.status === "Active").length;
  const totalCost = actions.reduce((sum, a) => sum + (a.cost?.estimated_cost_usd || 0), 0);

  const piiTotal = piiStats?.total ?? 0;
  const piiCritical = (piiStats?.by_severity["critical"] ?? 0) + (piiStats?.by_severity["high"] ?? 0);

  const safetyTotal = safetyStats?.total_files ?? 0;

  const shieldDomains = shieldStats?.unique_domains ?? 0;
  const shieldSuspicious = shieldStats?.suspicious_domains ?? 0;

  const mcpCount = agents.filter((a) => a.metadata?.has_mcp_servers).length;

  return (
    <div className="space-y-8">
      {/* Hero heading */}
      <div className="text-center pt-4 pb-2">
        <p className="text-xs font-medium text-white/30 uppercase tracking-widest mb-2">
          Unalome Agent Firewall <span className="text-white/20">v0.2.0</span>
        </p>
        <h1 className="text-3xl font-bold mb-2">
          {agents.length > 0
            ? `${activeAgents} agent${activeAgents !== 1 ? "s" : ""} active.`
            : "No agents discovered yet."}
        </h1>
        <p className="text-white/50 text-lg">
          {agents.length > 0
            ? "Monitor activity, costs, and security across all your AI agents."
            : "Install an AI agent like Claude Code or Cursor to get started."}
        </p>
      </div>

      {/* Summary stat cards */}
      {agents.length > 0 && (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <StatCard
            label="Agents"
            value={agents.length.toString()}
            sub={`${activeAgents} active`}
            iconClass="icon-container-purple"
            icon={LayoutGrid}
          />
          <StatCard
            label="Actions"
            value={actionsCount.toLocaleString()}
            sub="total recorded"
            iconClass="icon-container-cyan"
            icon={Activity}
            onClick={() => onNavigate("timeline")}
          />
          <StatCard
            label="Security"
            value={mcpCount > 0 ? `${mcpCount} MCP` : "0"}
            sub={mcpCount > 0 ? `${mcpCount} server${mcpCount !== 1 ? "s" : ""} scannable` : "no MCP servers"}
            iconClass="icon-container-rose"
            icon={Lock}
            onClick={() => onNavigate("security")}
          />
          <StatCard
            label="PII Guard"
            value={piiTotal > 0 ? piiTotal.toString() : "0"}
            sub={piiCritical > 0 ? `${piiCritical} critical/high` : "no PII detected"}
            iconClass="icon-container-rose"
            icon={Fingerprint}
            onClick={() => onNavigate("pii")}
          />
          <StatCard
            label="Safety Net"
            value={safetyTotal.toLocaleString()}
            sub={safetyTotal > 0 ? `file${safetyTotal !== 1 ? "s" : ""} protected` : "no files yet"}
            iconClass="icon-container-cyan"
            icon={ShieldCheck}
            onClick={() => onNavigate("safety")}
          />
          <StatCard
            label="Data Shield"
            value={shieldDomains.toString()}
            sub={shieldSuspicious > 0 ? `${shieldSuspicious} suspicious` : `domain${shieldDomains !== 1 ? "s" : ""} tracked`}
            iconClass="icon-container-emerald"
            icon={Wifi}
            onClick={() => onNavigate("datashield")}
          />
          <StatCard
            label="Firewall"
            value={firewallStats?.active_rules.toString() ?? "0"}
            sub={firewallStats && firewallStats.blocked_today > 0 ? `${firewallStats.blocked_today} blocked today` : "rules active"}
            iconClass="icon-container-rose"
            icon={ShieldAlert}
            onClick={() => onNavigate("firewall")}
          />
          <StatCard
            label="Est. Cost"
            value={`$${totalCost.toFixed(2)}`}
            sub="this session"
            iconClass="icon-container-emerald"
            icon={DollarSign}
            onClick={() => onNavigate("costs")}
          />
        </div>
      )}

      {/* Separator */}
      <div className="flex items-center gap-4">
        <div className="flex-1 h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />
        <span className="text-xs text-white/20 uppercase tracking-widest">Agents</span>
        <div className="flex-1 h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />
      </div>

      {/* Agent Grid */}
      <AgentGrid agents={agents} onAgentClick={onAgentClick} />
    </div>
  );
}

function StatCard({
  label,
  value,
  sub,
  iconClass,
  icon: Icon,
  onClick,
}: {
  label: string;
  value: string;
  sub: string;
  iconClass: string;
  icon: React.ElementType;
  onClick?: () => void;
}) {
  return (
    <div
      className={cn("glass-card p-5", onClick && "cursor-pointer card-lift")}
      onClick={onClick}
    >
      <div className="flex items-center gap-3 mb-3">
        <div className={cn("w-10 h-10 rounded-xl flex items-center justify-center", iconClass)}>
          <Icon className="w-5 h-5" />
        </div>
        <span className="text-sm text-white/50 font-medium">{label}</span>
      </div>
      <p className="text-2xl font-bold">{value}</p>
      <p className="text-xs text-white/40 mt-1">{sub}</p>
    </div>
  );
}

export default App;
