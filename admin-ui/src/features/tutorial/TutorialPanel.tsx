import { useState } from "react";
import { CheckCircle2, Copy, Download, FileJson2, FolderInput, RotateCcw, Settings2 } from "lucide-react";
import { downloadCodexCatalog } from "../../api/admin";
import { Metric } from "../../components/common/Metric";
import { PanelStack } from "../../components/common/PanelStack";
import { TextMuted } from "../../components/common/Text";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import type { CodexCatalogStatus, ToastState } from "../../types/admin";

type TutorialPanelProps = {
  catalogStatus: CodexCatalogStatus | null;
  refresh: () => Promise<void>;
  setToast: (toast: ToastState) => void;
};

const genericConfigSnippet = 'model_catalog_json = "~/.codex/model-catalog.json"';

export function TutorialPanel({ catalogStatus, refresh, setToast }: TutorialPanelProps) {
  const [busy, setBusy] = useState(false);

  const download = async () => {
    setBusy(true);
    try {
      const blob = await downloadCodexCatalog();
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = "model-catalog.json";
      document.body.appendChild(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
      setToast({ type: "ok", message: "model-catalog.json 已生成并开始下载" });
      await refresh();
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusy(false);
    }
  };

  const copySnippet = async () => {
    try {
      await navigator.clipboard.writeText(genericConfigSnippet);
      setToast({ type: "ok", message: "配置片段已复制" });
    } catch {
      setToast({ type: "error", message: "复制失败，请手动复制页面中的配置片段" });
    }
  };

  return (
    <PanelStack>

      <Card className="overflow-hidden rounded-[30px] border-blue-200/70 bg-gradient-to-br from-white via-blue-50/72 to-cyan-50/55 shadow-glass">
        <CardHeader>
          <div>
            <CardTitle>Codex 模型 Metadata</CardTitle>
          </div>
          <Badge variant="success">ready</Badge>
        </CardHeader>
        <CardContent className="grid gap-4">
          <div className="flex items-center justify-between gap-3 rounded-2xl border border-blue-200/70 bg-blue-50/70 p-4 max-[720px]:grid">
            <TextMuted>model-catalog.json</TextMuted>
            <Button onClick={download} disabled={busy}>
              <Download size={16} />
              {busy ? "生成中" : "生成并下载"}
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card className="rounded-[30px] border-slate-300/60 bg-gradient-to-br from-white via-slate-50 to-blue-50/55">
        <CardHeader>
          <div>
            <CardTitle className="text-red-600">
              Codex 提示：⚠ Model metadata for `xxxx` not found. Defaulting to fallback metadata;
              this can degrade performance and cause issues.
            </CardTitle>
            <CardTitle>解决方法</CardTitle>
          </div>
          <Settings2 size={20} className="text-blue-600" />
        </CardHeader>
        <CardContent className="grid gap-4">
          <ol className="grid gap-3 text-sm text-slate-700">
            <Step index={1}>点击“生成并下载”，得到 <code className="rounded bg-white px-1.5 py-0.5 font-mono text-xs">model-catalog.json</code>。</Step>
            <Step index={2}>把下载文件放到你的 Codex 配置目录，例如 <code className="rounded bg-white px-1.5 py-0.5 font-mono text-xs">~/.codex/model-catalog.json</code>。</Step>
            <Step index={3}>打开 <code className="rounded bg-white px-1.5 py-0.5 font-mono text-xs">~/.codex/config.toml</code>，在顶层加入下面这一行；如果你的系统不展开 <code className="rounded bg-white px-1.5 py-0.5 font-mono text-xs">~</code>，就改成自己的绝对路径。</Step>
            <div className="grid gap-2 rounded-2xl border border-slate-300/65 bg-slate-950 p-4 text-sm shadow-sm">
              <div className="flex items-center justify-between gap-3">
                <span className="text-xs font-semibold uppercase tracking-[0.08em] text-slate-400">config.toml</span>
                <Button type="button" variant="secondary" size="sm" onClick={copySnippet}>
                  <Copy size={14} />
                  复制
                </Button>
              </div>
              <code className="break-all font-mono text-cyan-100">{genericConfigSnippet}</code>
            </div>
            <Step index={4}>重启 Codex CLI 或当前 Codex 会话，让 catalog 重新加载。</Step>
            <Step index={5}>选择模型，开始使用。</Step>
          </ol>
        </CardContent>
      </Card>

    </PanelStack>
  );
}

function Step({ index, children }: { index: number; children: React.ReactNode }) {
  return (
    <li className="flex gap-3 rounded-2xl border border-slate-300/65 bg-white/74 p-3 shadow-sm">
      <span className="grid h-7 w-7 shrink-0 place-items-center rounded-full bg-blue-600 font-mono text-xs font-semibold text-white">{index}</span>
      <span className="min-w-0 leading-7">{children}</span>
    </li>
  );
}
