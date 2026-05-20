import { type FormEvent, useState } from "react";
import { LockKeyhole, ShieldCheck } from "lucide-react";
import { initAdmin, loginAdmin } from "../../api/admin";
import { BrandBlock } from "../../components/common/BrandBlock";
import { Field } from "../../components/common/Field";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Form } from "../../components/ui/form";
import { Input } from "../../components/ui/input";
import type { ToastState } from "../../types/admin";

type AuthScreenProps = {
  mode: "init" | "login";
  onDone: () => Promise<void>;
  setToast: (toast: ToastState) => void;
};

export function AuthScreen({ mode, onDone, setToast }: AuthScreenProps) {
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    try {
      if (mode === "init") {
        await initAdmin(username, password);
      } else {
        await loginAdmin(username, password);
      }
      await onDone();
    } catch (error) {
      setToast({ type: "error", message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="relative grid min-h-screen place-items-center overflow-hidden bg-background p-6">
      <div className="ambient-grid pointer-events-none absolute inset-0" />
      <div className="noise-overlay pointer-events-none absolute inset-0" />
      <div className="relative grid w-full max-w-[1040px] grid-cols-[1fr_430px] gap-6 max-[860px]:max-w-[460px] max-[860px]:grid-cols-1">
        <section className="glass-panel surface-glow grid content-center gap-6 rounded-[32px] p-8 max-[860px]:hidden">
          <BrandBlock subtitle="Local proxy console" />
          <div className="grid gap-3">
            <p className="font-mono text-xs uppercase tracking-[0.18em] text-primary">Responses -&gt; Chat</p>
            <h2 className="max-w-[560px] text-5xl font-semibold leading-[1.02] tracking-normal">
              Glass control for local model routing.
            </h2>
            <p className="max-w-[560px] text-sm leading-6 text-muted-foreground">
              Configure channels, publish model aliases, issue caller keys, and inspect request logs from a single admin surface.
            </p>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="rounded-2xl border border-white/65 bg-white/45 p-4 shadow-sm backdrop-blur-xl">
              <ShieldCheck className="mb-3 text-emerald-600" size={20} />
              <p className="text-sm font-semibold">Encrypted 渠道 keys</p>
              <p className="mt-1 text-xs leading-5 text-muted-foreground">Stored with the service secret, never shown again.</p>
            </div>
            <div className="rounded-2xl border border-white/65 bg-white/45 p-4 shadow-sm backdrop-blur-xl">
              <LockKeyhole className="mb-3 text-primary" size={20} />
              <p className="text-sm font-semibold">Session-only admin access</p>
              <p className="mt-1 text-xs leading-5 text-muted-foreground">Designed for localhost operation.</p>
            </div>
          </div>
        </section>
        <Card className="w-full rounded-[32px] shadow-glass">
          <CardHeader>
            <CardTitle>{mode === "init" ? "初始化管理员账号" : "登录管理后台"}</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-5 p-6">
            <BrandBlock subtitle={mode === "init" ? "Create first admin" : "Admin sign in"} />
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
      </div>
    </main>
  );
}
