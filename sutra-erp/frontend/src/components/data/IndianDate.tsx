import { cn } from "../../lib/utils";
import { formatIndianDate, formatIndianDateShort, formatRelativeDate } from "../../lib/formatters";

interface IndianDateProps {
  date: Date | string;
  format?: "full" | "short" | "relative";
  className?: string;
}

export function IndianDate({
  date,
  format = "full",
  className,
}: IndianDateProps) {
  const d = typeof date === "string" ? new Date(date) : date;
  if (isNaN(d.getTime())) {
    return <span className={cn("text-muted-foreground", className)}>—</span>;
  }

  let display: string;
  switch (format) {
    case "short":
      display = formatIndianDateShort(d);
      break;
    case "relative":
      display = formatRelativeDate(d);
      break;
    case "full":
    default:
      display = formatIndianDate(d);
  }

  return (
    <time
      dateTime={d.toISOString()}
      className={cn("whitespace-nowrap tabular-nums", className)}
    >
      {display}
    </time>
  );
}
