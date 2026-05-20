import { Check, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { Button } from "../ui/button";
import { Input } from "../ui/input";

type ModelMultiSelectProps = {
  models: string[];
  selected: string[];
  onChange: (models: string[]) => void;
  emptyLabel?: string;
  maxHeightClassName?: string;
};

export function ModelMultiSelect({ models, selected, onChange, emptyLabel = "暂无可选模型", maxHeightClassName = "max-h-56" }: ModelMultiSelectProps) {
  const [query, setQuery] = useState("");
  const selectedSet = useMemo(() => new Set(selected), [selected]);
  const filtered = models.filter((model) => model.toLowerCase().includes(query.toLowerCase()));

  const toggle = (model: string) => {
    if (selectedSet.has(model)) {
      onChange(selected.filter((item) => item !== model));
    } else {
      onChange([...selected, model].sort());
    }
  };

  return (
    <div className="grid gap-2 rounded-2xl border border-slate-300/70 bg-white/74 p-2 shadow-sm backdrop-blur-xl">
      <div className="flex gap-2 max-[520px]:grid">
        <div className="relative min-w-0 flex-1">
          <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" size={14} />
          <Input className="h-8 pl-8 text-xs" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索模型" />
        </div>
        <div className="flex gap-2">
          <Button type="button" variant="secondary" size="sm" onClick={() => onChange(models)}>
            全选
          </Button>
          <Button type="button" variant="ghost" size="sm" onClick={() => onChange([])}>
            清空
          </Button>
        </div>
      </div>
      <div className={`${maxHeightClassName} grid gap-1 overflow-auto pr-1`}>
        {filtered.length === 0 ? (
          <div className="rounded-xl border border-dashed border-slate-300/70 bg-blue-50/50 px-3 py-5 text-center text-xs text-muted-foreground">{models.length === 0 ? emptyLabel : "没有匹配的模型"}</div>
        ) : (
          filtered.map((model) => {
            const active = selectedSet.has(model);
            return (
              <button
                key={model}
                type="button"
                onClick={() => toggle(model)}
                className={`flex min-w-0 cursor-pointer items-center justify-between gap-3 rounded-xl border px-3 py-2 text-left text-xs transition-all ${
                  active
                    ? "border-blue-300 bg-blue-100/90 text-blue-950 shadow-sm"
                    : "border-slate-200/80 bg-white/68 text-slate-700 hover:border-blue-200 hover:bg-blue-50/80"
                }`}
              >
                <span className="min-w-0 break-all font-mono">{model}</span>
                <span className={`grid h-5 w-5 shrink-0 place-items-center rounded-full border ${active ? "border-blue-500 bg-blue-600 text-white" : "border-slate-300 bg-white"}`}>
                  {active ? <Check size={13} /> : null}
                </span>
              </button>
            );
          })
        )}
      </div>
      <div className="flex items-center justify-between gap-2 text-[11px] text-muted-foreground">
        <span>已选择 {selected.length}/{models.length}</span>
        <span>不选择则默认全部可用</span>
      </div>
    </div>
  );
}
