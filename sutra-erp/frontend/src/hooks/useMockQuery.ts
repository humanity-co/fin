import { useEffect, useState } from "react";
import { mockFetch } from "../lib/mock-data";

/**
 * Simulates a React-Query-shaped async fetch against the mock dataset.
 * Returns `{ data, isLoading }` — data is undefined while the skeleton
 * shows, then flips to the mock payload. Swap this for real API hooks
 * (useAccounts, useJournals, …) when the backend lands.
 */
export function useMockQuery<T>(data: T, delay = 400): { data: T | undefined; isLoading: boolean } {
  const [state, setState] = useState<{ data: T | undefined; isLoading: boolean }>({
    data: undefined,
    isLoading: true,
  });
  useEffect(() => {
    let cancelled = false;
    mockFetch(data, delay).then((resolved) => {
      if (!cancelled) setState({ data: resolved, isLoading: false });
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [delay]);
  return state;
}
