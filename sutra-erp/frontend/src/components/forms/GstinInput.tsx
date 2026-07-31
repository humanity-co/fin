import { forwardRef, useCallback } from "react";
import { cn } from "../../lib/utils";
import { isValidGstin } from "../../lib/formatters";

interface GstinInputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "onChange"> {
  value: string;
  onChange: (value: string) => void;
  error?: string;
}

export const GstinInput = forwardRef<HTMLInputElement, GstinInputProps>(
  ({ value, onChange, error, className, ...props }, ref) => {
    const normalized = value.replace(/\s/g, "").toUpperCase();
    const valid = normalized.length === 15 ? isValidGstin(normalized) : null;

    const handleChange = useCallback(
      (e: React.ChangeEvent<HTMLInputElement>) => {
        const raw = e.target.value.replace(/\s/g, "").toUpperCase();
        onChange(raw);
      },
      [onChange]
    );

    return (
      <div className="relative">
        <input
          ref={ref}
          type="text"
          maxLength={15}
          value={value}
          onChange={handleChange}
          placeholder="22AAAAA0000A1Z5"
          className={cn(
            "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm font-mono uppercase tracking-wider ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
            error && "border-destructive focus-visible:ring-destructive",
            valid === true && "border-emerald-500",
            valid === false && "border-destructive",
            className
          )}
          {...props}
        />
        {valid === true && (
          <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-emerald-600">
            ✓ Verified
          </span>
        )}
        {error && <p className="mt-1 text-xs text-destructive">{error}</p>}
      </div>
    );
  }
);

GstinInput.displayName = "GstinInput";
