/**
 * Application-wide constants for SutraERP.
 */

// GST tax rates as per Indian GST
export const GST_RATES = {
  EXEMPT: 0,
  NIL: 0,
  TAXABLE_5: 5,
  TAXABLE_12: 12,
  TAXABLE_18: 18,
  TAXABLE_28: 28,
} as const;

export const GST_RATE_LABELS: Record<number, string> = {
  0: "Exempt / Nil",
  5: "5%",
  12: "12%",
  18: "18%",
  28: "28%",
};

// TDS sections commonly used in educational institutions
export const TDS_SECTIONS = [
  { section: "194C", description: "Payment to Contractors", rate: 1 },
  { section: "194H", description: "Commission/Brokerage", rate: 5 },
  { section: "194I", description: "Rent", rate: 2 },
  { section: "194J", description: "Professional/Technical Services", rate: 10 },
  { section: "194A", description: "Interest (other than securities)", rate: 10 },
  { section: "194D", description: "Insurance Commission", rate: 5 },
  { section: "194G", description: "Commission on Lottery Tickets", rate: 5 },
] as const;

// Payment modes
export const PAYMENT_MODES = [
  "Cash",
  "Cheque",
  "Demand Draft",
  "NEFT",
  "RTGS",
  "IMPS",
  "UPI",
  "Credit Card",
  "Debit Card",
  "POS",
  "Payment Gateway",
] as const;

// Account types
export const ACCOUNT_TYPES = [
  "Asset",
  "Liability",
  "Equity",
  "Income",
  "Expense",
] as const;

// Journal types
export const JOURNAL_TYPES = [
  "Standard",
  "Reversing",
  "Adjustment",
  "Opening",
  "Closing",
  "RCM",
  "ITC Reversal",
  "TDS",
  "Accrual",
  "Prepayment",
] as const;

// Journal statuses
export const JOURNAL_STATUSES = [
  "Draft",
  "Posted",
  "Reversed",
  "Cancelled",
] as const;

// Fiscal year pattern
export const INDIAN_FISCAL_YEAR_START_MONTH = 3; // April (0-indexed)
export const INDIAN_FISCAL_YEAR_END_MONTH = 2; // March

// Pagination defaults
export const DEFAULT_PAGE_SIZE = 25;
export const PAGE_SIZE_OPTIONS = [10, 25, 50, 100];

// React Query defaults
export const QUERY_STALE_TIME = 30_000; // 30 seconds
export const QUERY_RETRY_COUNT = 2;
