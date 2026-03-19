import { cn } from "@/lib/utils";

interface CircularProgressProps {
  value: number;
  max?: number;
  size?: number;
  strokeWidth?: number;
  className?: string;
  showValue?: boolean;
  color?: "purple" | "cyan" | "rose" | "amber" | "emerald";
}

export function CircularProgress({
  value,
  max = 100,
  size = 120,
  strokeWidth = 8,
  className,
  showValue = true,
  color = "purple",
}: CircularProgressProps) {
  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  const percentage = Math.min(100, Math.max(0, (value / max) * 100));
  const strokeDashoffset = circumference - (percentage / 100) * circumference;

  const colorClasses = {
    purple: "stroke-purple-500",
    cyan: "stroke-cyan-500",
    rose: "stroke-rose-500",
    amber: "stroke-amber-500",
    emerald: "stroke-emerald-500",
  };

  const glowClasses = {
    purple: "drop-shadow-[0_0_10px_rgba(139,92,246,0.5)]",
    cyan: "drop-shadow-[0_0_10px_rgba(6,182,212,0.5)]",
    rose: "drop-shadow-[0_0_10px_rgba(244,63,94,0.5)]",
    amber: "drop-shadow-[0_0_10px_rgba(245,158,11,0.5)]",
    emerald: "drop-shadow-[0_0_10px_rgba(16,185,129,0.5)]",
  };

  return (
    <div className={cn("relative inline-flex items-center justify-center", className)}>
      <svg
        width={size}
        height={size}
        className="circular-progress"
      >
        {/* Track */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          className="circular-progress-track"
        />
        {/* Progress */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          className={cn(
            "circular-progress-fill",
            colorClasses[color],
            glowClasses[color]
          )}
          style={{
            strokeDasharray: circumference,
            strokeDashoffset,
          }}
        />
      </svg>
      {showValue && (
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className="text-2xl font-bold text-white">{Math.round(percentage)}</span>
          <span className="text-xs text-white/60">%</span>
        </div>
      )}
    </div>
  );
}
