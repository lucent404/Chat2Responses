import { useState } from "react";
import type React from "react";
import { Copy, Plus, Search, Trash2 } from "lucide-react";
import { deleteApiKey, disableApiKey, enableApiKey, revealApiKey } from "../../api/admin";
import { ConfirmDialog } from "../../components/common/ConfirmDialog";
import { PanelStack } from "../../components/common/PanelStack";
import { StatusBadge } from "../../components/common/StatusBadge";
import { TextMono, TextMuted, TextStrong } from "../../components/common/Text";
import { Button } from "../../components/ui/button";
import { DataTable, type Column } from "../../components/ui/data-table";
import { Input } from "../../components/ui/input";
import { formatDate } from "../../lib/format";
import type { ApiKey, ToastState } from "../../types/admin";
import { CreateApiKeyDialog } from "./CreateApiKeyDialog";
import type { PageState } from "../../types/admin";

type ApiKeysPanelProps = {
  rows: ApiKey[];
  pageState: PageState;
  setPageState: React.Dispatch<React.SetStateAction<PageState>>;
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
};

export function ApiKeysPanel({ rows, pageState, setPageState, refresh, setToast }: ApiKeysPanelProps) {
  const [createOpen, setCreateOpen] = useState(false);
  const [newKey, setNewKey] = useState("");
  const [confirm, setConfirm] = useState<{ type: "delete" | "toggle"; row: ApiKey } | null>(null);

  const toggle = async () => {
    if (!confirm || confirm.type !== "toggle") return;
    if (confirm.row.enabled) {
      await disableApiKey(confirm.row.id);
    } else {
      await enableApiKey(confirm.row.id);
    }
    setConfirm(null);
    await refresh();
  };

  const remove = async () => {
    if (!confirm || confirm.type !== "delete") return;
    await deleteApiKey(confirm.row.id);
    setConfirm(null);
    await refresh();
  };

  const copyKey = async (row: ApiKey) => {
    try {
      const result = await revealApiKey(row.id);
      await navigator.clipboard.writeText(result.key);
      setToast({ type: "ok", message: "完整 key 已复制" });
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    }
  };

  const columns: Column<ApiKey>[] = [
    { key: "name", header: "名称", cell: (row) => <TextStrong>{row.name}</TextStrong> },
    {
      key: "key",
      header: "Key",
      cell: (row) => <TextMono>{row.masked_key || (row.key_recoverable ? "••••" : "旧 key 不可恢复")}</TextMono>
    },
    {
      key: "models",
      header: "模型权限",
      cell: (row) => <TextMuted>{row.models.length === 0 ? "全部模型" : `${row.models.length} 个模型`}</TextMuted>
    },
    { key: "status", header: "状态", cell: (row) => <StatusBadge enabled={row.enabled} /> },
    { key: "created", header: "创建时间", cell: (row) => <TextMuted>{formatDate(row.created_at)}</TextMuted> },
    { key: "used", header: "最后使用", cell: (row) => <TextMuted>{row.last_used_at ? formatDate(row.last_used_at) : "-"}</TextMuted> },
    {
      key: "actions",
      header: "操作",
      width: "minmax(320px, 0.9fr)",
      cell: (row) => (
        <div className="grid w-full grid-cols-3 gap-2 min-[721px]:flex min-[721px]:w-auto min-[721px]:flex-nowrap min-[721px]:items-center">
          <Button className="w-full px-2 min-[721px]:w-[82px]" variant="secondary" size="sm" onClick={() => copyKey(row)} disabled={!row.key_recoverable}>
            <Copy size={14} />
            复制
          </Button>
          <Button className="w-full px-2 min-[721px]:w-[82px]" variant="secondary" size="sm" onClick={() => setConfirm({ type: "toggle", row })}>
            {row.enabled ? "停用" : "启用"}
          </Button>
          <Button className="w-full border-red-200 bg-red-50/80 px-2 text-red-700 hover:bg-red-100 min-[721px]:w-[82px]" variant="outline" size="sm" onClick={() => setConfirm({ type: "delete", row })}>
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
            placeholder="搜索调用方密钥"
          />
        </div>
        <span className="rounded-full border border-slate-300/70 bg-blue-50/80 px-3 py-2 font-mono text-xs text-slate-600">
          {rows.length}/{pageState.total} keys
        </span>
        <Button onClick={() => setCreateOpen(true)}>
          <Plus size={16} />
          创建密钥
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
        empty={pageState.q ? "没有匹配的密钥" : "还没有调用方密钥。"}
      />
      <CreateApiKeyDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        newKey={newKey}
        setNewKey={setNewKey}
        refresh={refresh}
        setToast={setToast}
      />
      <ConfirmDialog
        open={confirm?.type === "toggle"}
        onOpenChange={(next) => !next && setConfirm(null)}
        title={confirm?.row.enabled ? "停用密钥" : "启用密钥"}
        description="状态变更会立即影响调用方请求。"
        confirmText={confirm?.row.enabled ? "停用" : "启用"}
        onConfirm={toggle}
      />
      <ConfirmDialog
        open={confirm?.type === "delete"}
        onOpenChange={(next) => !next && setConfirm(null)}
        title="删除密钥"
        description="删除后这个 key 无法恢复，调用方会立即失效。"
        confirmText="删除"
        destructive
        onConfirm={remove}
      />
    </PanelStack>
  );
}
