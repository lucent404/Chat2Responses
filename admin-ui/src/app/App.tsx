import { useEffect, useState } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { getAdminStatus } from "../api/admin";
import { ToastMessage } from "../components/common/ToastMessage";
import { ToastProvider, ToastViewport } from "../components/ui/toast";
import { AuthScreen } from "../features/auth/AuthScreen";
import type { AdminStatus, ToastState } from "../types/admin";
import { AdminShell } from "./AdminShell";

export function App() {
  const [status, setStatus] = useState<AdminStatus | null>(null);
  const [toast, setToast] = useState<ToastState>(null);
  const refreshStatus = async () => setStatus(await getAdminStatus());

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
        <Routes>
          <Route path="/admin/:section?" element={<AdminShell user={status.user.username} onLogout={refreshStatus} setToast={setToast} />} />
          <Route path="*" element={<Navigate to="/admin/upstreams" replace />} />
        </Routes>
      )}
      <ToastMessage toast={toast} onOpenChange={(open) => !open && setToast(null)} />
      <ToastViewport />
    </ToastProvider>
  );
}

function ShellLoading() {
  return <div className="grid min-h-screen place-items-center bg-background text-sm text-muted-foreground">Loading</div>;
}
