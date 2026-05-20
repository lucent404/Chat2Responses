import { RefreshCcw } from "lucide-react";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import type { ToastState } from "../../types/admin";
import type { Tab } from "../../app/navigation";
import { tabSubtitle, tabTitle } from "../../app/navigation";

type PanelHeaderProps = {
  tab: Tab;
  refresh: () => Promise<void>;
  refreshing?: boolean;
  setToast: (toast: ToastState) => void;
};

export function PanelHeader({ tab, refresh, refreshing, setToast }: PanelHeaderProps) {
  return (
    <header className="glass-panel-strong mb-6 flex items-center justify-between gap-4 rounded-3xl px-5 py-4 max-[720px]:grid">
      <div className="min-w-0">
        <div className="mb-2 flex flex-wrap items-center gap-2">
          <Badge variant="outline">Admin console</Badge>
          <span className="font-mono text-xs text-muted-foreground">/ {tab}</span>
        </div>
        <h2 className="text-3xl font-semibold tracking-normal max-[620px]:text-2xl">{tabTitle(tab)}</h2>
        <p className="mt-1 max-w-[760px] text-sm text-muted-foreground">{tabSubtitle(tab)}</p>
      </div>
      <Button
        variant="secondary"
        disabled={refreshing}
        onClick={() => refresh().catch((error) => setToast({ type: "error", message: error.message }))}
      >
        <RefreshCcw size={16} className={refreshing ? "animate-spin" : ""} />
        {refreshing ? "刷新中" : "刷新"}
      </Button>
    </header>
  );
}
