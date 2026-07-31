import { useMemo } from "react";
import { getTenantId } from "../lib/api-client";

/**
 * Hook for accessing tenant context.
 * In v1, the tenant ID is set via the API client.
 */
export function useTenant() {
  const tenantId = getTenantId();

  return useMemo(
    () => ({
      tenantId,
      isReady: !!tenantId,
    }),
    [tenantId]
  );
}
