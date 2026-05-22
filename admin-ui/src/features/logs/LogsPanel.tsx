import { useMemo } from "react";
import type React from "react";
import { Search, ShieldAlert } from "lucide-react";
import { Metric } from "../../components/common/Metric";
import { PanelStack } from "../../components/common/PanelStack";
import { TextMuted } from "../../components/common/Text";
import { Badge } from "../../components/ui/badge";
import { DataTable, type Column } from "../../components/ui/data-table";
import { Input } from "../../components/ui/input";
import { formatDate } from "../../lib/format";
import type { AppSettings, RequestLog } from "../../types/admin";
import type { PageState } from "../../types/admin";

export function LogsPanel({ rows, pageState, setPageState, settings }: { rows: RequestLog[]; pageState: PageState; setPageState: React.Dispatch<React.SetStateAction<PageState>>; settings: AppSettings }) {
  const totals = useMemo(() => rows.reduce((sum, row) => sum + row.total_tokens, 0), [rows]);
  const columns: Column<RequestLog>[] = [
    { key: "time", header: "时间", cell: (row) => <TextMuted>{formatDate(row.created_at)}</TextMuted> },
    { key: "key", header: "Key", cell: (row) => row.api_key_name || "-" },
    { key: "model", header: "模型", cell: (row) => row.public_model || "-" },
    { key: "upstream", header: "渠道", cell: (row) => row.upstream_name || "-" },
    { key: "status", header: "状态", cell: (row) => <Badge variant={row.status_code >= 400 ? "destructive" : "success"}>{row.status_code}</Badge> },
    { key: "tokens", header: "Token", cell: (row) => row.total_tokens.toLocaleString() },
    { key: "duration", header: "耗时", cell: (row) => `${row.duration_ms} ms` },
    { key: "error", header: "错误", cell: (row) => <TextMuted>{row.error || "-"}</TextMuted> }
  ];

  return (
    <PanelStack>
      {!settings.request_logging_enabled ? (
        <div className="glass-panel-strong flex items-center gap-3 rounded-3xl border-amber-300/75 bg-amber-50/88 p-4 text-sm text-amber-800 shadow-soft">
          <span className="grid h-10 w-10 shrink-0 place-items-center rounded-2xl bg-amber-100 text-amber-700">
            <ShieldAlert size={18} />
          </span>
          <div>
            <p className="font-semibold">日志记录已关闭，仅显示历史记录。</p>
            <p className="mt-1 text-xs text-amber-700/80">开启后，后续请求会重新写入 request_logs。</p>
          </div>
        </div>
      ) : null}
      <div className="grid grid-cols-3 gap-3 max-[760px]:grid-cols-1">
        <Metric label="请求数" value={rows.length.toString()} />
        <Metric label="Token" value={totals.toLocaleString()} />
        <Metric label="错误" value={rows.filter((row) => row.status_code >= 400).length.toString()} tone={rows.some((row) => row.status_code >= 400) ? "danger" : "default"} />
      </div>
      <div className="glass-panel-strong flex items-center justify-between gap-3 rounded-3xl p-3 max-[720px]:grid">
        <div className="relative max-w-[460px] flex-1 max-[720px]:max-w-none">
          <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" size={16} />
          <Input
            className="pl-9"
            value={pageState.q}
            onChange={(event) => setPageState((current) => ({ ...current, page: 1, q: event.target.value }))}
            placeholder="搜索 key、模型、渠道或错误"
          />
        </div>
        <span className="rounded-full border border-slate-300/70 bg-blue-50/80 px-3 py-2 font-mono text-xs text-slate-600">显示 {rows.length} / {pageState.total}</span>
      </div>
      <DataTable
        columns={columns}
        rows={rows}
        pagination={{
          page: pageState.page,
          pageSize: pageState.pageSize,
          total: pageState.total,
          totalPages: pageState.totalPages,
          onPageChange: (page) => setPageState((current) => ({ ...current, page })),
          onPageSizeChange: (pageSize) => setPageState((current) => ({ ...current, page: 1, pageSize }))
        }}
        empty={pageState.q ? "没有匹配的日志" : "暂无请求日志。"}
      />
    </PanelStack>
  );
}
