import { Activity, AlertTriangle, ArrowRight, CheckCircle2, KeyRound, RadioTower, Route, Server, ShieldCheck, Sparkles, Zap } from "lucide-react";
import { NavLink } from "react-router-dom";
import { Metric } from "../../components/common/Metric";
import { PanelStack } from "../../components/common/PanelStack";
import { TextMuted, TextStrong } from "../../components/common/Text";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { formatDate } from "../../lib/format";
import type { ApiKey, AppSettings, AvailableModel, ModelRoute, RequestLog, Upstream } from "../../types/admin";

type OverviewPanelProps = {
  upstreams: Upstream[];
  upstreamTotal: number;
  models: ModelRoute[];
  modelTotal: number;
  availableModels: AvailableModel[];
  keys: ApiKey[];
  keyTotal: number;
  logs: RequestLog[];
  logTotal: number;
  settings: AppSettings;
};

export function OverviewPanel({ upstreams, upstreamTotal, models, modelTotal, availableModels, keys, keyTotal, logs, logTotal, settings }: OverviewPanelProps) {
  const activeChannels = upstreams.filter((row) => row.enabled).length;
  const activeMappings = models.filter((row) => row.enabled).length;
  const activeModels = availableModels.length;
  const enabledKeys = keys.filter((row) => row.enabled).length;
  const errors = logs.filter((row) => row.status_code >= 400);
  const tokenTotal = logs.reduce((sum, row) => sum + row.total_tokens, 0);
  const avgDuration = logs.length ? Math.round(logs.reduce((sum, row) => sum + row.duration_ms, 0) / logs.length) : 0;
  const ready = activeChannels > 0 && activeModels > 0 && enabledKeys > 0;

  const setupItems = [
    { label: "渠道", detail: "添加至少一个渠道", done: activeChannels > 0, to: "/admin/upstreams" },
    { label: "Models", detail: "同步渠道模型或新增映射", done: activeModels > 0, to: "/admin/upstreams" },
    { label: "Access", detail: "创建调用方密钥", done: enabledKeys > 0, to: "/admin/keys" }
  ];

  return (
    <PanelStack>
      <section className="grid grid-cols-[1.35fr_0.85fr] gap-5 max-[1080px]:grid-cols-1">
        <Card className="surface-glow overflow-hidden rounded-[32px] border-blue-200/70 bg-gradient-to-br from-white via-blue-50/82 to-cyan-50/65 shadow-glass">
          <CardContent className="relative grid gap-6 p-6 max-[560px]:p-4">
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div className="grid max-w-[780px] gap-3">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant={ready ? "success" : "warning"}>{ready ? "Route ready" : "Route incomplete"}</Badge>
                  <span className="font-mono text-xs text-muted-foreground">Responses -&gt; Chat Completions</span>
                </div>
                <h3 className="max-w-[820px] text-4xl font-semibold leading-[1.04] tracking-normal text-foreground max-[760px]:text-3xl max-[460px]:text-2xl">
                  总览
                </h3>
              </div>
              <Button asChild>
                <NavLink to="/admin/logs">
                  <Activity size={16} />
                  查看请求
                </NavLink>
              </Button>
            </div>

            <div className="grid grid-cols-[1fr_auto_1fr_auto_1fr] items-center gap-3 max-[760px]:grid-cols-1">
              {setupItems.map((item, index) => (
                <PipelineNode key={item.label} item={item} index={index} />
              )).flatMap((node, index) =>
                index < setupItems.length - 1
                  ? [node, <div key={`line-${index}`} className="h-[2px] min-w-8 rounded-full bg-gradient-to-r from-blue-400/45 via-blue-600/75 to-cyan-400/55 max-[760px]:h-8 max-[760px]:min-w-px max-[760px]:w-[2px] max-[760px]:justify-self-center" />]
                  : [node]
              )}
            </div>

            <div className="grid grid-cols-4 gap-3 max-[900px]:grid-cols-2 max-[520px]:grid-cols-1">
              <Metric label="渠道" value={`${activeChannels}/${upstreamTotal}`} icon={<Server size={17} />} />
              <Metric label="Models" value={activeModels.toString()} icon={<Route size={17} />} />
              <Metric label="Mappings" value={`${activeMappings}/${modelTotal}`} icon={<RadioTower size={17} />} />
              <Metric label="Keys" value={`${enabledKeys}/${keyTotal}`} icon={<KeyRound size={17} />} />
            </div>
            <div className={`rounded-2xl border px-4 py-3 text-sm shadow-sm backdrop-blur-xl ${
              settings.request_logging_enabled
                ? "border-emerald-300/70 bg-emerald-50/80 text-emerald-800"
                : "border-amber-300/75 bg-amber-50/88 text-amber-800"
            }`}>
              <span className="font-semibold">请求日志：{settings.request_logging_enabled ? "开启" : "关闭"}</span>
              <span className="ml-2 text-xs opacity-80">
                {settings.request_logging_enabled ? `错误最多记录 ${settings.log_error_max_chars} 字符` : "当前只展示历史日志，不再新增记录"}
              </span>
            </div>
          </CardContent>
        </Card>

        <Card className="rounded-[32px] border-slate-300/55 bg-gradient-to-br from-white via-slate-50 to-blue-50/65">
          <CardHeader>
            <div>
              <CardTitle>配置检查</CardTitle>
              <p className="mt-1 text-sm text-muted-foreground">三段式 relay 链路完整度</p>
            </div>
            <ShieldCheck size={20} className={ready ? "text-emerald-600" : "text-amber-600"} />
          </CardHeader>
          <CardContent className="grid gap-3">
            {setupItems.map((item) => (
              <NavLink
                key={item.label}
                to={item.to}
                className={`group flex items-center justify-between rounded-2xl border px-3 py-3 text-sm shadow-sm backdrop-blur-xl transition-all hover:-translate-y-0.5 ${
                  item.done
                    ? "border-emerald-300/70 bg-emerald-50/85 hover:bg-emerald-50"
                    : "border-amber-300/75 bg-amber-50/88 hover:bg-amber-50"
                }`}
              >
                <span className="flex items-center gap-3">
                  <span className={`grid h-9 w-9 place-items-center rounded-xl ${item.done ? "bg-emerald-500 text-white" : "bg-amber-100 text-amber-700"}`}>
                    {item.done ? <CheckCircle2 size={17} /> : <AlertTriangle size={17} />}
                  </span>
                  <span className="grid">
                    <span className="font-semibold">{item.label}</span>
                    <span className="text-xs text-muted-foreground">{item.detail}</span>
                  </span>
                </span>
                <ArrowRight size={15} className="text-muted-foreground transition-transform group-hover:translate-x-0.5" />
              </NavLink>
            ))}
          </CardContent>
        </Card>
      </section>

      <section className="grid grid-cols-[0.82fr_1.18fr] gap-5 max-[1000px]:grid-cols-1">
        <Card className="rounded-[28px] border-blue-200/65 bg-gradient-to-br from-white via-blue-50/60 to-white">
          <CardHeader>
            <div>
              <CardTitle>流量健康度</CardTitle>
              <p className="mt-1 text-sm text-muted-foreground">最近 100 条请求窗口</p>
            </div>
            <RadioTower size={19} className="text-primary" />
          </CardHeader>
          <CardContent className="grid grid-cols-3 gap-3 max-[620px]:grid-cols-1">
            <Metric label="近期请求" value={logTotal.toString()} />
            <Metric label="错误请求" value={errors.length.toString()} tone={errors.length ? "danger" : "default"} />
            <Metric label="平均耗时" value={`${avgDuration} ms`} />
          </CardContent>
        </Card>

        <Card className="rounded-[28px] border-slate-300/55 bg-gradient-to-br from-white via-slate-50 to-blue-50/45">
          <CardHeader className="flex-row items-center justify-between">
            <div>
              <CardTitle>最近错误</CardTitle>
              <p className="mt-1 text-sm text-muted-foreground">快速定位调用失败来源</p>
            </div>
            <Button asChild variant="ghost" size="sm">
              <NavLink to="/admin/logs">
                全部日志
                <ArrowRight size={14} />
              </NavLink>
            </Button>
          </CardHeader>
          <CardContent className="grid gap-2">
            {errors.slice(0, 4).length === 0 ? (
              <div className="grid gap-2 rounded-2xl border border-dashed border-cyan-300/60 bg-cyan-50/65 p-5 text-sm text-slate-600">
                <Sparkles size={18} className="text-emerald-600" />
              <span>最近 {logs.length} 条请求里没有错误。</span>
              </div>
            ) : (
              errors.slice(0, 4).map((row) => (
                <div key={row.id} className="grid gap-1 rounded-2xl border border-red-200/75 bg-red-50/75 px-3 py-2 text-sm shadow-sm backdrop-blur-xl">
                  <div className="flex items-center justify-between gap-3">
                    <TextStrong>{row.public_model || "未知模型"}</TextStrong>
                    <Badge variant="destructive">{row.status_code}</Badge>
                  </div>
                  <TextMuted>
                    {formatDate(row.created_at)} · {row.error || row.upstream_name || "-"}
                  </TextMuted>
                </div>
              ))
            )}
          </CardContent>
        </Card>
      </section>
    </PanelStack>
  );
}

function PipelineNode({
  item,
  index
}: {
  item: { label: string; detail: string; done: boolean; to: string };
  index: number;
}) {
  const icons = [Server, Route, KeyRound];
  const Icon = icons[index] || Server;
  return (
    <NavLink
      to={item.to}
      className={`group grid gap-3 rounded-3xl border p-4 shadow-soft backdrop-blur-xl transition-all hover:-translate-y-1 hover:shadow-glow ${
        item.done
          ? "border-blue-300/75 bg-gradient-to-br from-blue-600 to-cyan-500 text-white"
          : "border-blue-200/80 bg-gradient-to-br from-white via-blue-50/86 to-cyan-50/72"
      }`}
    >
      <div className="flex items-center justify-between">
        <span className={`grid h-11 w-11 place-items-center rounded-2xl shadow-sm ${item.done ? "bg-white/18 text-white ring-1 ring-white/45" : "bg-blue-600 text-white"}`}>
          <Icon size={20} />
        </span>
        <Badge variant={item.done ? "success" : "warning"}>{item.done ? "online" : "setup"}</Badge>
      </div>
      <div>
        <p className={`font-mono text-xs uppercase tracking-[0.16em] ${item.done ? "text-blue-50" : "text-slate-500"}`}>0{index + 1}</p>
        <p className="mt-1 text-lg font-semibold">{item.label}</p>
        <p className={`mt-1 text-sm ${item.done ? "text-blue-50/85" : "text-slate-600"}`}>{item.detail}</p>
      </div>
    </NavLink>
  );
}
