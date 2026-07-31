import { useMemo, useCallback } from "react";
import { useSearchParams } from "react-router-dom";
import { DEFAULT_PAGE_SIZE } from "../lib/constants";

interface PaginationState {
  page: number;
  pageSize: number;
  total: number;
  setPage: (page: number) => void;
  setPageSize: (size: number) => void;
  setTotal: (total: number) => void;
  offset: number;
}

/**
 * Hook for managing pagination state via URL search params.
 */
export function usePagination(defaultPageSize = DEFAULT_PAGE_SIZE): PaginationState {
  const [searchParams, setSearchParams] = useSearchParams();

  const page = Number(searchParams.get("page") || "1");
  const pageSize = Number(searchParams.get("pageSize") || String(defaultPageSize));
  const total = Number(searchParams.get("total") || "0");

  const setPage = useCallback(
    (p: number) => {
      setSearchParams((prev) => {
        prev.set("page", String(p));
        return prev;
      });
    },
    [setSearchParams]
  );

  const setPageSize = useCallback(
    (size: number) => {
      setSearchParams((prev) => {
        prev.set("pageSize", String(size));
        prev.set("page", "1");
        return prev;
      });
    },
    [setSearchParams]
  );

  const setTotal = useCallback(
    (_t: number) => {
      // Total is not stored in URL — it's just a side effect
    },
    []
  );

  const offset = useMemo(() => (page - 1) * pageSize, [page, pageSize]);

  return useMemo(
    () => ({ page, pageSize, total, setPage, setPageSize, setTotal, offset }),
    [page, pageSize, total, setPage, setPageSize, setTotal, offset]
  );
}
