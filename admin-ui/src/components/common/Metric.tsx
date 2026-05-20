import type { ReactNode } from "react";
import { Card, CardContent } from "../ui/card";
import { cn } from "../../lib/utils";

export function Metric({
  label,
  value,
  icon,
  tone = "default"
}: {
  label: string;
  value: string;
  icon?: ReactNode;
  tone?: "default" | "danger";
}) {
  return (
    <Card
      className={cn(
        "group overflow-hidden border-slate-300/55 bg-gradient-to-br from-white via-blue-50/70 to-cyan-50/50 shadow-soft transition-transform duration-200 hover:-translate-y-0.5 hover:shadow-glow",
        tone === "danger" && "border-red-300/80 from-red-50 via-white to-rose-50"
      )}
    >
      <CardContent className="grid gap-2 p-4">
        <span className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.08em] text-slate-600">
          {icon ? <span className="grid h-8 w-8 place-items-center rounded-xl bg-blue-600 text-white shadow-soft transition-transform group-hover:scale-105">{icon}</span> : null}
          {label}
        </span>
        <strong className="font-mono text-2xl font-semibold tracking-normal">{value}</strong>
      </CardContent>
    </Card>
  );
}
