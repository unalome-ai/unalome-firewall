import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import {
  AlertTriangle,
  Power,
  Shield,
  Pause,
  Play,
  AlertOctagon,
  Bot,
} from "lucide-react";
import { useState } from "react";
import type { Agent, AgentStatus } from "@/types";
import { cn } from "@/lib/utils";

interface KillSwitchProps {
  agents: Agent[];
  onStatusChange: () => void;
}

export function KillSwitch({ agents, onStatusChange: _onStatusChange }: KillSwitchProps) {
  const [globalPaused, setGlobalPaused] = useState(false);
  const [safeMode, setSafeMode] = useState(false);
  const [showConfirm, setShowConfirm] = useState(false);

  const activeAgents = agents.filter((a) => a.status === "Active");

  const handleGlobalPause = () => {
    if (!globalPaused) {
      setShowConfirm(true);
    } else {
      setGlobalPaused(false);
      setSafeMode(false);
    }
  };

  const confirmPause = () => {
    setGlobalPaused(true);
    setShowConfirm(false);
  };

  return (
    <div className="space-y-6">
      {/* Emergency Stop */}
      <Card
        className={cn(
          "glass-card border-2",
          globalPaused ? "border-rose-500/50" : "border-transparent"
        )}
      >
        <CardContent className="pt-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div
                className={cn(
                  "w-16 h-16 rounded-full flex items-center justify-center transition-all",
                  globalPaused
                    ? "bg-rose-500/20 text-rose-400"
                    : "bg-emerald-500/20 text-emerald-400"
                )}
              >
                <Power className="w-8 h-8" />
              </div>
              <div>
                <h3 className="text-lg font-semibold">
                  {globalPaused ? "All Agents Paused" : "Agents Running"}
                </h3>
                <p className="text-sm text-muted-foreground">
                  {globalPaused
                    ? "All agent activity is currently suspended"
                    : `${activeAgents.length} agents currently active`}
                </p>
              </div>
            </div>
            <Button
              size="lg"
              onClick={handleGlobalPause}
              className={cn(
                "gap-2 rounded-full px-8",
                globalPaused
                  ? "bg-emerald-500 hover:bg-emerald-600 text-white"
                  : "action-button border-0"
              )}
            >
              {globalPaused ? (
                <>
                  <Play className="w-4 h-4" />
                  Resume All
                </>
              ) : (
                <>
                  <Pause className="w-4 h-4" />
                  Pause All
                </>
              )}
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Safe Mode Toggle */}
      <Card>
        <CardContent className="pt-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div
                className={cn(
                  "w-12 h-12 rounded-lg flex items-center justify-center",
                  safeMode
                    ? "bg-amber-500/10 text-amber-400"
                    : "bg-muted text-muted-foreground"
                )}
              >
                <Shield className="w-6 h-6" />
              </div>
              <div>
                <h3 className="font-semibold">Safe Mode</h3>
                <p className="text-sm text-muted-foreground">
                  Restrict all agents to read-only operations
                </p>
              </div>
            </div>
            <Switch checked={safeMode} onCheckedChange={setSafeMode} />
          </div>
          {safeMode && (
            <div className="mt-4 p-3 rounded-lg bg-amber-500/10 border border-amber-500/20 text-sm text-amber-400">
              <div className="flex items-center gap-2">
                <AlertTriangle className="w-4 h-4" />
                <span className="font-medium">Safe mode is active</span>
              </div>
              <p className="mt-1 ml-6">
                Agents can only read files. Write operations are blocked.
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Individual Agent Controls */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Bot className="w-5 h-5 text-unalome-purple" />
            Agent Controls
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            {agents.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                No agents discovered yet.
              </div>
            ) : (
              agents.map((agent) => (
                <AgentControlRow
                  key={agent.id}
                  agent={agent}
                  disabled={globalPaused}
                />
              ))
            )}
          </div>
        </CardContent>
      </Card>

      {/* Confirmation Dialog */}
      {showConfirm && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <Card className="max-w-md w-full mx-4 border-rose-500/50">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-rose-500/10 flex items-center justify-center">
                  <AlertOctagon className="w-5 h-5 text-rose-400" />
                </div>
                <div>
                  <CardTitle>Pause All Agents?</CardTitle>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <p className="text-muted-foreground">
                This will immediately suspend all agent activity. Any ongoing
                operations will be interrupted.
              </p>
              <div className="flex gap-3 mt-6">
                <Button
                  variant="outline"
                  className="flex-1"
                  onClick={() => setShowConfirm(false)}
                >
                  Cancel
                </Button>
                <Button
                  variant="destructive"
                  className="flex-1"
                  onClick={confirmPause}
                >
                  Pause All
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}

function AgentControlRow({
  agent,
  disabled,
}: {
  agent: Agent;
  disabled: boolean;
}) {
  const [status, setStatus] = useState<AgentStatus>(agent.status);

  const isActive = status === "Active";

  return (
    <div
      className={cn(
        "flex items-center justify-between p-3 rounded-lg border transition-all",
        disabled && "opacity-50 pointer-events-none"
      )}
    >
      <div className="flex items-center gap-3">
        <div
          className={cn(
            "w-10 h-10 rounded-lg flex items-center justify-center",
            isActive
              ? "bg-emerald-500/10 text-emerald-400"
              : "bg-amber-500/10 text-amber-400"
          )}
        >
          <Bot className="w-5 h-5" />
        </div>
        <div>
          <p className="font-medium">{agent.name}</p>
          <div className="flex items-center gap-2">
            <Badge
              variant="outline"
              className={cn(
                "text-xs",
                isActive
                  ? "bg-emerald-500/10 text-emerald-400 border-emerald-500/20"
                  : "bg-amber-500/10 text-amber-400 border-amber-500/20"
              )}
            >
              {status}
            </Badge>
          </div>
        </div>
      </div>
      <Button
        variant="outline"
        size="sm"
        onClick={() => setStatus(isActive ? "Paused" : "Active")}
      >
        {isActive ? (
          <>
            <Pause className="w-4 h-4 mr-1" />
            Pause
          </>
        ) : (
          <>
            <Play className="w-4 h-4 mr-1" />
            Resume
          </>
        )}
      </Button>
    </div>
  );
}
