import { useMemo, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import {
  DollarSign,
  AlertCircle,
  Clock,
  Calendar,
} from "lucide-react";
import type { Action } from "@/types";
import { formatCurrency, formatNumber } from "@/lib/utils";
import { cn } from "@/lib/utils";

interface CostTrackerProps {
  actions: Action[];
}

type TimePeriod = "hour" | "day" | "week" | "month" | "all";

const TIME_PERIODS: { value: TimePeriod; label: string }[] = [
  { value: "hour", label: "1h" },
  { value: "day", label: "24h" },
  { value: "week", label: "7d" },
  { value: "month", label: "30d" },
  { value: "all", label: "All" },
];

function getTimeCutoff(period: TimePeriod): number {
  const now = Date.now();
  switch (period) {
    case "hour":
      return now - 60 * 60 * 1000;
    case "day":
      return now - 24 * 60 * 60 * 1000;
    case "week":
      return now - 7 * 24 * 60 * 60 * 1000;
    case "month":
      return now - 30 * 24 * 60 * 60 * 1000;
    case "all":
      return 0;
  }
}

export function CostTracker({ actions }: CostTrackerProps) {
  const [timePeriod, setTimePeriod] = useState<TimePeriod>("week");

  const { periodCost, agentCosts, totalInput, totalOutput, actionCount } = useMemo(() => {
    const cutoff = getTimeCutoff(timePeriod);

    let cost = 0;
    let inputTokens = 0;
    let outputTokens = 0;
    let count = 0;
    const agentMap: Record<string, number> = {};

    for (const action of actions) {
      if (!action.cost) continue;
      const ts = new Date(action.timestamp).getTime();
      if (cutoff > 0 && ts < cutoff) continue;

      const c = action.cost.estimated_cost_usd;
      cost += c;
      count++;
      agentMap[action.agent_id] = (agentMap[action.agent_id] || 0) + c;
      inputTokens += action.cost.tokens_input;
      outputTokens += action.cost.tokens_output;
    }

    const sortedAgents = Object.entries(agentMap)
      .map(([name, agentCost]) => ({
        name,
        cost: agentCost,
        percentage: cost > 0 ? Math.round((agentCost / cost) * 100) : 0,
      }))
      .sort((a, b) => b.cost - a.cost);

    return {
      periodCost: cost,
      agentCosts: sortedAgents,
      totalInput: inputTokens,
      totalOutput: outputTokens,
      actionCount: count,
    };
  }, [actions, timePeriod]);

  const BUDGET_KEY = "unalome_monthly_budget";
  const [budgetLimit, setBudgetLimit] = useState<number>(() => {
    const saved = localStorage.getItem(BUDGET_KEY);
    return saved ? parseFloat(saved) : 50;
  });
  const [editingBudget, setEditingBudget] = useState(false);
  const [budgetInput, setBudgetInput] = useState(budgetLimit.toString());

  function saveBudget() {
    const val = parseFloat(budgetInput);
    if (!isNaN(val) && val >= 0) {
      setBudgetLimit(val);
      localStorage.setItem(BUDGET_KEY, val.toString());
    }
    setEditingBudget(false);
  }

  const monthCost = useMemo(() => {
    const cutoff = getTimeCutoff("month");
    return actions.reduce((sum, a) => {
      if (!a.cost) return sum;
      const ts = new Date(a.timestamp).getTime();
      if (cutoff > 0 && ts < cutoff) return sum;
      return sum + a.cost.estimated_cost_usd;
    }, 0);
  }, [actions]);
  const budgetUsed = budgetLimit > 0 ? (monthCost / budgetLimit) * 100 : 0;

  return (
    <div className="space-y-6">
      {/* Header with segmented filter */}
      <div className="glass-card p-5 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="icon-container-emerald w-10 h-10 rounded-xl flex items-center justify-center">
            <DollarSign className="w-5 h-5" />
          </div>
          <div>
            <h2 className="text-lg font-semibold">Cost Tracker</h2>
            <p className="text-xs text-white/40">{actionCount} actions with cost data</p>
          </div>
        </div>
        <div className="flex items-center rounded-lg bg-white/5 p-0.5">
          {TIME_PERIODS.map((tp) => (
            <button
              key={tp.value}
              onClick={() => setTimePeriod(tp.value)}
              className={cn(
                "px-3 py-1.5 rounded-md text-xs font-medium transition-all",
                timePeriod === tp.value
                  ? "bg-white/10 text-white shadow-sm"
                  : "text-white/40 hover:text-white/60"
              )}
            >
              {tp.label}
            </button>
          ))}
        </div>
      </div>

      {/* Cost Overview Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <CostCard title="Period Total" amount={periodCost} icon={DollarSign} />
        <CostCard title="Input Tokens" amount={totalInput} icon={Clock} isTokens />
        <CostCard title="Output Tokens" amount={totalOutput} icon={Calendar} isTokens />
      </div>

      {/* Budget Progress */}
      <Card className="glass-card">
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle className="flex items-center gap-2">
              <DollarSign className="w-5 h-5 text-fuchsia-400" />
              Monthly Budget
            </CardTitle>
            {editingBudget ? (
              <div className="flex items-center gap-2">
                <span className="text-sm text-white/40">$</span>
                <input
                  type="number"
                  min="0"
                  step="10"
                  value={budgetInput}
                  onChange={(e) => setBudgetInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") saveBudget();
                    if (e.key === "Escape") setEditingBudget(false);
                  }}
                  onBlur={saveBudget}
                  autoFocus
                  className="w-20 px-2 py-1 rounded-md bg-white/5 border border-white/10 text-sm text-white outline-none focus:ring-1 focus:ring-fuchsia-500/50"
                />
              </div>
            ) : (
              <Badge
                variant="outline"
                className="cursor-pointer hover:bg-white/5 transition-colors"
                onClick={() => {
                  setBudgetInput(budgetLimit.toString());
                  setEditingBudget(true);
                }}
              >
                {formatCurrency(budgetLimit)} limit ✎
              </Badge>
            )}
          </div>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Used (30d)</span>
              <span className="font-medium">
                {formatCurrency(monthCost)} ({budgetUsed.toFixed(1)}%)
              </span>
            </div>
            <Progress value={Math.min(budgetUsed, 100)} />
            {budgetUsed > 80 && (
              <div className="flex items-center gap-2 text-amber-400 text-sm">
                <AlertCircle className="w-4 h-4" />
                <span>Approaching budget limit</span>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Agent Cost Breakdown */}
      <Card className="glass-card">
        <CardHeader>
          <CardTitle>Cost by Agent</CardTitle>
        </CardHeader>
        <CardContent>
          {agentCosts.length === 0 ? (
            <p className="text-center text-muted-foreground py-6">
              No cost data yet. Costs will appear as your agents work.
            </p>
          ) : (
            <div className="space-y-4">
              {agentCosts.map((agent) => (
                <div key={agent.name} className="space-y-2">
                  <div className="flex items-center justify-between text-sm">
                    <span>{agent.name}</span>
                    <span className="font-medium">
                      {formatCurrency(agent.cost)} ({agent.percentage}%)
                    </span>
                  </div>
                  <Progress value={agent.percentage} />
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function CostCard({
  title,
  amount,
  icon: Icon,
  alert,
  isTokens,
}: {
  title: string;
  amount: number;
  icon: React.ElementType;
  alert?: boolean;
  isTokens?: boolean;
}) {
  return (
    <Card className={`glass-card ${alert ? "border-amber-500/50" : ""}`}>
      <CardContent className="pt-6">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-muted-foreground">{title}</p>
            <p className="text-2xl font-bold mt-1">
              {isTokens ? formatNumber(amount) : formatCurrency(amount)}
            </p>
          </div>
          <div
            className={`w-10 h-10 rounded-lg flex items-center justify-center ${
              alert ? "bg-amber-500/10 text-amber-400" : "bg-muted"
            }`}
          >
            <Icon className="w-5 h-5" />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
