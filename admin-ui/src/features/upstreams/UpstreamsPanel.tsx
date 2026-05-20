import { useState } from "react";
import type React from "react";
import { Boxes, Edit3, Plus, Search, Trash2 } from "lucide-react";
import { deleteUpstream, listLocalUpstreamModels } from "../../api/admin";
import { ConfirmDialog } from "../../components/common/ConfirmDialog";
import { PanelStack } from "../../components/common/PanelStack";
import { StatusBadge } from "../../components/common/StatusBadge";
import { TextMono, TextMuted, TextStrong } from "../../components/common/Text";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "../../components/ui/dialog";
import { Button } from "../../components/ui/button";
import { DataTable, type Column } from "../../components/ui/data-table";
import { Input } from "../../components/ui/input";
import { formatDate } from "../../lib/format";
import type { ToastState, Upstream, UpstreamModel } from "../../types/admin";
import { UpstreamSheet } from "./UpstreamSheet";
import type { PageState } from "../../types/admin";

type UpstreamsPanelProps = {
  rows: Upstream[];
  pageState: PageState;
  setPageState: React.Dispatch<React.SetStateAction<PageState>>;
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
};

export function UpstreamsPanel({ rows, pageState, setPageState, refresh, setToast }: UpstreamsPanelProps) {
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<Upstream | null>(null);
  const [modelDialog, setModelDialog] = useState<{ upstream: Upstream; models: UpstreamModel[] } | null>(null);
  const [deleteId, setDeleteId] = useState<number | null>(null);
  const visibleModels = modelDialog?.models.filter((model) => model.enabled) ?? [];

  const remove = async () => {
    if (!deleteId) return;
    await deleteUpstream(deleteId);
    setDeleteId(null);
    await refresh();
    setToast({ type: "ok", message: "渠道已删除" });
  };

  const showModels = async (row: Upstream) => {
    try {
      const models = await listLocalUpstreamModels(row.id);
      setModelDialog({ upstream: row, models });
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    }
  };

  const columns: Column<Upstream>[] = [
    { key: "name", header: "名称", cell: (row) => <TextStrong>{row.name}</TextStrong>, width: "minmax(150px, 0.9fr)" },
    { key: "base", header: "地址", cell: (row) => <TextMono>{row.base_url}</TextMono>, width: "minmax(260px, 2.2fr)" },
    { key: "models", header: "模型", cell: (row) => <TextMuted>{row.enabled_model_count}/{row.model_count}</TextMuted>, width: "minmax(110px, 0.55fr)" },
    { key: "status", header: "状态", cell: (row) => <StatusBadge enabled={row.enabled} />, width: "minmax(120px, 0.7fr)" },
    { key: "updated", header: "更新时间", cell: (row) => <TextMuted>{formatDate(row.updated_at)}</TextMuted>, width: "minmax(180px, 0.9fr)" },
    {
      key: "actions",
      header: "操作",
      width: "minmax(320px, 0.9fr)",
      cell: (row) => (
        <div className="grid w-full grid-cols-3 gap-2 min-[721px]:flex min-[721px]:w-auto min-[721px]:flex-nowrap min-[721px]:items-center">
          <Button className="w-full px-2 min-[721px]:w-[82px]" variant="secondary" size="sm" onClick={() => showModels(row)}>
            <Boxes size={14} />
            模型
          </Button>
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
            placeholder="搜索渠道名称或地址"
          />
        </div>
        <span className="rounded-full border border-slate-300/70 bg-blue-50/80 px-3 py-2 font-mono text-xs text-slate-600">
          {rows.length}/{pageState.total} channels
        </span>
        <Button onClick={() => {
          setEditing(null);
          setOpen(true);
        }}>
          <Plus size={16} />
          添加渠道
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
        empty={
          pageState.q ? "没有匹配的渠道" : (
            <span>
              还没有渠道。点击 <b>添加渠道</b> 连接服务渠道。
            </span>
          )
        }
      />
      <UpstreamSheet
        open={open}
        onOpenChange={(next) => {
          setOpen(next);
          if (!next) setEditing(null);
        }}
        upstream={editing}
        refresh={refresh}
        setToast={setToast}
      />
      <ConfirmDialog
        open={deleteId !== null}
        onOpenChange={(next) => !next && setDeleteId(null)}
        title="删除渠道"
        description="删除渠道会同时删除依赖它的模型路由。"
        confirmText="删除"
        destructive
        onConfirm={remove}
      />
      <Dialog open={modelDialog !== null} onOpenChange={(next) => !next && setModelDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{modelDialog?.upstream.name || "渠道"} 模型库存</DialogTitle>
            <DialogDescription>当前对外提供的已启用渠道模型。</DialogDescription>
          </DialogHeader>
          <div className="grid max-h-[520px] gap-2 overflow-auto pr-1">
            {visibleModels.length ? visibleModels.map((model) => (
              <div key={model.model} className="rounded-2xl border border-slate-300/70 bg-white/78 p-3 text-sm">
                <div className="flex items-center justify-between gap-3">
                  <TextMono>{model.model}</TextMono>
                  <StatusBadge enabled={model.enabled} />
                </div>
                <div className="mt-2 grid grid-cols-2 gap-2 text-xs text-muted-foreground max-[520px]:grid-cols-1">
                  <span>context: {model.context_window}</span>
                  <span>max: {model.max_context_window}</span>
                  <span>tools: {model.supports_parallel_tool_calls ? "yes" : "no"}</span>
                  <span>reasoning: {model.supports_reasoning_summaries ? "yes" : "no"}</span>
                </div>
              </div>
            )) : <div className="rounded-2xl border border-dashed border-slate-300/70 p-6 text-center text-sm text-muted-foreground">当前没有已启用模型，请在编辑渠道里启用模型。</div>}
          </div>
        </DialogContent>
      </Dialog>
    </PanelStack>
  );
}
