import { type ReactNode, useState, useCallback, useMemo } from "react";
import {
  ChevronLeft,
  ChevronRight,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
} from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { TableSkeleton } from "../ui/skeleton";

// ── Types ──────────────────────────────────────────

export interface ColumnDef<T> {
  id: string;
  header: string;
  accessorFn?: (row: T) => ReactNode;
  accessorKey?: keyof T;
  sortable?: boolean;
  className?: string;
  headerClassName?: string;
  /** Align: left, right, center */
  align?: "left" | "right" | "center";
}

export interface DataTableProps<T> {
  data: T[];
  columns: ColumnDef<T>[];
  /** Unique key for each row */
  getRowId: (row: T) => string | number;
  /** Whether rows are selectable */
  selectable?: boolean;
  selectedIds?: Set<string | number>;
  onSelectionChange?: (ids: Set<string | number>) => void;
  /** Pagination */
  pagination?: {
    page: number;
    pageSize: number;
    total: number;
    onPageChange: (page: number) => void;
    onPageSizeChange: (size: number) => void;
  };
  /** Sorting */
  sortColumn?: string;
  sortDirection?: "asc" | "desc";
  onSort?: (column: string, direction: "asc" | "desc") => void;
  /** Loading state */
  isLoading?: boolean;
  /** Empty state */
  emptyMessage?: string;
  /** Extra class */
  className?: string;
}

// ── Component ──────────────────────────────────────

export function DataTable<T>({
  data,
  columns,
  getRowId,
  selectable = false,
  selectedIds,
  onSelectionChange,
  pagination,
  sortColumn,
  sortDirection,
  onSort,
  isLoading = false,
  emptyMessage = "No data found.",
  className,
}: DataTableProps<T>) {
  const [selectAll, setSelectAll] = useState(false);

  const allIds = useMemo(
    () => new Set(data.map((row) => getRowId(row))),
    [data, getRowId]
  );

  const handleSelectAll = useCallback(() => {
    if (!onSelectionChange) return;
    if (selectAll) {
      onSelectionChange(new Set());
    } else {
      onSelectionChange(allIds);
    }
    setSelectAll(!selectAll);
  }, [selectAll, allIds, onSelectionChange]);

  const handleSelectRow = useCallback(
    (id: string | number) => {
      if (!onSelectionChange || !selectedIds) return;
      const next = new Set(selectedIds);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      onSelectionChange(next);
    },
    [onSelectionChange, selectedIds]
  );

  const handleSort = useCallback(
    (columnId: string) => {
      if (!onSort) return;
      const newDirection =
        sortColumn === columnId && sortDirection === "asc" ? "desc" : "asc";
      onSort(columnId, newDirection);
    },
    [onSort, sortColumn, sortDirection]
  );

  const SortIcon = ({ columnId }: { columnId: string }) => {
    if (sortColumn !== columnId) {
      return <ArrowUpDown className="ml-1 h-3 w-3" />;
    }
    return sortDirection === "asc" ? (
      <ArrowUp className="ml-1 h-3 w-3" />
    ) : (
      <ArrowDown className="ml-1 h-3 w-3" />
    );
  };

  // ── Loading state ────────────────────
  if (isLoading) {
    return (
      <div className={cn("rounded-md border", className)}>
        <div className="p-4">
          <TableSkeleton rows={5} cols={columns.length} />
        </div>
      </div>
    );
  }

  // ── Empty state ──────────────────────
  if (data.length === 0) {
    return (
      <div className={cn("rounded-md border", className)}>
        <div className="flex h-40 items-center justify-center text-sm text-muted-foreground">
          {emptyMessage}
        </div>
      </div>
    );
  }

  const alignClass = {
    left: "text-left",
    right: "text-right",
    center: "text-center",
  };

  return (
    <div className={cn("space-y-4", className)}>
      <div className="overflow-x-auto rounded-md border">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b bg-muted/50">
              {selectable && (
                <th className="w-10 px-3 py-3">
                  <input
                    type="checkbox"
                    className="h-4 w-4 rounded border-gray-300"
                    checked={selectAll}
                    onChange={handleSelectAll}
                    aria-label="Select all rows"
                  />
                </th>
              )}
              {columns.map((col) => (
                <th
                  key={col.id}
                  className={cn(
                    "px-3 py-3 font-semibold text-muted-foreground",
                    alignClass[col.align || "left"],
                    col.headerClassName,
                    col.sortable && "cursor-pointer select-none hover:text-foreground"
                  )}
                  onClick={() => col.sortable && handleSort(col.id)}
                >
                  <span className="inline-flex items-center">
                    {col.header}
                    {col.sortable && <SortIcon columnId={col.id} />}
                  </span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {data.map((row) => {
              const id = getRowId(row);
              const isSelected = selectedIds?.has(id);
              return (
                <tr
                  key={id}
                  className={cn(
                    "border-b transition-colors hover:bg-muted/50",
                    isSelected && "bg-primary/5"
                  )}
                >
                  {selectable && (
                    <td className="px-3 py-2.5">
                      <input
                        type="checkbox"
                        className="h-4 w-4 rounded border-gray-300"
                        checked={isSelected || false}
                        onChange={() => handleSelectRow(id)}
                        aria-label={`Select row ${id}`}
                      />
                    </td>
                  )}
                  {columns.map((col) => (
                    <td
                      key={col.id}
                      className={cn(
                        "px-3 py-2.5",
                        alignClass[col.align || "left"],
                        col.className
                      )}
                    >
                      {col.accessorFn
                        ? col.accessorFn(row)
                        : col.accessorKey
                          ? String(row[col.accessorKey] ?? "")
                          : null}
                    </td>
                  ))}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      {pagination && (
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <div>
            Showing{" "}
            {Math.min(
              (pagination.page - 1) * pagination.pageSize + 1,
              pagination.total
            )}{" "}
            – {Math.min(pagination.page * pagination.pageSize, pagination.total)}{" "}
            of {pagination.total}
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={pagination.page <= 1}
              onClick={() => pagination.onPageChange(pagination.page - 1)}
            >
              <ChevronLeft className="h-4 w-4" />
              Previous
            </Button>
            <span className="px-2">
              Page {pagination.page} of{" "}
              {Math.ceil(pagination.total / pagination.pageSize)}
            </span>
            <Button
              variant="outline"
              size="sm"
              disabled={
                pagination.page >=
                Math.ceil(pagination.total / pagination.pageSize)
              }
              onClick={() => pagination.onPageChange(pagination.page + 1)}
            >
              Next
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
