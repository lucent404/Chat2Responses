import { Toast, ToastTitle } from "../ui/toast";
import type { ToastState } from "../../types/admin";

export function ToastMessage({ toast, onOpenChange }: { toast: ToastState; onOpenChange: (open: boolean) => void }) {
  if (!toast) return null;
  return (
    <Toast open={Boolean(toast)} onOpenChange={onOpenChange} tone={toast.type}>
      <ToastTitle>{toast.message}</ToastTitle>
    </Toast>
  );
}
