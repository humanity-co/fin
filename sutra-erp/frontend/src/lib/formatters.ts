/**
 * Indian financial formatting utilities.
 * All monetary values are stored as paise (number) and displayed as rupees.
 */

const PAISE_PER_RUPEE = 100;
const LAKH = 100_000;
const CRORE = 10_000_000;

/**
 * Format a paise amount to Indian currency display string.
 * Handles zero, negative, and very large amounts.
 *
 * @param paise - Amount in paise (integer)
 * @returns Formatted string like "₹1,23,45,678.00"
 */
export function formatIndianCurrency(paise: number): string {
  if (paise === 0) return "—";

  const isNegative = paise < 0;
  const absPaise = Math.abs(paise);
  const rupees = absPaise / PAISE_PER_RUPEE;

  const whole = Math.floor(rupees);
  const fraction = Math.round((rupees - whole) * 100);

  const wholeStr = toIndianNumberString(whole);

  const sign = isNegative ? "−" : "";
  return `${sign}₹${wholeStr}.${fraction.toString().padStart(2, "0")}`;
}

/**
 * Format a paise amount to compact Indian currency display.
 * Uses K, L (lakhs), Cr (crores).
 *
 * @param paise - Amount in paise (integer)
 * @returns Formatted string like "₹1.5L" or "₹1.23 Cr"
 */
export function formatCompactCurrency(paise: number): string {
  if (paise === 0) return "—";

  const isNegative = paise < 0;
  const rupees = Math.abs(paise) / PAISE_PER_RUPEE;
  const sign = isNegative ? "−" : "";

  if (rupees < 1_000) {
    return `${sign}₹${rupees.toFixed(rupees % 1 === 0 ? 0 : 1)}`;
  }

  if (rupees < LAKH) {
    const k = rupees / 1_000;
    return `${sign}₹${k.toFixed(1)}K`;
  }

  if (rupees < CRORE) {
    const l = rupees / LAKH;
    return `${sign}₹${l.toFixed(2)}L`;
  }

  const cr = rupees / CRORE;
  return `${sign}₹${cr.toFixed(2)} Cr`;
}

/**
 * Convert a number to Indian-style grouping with commas.
 * e.g., 12345678 → "1,23,45,678"
 */
export function toIndianNumberString(n: number): string {
  const s = n.toString();
  const len = s.length;

  if (len <= 3) return s;

  const last3 = s.slice(-3);
  const rest = s.slice(0, -3);

  // Add commas every 2 digits from the right
  const restGrouped = rest.replace(/\B(?=(\d{2})+(?!\d))/g, ",");

  return `${restGrouped},${last3}`;
}

/**
 * Format a number as a plain Indian-formatted number (no ₹).
 * e.g., 12345678 → "1,23,45,678"
 */
export function formatIndianNumber(n: number): string {
  if (n === 0) return "0";
  const isNegative = n < 0;
  const abs = Math.abs(n);
  const sign = isNegative ? "−" : "";
  return `${sign}${toIndianNumberString(Math.round(abs))}`;
}

/**
 * Format a date string to DD/MM/YYYY.
 */
export function formatIndianDate(date: Date | string): string {
  const d = typeof date === "string" ? new Date(date) : date;
  if (isNaN(d.getTime())) return "—";

  const day = d.getDate().toString().padStart(2, "0");
  const month = (d.getMonth() + 1).toString().padStart(2, "0");
  const year = d.getFullYear();

  return `${day}/${month}/${year}`;
}

/**
 * Format a date to short format: "21 Jul 2026"
 */
export function formatIndianDateShort(date: Date | string): string {
  const d = typeof date === "string" ? new Date(date) : date;
  if (isNaN(d.getTime())) return "—";

  const months = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
  ];

  return `${d.getDate()} ${months[d.getMonth()]} ${d.getFullYear()}`;
}

/**
 * Format a date to relative time: "2 days ago", "in 5 days"
 */
export function formatRelativeDate(date: Date | string): string {
  const d = typeof date === "string" ? new Date(date) : date;
  if (isNaN(d.getTime())) return "—";

  const now = new Date();
  const diffMs = d.getTime() - now.getTime();
  const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));

  if (Math.abs(diffDays) === 0) return "Today";
  if (diffDays < 0) {
    if (diffDays === -1) return "Yesterday";
    return `${Math.abs(diffDays)} days ago`;
  }
  if (diffDays === 1) return "Tomorrow";
  return `in ${diffDays} days`;
}

/**
 * Convert rupees (decimal) to paise (integer).
 */
export function rupeesToPaise(rupees: number): number {
  return Math.round(rupees * PAISE_PER_RUPEE);
}

/**
 * Convert paise (integer) to rupees (decimal).
 */
export function paiseToRupees(paise: number): number {
  return paise / PAISE_PER_RUPEE;
}

/**
 * Parse a PAN string: uppercase, trim, remove spaces.
 */
export function normalizePan(pan: string): string {
  return pan.replace(/\s/g, "").toUpperCase();
}

/**
 * Validate PAN format: 5 letters, 4 digits, 1 letter.
 */
export function isValidPan(pan: string): boolean {
  return /^[A-Z]{5}[0-9]{4}[A-Z]$/.test(normalizePan(pan));
}

/**
 * Validate GSTIN format: 15 characters.
 */
export function isValidGstin(gstin: string): boolean {
  return /^[0-9]{2}[A-Z]{5}[0-9]{4}[A-Z][1-9A-Z]Z[0-9A-Z]$/.test(
    gstin.replace(/\s/g, "").toUpperCase()
  );
}
