import { cn } from "../../lib/utils";
import { Badge } from "../ui/badge";

// ── Status color maps ──────────────────────────

type StatusVariant = "default" | "secondary" | "destructive" | "outline" | "success" | "warning" | "info";

const STATUS_STYLES: Record<string, StatusVariant> = {
  // Journal statuses
  Draft: "secondary",
  Posted: "success",
  Reversed: "destructive",
  Cancelled: "destructive",

  // Payment/Receipt statuses
  Pending: "warning",
  Completed: "success",
  Failed: "destructive",
  Refunded: "info",
  Uncleared: "warning",
  Bounced: "destructive",

  // Approval statuses
  Approved: "success",
  Rejected: "destructive",
  Submitted: "info",

  // Invoice statuses
  Paid: "success",
  Overdue: "destructive",
  PartiallyPaid: "warning",

  // Scholarship statuses
  Applied: "info",
  Verified: "secondary",
  Sanctioned: "success",
  Disbursed: "success",
  PartiallyDisbursed: "warning",
  Closed: "default",

  // Compliance/Filing statuses
  Filed: "success",
  "Not Filed": "destructive",
  "Due Soon": "warning",

  // Generic
  Active: "success",
  Inactive: "secondary",
  Expired: "destructive",
  Upcoming: "info",
};

interface StatusBadgeProps {
  status: string;
  className?: string;
  /** Override auto-detected variant */
  variant?: StatusVariant;
}

export function StatusBadge({ status, className, variant }: StatusBadgeProps) {
  const resolvedVariant = variant || STATUS_STYLES[status] || "default";

  return (
    <Badge
      variant={resolvedVariant}
      className={cn("whitespace-nowrap", className)}
    >
      {status}
    </Badge>
  );
}

/**
 * Get the variant for a given status string, for use in other components.
 */
export function getStatusVariant(status: string): StatusVariant {
  return STATUS_STYLES[status] || "default";
}
