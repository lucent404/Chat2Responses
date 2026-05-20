import type { ReactNode } from "react";

export function PanelStack({ children }: { children: ReactNode }) {
  return <div className="grid gap-5">{children}</div>;
}
