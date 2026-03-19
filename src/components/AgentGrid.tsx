import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Bot,
  Code2,
  Activity,
  AlertCircle,
  PauseCircle,
  ChevronRight,
} from "lucide-react";
import { getAgentIconComponent } from "@/components/AgentIcons";
import type { Agent, AgentStatus, AgentType } from "@/types";
import { cn } from "@/lib/utils";

interface AgentGridProps {
  agents: Agent[];
  onAgentClick: (agent: Agent) => void;
}

export function AgentGrid({ agents, onAgentClick }: AgentGridProps) {
  if (agents.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-24 text-center">
        <div className="w-24 h-24 rounded-full bg-gradient-to-br from-fuchsia-500/20 to-purple-500/20 flex items-center justify-center mb-6 animate-float">
          <Bot className="w-12 h-12 text-fuchsia-400" />
        </div>
        <h3 className="text-2xl font-bold mb-3">No Agents Discovered</h3>
        <p className="text-muted-foreground max-w-md leading-relaxed">
          Unalome scans for Claude Code, Claude Desktop, Cursor, Windsurf, and
          other MCP-compatible agents. Install an agent to see it here.
        </p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
      {agents.map((agent) => (
        <AgentCard key={agent.id} agent={agent} onClick={onAgentClick} />
      ))}
    </div>
  );
}

function AgentCard({
  agent,
  onClick,
}: {
  agent: Agent;
  onClick: (agent: Agent) => void;
}) {
  const AgentIcon = getAgentIconComponent(agent.agent_type);
  const statusColor = getStatusColor(agent.status);

  return (
    <Card
      className="glass-card card-lift cursor-pointer group overflow-hidden"
      onClick={() => onClick(agent)}
    >
      <CardContent className="p-6">
        <div className="flex items-start justify-between mb-4">
          <div className="flex items-center gap-4">
            <div
              className={cn(
                "w-14 h-14 rounded-2xl flex items-center justify-center transition-all duration-300",
                statusColor
              )}
            >
              <AgentIcon size={28} />
            </div>
            <div>
              <h3 className="text-lg font-semibold mb-0.5">{agent.name}</h3>
              <p className="text-sm text-muted-foreground">
                {getAgentTypeLabel(agent.agent_type)}
              </p>
            </div>
          </div>
          <StatusBadge status={agent.status} />
        </div>

        <div className="space-y-3">
          {agent.config_path && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Code2 className="w-4 h-4" />
              <span className="truncate">{agent.config_path}</span>
            </div>
          )}
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Activity className="w-4 h-4" />
            <span>Last seen {new Date(agent.last_seen).toLocaleString()}</span>
          </div>
        </div>

        {Boolean(agent.metadata.has_mcp_servers) && (
          <div className="mt-4 pt-4 border-t border-white/5">
            <Badge
              variant="outline"
              className="bg-gradient-to-r from-fuchsia-500/10 to-purple-500/10 text-fuchsia-300 border-fuchsia-500/20"
            >
              MCP Enabled
            </Badge>
          </div>
        )}

        <div className="absolute bottom-6 right-6 opacity-0 group-hover:opacity-100 transition-opacity">
          <ChevronRight className="w-5 h-5 text-muted-foreground" />
        </div>
      </CardContent>
    </Card>
  );
}

function getAgentTypeLabel(type: AgentType): string {
  if (typeof type === "object" && "Other" in type) {
    return type.Other;
  }
  return type;
}

function getStatusColor(status: AgentStatus): string {
  switch (status) {
    case "Active":
      return "icon-container-emerald";
    case "Paused":
      return "icon-container-amber";
    case "Offline":
      return "bg-white/5 text-slate-400";
    default:
      return "bg-white/5 text-muted-foreground";
  }
}

function StatusBadge({ status }: { status: AgentStatus }) {
  const icons = {
    Active: Activity,
    Paused: PauseCircle,
    Offline: AlertCircle,
    Unknown: AlertCircle,
  };
  const Icon = icons[status];

  const variants = {
    Active: "from-emerald-500/20 to-teal-500/20 text-emerald-300 border-emerald-500/30",
    Paused: "from-amber-500/20 to-orange-500/20 text-amber-300 border-amber-500/30",
    Offline: "from-slate-500/20 to-gray-500/20 text-slate-300 border-slate-500/30",
    Unknown: "from-gray-500/20 to-gray-600/20 text-gray-300 border-gray-500/30",
  };

  return (
    <div
      className={cn(
        "flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium border bg-gradient-to-r",
        variants[status]
      )}
    >
      <Icon className="w-3.5 h-3.5" />
      {status}
    </div>
  );
}
