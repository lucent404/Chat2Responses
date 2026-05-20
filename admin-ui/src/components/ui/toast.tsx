import * as React from "react";
import * as ToastPrimitive from "@radix-ui/react-toast";
import { cn } from "../../lib/utils";

export const ToastProvider = ToastPrimitive.Provider;
export const ToastViewport = React.forwardRef<
  React.ElementRef<typeof ToastPrimitive.Viewport>,
  React.ComponentPropsWithoutRef<typeof ToastPrimitive.Viewport>
>(({ className, ...props }, ref) => (
  <ToastPrimitive.Viewport
    ref={ref}
    className={cn("fixed right-4 top-4 z-[100] grid w-[min(380px,calc(100vw-32px))] gap-2", className)}
    {...props}
  />
));
ToastViewport.displayName = ToastPrimitive.Viewport.displayName;

export const Toast = React.forwardRef<
  React.ElementRef<typeof ToastPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof ToastPrimitive.Root> & { tone?: "ok" | "error" }
>(({ className, tone = "ok", ...props }, ref) => (
  <ToastPrimitive.Root
    ref={ref}
    className={cn(
      "rounded-2xl border px-4 py-3 text-sm shadow-panel backdrop-blur-2xl",
      tone === "ok" ? "border-emerald-200 bg-emerald-50 text-emerald-800" : "border-red-200 bg-red-50 text-red-800",
      className
    )}
    {...props}
  />
));
Toast.displayName = ToastPrimitive.Root.displayName;

export const ToastTitle = ToastPrimitive.Title;
