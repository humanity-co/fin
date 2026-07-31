import { forwardRef, useCallback } from "react";
import { cn } from "../../lib/utils";
import { normalizePan, isValidPan } from "../../lib/formatters";

interface PanInputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "onChange"> {
  value: string;
  onChange: (value: string) => void;
  error?: string;
}

export const PanInput = forwardRef<HTMLInputElement, PanInputProps>(
  ({ value, onChange, error, className, ...props }, ref) => {
    const valid = value ? isValidPan(value) : null;

    const handleChange = useCallback(
      (e: React.ChangeEvent<HTMLInputElement>) => {
        onChange(normalizePan(e.target.value));
      },
      [onChange]
    );

    return (
      <div className="relative">
        <input
          ref={ref}
          type="text"
          maxLength={10}
          value={value}
          onChange={handleChange}
          className={cn(
            "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm font-mono uppercase tracking-wider ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
            error && "border-destructive focus-visible:ring-destructive",
            valid === true && "border-emerald-500",
            valid === false && value.length === 10 && "border-destructive",
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

PanInput.displayName = "PanInput";
