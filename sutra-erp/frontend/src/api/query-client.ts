import { QueryClient } from "@tanstack/react-query";
import { QUERY_STALE_TIME, QUERY_RETRY_COUNT } from "../lib/constants";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: QUERY_STALE_TIME,
      retry: QUERY_RETRY_COUNT,
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: 0,
    },
  },
});
