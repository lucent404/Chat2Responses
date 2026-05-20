import * as React from "react";
import { cn } from "../../lib/utils";

export type Column<T> = {
  key: string;
  header: React.ReactNode;
  cell: (row: T) => React.ReactNode;
  className?: string;
  width?: string;
};

export type PaginationState = {
  page: number;
  pageSize: number;
  total: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  onPageSizeChange: (pageSize: number) => void;
};

export function DataTable<T>({
  columns,
  rows,
  empty = "暂无数据",
  pagination
}: {
  columns: Column<T>[];
  rows: T[];
  empty?: React.ReactNode;
  pagination?: PaginationState;
}) {
  const gridTemplateColumns = columns.map((column) => column.width || "minmax(0, 1fr)").join(" ");
  const pageStart = pagination && pagination.total > 0 ? (pagination.page - 1) * pagination.pageSize + 1 : 0;
  const pageEnd = pagination ? Math.min(pagination.page * pagination.pageSize, pagination.total) : 0;

  return (
    <div className="glass-panel-strong overflow-hidden rounded-2xl">
      <div className="overflow-auto max-[720px]:hidden">
        <div className="min-w-[760px]">
          <div
            className="grid border-b border-slate-300/55 bg-blue-50/70 text-xs font-semibold uppercase tracking-[0.08em] text-slate-600"
            style={{ gridTemplateColumns }}
          >
            {columns.map((column) => (
              <div key={column.key} className={cn("px-4 py-3", column.className)}>
                {column.header}
              </div>
            ))}
          </div>
          {rows.length === 0 ? (
            <div className="px-4 py-12 text-center text-sm text-muted-foreground">{empty}</div>
          ) : (
            rows.map((row, index) => (
              <div
                key={index}
                className="grid border-b border-slate-200/80 text-sm transition-all duration-200 last:border-b-0 hover:bg-blue-50/65"
                style={{ gridTemplateColumns }}
              >
                {columns.map((column) => (
                  <div key={column.key} className={cn("min-w-0 px-4 py-3", column.className)}>
                    {column.cell(row)}
                  </div>
                ))}
              </div>
            ))
          )}
        </div>
      </div>
      <div className="hidden p-3 max-[720px]:grid max-[720px]:gap-3">
        {rows.length === 0 ? (
          <div className="px-3 py-8 text-center text-sm text-muted-foreground">{empty}</div>
        ) : (
          rows.map((row, index) => (
            <div key={index} className="grid gap-3 rounded-2xl border border-slate-300/60 bg-white/86 p-3 text-sm shadow-sm backdrop-blur-xl">
              {columns.map((column) => (
                <div key={column.key} className="grid gap-1">
                  {column.header ? <span className="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">{column.header}</span> : null}
                  <div className="min-w-0">{column.cell(row)}</div>
                </div>
              ))}
            </div>
          ))
        )}
      </div>
      {pagination ? (
        <div className="flex items-center justify-between gap-3 border-t border-slate-300/55 bg-white/72 px-4 py-3 text-sm text-slate-600 max-[720px]:grid">
          <span className="font-mono text-xs">
            {pagination.total === 0 ? "0 条" : `${pageStart}-${pageEnd} / ${pagination.total} 条`}
          </span>
          <div className="flex flex-wrap items-center gap-2 max-[720px]:justify-between">
            <label className="flex items-center gap-2 text-xs text-muted-foreground">
              每页
              <select
                className="h-8 rounded-xl border border-slate-300/70 bg-white/90 px-2 text-xs text-foreground shadow-sm"
                value={pagination.pageSize}
                onChange={(event) => pagination.onPageSizeChange(Number(event.target.value))}
              >
                {[10, 20, 50, 100].map((size) => (
                  <option key={size} value={size}>
                    {size}
                  </option>
                ))}
              </select>
            </label>
            <span className="font-mono text-xs">
              {pagination.totalPages === 0 ? "第 0 / 0 页" : `第 ${pagination.page} / ${pagination.totalPages} 页`}
            </span>
            <div className="flex gap-2">
              <button
                type="button"
                className="h-8 rounded-xl border border-slate-300/70 bg-white/88 px-3 text-xs font-semibold text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                disabled={pagination.page <= 1}
                onClick={() => pagination.onPageChange(pagination.page - 1)}
              >
                上一页
              </button>
              <button
                type="button"
                className="h-8 rounded-xl border border-slate-300/70 bg-white/88 px-3 text-xs font-semibold text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                disabled={pagination.totalPages === 0 || pagination.page >= pagination.totalPages}
                onClick={() => pagination.onPageChange(pagination.page + 1)}
              >
                下一页
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
