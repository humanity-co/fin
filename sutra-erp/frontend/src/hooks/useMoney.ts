import { useCallback, useMemo } from "react";
import { rupeesToPaise, paiseToRupees, formatIndianCurrency, formatCompactCurrency } from "../lib/formatters";

/**
 * Hook for working with monetary values.
 * Converts between rupees (display) and paise (storage) formats.
 */
export function useMoney() {
  const toPaise = useCallback((rupees: number) => rupeesToPaise(rupees), []);
  const toRupees = useCallback((paise: number) => paiseToRupees(paise), []);
  const format = useCallback(
    (paise: number, compact?: boolean) =>
      compact ? formatCompactCurrency(paise) : formatIndianCurrency(paise),
    []
  );

  return useMemo(
    () => ({ toPaise, toRupees, format }),
    [toPaise, toRupees, format]
  );
}
