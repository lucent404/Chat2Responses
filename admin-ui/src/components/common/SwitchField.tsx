import { Label } from "../ui/label";
import { Switch } from "../ui/switch";

type SwitchFieldProps = {
  label: string;
  description?: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
};

export function SwitchField({ label, description, checked, onCheckedChange }: SwitchFieldProps) {
  return (
    <div className={`flex items-center justify-between gap-4 rounded-2xl border p-4 shadow-sm backdrop-blur-xl ${
      checked
        ? "border-blue-300/80 bg-blue-50/90"
        : "border-slate-300/80 bg-white/90"
    }`}>
      <div className="grid gap-1">
        <Label>{label}</Label>
        {description ? <p className="text-xs leading-5 text-muted-foreground">{description}</p> : null}
      </div>
      <div className="flex shrink-0 items-center gap-3">
        <span className={`rounded-full px-2.5 py-1 text-xs font-semibold ${
          checked ? "bg-blue-600 text-white" : "bg-slate-200 text-slate-700"
        }`}>
          {checked ? "已开启" : "已关闭"}
        </span>
        <Switch checked={checked} onCheckedChange={onCheckedChange} />
      </div>
    </div>
  );
}
