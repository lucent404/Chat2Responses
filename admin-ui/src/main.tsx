import React, { FormEvent, useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { Activity, Copy, Database, KeyRound, LogOut, Plus, RefreshCcw, Route, Server, Trash2 } from "lucide-react";
import { Badge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import { Card, CardContent } from "./components/ui/card";
import { DataTable, type Column } from "./components/ui/data-table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from "./components/ui/dialog";
import { Form } from "./components/ui/form";
import { Input } from "./components/ui/input";
import { Label } from "./components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./components/ui/select";
import { Sheet, SheetBody, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "./components/ui/sheet";
import { Switch } from "./components/ui/switch";
import { Toast, ToastProvider, ToastTitle, ToastViewport } from "./components/ui/toast";
import "./styles.css";

type AdminStatus = { initialized: boolean; user: null | { id: number; username: string } };
type Upstream = { id: number; name: string; base_url: string; enabled: boolean; created_at: string; updated_at: string };
type ModelRoute = {
  id: number;
  public_model: string;
  upstream_id: number;
  upstream_name: string;
  upstream_model: string;
  context_window: number;
  max_context_window: number;
  supports_parallel_tool_calls: boolean;
  supports_reasoning_summaries: boolean;
  enabled: boolean;
};
type ApiKey = { id: number; name: string; enabled: boolean; created_at: string; last_used_at: string | null };
type UpstreamModels = { data: unknown[]; models: string[] };
type RequestLog = {
  id: number;
  api_key_name: string | null;
  public_model: string | null;
  upstream_name: string | null;
  upstream_model: string | null;
  status_code: number;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  error: string | null;
  duration_ms: number;
  created_at: string;
};
type ToastState = { type: "ok" | "error"; message: string } | null;
type Tab = "upstreams" | "models" | "keys" | "logs";

const api = async <T,>(path: string, init?: RequestInit): Promise<T> => {
  const res = await fetch(path, {
    credentials: "include",
    headers: { "Content-Type": "application/json", ...(init?.headers || {}) },
    ...init
  });
  if (!res.ok) {
    const text = await res.text();
    try {
      const parsed = JSON.parse(text) as { message?: string };
      throw new Error(parsed.message || text || `${res.status} ${res.statusText}`);
    } catch (error) {
      if (error instanceof SyntaxError) {
        throw new Error(text || `${res.status} ${res.statusText}`);
      }
      throw error;
    }
  }
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
};

function App() {
  const [status, setStatus] = useState<AdminStatus | null>(null);
  const [toast, setToast] = useState<ToastState>(null);
  const refreshStatus = async () => setStatus(await api<AdminStatus>("/admin/api/status"));

  useEffect(() => {
    refreshStatus().catch((error) => setToast({ type: "error", message: error.message }));
  }, []);

  if (!status) return <ShellLoading />;

  return (
    <ToastProvider>
      {!status.initialized ? (
        <AuthScreen mode="init" onDone={refreshStatus} setToast={setToast} />
      ) : !status.user ? (
        <AuthScreen mode="login" onDone={refreshStatus} setToast={setToast} />
      ) : (
        <Dashboard user={status.user.username} onLogout={refreshStatus} setToast={setToast} />
      )}
      <ToastMessage toast={toast} onOpenChange={(open) => !open && setToast(null)} />
      <ToastViewport />
    </ToastProvider>
  );
}

function ShellLoading() {
  return <div className="grid min-h-screen place-items-center bg-background text-sm text-muted-foreground">Loading</div>;
}

function AuthScreen({
  mode,
  onDone,
  setToast
}: {
  mode: "init" | "login";
  onDone: () => Promise<void>;
  setToast: (toast: ToastState) => void;
}) {
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    try {
      await api(mode === "init" ? "/admin/api/init" : "/admin/api/login", {
        method: "POST",
        body: JSON.stringify({ username, password })
      });
      await onDone();
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="grid min-h-screen place-items-center bg-background p-6">
      <Card className="w-full max-w-[420px] shadow-panel">
        <CardContent className="grid gap-5 p-6">
          <BrandBlock subtitle={mode === "init" ? "初始化管理员账号" : "登录管理后台"} />
          <Form onSubmit={submit} className="grid gap-4">
            <Field label="用户名">
              <Input value={username} onChange={(event) => setUsername(event.target.value)} autoComplete="username" />
            </Field>
            <Field label="密码">
              <Input
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                type="password"
                autoComplete={mode === "init" ? "new-password" : "current-password"}
              />
            </Field>
            <Button disabled={busy}>{busy ? "处理中" : mode === "init" ? "创建管理员" : "登录"}</Button>
          </Form>
        </CardContent>
      </Card>
    </main>
  );
}

function Dashboard({
  user,
  onLogout,
  setToast
}: {
  user: string;
  onLogout: () => Promise<void>;
  setToast: (toast: ToastState) => void;
}) {
  const [tab, setTab] = useState<Tab>("upstreams");
  const [upstreams, setUpstreams] = useState<Upstream[]>([]);
  const [models, setModels] = useState<ModelRoute[]>([]);
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [logs, setLogs] = useState<RequestLog[]>([]);

  const refresh = async () => {
    const [nextUpstreams, nextModels, nextKeys, nextLogs] = await Promise.all([
      api<Upstream[]>("/admin/api/upstreams"),
      api<ModelRoute[]>("/admin/api/models"),
      api<ApiKey[]>("/admin/api/keys"),
      api<RequestLog[]>("/admin/api/logs?limit=100")
    ]);
    setUpstreams(nextUpstreams);
    setModels(nextModels);
    setKeys(nextKeys);
    setLogs(nextLogs);
  };

  useEffect(() => {
    refresh().catch((error) => setToast({ type: "error", message: error.message }));
  }, []);

  const logout = async () => {
    await api("/admin/api/logout", { method: "POST" });
    await onLogout();
  };

  return (
    <main className="grid min-h-screen grid-cols-[240px_1fr] bg-background max-[860px]:grid-cols-1">
      <aside className="flex flex-col gap-2 border-r border-border bg-card p-5 max-[860px]:sticky max-[860px]:top-0 max-[860px]:z-20 max-[860px]:flex-row max-[860px]:overflow-x-auto max-[860px]:border-b max-[860px]:border-r-0">
        <BrandBlock subtitle={user} compact />
        <NavButton active={tab === "upstreams"} icon={<Server size={18} />} label="上游" onClick={() => setTab("upstreams")} />
        <NavButton active={tab === "models"} icon={<Route size={18} />} label="模型" onClick={() => setTab("models")} />
        <NavButton active={tab === "keys"} icon={<KeyRound size={18} />} label="密钥" onClick={() => setTab("keys")} />
        <NavButton active={tab === "logs"} icon={<Activity size={18} />} label="日志" onClick={() => setTab("logs")} />
        <Button variant="ghost" className="mt-auto justify-start max-[860px]:mt-0" onClick={logout}>
          <LogOut size={18} />
          退出
        </Button>
      </aside>
      <section className="min-w-0 p-7 max-[860px]:p-4">
        <PanelHeader tab={tab} refresh={refresh} setToast={setToast} />
        {tab === "upstreams" && <UpstreamsPanel rows={upstreams} refresh={refresh} setToast={setToast} />}
        {tab === "models" && <ModelsPanel rows={models} upstreams={upstreams} refresh={refresh} setToast={setToast} />}
        {tab === "keys" && <KeysPanel rows={keys} refresh={refresh} setToast={setToast} />}
        {tab === "logs" && <LogsPanel rows={logs} />}
      </section>
    </main>
  );
}

function PanelHeader({
  tab,
  refresh,
  setToast
}: {
  tab: Tab;
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
}) {
  return (
    <header className="mb-5 flex items-center justify-between gap-4">
      <div>
        <h2 className="text-2xl font-semibold tracking-normal">{tabTitle(tab)}</h2>
        <p className="mt-1 text-sm text-muted-foreground">{tabSubtitle(tab)}</p>
      </div>
      <Button variant="secondary" onClick={() => refresh().catch((error) => setToast({ type: "error", message: error.message }))}>
        <RefreshCcw size={16} />
        刷新
      </Button>
    </header>
  );
}

function UpstreamsPanel({
  rows,
  refresh,
  setToast
}: {
  rows: Upstream[];
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
}) {
  const [open, setOpen] = useState(false);
  const [deleteId, setDeleteId] = useState<number | null>(null);

  const remove = async () => {
    if (!deleteId) return;
    await api(`/admin/api/upstreams/${deleteId}`, { method: "DELETE" });
    setDeleteId(null);
    await refresh();
    setToast({ type: "ok", message: "上游已删除" });
  };

  const columns: Column<Upstream>[] = [
    { key: "name", header: "名称", cell: (row) => <TextStrong>{row.name}</TextStrong> },
    { key: "base", header: "地址", cell: (row) => <TextMono>{row.base_url}</TextMono>, className: "col-span-2" },
    { key: "status", header: "状态", cell: (row) => <StatusBadge enabled={row.enabled} /> },
    { key: "updated", header: "更新时间", cell: (row) => <TextMuted>{formatDate(row.updated_at)}</TextMuted> },
    {
      key: "actions",
      header: "",
      cell: (row) => (
        <Button variant="ghost" size="icon" onClick={() => setDeleteId(row.id)} aria-label="删除上游">
          <Trash2 size={16} />
        </Button>
      )
    }
  ];

  return (
    <PanelStack>
      <div className="flex justify-end">
        <Button onClick={() => setOpen(true)}>
          <Plus size={16} />
          添加上游
        </Button>
      </div>
      <DataTable columns={columns} rows={rows} />
      <UpstreamSheet open={open} onOpenChange={setOpen} refresh={refresh} setToast={setToast} />
      <ConfirmDialog
        open={deleteId !== null}
        onOpenChange={(next) => !next && setDeleteId(null)}
        title="删除上游"
        description="删除上游会同时删除依赖它的模型路由。"
        confirmText="删除"
        destructive
        onConfirm={remove}
      />
    </PanelStack>
  );
}

function UpstreamSheet({
  open,
  onOpenChange,
  refresh,
  setToast
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
}) {
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [enabled, setEnabled] = useState(true);

  const create = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    try {
      await api("/admin/api/upstreams", {
        method: "POST",
        body: JSON.stringify({ name, base_url: baseUrl, api_key: apiKey, enabled })
      });
      setName("");
      setBaseUrl("");
      setApiKey("");
      setEnabled(true);
      onOpenChange(false);
      await refresh();
      setToast({ type: "ok", message: "上游已创建" });
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    }
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>添加上游</SheetTitle>
          <SheetDescription>配置 OpenAI-compatible Chat Completions 上游。</SheetDescription>
        </SheetHeader>
        <SheetBody>
          <Form onSubmit={create} className="grid gap-4">
            <Field label="名称">
              <Input value={name} onChange={(event) => setName(event.target.value)} placeholder="DeepSeek" />
            </Field>
            <Field label="Base URL">
              <Input value={baseUrl} onChange={(event) => setBaseUrl(event.target.value)} placeholder="https://api.example.com/v1" />
            </Field>
            <Field label="Provider API Key">
              <Input value={apiKey} onChange={(event) => setApiKey(event.target.value)} type="password" />
            </Field>
            <SwitchField label="启用" checked={enabled} onCheckedChange={setEnabled} />
            <Button>保存上游</Button>
          </Form>
        </SheetBody>
      </SheetContent>
    </Sheet>
  );
}

function ModelsPanel({
  rows,
  upstreams,
  refresh,
  setToast
}: {
  rows: ModelRoute[];
  upstreams: Upstream[];
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
}) {
  const [open, setOpen] = useState(false);
  const [deleteId, setDeleteId] = useState<number | null>(null);

  const remove = async () => {
    if (!deleteId) return;
    await api(`/admin/api/models/${deleteId}`, { method: "DELETE" });
    setDeleteId(null);
    await refresh();
    setToast({ type: "ok", message: "模型已删除" });
  };

  const columns: Column<ModelRoute>[] = [
    { key: "public", header: "对外模型", cell: (row) => <TextStrong>{row.public_model}</TextStrong> },
    { key: "upstream", header: "上游", cell: (row) => row.upstream_name },
    { key: "real", header: "真实模型", cell: (row) => <TextMono>{row.upstream_model}</TextMono> },
    { key: "cap", header: "能力", cell: (row) => <Badge variant="outline">{row.supports_reasoning_summaries ? "tools, reasoning" : "tools"}</Badge> },
    { key: "status", header: "状态", cell: (row) => <StatusBadge enabled={row.enabled} /> },
    {
      key: "actions",
      header: "",
      cell: (row) => (
        <Button variant="ghost" size="icon" onClick={() => setDeleteId(row.id)} aria-label="删除模型">
          <Trash2 size={16} />
        </Button>
      )
    }
  ];

  return (
    <PanelStack>
      <div className="flex justify-end">
        <Button onClick={() => setOpen(true)} disabled={upstreams.length === 0}>
          <Plus size={16} />
          发布模型
        </Button>
      </div>
      <DataTable columns={columns} rows={rows} />
      <ModelSheet open={open} onOpenChange={setOpen} upstreams={upstreams} refresh={refresh} setToast={setToast} />
      <ConfirmDialog
        open={deleteId !== null}
        onOpenChange={(next) => !next && setDeleteId(null)}
        title="删除模型"
        description="删除后调用方将无法继续使用这个对外模型名。"
        confirmText="删除"
        destructive
        onConfirm={remove}
      />
    </PanelStack>
  );
}

function ModelSheet({
  open,
  onOpenChange,
  upstreams,
  refresh,
  setToast
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  upstreams: Upstream[];
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
}) {
  const [publicModel, setPublicModel] = useState("");
  const [upstreamId, setUpstreamId] = useState("");
  const [upstreamModel, setUpstreamModel] = useState("");
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [loadingModels, setLoadingModels] = useState(false);
  const [supportsReasoning, setSupportsReasoning] = useState(false);

  const selectedUpstreamId = upstreamId || (upstreams[0]?.id ? String(upstreams[0].id) : "");

  useEffect(() => {
    if (!open) return;
    if (!selectedUpstreamId) {
      setAvailableModels([]);
      setUpstreamModel("");
      return;
    }
    setLoadingModels(true);
    api<UpstreamModels>(`/admin/api/upstreams/${selectedUpstreamId}/models`)
      .then((result) => {
        setAvailableModels(result.models);
        if (!result.models.includes(upstreamModel)) {
          const first = result.models[0] || "";
          setUpstreamModel(first);
          if (!publicModel && first) setPublicModel(first);
        }
      })
      .catch((error) => {
        setAvailableModels([]);
        setUpstreamModel("");
        setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
      })
      .finally(() => setLoadingModels(false));
  }, [open, selectedUpstreamId]);

  const create = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    try {
      await api("/admin/api/models", {
        method: "POST",
        body: JSON.stringify({
          public_model: publicModel,
          upstream_id: Number(selectedUpstreamId),
          upstream_model: upstreamModel,
          context_window: 128000,
          max_context_window: 128000,
          supports_parallel_tool_calls: true,
          supports_reasoning_summaries: supportsReasoning,
          enabled: true
        })
      });
      setPublicModel("");
      setUpstreamModel("");
      setAvailableModels([]);
      setSupportsReasoning(false);
      onOpenChange(false);
      await refresh();
      setToast({ type: "ok", message: "模型已发布" });
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    }
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>发布模型</SheetTitle>
          <SheetDescription>把对外模型名映射到一个上游真实模型。</SheetDescription>
        </SheetHeader>
        <SheetBody>
          <Form onSubmit={create} className="grid gap-4">
            <Field label="对外模型名">
              <Input value={publicModel} onChange={(event) => setPublicModel(event.target.value)} placeholder="chat-main" />
            </Field>
            <Field label="上游">
              <Select value={selectedUpstreamId} onValueChange={setUpstreamId}>
                <SelectTrigger>
                  <SelectValue placeholder="选择上游" />
                </SelectTrigger>
                <SelectContent>
                  {upstreams.map((upstream) => (
                    <SelectItem key={upstream.id} value={String(upstream.id)}>
                      {upstream.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field label="上游真实模型名">
              <Select
                value={upstreamModel}
                onValueChange={(value) => {
                  setUpstreamModel(value);
                  if (!publicModel) setPublicModel(value);
                }}
                disabled={loadingModels || availableModels.length === 0}
              >
                <SelectTrigger>
                  <SelectValue placeholder={loadingModels ? "正在获取模型..." : "选择上游模型"} />
                </SelectTrigger>
                <SelectContent>
                  {availableModels.map((model) => (
                    <SelectItem key={model} value={model}>
                      {model}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {loadingModels
                  ? "正在通过上游 /models 接口获取模型列表"
                  : availableModels.length > 0
                    ? `已获取 ${availableModels.length} 个模型`
                    : "当前上游没有返回可选模型"}
              </p>
            </Field>
            <SwitchField label="Reasoning summaries" checked={supportsReasoning} onCheckedChange={setSupportsReasoning} />
            <Button>发布模型</Button>
          </Form>
        </SheetBody>
      </SheetContent>
    </Sheet>
  );
}

function KeysPanel({
  rows,
  refresh,
  setToast
}: {
  rows: ApiKey[];
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
}) {
  const [createOpen, setCreateOpen] = useState(false);
  const [newKey, setNewKey] = useState("");
  const [confirm, setConfirm] = useState<{ type: "delete" | "toggle"; row: ApiKey } | null>(null);

  const toggle = async () => {
    if (!confirm || confirm.type !== "toggle") return;
    await api(`/admin/api/keys/${confirm.row.id}/${confirm.row.enabled ? "disable" : "enable"}`, { method: "POST" });
    setConfirm(null);
    await refresh();
  };

  const remove = async () => {
    if (!confirm || confirm.type !== "delete") return;
    await api(`/admin/api/keys/${confirm.row.id}`, { method: "DELETE" });
    setConfirm(null);
    await refresh();
  };

  const columns: Column<ApiKey>[] = [
    { key: "name", header: "名称", cell: (row) => <TextStrong>{row.name}</TextStrong> },
    { key: "status", header: "状态", cell: (row) => <StatusBadge enabled={row.enabled} /> },
    { key: "created", header: "创建时间", cell: (row) => <TextMuted>{formatDate(row.created_at)}</TextMuted> },
    { key: "used", header: "最后使用", cell: (row) => <TextMuted>{row.last_used_at ? formatDate(row.last_used_at) : "-"}</TextMuted> },
    {
      key: "actions",
      header: "操作",
      cell: (row) => (
        <div className="flex gap-2">
          <Button variant="secondary" size="sm" onClick={() => setConfirm({ type: "toggle", row })}>
            {row.enabled ? "停用" : "启用"}
          </Button>
          <Button variant="ghost" size="icon" onClick={() => setConfirm({ type: "delete", row })} aria-label="删除密钥">
            <Trash2 size={16} />
          </Button>
        </div>
      )
    }
  ];

  return (
    <PanelStack>
      <div className="flex justify-end">
        <Button onClick={() => setCreateOpen(true)}>
          <Plus size={16} />
          创建密钥
        </Button>
      </div>
      <DataTable columns={columns} rows={rows} />
      <CreateKeyDialog
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

function CreateKeyDialog({
  open,
  onOpenChange,
  newKey,
  setNewKey,
  refresh,
  setToast
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  newKey: string;
  setNewKey: (key: string) => void;
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
}) {
  const [name, setName] = useState("");

  const create = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    try {
      const result = await api<ApiKey & { key: string }>("/admin/api/keys", {
        method: "POST",
        body: JSON.stringify({ name, enabled: true })
      });
      setNewKey(result.key);
      setName("");
      await refresh();
      setToast({ type: "ok", message: "密钥已创建，明文只显示一次" });
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>创建分发密钥</DialogTitle>
          <DialogDescription>调用方使用这个服务 key 访问 `/v1/responses`。</DialogDescription>
        </DialogHeader>
        {newKey ? (
          <div className="grid gap-3">
            <div className="rounded-md border border-border bg-muted p-3 font-mono text-xs break-all">{newKey}</div>
            <DialogFooter>
              <Button variant="secondary" onClick={() => navigator.clipboard.writeText(newKey)}>
                <Copy size={16} />
                复制
              </Button>
              <Button onClick={() => { setNewKey(""); onOpenChange(false); }}>完成</Button>
            </DialogFooter>
          </div>
        ) : (
          <Form onSubmit={create} className="grid gap-4">
            <Field label="调用方名称">
              <Input value={name} onChange={(event) => setName(event.target.value)} placeholder="调用方应用" />
            </Field>
            <DialogFooter>
              <Button variant="secondary" type="button" onClick={() => onOpenChange(false)}>
                取消
              </Button>
              <Button>创建</Button>
            </DialogFooter>
          </Form>
        )}
      </DialogContent>
    </Dialog>
  );
}

function LogsPanel({ rows }: { rows: RequestLog[] }) {
  const totals = useMemo(() => rows.reduce((sum, row) => sum + row.total_tokens, 0), [rows]);
  const columns: Column<RequestLog>[] = [
    { key: "time", header: "时间", cell: (row) => <TextMuted>{formatDate(row.created_at)}</TextMuted> },
    { key: "key", header: "Key", cell: (row) => row.api_key_name || "-" },
    { key: "model", header: "模型", cell: (row) => row.public_model || "-" },
    { key: "upstream", header: "上游", cell: (row) => row.upstream_name || "-" },
    { key: "status", header: "状态", cell: (row) => <Badge variant={row.status_code >= 400 ? "destructive" : "success"}>{row.status_code}</Badge> },
    { key: "tokens", header: "Token", cell: (row) => row.total_tokens.toLocaleString() },
    { key: "duration", header: "耗时", cell: (row) => `${row.duration_ms} ms` },
    { key: "error", header: "错误", cell: (row) => <TextMuted>{row.error || "-"}</TextMuted> }
  ];

  return (
    <PanelStack>
      <div className="grid grid-cols-3 gap-3 max-[760px]:grid-cols-1">
        <Metric label="请求数" value={rows.length.toString()} />
        <Metric label="Token" value={totals.toLocaleString()} />
        <Metric label="错误" value={rows.filter((row) => row.status_code >= 400).length.toString()} />
      </div>
      <DataTable columns={columns} rows={rows} />
    </PanelStack>
  );
}

function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmText,
  destructive,
  onConfirm
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title?: string;
  description?: string;
  confirmText?: string;
  destructive?: boolean;
  onConfirm: () => Promise<void>;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>取消</Button>
          <Button variant={destructive ? "destructive" : "default"} onClick={onConfirm}>{confirmText || "确认"}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid gap-2">
      <Label>{label}</Label>
      {children}
    </div>
  );
}

function SwitchField({
  label,
  checked,
  onCheckedChange
}: {
  label: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between rounded-md border border-border p-3">
      <Label>{label}</Label>
      <Switch checked={checked} onCheckedChange={onCheckedChange} />
    </div>
  );
}

function BrandBlock({ subtitle, compact }: { subtitle: string; compact?: boolean }) {
  return (
    <div className={`flex items-center gap-3 text-primary ${compact ? "mb-6 min-w-[170px] max-[860px]:mb-0" : ""}`}>
      <Database size={compact ? 24 : 28} />
      <div>
        <h1 className={compact ? "text-lg font-semibold tracking-normal" : "text-[22px] font-semibold tracking-normal"}>Chat2Responses</h1>
        <p className="mt-0.5 text-xs text-muted-foreground">{subtitle}</p>
      </div>
    </div>
  );
}

function NavButton({ active, icon, label, onClick }: { active: boolean; icon: React.ReactNode; label: string; onClick: () => void }) {
  return (
    <Button variant={active ? "secondary" : "ghost"} className="justify-start" onClick={onClick}>
      {icon}
      {label}
    </Button>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <Card>
      <CardContent className="grid gap-1 p-4">
        <span className="text-xs font-semibold text-muted-foreground">{label}</span>
        <strong className="text-2xl font-semibold tracking-normal">{value}</strong>
      </CardContent>
    </Card>
  );
}

function PanelStack({ children }: { children: React.ReactNode }) {
  return <div className="grid gap-4">{children}</div>;
}

function StatusBadge({ enabled }: { enabled: boolean }) {
  return <Badge variant={enabled ? "success" : "default"}>{enabled ? "启用" : "停用"}</Badge>;
}

function TextStrong({ children }: { children: React.ReactNode }) {
  return <span className="font-medium text-foreground">{children}</span>;
}

function TextMuted({ children }: { children: React.ReactNode }) {
  return <span className="text-muted-foreground">{children}</span>;
}

function TextMono({ children }: { children: React.ReactNode }) {
  return <span className="break-all font-mono text-xs text-foreground">{children}</span>;
}

function ToastMessage({ toast, onOpenChange }: { toast: ToastState; onOpenChange: (open: boolean) => void }) {
  if (!toast) return null;
  return (
    <Toast open={Boolean(toast)} onOpenChange={onOpenChange} tone={toast.type}>
      <ToastTitle>{toast.message}</ToastTitle>
    </Toast>
  );
}

function tabTitle(tab: Tab) {
  return { upstreams: "上游配置", models: "全局模型目录", keys: "分发密钥", logs: "请求日志" }[tab];
}

function tabSubtitle(tab: Tab) {
  return {
    upstreams: "保存 provider 地址和上游 key",
    models: "把对外模型名映射到某个上游真实模型",
    keys: "调用方使用这里生成的新 key 访问代理",
    logs: "查看请求、状态和 token usage"
  }[tab];
}

function formatDate(value: string) {
  return new Date(value).toLocaleString();
}

createRoot(document.getElementById("root")!).render(<App />);
