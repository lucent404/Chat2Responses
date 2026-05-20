import type { ReactNode } from "react";

export function TextStrong({ children }: { children: ReactNode }) {
  return <span className="font-medium text-foreground">{children}</span>;
}

export function TextMuted({ children }: { children: ReactNode }) {
  return <span className="text-muted-foreground">{children}</span>;
}

export function TextMono({ children }: { children: ReactNode }) {
  return <span className="break-all font-mono text-xs text-foreground">{children}</span>;
}
