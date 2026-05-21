import { type FormEvent, useEffect, useState } from "react";
import { Activity, Clock3, FileText, Save, ShieldAlert } from "lucide-react";
import { Field } from "../../components/common/Field";
import { Metric } from "../../components/common/Metric";
import { PanelStack } from "../../components/common/PanelStack";
import { SwitchField } from "../../components/common/SwitchField";
import { TextMuted } from "../../components/common/Text";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Form } from "../../components/ui/form";
import { Input } from "../../components/ui/input";
import type { AppSettings, ToastState } from "../../types/admin";

type SettingsPanelProps = {
  settings: AppSettings;
  refresh: () => Promise<void>;
  saveSettings: (settings: AppSettings) => Promise<void>;
  setToast: (toast: ToastState) => void;
};

export function SettingsPanel({ settings, refresh, saveSettings, setToast }: SettingsPanelProps) {
  const [requestLoggingEnabled, setRequestLoggingEnabled] = useState(settings.request_logging_enabled);
  const [upstreamTimeoutSeconds, setUpstreamTimeoutSeconds] = useState(String(settings.upstream_timeout_seconds));
  const [logErrorMaxChars, setLogErrorMaxChars] = useState(String(settings.log_error_max_chars));
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setRequestLoggingEnabled(settings.request_logging_enabled);
    setUpstreamTimeoutSeconds(String(settings.upstream_timeout_seconds));
    setLogErrorMaxChars(String(settings.log_error_max_chars));
  }, [settings]);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const next: AppSettings = {
      request_logging_enabled: requestLoggingEnabled,
      upstream_timeout_seconds: Number(upstreamTimeoutSeconds),
      log_error_max_chars: Number(logErrorMaxChars)
    };
    if (!Number.isInteger(next.upstream_timeout_seconds) || next.upstream_timeout_seconds < 0 || next.upstream_timeout_seconds > 600) {
      setToast({ type: "error", message: "渠道请求超时必须在 0 到 600 秒之间" });
      return;
    }
    if (!Number.isInteger(next.log_error_max_chars) || next.log_error_max_chars < 100 || next.log_error_max_chars > 10000) {
      setToast({ type: "error", message: "错误内容长度必须在 100 到 10000 字符之间" });
      return;
    }
    setBusy(true);
    try {
      await saveSettings(next);
      setToast({ type: "ok", message: "设置已保存，后续请求立即生效" });
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <PanelStack>
      <div className="grid grid-cols-3 gap-3 max-[760px]:grid-cols-1">
        <Metric label="请求日志" value={settings.request_logging_enabled ? "开启" : "关闭"} icon={<Activity size={17} />} tone={settings.request_logging_enabled ? "default" : "danger"} />
        <Metric label="渠道超时" value={settings.upstream_timeout_seconds === 0 ? "无限制" : `${settings.upstream_timeout_seconds}s`} icon={<Clock3 size={17} />} />
        <Metric label="错误长度" value={`${settings.log_error_max_chars}`} icon={<FileText size={17} />} />
      </div>

      <Form onSubmit={submit} className="grid gap-5">
        <Card className="overflow-hidden rounded-[30px] border-blue-200/70 bg-gradient-to-br from-white via-blue-50/72 to-cyan-50/55 shadow-glass">
          <CardHeader>
            <div>
              <CardTitle>日志策略</CardTitle>
            </div>
            <Badge variant={requestLoggingEnabled ? "success" : "warning"}>{requestLoggingEnabled ? "recording" : "paused"}</Badge>
          </CardHeader>
          <CardContent className="grid gap-4">
            <SwitchField
              label="记录请求日志"
              description="默认关闭。关闭后不会新增请求日志，但历史日志仍然保留并可查看。"
              checked={requestLoggingEnabled}
              onCheckedChange={setRequestLoggingEnabled}
            />
            <Field label="错误内容最大长度" hint="保存渠道错误或流式错误时的截断长度，范围 100 - 10000 字符。">
              <Input type="number" min={100} max={10000} step={1} value={logErrorMaxChars} onChange={(event) => setLogErrorMaxChars(event.target.value)} />
            </Field>
          </CardContent>
        </Card>

        <Card className="rounded-[30px] border-slate-300/60 bg-gradient-to-br from-white via-slate-50 to-blue-50/55">
          <CardHeader>
            <div>
              <CardTitle>渠道转发</CardTitle>
            </div>
            <ShieldAlert size={20} className="text-blue-600" />
          </CardHeader>
          <CardContent className="grid gap-4">
            <Field label="渠道请求超时（秒）" hint="0 表示保持当前行为，不设置额外超时；范围 0 - 600 秒。流式请求只限制连接阶段，不限制后续长输出。">
              <Input type="number" min={0} max={600} step={1} value={upstreamTimeoutSeconds} onChange={(event) => setUpstreamTimeoutSeconds(event.target.value)} />
            </Field>
            <div className="rounded-2xl border border-slate-300/65 bg-white/76 p-4 text-sm shadow-sm backdrop-blur-xl">
              <TextMuted>
                当前配置会立即用于后续请求：blocking 请求覆盖完整渠道调用；streaming 请求只覆盖渠道响应建立阶段。
              </TextMuted>
            </div>
          </CardContent>
        </Card>

        <div className="glass-panel-strong flex items-center justify-between gap-3 rounded-3xl p-3 max-[640px]:grid">
          <TextMuted>操作</TextMuted>
          <div className="flex gap-2 max-[640px]:grid max-[640px]:grid-cols-2">
            <Button type="button" variant="secondary" onClick={refresh} disabled={busy}>
              重新加载
            </Button>
            <Button disabled={busy}>
              <Save size={16} />
              {busy ? "保存中" : "保存设置"}
            </Button>
          </div>
        </div>
      </Form>
    </PanelStack>
  );
}
