import { cloneElement, isValidElement, useId, type ReactElement, type ReactNode } from "react";
import { Label } from "../ui/label";

export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  const generatedId = useId();
  const childElement = isValidElement(children) ? (children as ReactElement<{ id?: string }>) : null;
  const control =
    childElement && !childElement.props.id
      ? cloneElement(childElement, { id: generatedId })
      : children;
  const controlElement = isValidElement(control) ? (control as ReactElement<{ id?: string }>) : null;
  const labelFor = controlElement?.props.id || generatedId;

  return (
    <div className="grid gap-2">
      <Label htmlFor={labelFor}>{label}</Label>
      {control}
      {hint ? <p className="text-xs leading-5 text-muted-foreground">{hint}</p> : null}
    </div>
  );
}
