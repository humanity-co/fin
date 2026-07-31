import { cn } from "../../lib/utils";
import { formatIndianCurrency, formatCompactCurrency } from "../../lib/formatters";

interface MoneyDisplayProps {
  /** Amount in paise (integer) */
  amount: number;
  /** Display variant */
  variant?: "default" | "compact" | "accounting";
  /** Custom class for the container */
  className?: string;
  /** Show currency symbol */
  showCurrency?: boolean;
}

/**
 * Signature component: displays monetary values with Indian formatting.
 *
 * - Zero renders as "—"
 * - Negative amounts are shown in red
 * - "accounting" variant shows negative in parentheses without minus sign
 * - "compact" variant uses L (lakhs) and Cr (crores)
 */
export function MoneyDisplay({
  amount,
  variant = "default",
  className,
  showCurrency = true,
}: MoneyDisplayProps) {
  if (amount === 0) {
    return (
      <span className={cn("text-muted-foreground", className)} aria-label="Zero rupees">
        —
      </span>
    );
  }

  const isNegative = amount < 0;
  const absAmount = Math.abs(amount);

  let display: string;

  if (variant === "compact") {
    // Remove ₹ prefix if not showing currency
    const formatted = formatCompactCurrency(absAmount);
    display = showCurrency ? formatted : formatted.replace("₹", "");
  } else {
    const formatted = formatIndianCurrency(absAmount);
    display = showCurrency ? formatted : formatted.replace("₹", "");
  }

  if (variant === "accounting" && isNegative) {
    return (
      <span
        className={cn("tabular-nums text-destructive", className)}
        aria-label={`Negative ${absAmount} rupees`}
      >
        ({display})
      </span>
    );
  }

  return (
    <span
      className={cn(
        "tabular-nums",
        isNegative && "text-destructive",
        className
      )}
      aria-label={`${isNegative ? "Negative " : ""}${absAmount} rupees`}
    >
      {isNegative ? `−${display}` : display}
    </span>
  );
}
