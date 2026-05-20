import { Badge } from "../ui/badge";

export function StatusBadge({ enabled }: { enabled: boolean }) {
  return <Badge variant={enabled ? "success" : "default"}>{enabled ? "Enabled" : "Paused"}</Badge>;
}
