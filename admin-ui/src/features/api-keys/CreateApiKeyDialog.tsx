import { type FormEvent, useEffect, useState } from "react";
import { Copy } from "lucide-react";
import { createApiKey, listAvailableModels } from "../../api/admin";
import { Field } from "../../components/common/Field";
import { ModelMultiSelect } from "../../components/common/ModelMultiSelect";
import { Button } from "../../components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "../../components/ui/dialog";
import { Form } from "../../components/ui/form";
import { Input } from "../../components/ui/input";
import type { AvailableModel, ToastState } from "../../types/admin";

type CreateApiKeyDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  newKey: string;
  setNewKey: (key: string) => void;
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
};

export function CreateApiKeyDialog({ open, onOpenChange, newKey, setNewKey, refresh, setToast }: CreateApiKeyDialogProps) {
  const [name, setName] = useState("");
  const [models, setModels] = useState<AvailableModel[]>([]);
  const [selectedModels, setSelectedModels] = useState<string[]>([]);
  const [loadingModels, setLoadingModels] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open || newKey) return;
    setLoadingModels(true);
    listAvailableModels()
      .then(setModels)
      .catch((error) => setToast({ type: "error", message: error instanceof Error ? error.message : String(error) }))
      .finally(() => setLoadingModels(false));
  }, [open, newKey]);

  const create = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    try {
      const result = await createApiKey({ name, enabled: true, models: selectedModels });
      setNewKey(result.key);
      setName("");
      setSelectedModels([]);
      await refresh();
      setToast({ type: "ok", message: "密钥已创建，可在列表中打码查看并复制" });
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusy(false);
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
              <Button
                variant="secondary"
                onClick={() => {
                  navigator.clipboard.writeText(newKey);
                  setToast({ type: "ok", message: "密钥已复制" });
                }}
              >
                <Copy size={16} />
                复制
              </Button>
              <Button
                onClick={() => {
                  setNewKey("");
                  onOpenChange(false);
                }}
              >
                完成
              </Button>
            </DialogFooter>
          </div>
        ) : (
          <Form onSubmit={create} className="grid gap-4">
            <Field label="调用方名称" hint="建议使用应用名或环境名，便于在日志里追踪来源。">
              <Input value={name} onChange={(event) => setName(event.target.value)} placeholder="调用方应用" required />
            </Field>
            <Field label="可访问模型" hint="不选择表示允许访问全部对外模型。">
              {loadingModels ? (
                <div className="rounded-2xl border border-slate-300/70 bg-white/74 px-3 py-5 text-center text-xs text-muted-foreground">正在加载模型目录...</div>
              ) : (
                <ModelMultiSelect
                  models={models.map((model) => model.id)}
                  selected={selectedModels}
                  onChange={setSelectedModels}
                  emptyLabel="暂无可授权模型；创建后可先使用全部策略"
                />
              )}
            </Field>
            <DialogFooter>
              <Button variant="secondary" type="button" onClick={() => onOpenChange(false)}>
                取消
              </Button>
              <Button disabled={busy}>{busy ? "创建中" : "创建"}</Button>
            </DialogFooter>
          </Form>
        )}
      </DialogContent>
    </Dialog>
  );
}
