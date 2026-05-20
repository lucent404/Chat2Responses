import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";

const badgeVariants = cva("inline-flex items-center rounded-full px-2.5 py-1 text-xs font-semibold shadow-sm", {
  variants: {
    variant: {
      default: "border border-slate-300/70 bg-white/80 text-slate-700 backdrop-blur-xl",
      success: "border border-emerald-200/80 bg-emerald-50/80 text-emerald-700",
      warning: "border border-amber-200/80 bg-amber-50/85 text-amber-700",
      destructive: "border border-red-200/80 bg-red-50/85 text-red-700",
      outline: "border border-slate-300/70 bg-white/85 text-foreground backdrop-blur-xl"
    }
  },
  defaultVariants: {
    variant: "default"
  }
});

export interface BadgeProps extends React.HTMLAttributes<HTMLDivElement>, VariantProps<typeof badgeVariants> {}

export const Badge = ({ className, variant, ...props }: BadgeProps) => (
  <div className={cn(badgeVariants({ variant, className }))} {...props} />
);
