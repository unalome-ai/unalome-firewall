import { Button } from "@/components/ui/button";
import { RefreshCw, Shield, Activity, Zap } from "lucide-react";
import type { DashboardStats } from "@/types";
import { formatCurrency, formatNumber } from "@/lib/utils";
import { cn } from "@/lib/utils";

interface DashboardHeaderProps {
  stats: DashboardStats;
  onRefresh: () => void;
}

export function DashboardHeader({ stats, onRefresh }: DashboardHeaderProps) {
  // Calculate security score (0-100)
  const securityScore = Math.max(0, 100 - stats.highRiskEvents * 10);

  return (
    <header className="sticky top-0 z-50 glass-card border-b-0">
      <div className="container mx-auto px-6 py-5">
        <div className="flex items-center justify-between">
          {/* Logo */}
          <div className="flex items-center gap-4">
            <div className="relative">
              <div className="w-14 h-14 rounded-2xl gradient-purple flex items-center justify-center glow-purple">
                <Shield className="w-7 h-7 text-white" />
              </div>
              {stats.activeAgents > 0 && (
                <div className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-emerald-500 border-2 border-background pulse-live" />
              )}
            </div>
            <div>
              <h1 className="text-2xl font-bold tracking-tight">
                <span className="bg-gradient-to-r from-white to-white/70 bg-clip-text text-transparent">
                  Unalome
                </span>
              </h1>
              <p className="text-sm text-muted-foreground">
                Agent Firewall & Observatory
              </p>
            </div>
          </div>

          {/* Center Stats */}
          <div className="hidden lg:flex items-center gap-8">
            <CircularScore
              value={securityScore}
              label="Security"
              color={securityScore >= 80 ? "emerald" : securityScore >= 60 ? "amber" : "rose"}
            />
            <StatPill
              icon={Activity}
              value={stats.activeAgents}
              label="Active"
              color="cyan"
            />
            <StatPill
              icon={Zap}
              value={formatNumber(stats.totalActions)}
              label="Actions"
              color="purple"
            />
            <CostPill amount={stats.totalCost} />
          </div>

          {/* Refresh Button */}
          <Button
            variant="ghost"
            size="icon"
            onClick={onRefresh}
            className="rounded-xl hover:bg-white/5"
          >
            <RefreshCw className="w-5 h-5 text-muted-foreground" />
          </Button>
        </div>
      </div>
    </header>
  );
}

function CircularScore({
  value,
  label,
  color,
}: {
  value: number;
  label: string;
  color: "emerald" | "amber" | "rose";
}) {
  const circumference = 2 * Math.PI * 20;
  const strokeDashoffset = circumference - (value / 100) * circumference;

  const colorClasses = {
    emerald: "stroke-emerald-500",
    amber: "stroke-amber-500",
    rose: "stroke-rose-500",
  };

  return (
    <div className="flex flex-col items-center gap-1">
      <div className="relative w-14 h-14">
        <svg className="circular-progress w-full h-full" viewBox="0 0 48 48">
          <circle
            className="circular-progress-track"
            cx="24"
            cy="24"
            r="20"
          />
          <circle
            className={cn("circular-progress-fill", colorClasses[color])}
            cx="24"
            cy="24"
            r="20"
            style={{
              strokeDasharray: circumference,
              strokeDashoffset,
            }}
          />
        </svg>
        <div className="absolute inset-0 flex items-center justify-center">
          <span className="text-sm font-bold">{value}</span>
        </div>
      </div>
      <span className="text-xs text-muted-foreground">{label}</span>
    </div>
  );
}

function StatPill({
  icon: Icon,
  value,
  label,
  color,
}: {
  icon: React.ElementType;
  value: string | number;
  label: string;
  color: "purple" | "cyan" | "amber";
}) {
  const colorClasses = {
    purple: "from-purple-500/20 to-violet-500/20 text-purple-300",
    cyan: "from-cyan-500/20 to-blue-500/20 text-cyan-300",
    amber: "from-amber-500/20 to-orange-500/20 text-amber-300",
  };

  return (
    <div className="flex items-center gap-3 px-4 py-2 rounded-2xl bg-gradient-to-r border border-white/5">
      <div className={cn("w-10 h-10 rounded-xl flex items-center justify-center bg-gradient-to-br", colorClasses[color])}>
        <Icon className="w-5 h-5" />
      </div>
      <div>
        <p className="text-lg font-bold leading-none">{value}</p>
        <p className="text-xs text-muted-foreground">{label}</p>
      </div>
    </div>
  );
}

function CostPill({ amount }: { amount: number }) {
  const isHigh = amount > 10;

  return (
    <div
      className={cn(
        "flex items-center gap-3 px-4 py-2 rounded-2xl border",
        isHigh
          ? "bg-gradient-to-r from-rose-500/10 to-orange-500/10 border-rose-500/20"
          : "bg-gradient-to-r from-emerald-500/10 to-teal-500/10 border-emerald-500/20"
      )}
    >
      <div className={cn(
        "w-10 h-10 rounded-xl flex items-center justify-center bg-gradient-to-br",
        isHigh
          ? "from-rose-500/30 to-orange-500/20 text-rose-300"
          : "from-emerald-500/30 to-teal-500/20 text-emerald-300"
      )}>
        <span className="text-lg font-bold">$</span>
      </div>
      <div>
        <p className={cn("text-lg font-bold leading-none", isHigh ? "text-rose-300" : "text-emerald-300")}>
          {formatCurrency(amount)}
        </p>
        <p className="text-xs text-muted-foreground">Est. Cost</p>
      </div>
    </div>
  );
}
