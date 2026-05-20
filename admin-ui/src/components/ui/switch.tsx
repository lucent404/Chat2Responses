import * as React from "react";
import * as SwitchPrimitive from "@radix-ui/react-switch";
import { cn } from "../../lib/utils";

export const Switch = React.forwardRef<
  React.ElementRef<typeof SwitchPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof SwitchPrimitive.Root>
>(({ className, ...props }, ref) => (
  <SwitchPrimitive.Root
    ref={ref}
    className={cn(
      "peer relative inline-flex h-8 w-16 shrink-0 cursor-pointer items-center rounded-full border-2 border-slate-400 bg-slate-300 shadow-inner transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 data-[state=checked]:border-blue-600 data-[state=checked]:bg-blue-600 disabled:cursor-not-allowed disabled:opacity-50",
      className
    )}
    {...props}
  >
    <span className="pointer-events-none absolute left-2 text-[10px] font-bold uppercase text-white opacity-0 transition-opacity data-[state=checked]:opacity-100" data-state={props.checked ? "checked" : "unchecked"}>
      ON
    </span>
    <span className="pointer-events-none absolute right-2 text-[10px] font-bold uppercase text-slate-600 opacity-100 transition-opacity data-[state=checked]:opacity-0" data-state={props.checked ? "checked" : "unchecked"}>
      OFF
    </span>
    <SwitchPrimitive.Thumb className="pointer-events-none relative z-10 block h-6 w-6 translate-x-0.5 rounded-full bg-white shadow-lg ring-1 ring-slate-400 transition-transform data-[state=checked]:translate-x-8 data-[state=unchecked]:translate-x-0.5" />
  </SwitchPrimitive.Root>
));

Switch.displayName = SwitchPrimitive.Root.displayName;
