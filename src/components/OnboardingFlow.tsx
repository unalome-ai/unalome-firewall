import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Shield,
  Eye,
  Zap,
  DollarSign,
  ChevronRight,
  ChevronLeft,
  Check,
  Bot,
  Activity,
  Lock,
  Sparkles,
  Fingerprint,
  HardDrive,
  Wifi,
  Power,
  FileBarChart,
} from "lucide-react";
import { cn } from "@/lib/utils";

interface OnboardingFlowProps {
  onComplete: () => void;
}

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const [step, setStep] = useState(0);
  const [discoveredAgents, setDiscoveredAgents] = useState<string[]>([]);

  const steps = [
    {
      title: "Welcome to Unalome Agent Firewall",
      description: "Your personal agent firewall and observatory",
      content: <WelcomeStep />,
    },
    {
      title: "Powerful Features",
      description: "Everything you need to monitor your AI agents",
      content: <FeaturesStep />,
    },
    {
      title: "Discover Agents",
      description: "Scanning for installed agents...",
      content: (
        <DiscoveryStep
          discovered={discoveredAgents}
          onDiscovery={setDiscoveredAgents}
        />
      ),
    },
    {
      title: "Ready to Go",
      description: "Start monitoring your agents",
      content: <CompleteStep discoveredCount={discoveredAgents.length} />,
    },
  ];

  const currentStep = steps[step];

  return (
    <div className="min-h-screen bg-background flex items-center justify-center p-4">
      <Card className="w-full max-w-3xl glass-card overflow-hidden">
        {/* Progress bar */}
        <div className="h-1 bg-white/5">
          <div
            className="h-full gradient-purple transition-all duration-500"
            style={{ width: `${((step + 1) / steps.length) * 100}%` }}
          />
        </div>

        <CardHeader className="pb-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <img
                src="/unalome.svg"
                alt="Unalome Agent Firewall"
                className="w-12 h-12 rounded-2xl"
              />
              <div>
                <CardTitle className="text-xl">{currentStep.title}</CardTitle>
                <p className="text-sm text-muted-foreground">
                  {currentStep.description}
                </p>
              </div>
            </div>
            <div className="text-sm text-muted-foreground">
              {step + 1} / {steps.length}
            </div>
          </div>
        </CardHeader>

        <CardContent>
          <div className="min-h-[400px] py-4">{currentStep.content}</div>

          <div className="flex justify-between mt-6 pt-6 border-t border-white/5">
            <Button
              variant="ghost"
              onClick={() => setStep(step - 1)}
              disabled={step === 0}
              className="gap-2"
            >
              <ChevronLeft className="w-4 h-4" />
              Back
            </Button>

            {step < steps.length - 1 ? (
              <Button onClick={() => setStep(step + 1)} className="gap-2 gradient-purple">
                Next
                <ChevronRight className="w-4 h-4" />
              </Button>
            ) : (
              <Button onClick={onComplete} className="gap-2 gradient-emerald">
                Get Started
                <Check className="w-4 h-4" />
              </Button>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function WelcomeStep() {
  return (
    <div className="text-center space-y-8 py-8">
      <div className="relative">
        <img
          src="/unalome.svg"
          alt="Unalome Agent Firewall"
          className="w-32 h-32 mx-auto rounded-3xl glow-purple animate-float"
        />
        <div className="absolute -bottom-2 left-1/2 -translate-x-1/2">
          <Badge variant="outline" className="bg-emerald-500/20 text-emerald-300 border-emerald-500/30">
            v0.1.0
          </Badge>
        </div>
      </div>

      <div>
        <h2 className="text-3xl font-bold mb-4">
          <span className="bg-gradient-to-r from-purple-400 via-cyan-400 to-purple-400 bg-clip-text text-transparent">
            See What Your Agents See
          </span>
        </h2>
        <p className="text-muted-foreground max-w-md mx-auto text-lg leading-relaxed">
          Unalome Agent Firewall sits between you and your AI agents, giving you complete
          visibility and control over everything they do on your behalf.
        </p>
      </div>

      <div className="flex justify-center gap-6">
        <FeatureTag icon={Shield} label="Secure" />
        <FeatureTag icon={Eye} label="Transparent" />
        <FeatureTag icon={Zap} label="Fast" />
      </div>
    </div>
  );
}

function FeatureTag({ icon: Icon, label }: { icon: React.ElementType; label: string }) {
  return (
    <div className="flex flex-col items-center gap-2">
      <div className="w-12 h-12 rounded-2xl bg-white/5 flex items-center justify-center border border-white/10">
        <Icon className="w-5 h-5 text-purple-400" />
      </div>
      <span className="text-sm text-muted-foreground">{label}</span>
    </div>
  );
}

function FeaturesStep() {
  const features = [
    {
      icon: Activity,
      title: "Action Timeline",
      description: "Real-time feed of every tool call, file access, and API request",
      color: "from-cyan-500/20 to-blue-500/20",
      iconColor: "text-cyan-400",
    },
    {
      icon: Lock,
      title: "Security Dashboard",
      description: "Security score and automatic MCP server risk assessment",
      color: "from-purple-500/20 to-violet-500/20",
      iconColor: "text-purple-400",
    },
    {
      icon: DollarSign,
      title: "Cost Tracker",
      description: "Token usage, cost per agent, and monthly budget limits",
      color: "from-emerald-500/20 to-teal-500/20",
      iconColor: "text-emerald-400",
    },
    {
      icon: Fingerprint,
      title: "PII Guardian",
      description: "Detect API keys, passwords, SSNs, and sensitive data in real-time",
      color: "from-rose-500/20 to-amber-500/20",
      iconColor: "text-rose-400",
    },
    {
      icon: HardDrive,
      title: "Safety Net",
      description: "Auto-backup files before agents modify them. One-click restore",
      color: "from-violet-500/20 to-indigo-500/20",
      iconColor: "text-violet-400",
    },
    {
      icon: Wifi,
      title: "Data Shield",
      description: "Monitor outbound connections and classify domains by trust level",
      color: "from-cyan-500/20 to-teal-500/20",
      iconColor: "text-cyan-400",
    },
    {
      icon: Power,
      title: "Kill Switch",
      description: "Emergency pause, safe mode, and per-agent controls",
      color: "from-rose-500/20 to-orange-500/20",
      iconColor: "text-rose-400",
    },
    {
      icon: FileBarChart,
      title: "Weekly Reports",
      description: "Auto-generated summaries with scores, costs, and trends",
      color: "from-amber-500/20 to-yellow-500/20",
      iconColor: "text-amber-400",
    },
  ];

  return (
    <div className="grid grid-cols-2 gap-3 overflow-y-auto max-h-[400px] pr-1">
      {features.map((feature) => {
        const Icon = feature.icon;
        return (
          <div
            key={feature.title}
            className="p-4 rounded-2xl glass-card card-lift group"
          >
            <div
              className={cn(
                "w-10 h-10 rounded-xl flex items-center justify-center mb-3 bg-gradient-to-br",
                feature.color
              )}
            >
              <Icon className={cn("w-5 h-5", feature.iconColor)} />
            </div>
            <h3 className="font-semibold mb-1">{feature.title}</h3>
            <p className="text-xs text-muted-foreground leading-relaxed">{feature.description}</p>
          </div>
        );
      })}
    </div>
  );
}

function DiscoveryStep({
  discovered,
  onDiscovery,
}: {
  discovered: string[];
  onDiscovery: (agents: string[]) => void;
}) {
  const [scanning, setScanning] = useState(false);

  const handleScan = async () => {
    setScanning(true);
    try {
      const agents = await invoke<Array<{ name: string }>>("discover_agents");
      onDiscovery(agents.map((a) => a.name));
    } catch (e) {
      console.error("Scan failed:", e);
      onDiscovery([]);
    } finally {
      setScanning(false);
    }
  };

  if (discovered.length === 0) {
    return (
      <div className="text-center py-12">
        <div className="w-24 h-24 mx-auto rounded-full bg-gradient-to-br from-purple-500/20 to-cyan-500/20 flex items-center justify-center mb-6">
          <Bot className="w-12 h-12 text-purple-400" />
        </div>
        <Button
          size="lg"
          onClick={handleScan}
          disabled={scanning}
          className="gap-2 gradient-purple px-8"
        >
          {scanning ? (
            <>
              <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
              Scanning...
            </>
          ) : (
            <>
              <Sparkles className="w-5 h-5" />
              Scan for Agents
            </>
          )}
        </Button>
        <p className="text-sm text-muted-foreground mt-6 max-w-sm mx-auto">
          We'll look for Claude Code, Cursor, Windsurf, OpenClaw, Codex, and other MCP-compatible agents
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 py-4 h-[400px]">
      <div className="flex items-center justify-center gap-2 text-emerald-400 text-lg shrink-0">
        <div className="w-8 h-8 rounded-full bg-emerald-500/20 flex items-center justify-center">
          <Check className="w-4 h-4" />
        </div>
        <span className="font-semibold">
          Found {discovered.length} agent{discovered.length !== 1 ? "s" : ""}
        </span>
      </div>

      <div className="space-y-3 max-w-md mx-auto w-full overflow-y-auto flex-1 pr-1">
        {discovered.map((agent) => (
          <div
            key={agent}
            className="flex items-center gap-4 p-4 rounded-2xl glass-card border-emerald-500/20"
          >
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-emerald-500/20 to-teal-500/20 flex items-center justify-center shrink-0">
              <Bot className="w-6 h-6 text-emerald-400" />
            </div>
            <div className="flex-1 min-w-0">
              <p className="font-semibold text-lg truncate">{agent}</p>
              <p className="text-sm text-muted-foreground">Connected and ready</p>
            </div>
            <Badge
              variant="outline"
              className="bg-emerald-500/10 text-emerald-300 border-emerald-500/20 shrink-0"
            >
              Active
            </Badge>
          </div>
        ))}
      </div>

      <Button
        variant="outline"
        size="sm"
        onClick={handleScan}
        disabled={scanning}
        className="w-full max-w-md mx-auto shrink-0"
      >
        {scanning ? "Scanning..." : "Scan Again"}
      </Button>
    </div>
  );
}

function CompleteStep({ discoveredCount }: { discoveredCount: number }) {
  return (
    <div className="text-center space-y-8 py-4">
      <div className="w-24 h-24 mx-auto rounded-full bg-gradient-to-br from-emerald-500/20 to-teal-500/20 flex items-center justify-center">
        <Check className="w-12 h-12 text-emerald-400" />
      </div>

      <div>
        <h2 className="text-3xl font-bold mb-3">You're All Set!</h2>
        <p className="text-muted-foreground max-w-md mx-auto">
          {discoveredCount > 0 ? (
            <>
              We've connected to {discoveredCount} agent
              {discoveredCount !== 1 ? "s" : ""}. You can now monitor their
              activity in real-time.
            </>
          ) : (
            <>
              You can add agents anytime. Unalome Agent Firewall will automatically detect them
              when you install compatible tools.
            </>
          )}
        </p>
      </div>

      <div className="grid grid-cols-2 gap-4 max-w-lg mx-auto">
        <TipCard
          icon={Activity}
          title="Timeline"
          description="View agent activity"
        />
        <TipCard icon={Lock} title="Security" description="Check risk scores" />
        <TipCard
          icon={DollarSign}
          title="Costs"
          description="Monitor spending"
        />
        <TipCard icon={Power} title="Kill Switch" description="Emergency stop" />
      </div>
    </div>
  );
}

function TipCard({
  icon: Icon,
  title,
  description,
}: {
  icon: React.ElementType;
  title: string;
  description: string;
}) {
  return (
    <div className="p-4 rounded-xl glass-card text-left">
      <div className="flex items-center gap-3 mb-2">
        <Icon className="w-5 h-5 text-purple-400" />
        <span className="font-semibold">{title}</span>
      </div>
      <p className="text-sm text-muted-foreground">{description}</p>
    </div>
  );
}
