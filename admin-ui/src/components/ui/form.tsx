import * as React from "react";
import { cn } from "../../lib/utils";

export function Form({ className, ...props }: React.FormHTMLAttributes<HTMLFormElement>) {
  return <form className={cn(className)} {...props} />;
}
