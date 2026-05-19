import * as React from "react";
import { cn } from "../../lib/utils";

export type Column<T> = {
  key: string;
  header: React.ReactNode;
  cell: (row: T) => React.ReactNode;
  className?: string;
};

export function DataTable<T>({
  columns,
  rows,
  empty = "暂无数据"
}: {
  columns: Column<T>[];
  rows: T[];
  empty?: string;
}) {
  return (
    <div className="overflow-auto rounded-lg border border-border bg-card">
      <div className="min-w-[760px]">
        <div
          className="grid border-b border-border bg-muted/40 text-xs font-semibold text-muted-foreground"
          style={{ gridTemplateColumns: `repeat(${columns.length}, minmax(0, 1fr))` }}
        >
          {columns.map((column) => (
            <div key={column.key} className={cn("px-4 py-3", column.className)}>
              {column.header}
            </div>
          ))}
        </div>
        {rows.length === 0 ? (
          <div className="px-4 py-10 text-center text-sm text-muted-foreground">{empty}</div>
        ) : (
          rows.map((row, index) => (
            <div
              key={index}
              className="grid border-b border-border text-sm last:border-b-0"
              style={{ gridTemplateColumns: `repeat(${columns.length}, minmax(0, 1fr))` }}
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
  );
}
