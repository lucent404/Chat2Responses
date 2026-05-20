import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";

const buttonVariants = cva(
  "inline-flex h-9 cursor-pointer items-center justify-center gap-2 whitespace-nowrap rounded-xl text-sm font-semibold transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground shadow-glow hover:-translate-y-0.5 hover:bg-primary/90",
        secondary: "border border-slate-300/70 bg-white/88 text-foreground shadow-soft backdrop-blur-xl hover:-translate-y-0.5 hover:border-primary/40 hover:bg-blue-50/90",
        ghost: "text-muted-foreground hover:bg-blue-50/75 hover:text-foreground",
        destructive: "bg-destructive text-destructive-foreground shadow-soft hover:-translate-y-0.5 hover:bg-destructive/90",
        outline: "border border-slate-300/70 bg-white/75 text-foreground backdrop-blur-xl hover:bg-blue-50/90"
      },
      size: {
        default: "px-4",
        sm: "h-8 px-3 text-xs",
        icon: "h-9 w-9"
      }
    },
    defaultVariants: {
      variant: "default",
      size: "default"
    }
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return <Comp className={cn(buttonVariants({ variant, size, className }))} ref={ref} {...props} />;
  }
);

Button.displayName = "Button";
