import { Waypoints } from "lucide-react";

export function BrandBlock({ subtitle, compact }: { subtitle: string; compact?: boolean }) {
  return (
    <div className={`flex items-center gap-3 text-primary ${compact ? "min-w-[190px]" : ""}`}>
      <span className="relative grid h-11 w-11 place-items-center overflow-hidden rounded-2xl bg-primary text-primary-foreground shadow-glow">
        <span className="absolute inset-0 bg-[radial-gradient(circle_at_35%_20%,rgba(255,255,255,0.7),transparent_36%)]" />
        <Waypoints size={compact ? 22 : 24} />
      </span>
      <div>
        <h1 className={compact ? "text-lg font-semibold tracking-normal text-foreground" : "text-[22px] font-semibold tracking-normal text-foreground"}>Chat2Responses</h1>
        <p className="mt-0.5 font-mono text-xs text-muted-foreground">{subtitle}</p>
      </div>
    </div>
  );
}
