import { type FormEvent, useEffect, useState } from "react";
import { createModelRoute, listLocalUpstreamModels, updateModelRoute } from "../../api/admin";
import { Field } from "../../components/common/Field";
import { SwitchField } from "../../components/common/SwitchField";
import { Button } from "../../components/ui/button";
import { Form } from "../../components/ui/form";
import { Input } from "../../components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../../components/ui/select";
import { Sheet, SheetBody, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "../../components/ui/sheet";
import type { ModelRoute, ToastState, Upstream, UpstreamModel } from "../../types/admin";

type ModelSheetProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  route?: ModelRoute | null;
  upstreams: Upstream[];
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
};

export function ModelSheet({ open, onOpenChange, route, upstreams, refresh, setToast }: ModelSheetProps) {
  const [publicModel, setPublicModel] = useState("");
  const [upstreamId, setUpstreamId] = useState("");
  const [upstreamModel, setUpstreamModel] = useState("");
  const [availableModels, setAvailableModels] = useState<UpstreamModel[]>([]);
  const [loadingModels, setLoadingModels] = useState(false);
  const [contextWindow, setContextWindow] = useState(128000);
  const [maxContextWindow, setMaxContextWindow] = useState(128000);
  const [supportsParallelTools, setSupportsParallelTools] = useState(true);
  const [supportsReasoning, setSupportsReasoning] = useState(false);
  const [enabled, setEnabled] = useState(true);
  const [busy, setBusy] = useState(false);
  const editing = Boolean(route);

  const selectedUpstreamId = upstreamId || (upstreams[0]?.id ? String(upstreams[0].id) : "");
  const selectedUpstreamModel = upstreamModel || (!route ? availableModels[0]?.model || "" : "");

  useEffect(() => {
    if (!open) return;
    setPublicModel(route?.public_model || "");
    setUpstreamId(route?.upstream_id ? String(route.upstream_id) : "");
    setUpstreamModel(route?.upstream_model || "");
    setContextWindow(route?.context_window || 128000);
    setMaxContextWindow(route?.max_context_window || 128000);
    setSupportsParallelTools(route?.supports_parallel_tool_calls ?? true);
    setSupportsReasoning(route?.supports_reasoning_summaries ?? false);
    setEnabled(route?.enabled ?? true);
  }, [open, route]);

  useEffect(() => {
    if (!open) return;
    if (!selectedUpstreamId) {
      setAvailableModels([]);
      setUpstreamModel("");
      return;
    }
    setLoadingModels(true);
    listLocalUpstreamModels(selectedUpstreamId)
      .then((result) => {
        const enabledModels = result.filter((model) => model.enabled || model.model === route?.upstream_model);
        setAvailableModels(enabledModels);
        if (!route && !enabledModels.some((model) => model.model === upstreamModel)) {
          const first = enabledModels[0];
          setUpstreamModel(first?.model || "");
          if (!publicModel && first?.model) setPublicModel(first.model);
          if (first) {
            setContextWindow(first.context_window);
            setMaxContextWindow(first.max_context_window);
            setSupportsParallelTools(first.supports_parallel_tool_calls);
            setSupportsReasoning(first.supports_reasoning_summaries);
          }
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
    setBusy(true);
    try {
      const input = {
        public_model: publicModel.trim(),
        upstream_id: Number(selectedUpstreamId),
        upstream_model: selectedUpstreamModel.trim(),
        context_window: contextWindow,
        max_context_window: maxContextWindow,
        supports_parallel_tool_calls: supportsParallelTools,
        supports_reasoning_summaries: supportsReasoning,
        enabled
      };
      if (route) {
        await updateModelRoute(route.id, input);
      } else {
        await createModelRoute(input);
      }
      setPublicModel("");
      setUpstreamModel("");
      setAvailableModels([]);
      setContextWindow(128000);
      setMaxContextWindow(128000);
      setSupportsParallelTools(true);
      setSupportsReasoning(false);
      setEnabled(true);
      onOpenChange(false);
      await refresh();
      setToast({ type: "ok", message: route ? "模型已更新" : "模型已发布" });
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
          <SheetTitle>{editing ? "编辑模型映射" : "新增模型映射"}</SheetTitle>
          <SheetDescription>同一个对外模型名可以有多个候选，渠道请求会随机分发到可用候选。</SheetDescription>
        </SheetHeader>
        <SheetBody>
          <Form onSubmit={create} className="grid gap-4">
            <Field label="映射后的对外模型名" hint="调用方在 /v1/responses 中看到并使用的模型名。">
              <Input value={publicModel} onChange={(event) => setPublicModel(event.target.value)} placeholder="chat-main" required />
            </Field>
            <Field label="渠道">
              <Select value={selectedUpstreamId} onValueChange={setUpstreamId}>
                <SelectTrigger>
                  <SelectValue placeholder="选择渠道" />
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
            <Field label="渠道真实模型名">
              <Select
                value={selectedUpstreamModel}
                onValueChange={(value) => {
                  setUpstreamModel(value);
                  const selected = availableModels.find((model) => model.model === value);
                  if (selected && !route) {
                    setContextWindow(selected.context_window);
                    setMaxContextWindow(selected.max_context_window);
                    setSupportsParallelTools(selected.supports_parallel_tool_calls);
                    setSupportsReasoning(selected.supports_reasoning_summaries);
                  }
                  if (!publicModel) setPublicModel(value);
                }}
                disabled={loadingModels || availableModels.length === 0}
              >
                <SelectTrigger>
                  {selectedUpstreamModel ? (
                    <span className="min-w-0 truncate">{selectedUpstreamModel}</span>
                  ) : (
                    <SelectValue placeholder={loadingModels ? "正在获取模型..." : "选择渠道模型"} />
                  )}
                </SelectTrigger>
                <SelectContent>
                  {availableModels.map((model) => (
                    <SelectItem key={model.model} value={model.model}>
                      {model.model}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {loadingModels
                  ? "正在读取本地模型库存"
                  : availableModels.length > 0
                    ? `本地可选 ${availableModels.length} 个已启用模型`
                    : "当前渠道没有已启用的本地模型，请先去渠道页同步并启用模型"}
              </p>
            </Field>
            <SwitchField label="Parallel tool calls" description="渠道真实模型支持并行工具调用时打开。" checked={supportsParallelTools} onCheckedChange={setSupportsParallelTools} />
            <SwitchField label="Reasoning summaries" description="渠道支持 reasoning summary 时打开。" checked={supportsReasoning} onCheckedChange={setSupportsReasoning} />
            <SwitchField label="启用映射" description="停用后这个映射候选不再参与随机路由。" checked={enabled} onCheckedChange={setEnabled} />
            <Button disabled={busy || !selectedUpstreamId || !selectedUpstreamModel}>{busy ? "保存中" : editing ? "保存修改" : "保存映射"}</Button>
          </Form>
        </SheetBody>
      </SheetContent>
    </Sheet>
  );
}
