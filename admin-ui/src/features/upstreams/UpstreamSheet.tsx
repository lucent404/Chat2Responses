import { type FormEvent, useEffect, useState } from "react";
import { createUpstream, discoverUpstreamModels, fetchUpstreamModels, listLocalUpstreamModels, saveLocalUpstreamModels, updateUpstream } from "../../api/admin";
import { Field } from "../../components/common/Field";
import { SwitchField } from "../../components/common/SwitchField";
import { Button } from "../../components/ui/button";
import { Form } from "../../components/ui/form";
import { Input } from "../../components/ui/input";
import { Sheet, SheetBody, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "../../components/ui/sheet";
import type { ToastState, Upstream, UpstreamModel } from "../../types/admin";

type UpstreamSheetProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  upstream?: Upstream | null;
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
};

export function UpstreamSheet({ open, onOpenChange, upstream, refresh, setToast }: UpstreamSheetProps) {
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [enabled, setEnabled] = useState(true);
  const [modelConfigs, setModelConfigs] = useState<UpstreamModel[]>([]);
  const [expandedModel, setExpandedModel] = useState<string | null>(null);
  const [loadingLocalModels, setLoadingLocalModels] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [busy, setBusy] = useState(false);
  const editing = Boolean(upstream);

  useEffect(() => {
    if (!open) return;
    setName(upstream?.name || "");
    setBaseUrl(upstream?.base_url || "");
    setApiKey("");
    setEnabled(upstream?.enabled ?? true);
    setModelConfigs([]);
    setExpandedModel(null);
  }, [open, upstream]);

  useEffect(() => {
    if (!open || !upstream) return;
    setLoadingLocalModels(true);
    listLocalUpstreamModels(upstream.id)
      .then(setModelConfigs)
      .catch((error) => setToast({ type: "error", message: error instanceof Error ? error.message : String(error) }))
      .finally(() => setLoadingLocalModels(false));
  }, [open, upstream?.id]);

  const syncModels = async () => {
    setSyncing(true);
    try {
      const result = upstream
        ? await fetchUpstreamModels(String(upstream.id))
        : await discoverUpstreamModels({ base_url: baseUrl.trim(), api_key: apiKey });
      setModelConfigs((current) => mergeModelConfigs(current, result.model_configs));
      setToast({ type: "ok", message: `已同步 ${result.models.length} 个渠道模型` });
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    } finally {
      setSyncing(false);
    }
  };

  const create = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    try {
      if (upstream) {
        await updateUpstream(upstream.id, { name: name.trim(), base_url: baseUrl.trim(), api_key: apiKey, enabled });
        await saveLocalUpstreamModels(upstream.id, modelConfigs);
      } else {
        const models = modelConfigs.length > 0 ? modelConfigs.filter((model) => model.enabled).map((model) => model.model) : undefined;
        const input = { name: name.trim(), base_url: baseUrl.trim(), api_key: apiKey, enabled, models, model_configs: modelConfigs };
        await createUpstream(input);
      }
      setName("");
      setBaseUrl("");
      setApiKey("");
      setEnabled(true);
      setModelConfigs([]);
      setExpandedModel(null);
      onOpenChange(false);
      await refresh();
      setToast({ type: "ok", message: upstream ? "渠道已更新" : "渠道已创建" });
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>{editing ? "编辑渠道" : "添加渠道"}</SheetTitle>
          <SheetDescription>配置 OpenAI-compatible Chat Completions 渠道。</SheetDescription>
        </SheetHeader>
        <SheetBody>
          <Form onSubmit={create} className="grid gap-4">
            <Field label="名称">
              <Input value={name} onChange={(event) => setName(event.target.value)} placeholder="DeepSeek" required />
            </Field>
            <Field label="Base URL" hint="末尾是否带 / 均可，服务端会拼接 /models 与 Chat Completions 路径。">
              <Input value={baseUrl} onChange={(event) => setBaseUrl(event.target.value)} placeholder="https://api.example.com/v1" required />
            </Field>
            <Field label="渠道 API Key" hint={editing ? "编辑时留空表示沿用已保存的渠道 key。" : "会使用 CHAT2RESPONSES_SECRET 加密保存。"}>
              <Input value={apiKey} onChange={(event) => setApiKey(event.target.value)} type="password" required={!editing} />
            </Field>
            <Field label="渠道模型" hint={editing ? "这里展示本地模型库存；同步只刷新库存，不会覆盖你已禁用的模型。" : "先同步模型再选择；不改选择时默认启用全部同步模型。"}>
              <div className="grid gap-3">
                <Button type="button" variant="secondary" onClick={syncModels} disabled={syncing || !baseUrl.trim() || (!editing && !apiKey.trim())}>
                  {syncing ? "同步中" : modelConfigs.length > 0 ? "重新同步模型" : "同步渠道模型"}
                </Button>
                {loadingLocalModels ? (
                  <div className="rounded-2xl border border-slate-300/70 bg-white/74 px-3 py-5 text-center text-xs text-muted-foreground">正在加载本地模型库存...</div>
                ) : modelConfigs.length > 0 ? (
                  <UpstreamModelEditor models={modelConfigs} setModels={setModelConfigs} expandedModel={expandedModel} setExpandedModel={setExpandedModel} />
                ) : (
                  <div className="rounded-2xl border border-dashed border-slate-300/70 bg-blue-50/55 px-3 py-5 text-center text-xs text-muted-foreground">
                    还没有本地模型库存。请先同步模型。
                  </div>
                )}
              </div>
            </Field>
            <SwitchField label="启用渠道" description="停用后不会作为可用渠道使用。" checked={enabled} onCheckedChange={setEnabled} />
            <Button disabled={busy}>{busy ? "保存中" : editing ? "保存修改" : "保存渠道"}</Button>
          </Form>
        </SheetBody>
      </SheetContent>
    </Sheet>
  );
}

function mergeModelConfigs(current: UpstreamModel[], incoming: UpstreamModel[]) {
  const byName = new Map(current.map((model) => [model.model, model]));
  for (const model of incoming) {
    const existing = byName.get(model.model);
    byName.set(model.model, existing ? { ...model, enabled: existing.enabled } : model);
  }
  return Array.from(byName.values()).sort((a, b) => a.model.localeCompare(b.model));
}

function UpstreamModelEditor({
  models,
  setModels,
  expandedModel,
  setExpandedModel
}: {
  models: UpstreamModel[];
  setModels: (models: UpstreamModel[]) => void;
  expandedModel: string | null;
  setExpandedModel: (model: string | null) => void;
}) {
  const enabledCount = models.filter((model) => model.enabled).length;
  const update = (name: string, patch: Partial<UpstreamModel>) => {
    setModels(models.map((model) => (model.model === name ? { ...model, ...patch } : model)));
  };
  return (
    <div className="grid gap-2 rounded-2xl border border-slate-300/70 bg-white/74 p-2 shadow-sm backdrop-blur-xl">
      <div className="flex items-center justify-between gap-2 text-[11px] text-muted-foreground">
        <span>已启用 {enabledCount}/{models.length}</span>
        <div className="flex gap-2">
          <Button type="button" variant="secondary" size="sm" onClick={() => setModels(models.map((model) => ({ ...model, enabled: true })))}>
            全选
          </Button>
          <Button type="button" variant="ghost" size="sm" onClick={() => setModels(models.map((model) => ({ ...model, enabled: false })))}>
            清空
          </Button>
        </div>
      </div>
      <div className="grid max-h-72 gap-2 overflow-auto pr-1">
        {models.map((model) => {
          const expanded = expandedModel === model.model;
          return (
            <div key={model.model} className={`rounded-xl border p-2 ${model.enabled ? "border-blue-300 bg-blue-50/85" : "border-slate-200 bg-white/72"}`}>
              <div className="flex min-w-0 items-center justify-between gap-2">
                <button type="button" className="min-w-0 break-all text-left font-mono text-xs font-semibold" onClick={() => update(model.model, { enabled: !model.enabled })}>
                  {model.model}
                </button>
                <div className="flex shrink-0 gap-2">
                  <Button type="button" variant={model.enabled ? "secondary" : "ghost"} size="sm" onClick={() => update(model.model, { enabled: !model.enabled })}>
                    {model.enabled ? "停用" : "启用"}
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => setExpandedModel(expanded ? null : model.model)}>
                    参数
                  </Button>
                </div>
              </div>
              {expanded ? (
                <div className="mt-3 grid grid-cols-2 gap-2 max-[520px]:grid-cols-1">
                  <label className="grid gap-1 text-[11px] font-semibold text-slate-600">
                    上下文窗口
                    <Input type="number" min={1} value={model.context_window} onChange={(event) => update(model.model, { context_window: Number(event.target.value) || 1 })} />
                  </label>
                  <label className="grid gap-1 text-[11px] font-semibold text-slate-600">
                    最大上下文
                    <Input type="number" min={1} value={model.max_context_window} onChange={(event) => update(model.model, { max_context_window: Number(event.target.value) || 1 })} />
                  </label>
                  <label className="flex items-center gap-2 text-xs text-muted-foreground">
                    <input type="checkbox" checked={model.supports_parallel_tool_calls} onChange={(event) => update(model.model, { supports_parallel_tool_calls: event.target.checked })} />
                    Parallel tools
                  </label>
                  <label className="flex items-center gap-2 text-xs text-muted-foreground">
                    <input type="checkbox" checked={model.supports_reasoning_summaries} onChange={(event) => update(model.model, { supports_reasoning_summaries: event.target.checked })} />
                    Reasoning summaries
                  </label>
                  <label className="flex items-center gap-2 text-xs text-muted-foreground">
                    <input type="checkbox" checked={model.supports_image_input} onChange={(event) => update(model.model, { supports_image_input: event.target.checked })} />
                    Image input
                  </label>
                </div>
              ) : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}
