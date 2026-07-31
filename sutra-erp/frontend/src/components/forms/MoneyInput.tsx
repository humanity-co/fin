import { forwardRef, useCallback } from "react";
import { cn } from "../../lib/utils";
import { rupeesToPaise } from "../../lib/formatters";

interface MoneyInputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "value" | "onChange"> {
  /** Value in paise */
  value: number;
  /** Called with value in paise */
  onChange: (paise: number) => void;
  error?: string;
}

/**
 * ₹-prefixed money input.
 * Displays rupees (decimal) to the user, stores paise (integer).
 */
export const MoneyInput = forwardRef<HTMLInputElement, MoneyInputProps>(
  ({ value, onChange, error, className, disabled, ...props }, ref) => {
    const displayValue = value ? (value / 100).toString() : "";

    const handleChange = useCallback(
      (e: React.ChangeEvent<HTMLInputElement>) => {
        const raw = e.target.value;
        // Allow empty input
        if (raw === "" || raw === "-") {
          onChange(0);
          return;
        }
        // Parse as rupees and convert to paise
        const parsed = parseFloat(raw);
        if (!isNaN(parsed)) {
          onChange(rupeesToPaise(parsed));
        }
      },
      [onChange]
    );

    return (
      <div className="relative">
        <span
          className={cn(
            "absolute left-3 top-1/2 -translate-y-1/2 text-sm font-medium pointer-events-none",
            disabled ? "text-muted-foreground/50" : "text-muted-foreground"
          )}
        >
          ₹
        </span>
        <input
          ref={ref}
          type="number"
          step="0.01"
          min="0"
          inputMode="decimal"
          value={displayValue}
          onChange={handleChange}
          disabled={disabled}
          className={cn(
            "flex h-10 w-full rounded-md border border-input bg-background pl-8 pr-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 tabular-nums text-right",
            error && "border-destructive focus-visible:ring-destructive",
            className
          )}
          {...props}
        />
        {error && (
          <p className="mt-1 text-xs text-destructive">{error}</p>
        )}
      </div>
    );
  }
);

MoneyInput.displayName = "MoneyInput";
