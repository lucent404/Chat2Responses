import { useState } from "react";
import type React from "react";
import { Edit3, Plus, Search, Trash2 } from "lucide-react";
import { deleteModelRoute } from "../../api/admin";
import { ConfirmDialog } from "../../components/common/ConfirmDialog";
import { PanelStack } from "../../components/common/PanelStack";
import { StatusBadge } from "../../components/common/StatusBadge";
import { TextMono, TextStrong } from "../../components/common/Text";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { DataTable, type Column } from "../../components/ui/data-table";
import { Input } from "../../components/ui/input";
import type { ModelRoute, ToastState, Upstream } from "../../types/admin";
import { ModelSheet } from "./ModelSheet";
import type { PageState } from "../../types/admin";

type ModelsPanelProps = {
  rows: ModelRoute[];
  pageState: PageState;
  setPageState: React.Dispatch<React.SetStateAction<PageState>>;
  upstreams: Upstream[];
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
};

export function ModelsPanel({ rows, pageState, setPageState, upstreams, refresh, setToast }: ModelsPanelProps) {
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<ModelRoute | null>(null);
  const [deleteId, setDeleteId] = useState<number | null>(null);

  const remove = async () => {
    if (!deleteId) return;
    await deleteModelRoute(deleteId);
    setDeleteId(null);
    await refresh();
    setToast({ type: "ok", message: "模型映射已删除" });
  };

  const columns: Column<ModelRoute>[] = [
    { key: "public", header: "映射模型", cell: (row) => <TextStrong>{row.public_model}</TextStrong> },
    { key: "upstream", header: "渠道", cell: (row) => row.upstream_name },
    { key: "real", header: "真实模型", cell: (row) => <TextMono>{row.upstream_model}</TextMono> },
    { key: "cap", header: "能力", cell: (row) => <Badge variant="outline">{row.supports_reasoning_summaries ? "tools, reasoning" : "tools"}</Badge> },
    { key: "status", header: "映射状态", cell: (row) => <StatusBadge enabled={row.enabled} /> },
    {
      key: "actions",
      header: "",
      width: "minmax(220px, 0.7fr)",
      cell: (row) => (
        <div className="grid w-full grid-cols-2 gap-2 min-[721px]:flex min-[721px]:w-auto min-[721px]:flex-nowrap min-[721px]:items-center">
          <Button className="w-full px-2 min-[721px]:w-[82px]" variant="secondary" size="sm" onClick={() => {
            setEditing(row);
            setOpen(true);
          }}>
            <Edit3 size={14} />
            编辑
          </Button>
          <Button className="w-full border-red-200 bg-red-50/80 px-2 text-red-700 hover:bg-red-100 min-[721px]:w-[82px]" variant="outline" size="sm" onClick={() => setDeleteId(row.id)}>
            <Trash2 size={14} />
            删除
          </Button>
        </div>
      )
    }
  ];

  return (
    <PanelStack>
      <div className="glass-panel-strong flex items-center justify-between gap-3 rounded-3xl p-3 max-[720px]:grid">
        <div className="relative max-w-[420px] flex-1 max-[720px]:max-w-none">
          <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" size={16} />
          <Input
            className="pl-9"
            value={pageState.q}
            onChange={(event) => setPageState((current) => ({ ...current, page: 1, q: event.target.value }))}
            placeholder="搜索映射模型、渠道或真实模型"
          />
        </div>
        <span className="rounded-full border border-slate-300/70 bg-blue-50/80 px-3 py-2 font-mono text-xs text-slate-600">
          {rows.length}/{pageState.total} mappings
        </span>
        <Button onClick={() => {
          setEditing(null);
          setOpen(true);
        }} disabled={upstreams.length === 0}>
          <Plus size={16} />
          新增映射
        </Button>
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
        empty={upstreams.length === 0 ? "先添加渠道并同步模型，才能新增映射。" : pageState.q ? "没有匹配的模型映射" : "还没有模型映射；未映射时会直接提供已启用的渠道模型。"}
      />
      <ModelSheet
        open={open}
        onOpenChange={(next) => {
          setOpen(next);
          if (!next) setEditing(null);
        }}
        route={editing}
        upstreams={upstreams}
        refresh={refresh}
        setToast={setToast}
      />
      <ConfirmDialog
        open={deleteId !== null}
        onOpenChange={(next) => !next && setDeleteId(null)}
        title="删除模型映射"
        description="删除后这个映射候选不再参与路由；同名渠道模型仍会按渠道模型配置直接提供。"
        confirmText="删除"
        destructive
        onConfirm={remove}
      />
    </PanelStack>
  );
}
