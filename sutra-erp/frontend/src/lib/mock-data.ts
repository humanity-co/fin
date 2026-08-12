/**
 * SutraERP — Realistic mock data for UI development.
 *
 * All monetary values are in PAISE (integer) per `lib/formatters.ts` —
 * use the `rs()` helper to express rupee amounts, e.g. rs(125000) = ₹1,25,000.
 * Dates are ISO strings (YYYY-MM-DD); display with `formatIndianDate`.
 *
 * This module exists so every screen renders a believable, dense dataset
 * until the live API is wired in. Replace with server data incrementally.
 */
import { rupeesToPaise as rs } from "./formatters";

// ── General Ledger ──────────────────────────────────────────────────────

export type AccountType = "Asset" | "Liability" | "Equity" | "Income" | "Expense";

export interface Account {
  account_id: string;
  account_code: string;
  account_name: string;
  account_type: AccountType;
  parent_account_id: string | null;
  is_active: boolean;
  /** Balance in paise — positive = debit balance (assets/expenses), negative = credit balance */
  current_balance: number;
  gst_applicable: boolean;
  hsn_sac?: string;
  aishe_mapped?: boolean;
  naac_mapped?: boolean;
  opening_balance?: number;
}

export const COA_ACCOUNTS: Account[] = [
  // ── Assets (1000) ──
  { account_id: "acc-1000", account_code: "1000", account_name: "ASSETS", account_type: "Asset", parent_account_id: null, is_active: true, current_balance: 0, gst_applicable: false },
  { account_id: "acc-1100", account_code: "1100", account_name: "Current Assets", account_type: "Asset", parent_account_id: "acc-1000", is_active: true, current_balance: 0, gst_applicable: false },
  { account_id: "acc-1110", account_code: "1110", account_name: "Bank A/c — SBI Current", account_type: "Asset", parent_account_id: "acc-1100", is_active: true, current_balance: rs(2845000), gst_applicable: false },
  { account_id: "acc-1111", account_code: "1111", account_name: "Bank A/c — HDFC", account_type: "Asset", parent_account_id: "acc-1100", is_active: true, current_balance: rs(1287500), gst_applicable: false },
  { account_id: "acc-1112", account_code: "1112", account_name: "Bank A/c — ICICI (Fees)", account_type: "Asset", parent_account_id: "acc-1100", is_active: true, current_balance: rs(6210000), gst_applicable: false },
  { account_id: "acc-1120", account_code: "1120", account_name: "Cash in Hand", account_type: "Asset", parent_account_id: "acc-1100", is_active: true, current_balance: rs(48500), gst_applicable: false },
  { account_id: "acc-1130", account_code: "1130", account_name: "Prepaid Expenses", account_type: "Asset", parent_account_id: "acc-1100", is_active: true, current_balance: rs(440000), gst_applicable: false },
  { account_id: "acc-1200", account_code: "1200", account_name: "Tuition Fee Receivable", account_type: "Asset", parent_account_id: "acc-1100", is_active: true, current_balance: rs(3420000), gst_applicable: false },
  { account_id: "acc-1210", account_code: "1210", account_name: "Hostel Fee Receivable", account_type: "Asset", parent_account_id: "acc-1100", is_active: true, current_balance: rs(1120000), gst_applicable: false },
  { account_id: "acc-1300", account_code: "1300", account_name: "Security Deposits (Refundable)", account_type: "Asset", parent_account_id: "acc-1100", is_active: true, current_balance: rs(860000), gst_applicable: false },
  { account_id: "acc-1400", account_code: "1400", account_name: "Fixed Assets", account_type: "Asset", parent_account_id: "acc-1000", is_active: true, current_balance: rs(18450000), gst_applicable: true, hsn_sac: "9987" },
  { account_id: "acc-1410", account_code: "1410", account_name: "Computers & Peripherals", account_type: "Asset", parent_account_id: "acc-1400", is_active: true, current_balance: rs(6200000), gst_applicable: true, hsn_sac: "8471" },
  { account_id: "acc-1420", account_code: "1420", account_name: "Furniture & Fixtures", account_type: "Asset", parent_account_id: "acc-1400", is_active: true, current_balance: rs(3800000), gst_applicable: true, hsn_sac: "9403" },
  // ── Liabilities (2000) ──
  { account_id: "acc-2000", account_code: "2000", account_name: "LIABILITIES", account_type: "Liability", parent_account_id: null, is_active: true, current_balance: 0, gst_applicable: false },
  { account_id: "acc-2100", account_code: "2100", account_name: "Sundry Creditors (Vendor Payables)", account_type: "Liability", parent_account_id: "acc-2000", is_active: true, current_balance: -rs(1845000), gst_applicable: false },
  { account_id: "acc-2200", account_code: "2200", account_name: "TDS Payable", account_type: "Liability", parent_account_id: "acc-2000", is_active: true, current_balance: -rs(248000), gst_applicable: false },
  { account_id: "acc-2210", account_code: "2210", account_name: "GST Payable (Output)", account_type: "Liability", parent_account_id: "acc-2000", is_active: true, current_balance: -rs(415000), gst_applicable: false },
  { account_id: "acc-2220", account_code: "2220", account_name: "GST Input Credit (ITC)", account_type: "Liability", parent_account_id: "acc-2000", is_active: true, current_balance: -rs(298000), gst_applicable: false },
  { account_id: "acc-2300", account_code: "2300", account_name: "Scholarship Payable", account_type: "Liability", parent_account_id: "acc-2000", is_active: true, current_balance: -rs(760000), gst_applicable: false },
  { account_id: "acc-2400", account_code: "2400", account_name: "Caution Deposit Payable", account_type: "Liability", parent_account_id: "acc-2000", is_active: true, current_balance: -rs(860000), gst_applicable: false },
  { account_id: "acc-2500", account_code: "2500", account_name: "Salary Payable", account_type: "Liability", parent_account_id: "acc-2000", is_active: true, current_balance: -rs(1240000), gst_applicable: false },
  // ── Equity / Funds (3000) ──
  { account_id: "acc-3000", account_code: "3000", account_name: "FUNDS & RESERVES", account_type: "Equity", parent_account_id: null, is_active: true, current_balance: 0, gst_applicable: false },
  { account_id: "acc-3100", account_code: "3100", account_name: "General Corpus Fund", account_type: "Equity", parent_account_id: "acc-3000", is_active: true, current_balance: -rs(28500000), gst_applicable: false },
  { account_id: "acc-3200", account_code: "3200", account_name: "Development Fund (FRC)", account_type: "Equity", parent_account_id: "acc-3000", is_active: true, current_balance: -rs(8600000), gst_applicable: false },
  { account_id: "acc-3300", account_code: "3300", account_name: "Research & Innovation Fund", account_type: "Equity", parent_account_id: "acc-3000", is_active: true, current_balance: -rs(1450000), gst_applicable: false },
  // ── Income (4000) ──
  { account_id: "acc-4000", account_code: "4000", account_name: "INCOME", account_type: "Income", parent_account_id: null, is_active: true, current_balance: 0, gst_applicable: false },
  { account_id: "acc-4100", account_code: "4100", account_name: "Tuition Fee Income", account_type: "Income", parent_account_id: "acc-4000", is_active: true, current_balance: -rs(42800000), gst_applicable: false, aishe_mapped: true, naac_mapped: true },
  { account_id: "acc-4110", account_code: "4110", account_name: "Development Fee Income", account_type: "Income", parent_account_id: "acc-4000", is_active: true, current_balance: -rs(9200000), gst_applicable: false, naac_mapped: true },
  { account_id: "acc-4120", account_code: "4120", account_name: "Examination Fee Income", account_type: "Income", parent_account_id: "acc-4000", is_active: true, current_balance: -rs(2860000), gst_applicable: false },
  { account_id: "acc-4130", account_code: "4130", account_name: "Hostel & Mess Fee Income", account_type: "Income", parent_account_id: "acc-4000", is_active: true, current_balance: -rs(11400000), gst_applicable: false },
  { account_id: "acc-4200", account_code: "4200", account_name: "Grant Income (State)", account_type: "Income", parent_account_id: "acc-4000", is_active: true, current_balance: -rs(7800000), gst_applicable: false, aishe_mapped: true },
  { account_id: "acc-4210", account_code: "4210", account_name: "Grant Income (Central/UGC)", account_type: "Income", parent_account_id: "acc-4000", is_active: true, current_balance: -rs(4600000), gst_applicable: false, aishe_mapped: true },
  { account_id: "acc-4300", account_code: "4300", account_name: "Interest Income (FDs)", account_type: "Income", parent_account_id: "acc-4000", is_active: true, current_balance: -rs(1180000), gst_applicable: false },
  { account_id: "acc-4400", account_code: "4400", account_name: "Consultancy Income", account_type: "Income", parent_account_id: "acc-4000", is_active: true, current_balance: -rs(940000), gst_applicable: true, hsn_sac: "9983", naac_mapped: true },
  // ── Expenses (5000) ──
  { account_id: "acc-5000", account_code: "5000", account_name: "EXPENSES", account_type: "Expense", parent_account_id: null, is_active: true, current_balance: 0, gst_applicable: false },
  { account_id: "acc-5100", account_code: "5100", account_name: "Faculty Salaries & Wages", account_type: "Expense", parent_account_id: "acc-5000", is_active: true, current_balance: rs(24600000), gst_applicable: false, aishe_mapped: true, naac_mapped: true },
  { account_id: "acc-5110", account_code: "5110", account_name: "Non-Teaching Staff Salaries", account_type: "Expense", parent_account_id: "acc-5000", is_active: true, current_balance: rs(9800000), gst_applicable: false },
  { account_id: "acc-5200", account_code: "5200", account_name: "Electricity & Utilities", account_type: "Expense", parent_account_id: "acc-5000", is_active: true, current_balance: rs(2650000), gst_applicable: true, hsn_sac: "2716" },
  { account_id: "acc-5210", account_code: "5210", account_name: "Internet & Communication", account_type: "Expense", parent_account_id: "acc-5000", is_active: true, current_balance: rs(620000), gst_applicable: true, hsn_sac: "9984" },
  { account_id: "acc-5300", account_code: "5300", account_name: "Library Books & Journals", account_type: "Expense", parent_account_id: "acc-5000", is_active: true, current_balance: rs(980000), gst_applicable: true, hsn_sac: "4901", naac_mapped: true },
  { account_id: "acc-5400", account_code: "5400", account_name: "Repairs & Maintenance", account_type: "Expense", parent_account_id: "acc-5000", is_active: true, current_balance: rs(1450000), gst_applicable: true, hsn_sac: "9987" },
  { account_id: "acc-5500", account_code: "5500", account_name: "Scholarship & Concession Expense", account_type: "Expense", parent_account_id: "acc-5000", is_active: true, current_balance: rs(3600000), gst_applicable: false, naac_mapped: true },
  { account_id: "acc-5600", account_code: "5600", account_name: "Office & Stationery Expenses", account_type: "Expense", parent_account_id: "acc-5000", is_active: true, current_balance: rs(420000), gst_applicable: true, hsn_sac: "4820" },
  { account_id: "acc-5700", account_code: "5700", account_name: "Exam Conduct Expenses", account_type: "Expense", parent_account_id: "acc-5000", is_active: true, current_balance: rs(1900000), gst_applicable: false },
  { account_id: "acc-5800", account_code: "5800", account_name: "Audit & Professional Fees", account_type: "Expense", parent_account_id: "acc-5000", is_active: true, current_balance: rs(780000), gst_applicable: true, hsn_sac: "9982" },
];

export type JournalStatus = "Draft" | "Posted" | "Reversed" | "Cancelled";
export type JournalType =
  | "Standard" | "Adjustment" | "RCM" | "TDS" | "Accrual"
  | "Reversing" | "ITC Reversal" | "Prepayment" | "Opening" | "Closing";

export interface JournalLine {
  journal_line_id: string;
  line_number: number;
  account_id: string;
  account_name: string;
  account_code: string;
  debit_amount: number | null; // paise
  credit_amount: number | null; // paise
  description: string;
}

export interface Journal {
  journal_id: string;
  journal_number: string; // JV-2026-0001
  journal_date: string; // ISO
  journal_type: JournalType;
  description: string;
  status: JournalStatus;
  total_amount: number; // paise (either side)
  reference?: string;
  created_by: string;
  lines: JournalLine[];
}

const jl = (
  id: string, n: number, accountId: string, dr: number | null, cr: number | null, desc: string
): JournalLine => {
  const acct = COA_ACCOUNTS.find((a) => a.account_id === accountId)!;
  return {
    journal_line_id: id, line_number: n, account_id: accountId,
    account_name: acct.account_name, account_code: acct.account_code,
    debit_amount: dr, credit_amount: cr, description: desc,
  };
};

export const MOCK_JOURNALS: Journal[] = [
  {
    journal_id: "jv-001", journal_number: "JV-2026-0001", journal_date: "2026-04-01",
    journal_type: "Opening", description: "Opening entry for FY 2026-27 — brought forward balances",
    status: "Posted", total_amount: rs(28500000), created_by: "Meena Kulkarni (Accountant)",
    lines: [
      jl("jl-001-1", 1, "acc-1110", rs(2845000), null, "Bank balance brought forward"),
      jl("jl-001-2", 2, "acc-1200", rs(3420000), null, "Fee receivable brought forward"),
      jl("jl-001-3", 3, "acc-3100", null, rs(28500000), "Corpus fund brought forward"),
    ],
  },
  {
    journal_id: "jv-002", journal_number: "JV-2026-0002", journal_date: "2026-04-10",
    journal_type: "Standard", description: "Tuition fee collection deposited into ICICI fee account",
    status: "Posted", total_amount: rs(1850000), reference: "RCPT-2026-0142", created_by: "Meena Kulkarni (Accountant)",
    lines: [
      jl("jl-002-1", 1, "acc-1112", rs(1850000), null, "Fees collected via UPI/NEFT"),
      jl("jl-002-2", 2, "acc-4100", null, rs(1850000), "Tuition fee income recognised"),
    ],
  },
  {
    journal_id: "jv-003", journal_number: "JV-2026-0003", journal_date: "2026-04-15",
    journal_type: "TDS", description: "TDS on professional fees — TechServe IT Solutions (194J)",
    status: "Posted", total_amount: rs(18500), reference: "INV-TS-2026-011", created_by: "Rajesh Iyer (Accountant)",
    lines: [
      jl("jl-003-1", 1, "acc-5800", rs(185000), null, "Professional fees (gross)"),
      jl("jl-003-2", 2, "acc-2200", null, rs(18500), "TDS @10% u/s 194J"),
      jl("jl-003-3", 3, "acc-2100", null, rs(166500), "Net payable to vendor"),
    ],
  },
  {
    journal_id: "jv-004", journal_number: "JV-2026-0004", journal_date: "2026-04-20",
    journal_type: "RCM", description: "RCM on legal services under reverse charge — GST 18%",
    status: "Posted", total_amount: rs(216000), reference: "INV-LEGAL-008", created_by: "Rajesh Iyer (Accountant)",
    lines: [
      jl("jl-004-1", 1, "acc-5800", rs(1200000), null, "Legal retainer fees"),
      jl("jl-004-2", 2, "acc-2220", rs(216000), null, "RCM ITC availed @18%"),
      jl("jl-004-3", 3, "acc-2210", null, rs(216000), "RCM GST payable"),
      jl("jl-004-4", 4, "acc-2100", null, rs(1200000), "Vendor payable — Legal Associates"),
    ],
  },
  {
    journal_id: "jv-005", journal_number: "JV-2026-0005", journal_date: "2026-05-02",
    journal_type: "Standard", description: "Hostel mess advance received from students",
    status: "Posted", total_amount: rs(640000), reference: "RCPT-2026-0231", created_by: "Meena Kulkarni (Accountant)",
    lines: [
      jl("jl-005-1", 1, "acc-1110", rs(640000), null, "Mess advance via NEFT"),
      jl("jl-005-2", 2, "acc-4130", null, rs(640000), "Hostel & mess fee income"),
    ],
  },
  {
    journal_id: "jv-006", journal_number: "JV-2026-0006", journal_date: "2026-05-15",
    journal_type: "Accrual", description: "May salary accrual — teaching faculty",
    status: "Draft", total_amount: rs(2150000), created_by: "Meena Kulkarni (Accountant)",
    lines: [
      jl("jl-006-1", 1, "acc-5100", rs(2150000), null, "Faculty salaries for May"),
      jl("jl-006-2", 2, "acc-2500", null, rs(2150000), "Salary payable"),
    ],
  },
  {
    journal_id: "jv-007", journal_number: "JV-2026-0007", journal_date: "2026-05-20",
    journal_type: "Adjustment", description: "Reversal of excess ITC claimed in April (Rule 42/43)",
    status: "Posted", total_amount: rs(42500), created_by: "Rajesh Iyer (Accountant)",
    lines: [
      jl("jl-007-1", 1, "acc-2210", rs(42500), null, "ITC reversal — common credit"),
      jl("jl-007-2", 2, "acc-2220", null, rs(42500), "Input credit reversed"),
    ],
  },
  {
    journal_id: "jv-008", journal_number: "JV-2026-0008", journal_date: "2026-06-01",
    journal_type: "Standard", description: "Purchase of laboratory equipment from ABC Scientific Supplies",
    status: "Posted", total_amount: rs(1121000), reference: "PO-2026-0042", created_by: "Rajesh Iyer (Accountant)",
    lines: [
      jl("jl-008-1", 1, "acc-1410", rs(950000), null, "Spectrophotometer + centrifuge"),
      jl("jl-008-2", 2, "acc-2220", rs(171000), null, "ITC @18%"),
      jl("jl-008-3", 3, "acc-2100", null, rs(1121000), "ABC Scientific Supplies — payable"),
    ],
  },
  {
    journal_id: "jv-009", journal_number: "JV-2026-0009", journal_date: "2026-06-05",
    journal_type: "Prepayment", description: "Annual insurance premium paid upfront (12 months)",
    status: "Posted", total_amount: rs(480000), reference: "INS-2026-77", created_by: "Meena Kulkarni (Accountant)",
    lines: [
      jl("jl-009-1", 1, "acc-1130", rs(480000), null, "Insurance premium — prepaid (Apr 26–Mar 27)"),
      jl("jl-009-2", 2, "acc-1110", null, rs(480000), "Paid by NEFT to United India Insurance"),
    ],
  },
  {
    journal_id: "jv-010", journal_number: "JV-2026-0010", journal_date: "2026-06-12",
    journal_type: "Reversing", description: "Reversal of May salary accrual (auto-reverse)",
    status: "Posted", total_amount: rs(2150000), created_by: "System (Auto-reverse)",
    lines: [
      jl("jl-010-1", 1, "acc-2500", rs(2150000), null, "Reversal of salary payable"),
      jl("jl-010-2", 2, "acc-5100", null, rs(2150000), "Reversal of salary accrual"),
    ],
  },
  {
    journal_id: "jv-011", journal_number: "JV-2026-0011", journal_date: "2026-06-18",
    journal_type: "Standard", description: "Scholarship adjustment — Rajarshi Shahu Maharaj Merit Scholarship",
    status: "Draft", total_amount: rs(240000), reference: "SCH-2026-088", created_by: "Priya Deshmukh (Accountant)",
    lines: [
      jl("jl-011-1", 1, "acc-5500", rs(240000), null, "Merit scholarship for 12 students"),
      jl("jl-011-2", 2, "acc-2300", null, rs(240000), "Scholarship payable to students"),
    ],
  },
  {
    journal_id: "jv-012", journal_number: "JV-2026-0012", journal_date: "2026-06-20",
    journal_type: "Closing", description: "Provisional monthly closing — June 2026",
    status: "Cancelled", total_amount: rs(0), created_by: "Meena Kulkarni (Accountant)",
    lines: [
      jl("jl-012-1", 1, "acc-3000", rs(0), null, "Cancelled — period not yet closed"),
    ],
  },
];

// ── Trial Balance ───────────────────────────────────────────────────────

export interface TrialBalanceRow {
  accountId: string;
  accountCode: string;
  accountName: string;
  accountType: AccountType;
  openingDebit: number;
  openingCredit: number;
  periodDebit: number;
  periodCredit: number;
  closingDebit: number;
  closingCredit: number;
}

export const MOCK_TRIAL_BALANCE: TrialBalanceRow[] = [
  { accountId: "acc-1110", accountCode: "1110", accountName: "Bank A/c — SBI Current", accountType: "Asset", openingDebit: rs(2620000), openingCredit: 0, periodDebit: rs(3480000), periodCredit: rs(3255000), closingDebit: rs(2845000), closingCredit: 0 },
  { accountId: "acc-1111", accountCode: "1111", accountName: "Bank A/c — HDFC", accountType: "Asset", openingDebit: rs(1190000), openingCredit: 0, periodDebit: rs(1270000), periodCredit: rs(1172500), closingDebit: rs(1287500), closingCredit: 0 },
  { accountId: "acc-1112", accountCode: "1112", accountName: "Bank A/c — ICICI (Fees)", accountType: "Asset", openingDebit: rs(4180000), openingCredit: 0, periodDebit: rs(6870000), periodCredit: rs(4840000), closingDebit: rs(6210000), closingCredit: 0 },
  { accountId: "acc-1120", accountCode: "1120", accountName: "Cash in Hand", accountType: "Asset", openingDebit: rs(62000), openingCredit: 0, periodDebit: rs(184000), periodCredit: rs(197500), closingDebit: rs(48500), closingCredit: 0 },
  { accountId: "acc-1200", accountCode: "1200", accountName: "Tuition Fee Receivable", accountType: "Asset", openingDebit: rs(2980000), openingCredit: 0, periodDebit: rs(3940000), periodCredit: rs(3500000), closingDebit: rs(3420000), closingCredit: 0 },
  { accountId: "acc-2100", accountCode: "2100", accountName: "Sundry Creditors", accountType: "Liability", openingDebit: 0, openingCredit: rs(2120000), periodDebit: rs(2540000), periodCredit: rs(2265000), closingDebit: 0, closingCredit: rs(1845000) },
  { accountId: "acc-2200", accountCode: "2200", accountName: "TDS Payable", accountType: "Liability", openingDebit: 0, openingCredit: rs(214000), periodDebit: rs(148000), periodCredit: rs(182000), closingDebit: 0, closingCredit: rs(248000) },
  { accountId: "acc-2210", accountCode: "2210", accountName: "GST Payable (Output)", accountType: "Liability", openingDebit: 0, openingCredit: rs(372000), periodDebit: rs(301000), periodCredit: rs(344000), closingDebit: 0, closingCredit: rs(415000) },
  { accountId: "acc-2300", accountCode: "2300", accountName: "Scholarship Payable", accountType: "Liability", openingDebit: 0, openingCredit: rs(520000), periodDebit: rs(210000), periodCredit: rs(450000), closingDebit: 0, closingCredit: rs(760000) },
  { accountId: "acc-3100", accountCode: "3100", accountName: "General Corpus Fund", accountType: "Equity", openingDebit: 0, openingCredit: rs(28500000), periodDebit: 0, periodCredit: 0, closingDebit: 0, closingCredit: rs(28500000) },
  { accountId: "acc-4100", accountCode: "4100", accountName: "Tuition Fee Income", accountType: "Income", openingDebit: 0, openingCredit: 0, periodDebit: rs(800000), periodCredit: rs(43600000), closingDebit: 0, closingCredit: rs(42800000) },
  { accountId: "acc-5100", accountCode: "5100", accountName: "Faculty Salaries & Wages", accountType: "Expense", openingDebit: 0, openingCredit: 0, periodDebit: rs(24600000), periodCredit: 0, closingDebit: rs(24600000), closingCredit: 0 },
];

// ── Account Ledger ──────────────────────────────────────────────────────

export interface LedgerEntry {
  entry_id: string;
  date: string;
  voucher: string; // JV number or receipt/payment ref
  particular: string;
  debit: number; // paise
  credit: number; // paise
  balance: number; // running balance in paise (positive debit)
}

export const MOCK_LEDGER: Record<string, LedgerEntry[]> = {
  "acc-1110": [
    { entry_id: "le-1", date: "2026-04-01", voucher: "JV-2026-0001", particular: "To Balance b/d", debit: rs(2620000), credit: 0, balance: rs(2620000) },
    { entry_id: "le-2", date: "2026-05-02", voucher: "RCPT-2026-0231", particular: "To Hostel & Mess Fee A/c (mess advance)", debit: rs(640000), credit: 0, balance: rs(3260000) },
    { entry_id: "le-3", date: "2026-05-25", voucher: "PMT-2026-0101", particular: "By ABC Scientific Supplies — lab glassware", debit: 0, credit: rs(185000), balance: rs(3075000) },
    { entry_id: "le-4", date: "2026-06-05", voucher: "PMT-2026-0109", particular: "By GreenLeaf Catering — mess vendor May", debit: 0, credit: rs(420000), balance: rs(2655000) },
    { entry_id: "le-5", date: "2026-06-10", voucher: "PMT-2026-0112", particular: "By Nashik Stationery Mart — exam stationery", debit: 0, credit: rs(128000), balance: rs(2527000) },
    { entry_id: "le-6", date: "2026-06-15", voucher: "RCPT-2026-0318", particular: "To Hostel & Mess Fee A/c (advance)", debit: rs(284000), credit: 0, balance: rs(2811000) },
    { entry_id: "le-7", date: "2026-06-18", voucher: "PMT-2026-0117", particular: "By TDS Payable (u/s 194C, 194J)", debit: 0, credit: rs(248000), balance: rs(2563000) },
    { entry_id: "le-8", date: "2026-06-25", voucher: "RCPT-2026-0350", particular: "To Grant Income — State (DBT)", debit: rs(282000), credit: 0, balance: rs(2845000) },
  ],
  "acc-4100": [
    { entry_id: "le-1", date: "2026-04-10", voucher: "RCPT-2026-0142", particular: "By Tuition fees collected (B.Tech CSE)", debit: 0, credit: rs(1850000), balance: -rs(1850000) },
    { entry_id: "le-2", date: "2026-05-06", voucher: "RCPT-2026-0241", particular: "By Tuition fees collected (B.Com)", debit: 0, credit: rs(940000), balance: -rs(2790000) },
    { entry_id: "le-3", date: "2026-05-20", voucher: "JV-2026-0005", particular: "By Hostel & Mess Fee A/c (transfer)", debit: 0, credit: rs(640000), balance: -rs(3430000) },
    { entry_id: "le-4", date: "2026-06-02", voucher: "RCPT-2026-0270", particular: "By Tuition fees collected (MBA)", debit: 0, credit: rs(1220000), balance: -rs(4650000) },
  ],
  "acc-5100": [
    { entry_id: "le-1", date: "2026-04-30", voucher: "PMT-2026-0088", particular: "To Faculty salaries — April payroll", debit: rs(1980000), credit: 0, balance: rs(1980000) },
    { entry_id: "le-2", date: "2026-05-31", voucher: "PMT-2026-0120", particular: "To Faculty salaries — May payroll", debit: rs(2150000), credit: 0, balance: rs(4130000) },
    { entry_id: "le-3", date: "2026-06-12", voucher: "JV-2026-0010", particular: "By Reversal of May accrual (auto)", debit: 0, credit: rs(2150000), balance: rs(1980000) },
  ],
  "acc-2100": [
    { entry_id: "le-1", date: "2026-04-01", voucher: "JV-2026-0001", particular: "To Balance b/d", debit: 0, credit: rs(2120000), balance: -rs(2120000) },
    { entry_id: "le-2", date: "2026-04-20", voucher: "JV-2026-0004", particular: "By Legal Associates LLP — retainer", debit: rs(1200000), credit: 0, balance: -rs(920000) },
    { entry_id: "le-3", date: "2026-06-01", voucher: "JV-2026-0008", particular: "By ABC Scientific Supplies — lab equipment", debit: rs(1121000), credit: 0, balance: rs(201000) },
    { entry_id: "le-4", date: "2026-06-10", voucher: "INV-SS-2026-044", particular: "To Nashik Stationery Mart — invoice", debit: 0, credit: rs(128000), balance: rs(73000) },
  ],
};

/** Fallback generator for any account without a bespoke ledger. */
export function generateLedger(account: Account): LedgerEntry[] {
  const isCredit = account.account_type === "Income" || account.account_type === "Liability" || account.account_type === "Equity";
  const base = Math.abs(account.current_balance);
  const entries: LedgerEntry[] = [];
  const n = 4;
  for (let i = 0; i < n; i++) {
    const frac = (i + 1) / n;
    const running = Math.round(base * frac);
    const amount = i === 0 ? Math.round(base / n) : Math.round(base / n);
    const date = new Date(2026, 3 + i, 10 + i * 3);
    entries.push({
      entry_id: `${account.account_id}-le-${i}`,
      date: date.toISOString().slice(0, 10),
      voucher: `JV-2026-${String(20 + i).padStart(4, "0")}`,
      particular: isCredit
        ? `By ${account.account_name.replace("A/c", "").trim()} — periodic booking`
        : `To ${account.account_name.replace("A/c", "").trim()} — periodic booking`,
      debit: isCredit ? 0 : amount,
      credit: isCredit ? amount : 0,
      balance: isCredit ? -running : running,
    });
  }
  return entries;
}

// ── Fee Structures (AR) ─────────────────────────────────────────────────

export type FeeHeadType =
  | "Tuition" | "Development" | "Examination" | "Library" | "Laboratory"
  | "Sports" | "Cultural" | "Admission" | "Registration" | "Hostel"
  | "Mess" | "Transportation" | "CautionDeposit" | "Other";

export interface FeeStructureLine {
  fee_head: string;
  fee_head_type: FeeHeadType;
  amount: number; // paise, annual
  is_optional: boolean;
  installment_allowed: boolean;
  is_refundable: boolean;
}

export interface Installment {
  number: number;
  percentage: number;
  due_date: string;
  label: string;
}

export interface FeeStructure {
  fee_structure_id: string;
  name: string;
  program: string;
  batch: string;
  academic_year: string;
  category: string; // e.g. "Open", "OBC", "EWS"
  status: "Draft" | "Active" | "Archived";
  frc_approval: string;
  effective_from: string;
  effective_to: string | null;
  total_annual: number; // paise
  lines: FeeStructureLine[];
  installment_plans: { name: string; installments: Installment[] }[];
}

export const MOCK_FEE_STRUCTURES: FeeStructure[] = [
  {
    fee_structure_id: "fs-001",
    name: "B.Tech CSE — AY 2026-27",
    program: "B.Tech (Computer Science)",
    batch: "2026-27",
    academic_year: "2026-27",
    category: "Open",
    status: "Active",
    frc_approval: "FRC/SRCOE/2026-27/014",
    effective_from: "2026-04-01",
    effective_to: null,
    total_annual: rs(148000),
    lines: [
      { fee_head: "Tuition Fee", fee_head_type: "Tuition", amount: rs(85000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Development Fee", fee_head_type: "Development", amount: rs(20000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Examination Fee", fee_head_type: "Examination", amount: rs(6000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Library Fee", fee_head_type: "Library", amount: rs(5000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Laboratory Fee", fee_head_type: "Laboratory", amount: rs(8000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Sports & Cultural", fee_head_type: "Sports", amount: rs(3000), is_optional: false, installment_allowed: false, is_refundable: false },
      { fee_head: "Caution Deposit", fee_head_type: "CautionDeposit", amount: rs(10000), is_optional: false, installment_allowed: false, is_refundable: true },
      { fee_head: "Student Welfare Fund", fee_head_type: "Other", amount: rs(3000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Alumni Association", fee_head_type: "Other", amount: rs(2000), is_optional: true, installment_allowed: false, is_refundable: false },
      { fee_head: "Hostel & Mess (optional)", fee_head_type: "Hostel", amount: rs(65000), is_optional: true, installment_allowed: true, is_refundable: false },
    ],
    installment_plans: [
      { name: "2 Installments", installments: [
        { number: 1, percentage: 50, due_date: "2026-07-15", label: "Term I" },
        { number: 2, percentage: 50, due_date: "2026-12-15", label: "Term II" },
      ]},
      { name: "4 Installments", installments: [
        { number: 1, percentage: 25, due_date: "2026-07-15", label: "Quarter 1" },
        { number: 2, percentage: 25, due_date: "2026-09-15", label: "Quarter 2" },
        { number: 3, percentage: 25, due_date: "2026-12-15", label: "Quarter 3" },
        { number: 4, percentage: 25, due_date: "2027-02-15", label: "Quarter 4" },
      ]},
    ],
  },
  {
    fee_structure_id: "fs-002",
    name: "B.Com — AY 2026-27",
    program: "B.Com",
    batch: "2026-27",
    academic_year: "2026-27",
    category: "Open",
    status: "Active",
    frc_approval: "FRC/SRCOE/2026-27/015",
    effective_from: "2026-04-01",
    effective_to: null,
    total_annual: rs(42000),
    lines: [
      { fee_head: "Tuition Fee", fee_head_type: "Tuition", amount: rs(25000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Development Fee", fee_head_type: "Development", amount: rs(8000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Examination Fee", fee_head_type: "Examination", amount: rs(3000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Library Fee", fee_head_type: "Library", amount: rs(2000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Sports & Cultural", fee_head_type: "Sports", amount: rs(2000), is_optional: false, installment_allowed: false, is_refundable: false },
      { fee_head: "Caution Deposit", fee_head_type: "CautionDeposit", amount: rs(2000), is_optional: false, installment_allowed: false, is_refundable: true },
    ],
    installment_plans: [
      { name: "2 Installments", installments: [
        { number: 1, percentage: 50, due_date: "2026-07-15", label: "Term I" },
        { number: 2, percentage: 50, due_date: "2026-12-15", label: "Term II" },
      ]},
    ],
  },
  {
    fee_structure_id: "fs-003",
    name: "B.Sc Computer Science — AY 2026-27",
    program: "B.Sc (Computer Science)",
    batch: "2026-27",
    academic_year: "2026-27",
    category: "Open",
    status: "Active",
    frc_approval: "FRC/SRCOE/2026-27/016",
    effective_from: "2026-04-01",
    effective_to: null,
    total_annual: rs(56000),
    lines: [
      { fee_head: "Tuition Fee", fee_head_type: "Tuition", amount: rs(34000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Development Fee", fee_head_type: "Development", amount: rs(10000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Examination Fee", fee_head_type: "Examination", amount: rs(4000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Library Fee", fee_head_type: "Library", amount: rs(3000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Laboratory Fee", fee_head_type: "Laboratory", amount: rs(3000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Caution Deposit", fee_head_type: "CautionDeposit", amount: rs(2000), is_optional: false, installment_allowed: false, is_refundable: true },
    ],
    installment_plans: [
      { name: "2 Installments", installments: [
        { number: 1, percentage: 50, due_date: "2026-07-15", label: "Term I" },
        { number: 2, percentage: 50, due_date: "2026-12-15", label: "Term II" },
      ]},
    ],
  },
  {
    fee_structure_id: "fs-004",
    name: "MBA — AY 2026-27",
    program: "MBA",
    batch: "2026-27",
    academic_year: "2026-27",
    category: "Open",
    status: "Draft",
    frc_approval: "FRC/SRCOE/2026-27/019 (pending)",
    effective_from: "2026-08-01",
    effective_to: null,
    total_annual: rs(185000),
    lines: [
      { fee_head: "Tuition Fee", fee_head_type: "Tuition", amount: rs(120000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Development Fee", fee_head_type: "Development", amount: rs(30000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Examination Fee", fee_head_type: "Examination", amount: rs(8000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Library & Digital Resources", fee_head_type: "Library", amount: rs(10000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Placement Support", fee_head_type: "Other", amount: rs(15000), is_optional: false, installment_allowed: true, is_refundable: false },
      { fee_head: "Caution Deposit", fee_head_type: "CautionDeposit", amount: rs(2000), is_optional: false, installment_allowed: false, is_refundable: true },
    ],
    installment_plans: [
      { name: "2 Installments", installments: [
        { number: 1, percentage: 50, due_date: "2026-08-15", label: "Term I" },
        { number: 2, percentage: 50, due_date: "2027-01-15", label: "Term II" },
      ]},
    ],
  },
];

// ── Students & Fee Accounts (AR) ────────────────────────────────────────

export interface Student {
  student_id: string;
  enrollment: string;
  name: string;
  program: string;
  batch: string;
  category: string; // Open / OBC / SC / ST / EWS
  father_name: string;
  phone: string;
  email: string;
  hostel: boolean;
}

export const MOCK_STUDENTS: Student[] = [
  { student_id: "stu-001", enrollment: "SRCOE2026CSE001", name: "Aarav Sharma", program: "B.Tech (Computer Science)", batch: "2026-27", category: "Open", father_name: "Rajesh Sharma", phone: "9822012345", email: "aarav.sharma@srcoe.edu.in", hostel: true },
  { student_id: "stu-002", enrollment: "SRCOE2026CSE002", name: "Sneha Patil", program: "B.Tech (Computer Science)", batch: "2026-27", category: "OBC", father_name: "Vijay Patil", phone: "9822056789", email: "sneha.patil@srcoe.edu.in", hostel: true },
  { student_id: "stu-003", enrollment: "SRCOE2026CSE003", name: "Rohan Deshmukh", program: "B.Tech (Computer Science)", batch: "2026-27", category: "EWS", father_name: "Nitin Deshmukh", phone: "9890012345", email: "rohan.deshmukh@srcoe.edu.in", hostel: false },
  { student_id: "stu-004", enrollment: "SRCOE2026CSE004", name: "Ananya Kulkarni", program: "B.Tech (Computer Science)", batch: "2026-27", category: "SC", father_name: "Dattatreya Kulkarni", phone: "9822078901", email: "ananya.kulkarni@srcoe.edu.in", hostel: true },
  { student_id: "stu-005", enrollment: "SRCOE2026BCOM001", name: "Isha Joshi", program: "B.Com", batch: "2026-27", category: "Open", father_name: "Mahesh Joshi", phone: "9822043210", email: "isha.joshi@srcoe.edu.in", hostel: false },
  { student_id: "stu-006", enrollment: "SRCOE2026BCOM002", name: "Kunal More", program: "B.Com", batch: "2026-27", category: "NT", father_name: "Sanjay More", phone: "9822076543", email: "kunal.more@srcoe.edu.in", hostel: false },
  { student_id: "stu-007", enrollment: "SRCOE2026BSC001", name: "Prerna Jadhav", program: "B.Sc (Computer Science)", batch: "2026-27", category: "OBC", father_name: "Suresh Jadhav", phone: "9890098765", email: "prerna.jadhav@srcoe.edu.in", hostel: false },
  { student_id: "stu-008", enrollment: "SRCOE2026MBA001", name: "Aditya Pawar", program: "MBA", batch: "2026-27", category: "Open", father_name: "Deepak Pawar", phone: "9822054321", email: "aditya.pawar@srcoe.edu.in", hostel: true },
];

export interface FeePayment {
  receipt_no: string;
  date: string;
  head: string;
  amount: number; // paise
  mode: string;
  reference: string;
  status: "Completed" | "Uncleared" | "Bounced" | "Refunded";
}

export interface StudentFeeAccount {
  student_id: string;
  fee_structure_id: string;
  gross_fees: number; // paise (incl. hostel if applicable)
  concession: number; // paise
  scholarship: number; // paise
  net_payable: number; // paise
  paid: number; // paise
  balance: number; // paise
  due_date: string;
  installment_schedule: { number: number; label: string; due_date: string; amount: number; paid: number; status: "Paid" | "Due" | "Overdue" | "Upcoming" }[];
  payments: FeePayment[];
}

export const MOCK_STUDENT_FEE_ACCOUNTS: Record<string, StudentFeeAccount> = {
  "stu-001": {
    student_id: "stu-001",
    fee_structure_id: "fs-001",
    gross_fees: rs(213000), // 148000 + 65000 hostel
    concession: 0,
    scholarship: rs(20000), // Rajarshi Shahu Maharaj Merit Scholarship
    net_payable: rs(193000),
    paid: rs(96500),
    balance: rs(96500),
    due_date: "2026-12-15",
    installment_schedule: [
      { number: 1, label: "Term I (50%)", due_date: "2026-07-15", amount: rs(96500), paid: rs(96500), status: "Paid" },
      { number: 2, label: "Term II (50%)", due_date: "2026-12-15", amount: rs(96500), paid: 0, status: "Due" },
    ],
    payments: [
      { receipt_no: "RCPT-2026-0001", date: "2026-07-10", head: "Term I — Tuition & Hostel", amount: rs(96500), mode: "UPI", reference: "UPI/406988123456", status: "Completed" },
    ],
  },
  "stu-002": {
    student_id: "stu-002",
    fee_structure_id: "fs-001",
    gross_fees: rs(213000),
    concession: rs(10650), // 5% OBC concession
    scholarship: rs(45000), // Post-Matric OBC
    net_payable: rs(157350),
    paid: rs(78675),
    balance: rs(78675),
    due_date: "2026-12-15",
    installment_schedule: [
      { number: 1, label: "Term I (50%)", due_date: "2026-07-15", amount: rs(78675), paid: rs(78675), status: "Paid" },
      { number: 2, label: "Term II (50%)", due_date: "2026-12-15", amount: rs(78675), paid: 0, status: "Due" },
    ],
    payments: [
      { receipt_no: "RCPT-2026-0002", date: "2026-07-12", head: "Term I — Tuition & Hostel", amount: rs(78675), mode: "NEFT", reference: "NEFT/SBIN/774521009", status: "Completed" },
    ],
  },
  "stu-003": {
    student_id: "stu-003",
    fee_structure_id: "fs-001",
    gross_fees: rs(148000),
    concession: rs(14800), // 10% EWS
    scholarship: rs(85000), // Rajarshi Shahu full
    net_payable: rs(48200),
    paid: 0,
    balance: rs(48200),
    due_date: "2026-07-15",
    installment_schedule: [
      { number: 1, label: "Term I (50%)", due_date: "2026-07-15", amount: rs(24100), paid: 0, status: "Overdue" },
      { number: 2, label: "Term II (50%)", due_date: "2026-12-15", amount: rs(24100), paid: 0, status: "Upcoming" },
    ],
    payments: [],
  },
  "stu-004": {
    student_id: "stu-004",
    fee_structure_id: "fs-001",
    gross_fees: rs(213000),
    concession: rs(21300), // 10% SC
    scholarship: rs(60000), // Post-Matric SC
    net_payable: rs(131700),
    paid: rs(131700),
    balance: 0,
    due_date: "2026-07-15",
    installment_schedule: [
      { number: 1, label: "Term I (50%)", due_date: "2026-07-15", amount: rs(65850), paid: rs(65850), status: "Paid" },
      { number: 2, label: "Term II (50%)", due_date: "2026-12-15", amount: rs(65850), paid: rs(65850), status: "Paid" },
    ],
    payments: [
      { receipt_no: "RCPT-2026-0003", date: "2026-07-14", head: "Term I", amount: rs(65850), mode: "Cheque", reference: "CHQ/004782", status: "Completed" },
      { receipt_no: "RCPT-2026-0018", date: "2026-12-10", head: "Term II", amount: rs(65850), mode: "RTGS", reference: "RTGS/HDFC/99182200", status: "Completed" },
    ],
  },
  "stu-005": {
    student_id: "stu-005",
    fee_structure_id: "fs-002",
    gross_fees: rs(42000),
    concession: 0,
    scholarship: 0,
    net_payable: rs(42000),
    paid: rs(21000),
    balance: rs(21000),
    due_date: "2026-12-15",
    installment_schedule: [
      { number: 1, label: "Term I (50%)", due_date: "2026-07-15", amount: rs(21000), paid: rs(21000), status: "Paid" },
      { number: 2, label: "Term II (50%)", due_date: "2026-12-15", amount: rs(21000), paid: 0, status: "Due" },
    ],
    payments: [
      { receipt_no: "RCPT-2026-0004", date: "2026-07-08", head: "Term I", amount: rs(21000), mode: "UPI", reference: "UPI/408912347756", status: "Completed" },
    ],
  },
  "stu-007": {
    student_id: "stu-007",
    fee_structure_id: "fs-003",
    gross_fees: rs(56000),
    concession: rs(2800), // 5% OBC
    scholarship: rs(25000), // EBC
    net_payable: rs(28200),
    paid: rs(14100),
    balance: rs(14100),
    due_date: "2026-12-15",
    installment_schedule: [
      { number: 1, label: "Term I (50%)", due_date: "2026-07-15", amount: rs(14100), paid: rs(14100), status: "Paid" },
      { number: 2, label: "Term II (50%)", due_date: "2026-12-15", amount: rs(14100), paid: 0, status: "Due" },
    ],
    payments: [
      { receipt_no: "RCPT-2026-0005", date: "2026-07-11", head: "Term I", amount: rs(14100), mode: "Cash", reference: "CASH/2234", status: "Completed" },
    ],
  },
};

// ── Scholarships (AR) ───────────────────────────────────────────────────

export type ScholarshipStatus = "Applied" | "Verified" | "Sanctioned" | "PartiallyDisbursed" | "Disbursed" | "Rejected" | "Closed";

export interface ScholarshipRecord {
  scholarship_id: string;
  scheme: string;
  agency: string;
  student_id: string;
  student_name: string;
  enrollment: string;
  sanctioned_amount: number; // paise
  disbursed_amount: number; // paise
  mahadbt_reference: string;
  mahadbt_status: "Submitted" | "Verified" | "Disbursed" | "Pending" | "Rejected";
  status: ScholarshipStatus;
  applied_on: string;
  reconciled: boolean;
  bank: string;
  ifsc: string;
}

export const SCHOLARSHIP_SCHEMES = [
  "Rajarshi Shahu Maharaj Merit Scholarship",
  "Dr. Punjabrao Deshmukh Vasatigruh Nirvah Bhatta Yojana",
  "Post-Matric Scholarship — OBC",
  "Post-Matric Scholarship — SC",
  "EBC (Economic Backward Class) Scholarship",
  "Mukhyamantri Yuva Karyabhumi Scholarship",
];

export const MOCK_SCHOLARSHIPS: ScholarshipRecord[] = [
  { scholarship_id: "sch-001", scheme: "Rajarshi Shahu Maharaj Merit Scholarship", agency: "Government of Maharashtra (SSD)", student_id: "stu-001", student_name: "Aarav Sharma", enrollment: "SRCOE2026CSE001", sanctioned_amount: rs(20000), disbursed_amount: rs(10000), mahadbt_reference: "MH-SSD-2026-88213", mahadbt_status: "Disbursed", status: "PartiallyDisbursed", applied_on: "2026-06-01", reconciled: true, bank: "State Bank of India", ifsc: "SBIN0001234" },
  { scholarship_id: "sch-002", scheme: "Rajarshi Shahu Maharaj Merit Scholarship", agency: "Government of Maharashtra (SSD)", student_id: "stu-003", student_name: "Rohan Deshmukh", enrollment: "SRCOE2026CSE003", sanctioned_amount: rs(85000), disbursed_amount: rs(42500), mahadbt_reference: "MH-SSD-2026-88214", mahadbt_status: "Disbursed", status: "PartiallyDisbursed", applied_on: "2026-06-01", reconciled: true, bank: "Bank of Maharashtra", ifsc: "MAHB0000987" },
  { scholarship_id: "sch-003", scheme: "Post-Matric Scholarship — OBC", agency: "Directorate of Higher Education", student_id: "stu-002", student_name: "Sneha Patil", enrollment: "SRCOE2026CSE002", sanctioned_amount: rs(45000), disbursed_amount: rs(45000), mahadbt_reference: "MH-DHE-2026-44190", mahadbt_status: "Disbursed", status: "Disbursed", applied_on: "2026-05-20", reconciled: true, bank: "State Bank of India", ifsc: "SBIN0001234" },
  { scholarship_id: "sch-004", scheme: "Post-Matric Scholarship — SC", agency: "Directorate of Higher Education", student_id: "stu-004", student_name: "Ananya Kulkarni", enrollment: "SRCOE2026CSE004", sanctioned_amount: rs(60000), disbursed_amount: rs(30000), mahadbt_reference: "MH-DHE-2026-44210", mahadbt_status: "Verified", status: "Sanctioned", applied_on: "2026-05-22", reconciled: false, bank: "Bank of Baroda", ifsc: "BARB0NASHIK" },
  { scholarship_id: "sch-005", scheme: "EBC (Economic Backward Class) Scholarship", agency: "Government of Maharashtra (SSD)", student_id: "stu-007", student_name: "Prerna Jadhav", enrollment: "SRCOE2026BSC001", sanctioned_amount: rs(25000), disbursed_amount: 0, mahadbt_reference: "MH-SSD-2026-77112", mahadbt_status: "Pending", status: "Sanctioned", applied_on: "2026-06-10", reconciled: false, bank: "Central Bank of India", ifsc: "CBIN0281234" },
  { scholarship_id: "sch-006", scheme: "Dr. Punjabrao Deshmukh Vasatigruh Nirvah Bhatta Yojana", agency: "Social Justice Department", student_id: "stu-002", student_name: "Sneha Patil", enrollment: "SRCOE2026CSE002", sanctioned_amount: rs(12000), disbursed_amount: rs(12000), mahadbt_reference: "MH-SJD-2026-19023", mahadbt_status: "Disbursed", status: "Disbursed", applied_on: "2026-05-10", reconciled: true, bank: "State Bank of India", ifsc: "SBIN0001234" },
  { scholarship_id: "sch-007", scheme: "Mukhyamantri Yuva Karyabhumi Scholarship", agency: "Directorate of Technical Education", student_id: "stu-008", student_name: "Aditya Pawar", enrollment: "SRCOE2026MBA001", sanctioned_amount: rs(30000), disbursed_amount: 0, mahadbt_reference: "MH-DTE-2026-55230", mahadbt_status: "Submitted", status: "Verified", applied_on: "2026-06-15", reconciled: false, bank: "HDFC Bank", ifsc: "HDFC0001234" },
];

// ── Refunds (AR) ────────────────────────────────────────────────────────

export type RefundStatus = "Pending" | "Approved" | "Processed" | "Rejected";

export interface RefundRequest {
  refund_id: string;
  student_id: string;
  student_name: string;
  enrollment: string;
  original_receipt: string;
  original_payment_date: string;
  original_amount: number; // paise
  refundable_amount: number; // paise (after FRC-compliant deductions)
  deduction_breakup: { head: string; amount: number }[]; // deducted heads
  mode: "NEFT" | "RTGS" | "UPI" | "Cheque";
  beneficiary: string;
  account_no: string;
  ifsc: string;
  reason: string;
  status: RefundStatus;
  requested_on: string;
  frc_reference: string;
}

export const MOCK_REFUNDS: RefundRequest[] = [
  { refund_id: "rf-001", student_id: "stu-009", student_name: "Rutuja Bhosale", enrollment: "SRCOE2026CSE009", original_receipt: "RCPT-2026-0041", original_payment_date: "2026-07-12", original_amount: rs(148000), refundable_amount: rs(131200), deduction_breakup: [
    { head: "Caution Deposit (held — refundable at exit)", amount: 0 },
    { head: "Tuition Fee (10% deduction per FRC clause 7.2)", amount: rs(14800) },
    { head: "Examination Fee (non-refundable after form fill)", amount: rs(2000) },
  ], mode: "NEFT", beneficiary: "Rutuja Bhosale", account_no: "40218877331", ifsc: "SBIN0001234", reason: "Withdrew admission before commencement of classes (cancellation under FRC rules)", status: "Pending", requested_on: "2026-07-18", frc_reference: "FRC/SRCOE/REF/2026-07/003" },
  { refund_id: "rf-002", student_id: "stu-010", student_name: "Sahil Chavan", enrollment: "SRCOE2026BCOM004", original_receipt: "RCPT-2026-0052", original_payment_date: "2026-07-14", original_amount: rs(42000), refundable_amount: rs(38000), deduction_breakup: [
    { head: "Tuition Fee (10% deduction per FRC clause 7.2)", amount: rs(4000) },
  ], mode: "UPI", beneficiary: "Sahil Chavan", account_no: "sahil.chavan@okhdfc", ifsc: "UPI", reason: "Duplicate fee payment — paid both online and at counter", status: "Approved", requested_on: "2026-07-19", frc_reference: "FRC/SRCOE/REF/2026-07/004" },
  { refund_id: "rf-003", student_id: "stu-011", student_name: "Tanvi Wagh", enrollment: "SRCOE2026MBA003", original_receipt: "RCPT-2026-0060", original_payment_date: "2026-07-20", original_amount: rs(185000), refundable_amount: rs(167200), deduction_breakup: [
    { head: "Tuition Fee (10% deduction per FRC clause 7.2)", amount: rs(16800) },
    { head: "Placement Support (non-refundable)", amount: rs(1000) },
  ], mode: "RTGS", beneficiary: "Tanvi Wagh", account_no: "77881234090", ifsc: "HDFC0001234", reason: "Medical withdrawal — documented by MBBS certificate", status: "Processed", requested_on: "2026-07-22", frc_reference: "FRC/SRCOE/REF/2026-07/005" },
  { refund_id: "rf-004", student_id: "stu-012", student_name: "Om Ghadge", enrollment: "SRCOE2026CSE012", original_receipt: "RCPT-2026-0066", original_payment_date: "2026-07-25", original_amount: rs(148000), refundable_amount: 0, deduction_breakup: [
    { head: "Fee refund not admissible — classes commenced & seat confirmed", amount: rs(148000) },
  ], mode: "NEFT", beneficiary: "Om Ghadge", account_no: "33551199887", ifsc: "MAHB0000987", reason: "Late cancellation after 30 days of commencement", status: "Rejected", requested_on: "2026-07-27", frc_reference: "FRC/SRCOE/REF/2026-07/006" },
  { refund_id: "rf-005", student_id: "stu-013", student_name: "Shreya Nikam", enrollment: "SRCOE2026BSC005", original_receipt: "RCPT-2026-0071", original_payment_date: "2026-07-28", original_amount: rs(56000), refundable_amount: rs(56000), deduction_breakup: [], mode: "NEFT", beneficiary: "Shreya Nikam", account_no: "40017766550", ifsc: "SBIN0004321", reason: "Fee structure revised — excess recovered refunded", status: "Pending", requested_on: "2026-07-29", frc_reference: "FRC/SRCOE/REF/2026-07/007" },
];

// ── Vendors & Procurement (AP) ──────────────────────────────────────────

export type VendorType = "Goods" | "Service" | "Capital" | "Consultancy" | "Catering" | "Utilities";

export interface VendorMock {
  vendorId: string;
  vendorCode: string;
  vendorName: string;
  vendorType: VendorType;
  pan: string;
  gstin: string | null;
  contactPerson: string;
  phone: string;
  email: string;
  address: string;
  city: string;
  state: string;
  msme: boolean;
  msme_udyam?: string;
  tds_section: string;
  section197: boolean;
  payment_terms: number; // days
  is_active: boolean;
  gst_composition: boolean;
}

export const MOCK_VENDORS: VendorMock[] = [
  { vendorId: "ven-001", vendorCode: "VND-1001", vendorName: "ABC Scientific Supplies", vendorType: "Goods", pan: "AAFCB1234K", gstin: "27AAFCB1234K1Z5", contactPerson: "Bharat Mehta", phone: "9823012345", email: "sales@abcscientific.in", address: "Plot 12, MIDC Ambad", city: "Nashik", state: "Maharashtra", msme: true, msme_udyam: "MH27D0001234", tds_section: "194C", section197: false, payment_terms: 30, is_active: true, gst_composition: false },
  { vendorId: "ven-002", vendorCode: "VND-1002", vendorName: "Nashik Stationery Mart", vendorType: "Goods", pan: "AAJPN7788Q", gstin: "27AAJPN7788Q1Z9", contactPerson: "Pravin Jain", phone: "9823076543", email: "orders@nashikstationery.in", address: "Shop 4, College Road", city: "Nashik", state: "Maharashtra", msme: true, msme_udyam: "MH27D0005678", tds_section: "194C", section197: false, payment_terms: 15, is_active: true, gst_composition: false },
  { vendorId: "ven-003", vendorCode: "VND-1003", vendorName: "GreenLeaf Catering Services", vendorType: "Catering", pan: "AALPG5566R", gstin: "27AALPG5566R1Z2", contactPerson: "Farhan Sheikh", phone: "9890011223", email: "info@greenleafcatering.in", address: "12/3, Trimbak Road", city: "Nashik", state: "Maharashtra", msme: false, tds_section: "194C", section197: true, payment_terms: 30, is_active: true, gst_composition: false },
  { vendorId: "ven-004", vendorCode: "VND-1004", vendorName: "TechServe IT Solutions", vendorType: "Service", pan: "AAJCT3322P", gstin: "27AAJCT3322P1Z8", contactPerson: "Nidhi Agarwal", phone: "9823022445", email: "contact@techserveit.in", address: "Orbit Tower, Gangapur Road", city: "Nashik", state: "Maharashtra", msme: true, msme_udyam: "MH27D0009012", tds_section: "194J", section197: true, payment_terms: 45, is_active: true, gst_composition: false },
  { vendorId: "ven-005", vendorCode: "VND-1005", vendorName: "Sharma Electricals & Fittings", vendorType: "Goods", pan: "AAJPS8899L", gstin: "27AAJPS8899L1Z3", contactPerson: "Rakesh Sharma", phone: "9823055667", email: "sharmaelectricals@gmail.com", address: "Gala 8, Satpur MIDC", city: "Nashik", state: "Maharashtra", msme: true, msme_udyam: "MH27D0012345", tds_section: "194C", section197: false, payment_terms: 30, is_active: true, gst_composition: false },
  { vendorId: "ven-006", vendorCode: "VND-1006", vendorName: "Vidya Books Distributors", vendorType: "Goods", pan: "AAEFV1122M", gstin: "27AAEFV1122M1Z7", contactPerson: "Anita Deshpande", phone: "9823088990", email: "sales@vidyabooks.in", address: "Plot 3, Sharanpur Road", city: "Nashik", state: "Maharashtra", msme: false, tds_section: "194C", section197: false, payment_terms: 30, is_active: true, gst_composition: false },
  { vendorId: "ven-007", vendorCode: "VND-1007", vendorName: "Legal Associates LLP", vendorType: "Consultancy", pan: "AATFA2345M", gstin: null, contactPerson: "Adv. S. Kulkarni", phone: "9823111222", email: "legala@gmail.com", address: "3rd Floor, City Centre, College Road", city: "Nashik", state: "Maharashtra", msme: false, tds_section: "194J", section197: false, payment_terms: 30, is_active: true, gst_composition: true },
  { vendorId: "ven-008", vendorCode: "VND-1008", vendorName: "Mahavitaran (MSEDCL)", vendorType: "Utilities", pan: "AACCM9988K", gstin: "27AACCM9988K1Z0", contactPerson: "Zonal Office", phone: "1912", email: "nashikzone@mahadiscom.in", address: "MSEDCL Zonal Office, Nashik", city: "Nashik", state: "Maharashtra", msme: false, tds_section: "194I", section197: false, payment_terms: 10, is_active: true, gst_composition: false },
];

export type PoStatus = "Draft" | "Issued" | "Partially Received" | "Fully Received" | "Closed";

export interface PoLineItem {
  item: string;
  hsn: string;
  qty: number;
  unit: string;
  rate: number; // paise per unit
  amount: number; // paise
  gst_rate: number; // %
  gst_amount: number; // paise
  rcm: boolean;
}

export interface PurchaseOrderMock {
  purchaseOrderId: string;
  poNumber: string;
  vendorId: string;
  vendorName: string;
  orderDate: string;
  deliveryDate: string | null;
  status: PoStatus;
  totalAmount: number; // paise ex-GST
  taxAmount: number; // paise
  netAmount: number; // paise
  isRcmApplicable: boolean;
  approvedBy: string;
  lines: PoLineItem[];
}

export const MOCK_PURCHASE_ORDERS: PurchaseOrderMock[] = [
  { purchaseOrderId: "po-001", poNumber: "PO-2026-0042", vendorId: "ven-001", vendorName: "ABC Scientific Supplies", orderDate: "2026-05-28", deliveryDate: "2026-06-10", status: "Fully Received", totalAmount: rs(950000), taxAmount: rs(171000), netAmount: rs(1121000), isRcmApplicable: false, approvedBy: "Prof. S. Deshpande (Registrar)", lines: [
    { item: "UV-Vis Spectrophotometer", hsn: "9027", qty: 1, unit: "unit", rate: rs(650000), amount: rs(650000), gst_rate: 18, gst_amount: rs(117000), rcm: false },
    { item: "Research Centrifuge (4x100ml)", hsn: "8421", qty: 2, unit: "unit", rate: rs(150000), amount: rs(300000), gst_rate: 18, gst_amount: rs(54000), rcm: false },
  ]},
  { purchaseOrderId: "po-002", poNumber: "PO-2026-0043", vendorId: "ven-004", vendorName: "TechServe IT Solutions", orderDate: "2026-06-02", deliveryDate: "2026-06-20", status: "Partially Received", totalAmount: rs(420000), taxAmount: rs(75600), netAmount: rs(495600), isRcmApplicable: false, approvedBy: "Prof. S. Deshpande (Registrar)", lines: [
    { item: "Annual LMS licence & AMC", hsn: "9983", qty: 1, unit: "year", rate: rs(420000), amount: rs(420000), gst_rate: 18, gst_amount: rs(75600), rcm: false },
  ]},
  { purchaseOrderId: "po-003", poNumber: "PO-2026-0044", vendorId: "ven-006", vendorName: "Vidya Books Distributors", orderDate: "2026-06-05", deliveryDate: "2026-06-25", status: "Issued", totalAmount: rs(285000), taxAmount: rs(0), netAmount: rs(285000), isRcmApplicable: false, approvedBy: "Prof. S. Deshpande (Registrar)", lines: [
    { item: "Engineering reference books (2026 ed.)", hsn: "4901", qty: 300, unit: "nos", rate: rs(950), amount: rs(285000), gst_rate: 0, gst_amount: 0, rcm: false },
  ]},
  { purchaseOrderId: "po-004", poNumber: "PO-2026-0045", vendorId: "ven-005", vendorName: "Sharma Electricals & Fittings", orderDate: "2026-06-10", deliveryDate: null, status: "Draft", totalAmount: rs(180000), taxAmount: rs(32400), netAmount: rs(212400), isRcmApplicable: false, approvedBy: "", lines: [
    { item: "LED lighting retrofit — library wing", hsn: "9405", qty: 60, unit: "nos", rate: rs(3000), amount: rs(180000), gst_rate: 18, gst_amount: rs(32400), rcm: false },
  ]},
  { purchaseOrderId: "po-005", poNumber: "PO-2026-0046", vendorId: "ven-003", vendorName: "GreenLeaf Catering Services", orderDate: "2026-06-12", deliveryDate: "2026-07-01", status: "Closed", totalAmount: rs(2400000), taxAmount: rs(0), netAmount: rs(2400000), isRcmApplicable: true, approvedBy: "Prof. S. Deshpande (Registrar)", lines: [
    { item: "Hostel mess services — Q1 (Jul–Sep)", hsn: "9963", qty: 1, unit: "quarter", rate: rs(2400000), amount: rs(2400000), gst_rate: 5, gst_amount: rs(120000), rcm: true },
  ]},
  { purchaseOrderId: "po-006", poNumber: "PO-2026-0047", vendorId: "ven-002", vendorName: "Nashik Stationery Mart", orderDate: "2026-06-15", deliveryDate: "2026-06-22", status: "Fully Received", totalAmount: rs(118000), taxAmount: rs(21240), netAmount: rs(139240), isRcmApplicable: false, approvedBy: "Prof. S. Deshpande (Registrar)", lines: [
    { item: "Exam answer books (100 pages)", hsn: "4820", qty: 5000, unit: "nos", rate: rs(14), amount: rs(70000), gst_rate: 18, gst_amount: rs(12600), rcm: false },
    { item: "Printer paper A4 (75 GSM)", hsn: "4802", qty: 200, unit: "ream", rate: rs(240), amount: rs(48000), gst_rate: 18, gst_amount: rs(8640), rcm: false },
  ]},
];

// ── Invoices & 3-way matching (AP) ──────────────────────────────────────

export type InvoiceStatus = "Pending" | "Matched" | "Approved" | "Rejected" | "Paid" | "PartiallyPaid";

export interface InvoiceMock {
  invoiceId: string;
  invoiceNumber: string;
  vendorId: string;
  vendorName: string;
  poNumber: string | null;
  grnNumber: string | null;
  invoiceDate: string;
  dueDate: string;
  grossAmount: number; // paise
  gstAmount: number;
  netAmount: number; // paise
  status: InvoiceStatus;
  match: { poQty: number; grnQty: number; invQty: number; poAmount: number; grnAmount: number; invAmount: number; status: "Full" | "Partial" | "Mismatch" };
  tdsSection?: string;
}

export const MOCK_INVOICES: InvoiceMock[] = [
  { invoiceId: "inv-001", invoiceNumber: "INV-ABC-2026/118", vendorId: "ven-001", vendorName: "ABC Scientific Supplies", poNumber: "PO-2026-0042", grnNumber: "GRN-2026-0038", invoiceDate: "2026-06-12", dueDate: "2026-07-12", grossAmount: rs(950000), gstAmount: rs(171000), netAmount: rs(1121000), status: "Approved", match: { poQty: 3, grnQty: 3, invQty: 3, poAmount: rs(950000), grnAmount: rs(950000), invAmount: rs(950000), status: "Full" }, tdsSection: "194C" },
  { invoiceId: "inv-002", invoiceNumber: "INV-TS-2026/011", vendorId: "ven-004", vendorName: "TechServe IT Solutions", poNumber: "PO-2026-0043", grnNumber: "GRN-2026-0041", invoiceDate: "2026-06-18", dueDate: "2026-07-18", grossAmount: rs(420000), gstAmount: rs(75600), netAmount: rs(495600), status: "Pending", match: { poQty: 1, grnQty: 1, invQty: 1, poAmount: rs(420000), grnAmount: rs(420000), invAmount: rs(420000), status: "Full" }, tdsSection: "194J" },
  { invoiceId: "inv-003", invoiceNumber: "INV-SS-2026/044", vendorId: "ven-002", vendorName: "Nashik Stationery Mart", poNumber: "PO-2026-0047", grnNumber: "GRN-2026-0043", invoiceDate: "2026-06-22", dueDate: "2026-07-07", grossAmount: rs(118000), gstAmount: rs(21240), netAmount: rs(139240), status: "Paid", match: { poQty: 5200, grnQty: 5200, invQty: 5180, poAmount: rs(118000), grnAmount: rs(118000), invAmount: rs(116900), status: "Mismatch" }, tdsSection: "194C" },
  { invoiceId: "inv-004", invoiceNumber: "INV-GL-2026/020", vendorId: "ven-003", vendorName: "GreenLeaf Catering Services", poNumber: "PO-2026-0046", grnNumber: "GRN-2026-0045", invoiceDate: "2026-07-01", dueDate: "2026-07-31", grossAmount: rs(2400000), gstAmount: rs(120000), netAmount: rs(2520000), status: "Pending", match: { poQty: 1, grnQty: 1, invQty: 1, poAmount: rs(2400000), grnAmount: rs(2400000), invAmount: rs(2400000), status: "Full" }, tdsSection: "194C" },
  { invoiceId: "inv-005", invoiceNumber: "INV-LEGAL-008", vendorId: "ven-007", vendorName: "Legal Associates LLP", poNumber: null, grnNumber: null, invoiceDate: "2026-04-15", dueDate: "2026-05-15", grossAmount: rs(1200000), gstAmount: rs(216000), netAmount: rs(1416000), status: "PartiallyPaid", match: { poQty: 0, grnQty: 0, invQty: 1, poAmount: 0, grnAmount: 0, invAmount: rs(1200000), status: "Mismatch" }, tdsSection: "194J" },
  { invoiceId: "inv-006", invoiceNumber: "INV-SE-2026/031", vendorId: "ven-005", vendorName: "Sharma Electricals & Fittings", poNumber: "PO-2026-0045", grnNumber: null, invoiceDate: "2026-07-05", dueDate: "2026-08-04", grossAmount: rs(180000), gstAmount: rs(32400), netAmount: rs(212400), status: "Rejected", match: { poQty: 60, grnQty: 0, invQty: 60, poAmount: rs(180000), grnAmount: 0, invAmount: rs(180000), status: "Partial" }, tdsSection: "194C" },
];

// ── Payments (AP) ───────────────────────────────────────────────────────

export type PaymentStatus = "Due" | "Overdue" | "Scheduled" | "Paid" | "Initiated";

export interface PaymentDue {
  paymentId: string;
  invoiceId: string;
  invoiceNumber: string;
  vendorId: string;
  vendorName: string;
  invoiceDate: string;
  dueDate: string;
  grossAmount: number; // paise
  gstAmount: number;
  tdsSection: string;
  tdsRate: number; // %
  tdsAmount: number; // paise
  netPayable: number; // paise
  status: PaymentStatus;
  bankAccountId?: string;
  mode?: string;
  paidDate?: string;
  section197: boolean;
}

export const MOCK_PAYMENTS: PaymentDue[] = [
  { paymentId: "pay-001", invoiceId: "inv-001", invoiceNumber: "INV-ABC-2026/118", vendorId: "ven-001", vendorName: "ABC Scientific Supplies", invoiceDate: "2026-06-12", dueDate: "2026-07-12", grossAmount: rs(950000), gstAmount: rs(171000), tdsSection: "194C", tdsRate: 1, tdsAmount: rs(11210), netPayable: rs(1109790), status: "Overdue", section197: false },
  { paymentId: "pay-002", invoiceId: "inv-002", invoiceNumber: "INV-TS-2026/011", vendorId: "ven-004", vendorName: "TechServe IT Solutions", invoiceDate: "2026-06-18", dueDate: "2026-07-18", grossAmount: rs(420000), gstAmount: rs(75600), tdsSection: "194J", tdsRate: 10, tdsAmount: rs(49560), netPayable: rs(446040), status: "Due", section197: true },
  { paymentId: "pay-003", invoiceId: "inv-004", invoiceNumber: "INV-GL-2026/020", vendorId: "ven-003", vendorName: "GreenLeaf Catering Services", invoiceDate: "2026-07-01", dueDate: "2026-07-31", grossAmount: rs(2400000), gstAmount: rs(120000), tdsSection: "194C", tdsRate: 1, tdsAmount: rs(25200), netPayable: rs(2494800), status: "Due", section197: true },
  { paymentId: "pay-004", invoiceId: "inv-005", invoiceNumber: "INV-LEGAL-008", vendorId: "ven-007", vendorName: "Legal Associates LLP", invoiceDate: "2026-04-15", dueDate: "2026-05-15", grossAmount: rs(1200000), gstAmount: rs(216000), tdsSection: "194J", tdsRate: 10, tdsAmount: rs(120000), netPayable: rs(1080000), status: "Paid", bankAccountId: "ba-001", mode: "NEFT", paidDate: "2026-05-10", section197: false },
  { paymentId: "pay-005", invoiceId: "inv-003", invoiceNumber: "INV-SS-2026/044", vendorId: "ven-002", vendorName: "Nashik Stationery Mart", invoiceDate: "2026-06-22", dueDate: "2026-07-07", grossAmount: rs(118000), gstAmount: rs(21240), tdsSection: "194C", tdsRate: 1, tdsAmount: rs(1392), netPayable: rs(137848), status: "Paid", bankAccountId: "ba-001", mode: "RTGS", paidDate: "2026-07-05", section197: false },
  { paymentId: "pay-006", invoiceId: "inv-006", invoiceNumber: "INV-SE-2026/031", vendorId: "ven-005", vendorName: "Sharma Electricals & Fittings", invoiceDate: "2026-07-05", dueDate: "2026-08-04", grossAmount: rs(180000), gstAmount: rs(32400), tdsSection: "194C", tdsRate: 1, tdsAmount: rs(2124), netPayable: rs(210276), status: "Scheduled", bankAccountId: "ba-003", mode: "Cheque", section197: false },
];

// ── Treasury ────────────────────────────────────────────────────────────

export interface BankAccountMock {
  bankAccountId: string;
  accountName: string;
  bankName: string;
  branch: string;
  accountNumber: string;
  ifsc: string;
  accountType: "Current" | "Savings" | "FD" | "Escrow";
  balance: number; // paise
  isPrimary: boolean;
  purpose: string;
  lastReconciled: string;
}

export const MOCK_BANK_ACCOUNTS: BankAccountMock[] = [
  { bankAccountId: "ba-001", accountName: "SRCOE Operating A/c", bankName: "State Bank of India", branch: "College Road, Nashik", accountNumber: "31234567890", ifsc: "SBIN0001234", accountType: "Current", balance: rs(2845000), isPrimary: true, purpose: "Operating expenses, salaries", lastReconciled: "2026-06-30" },
  { bankAccountId: "ba-002", accountName: "SRCOE Fee Collection A/c", bankName: "ICICI Bank", branch: "Gangapur Road, Nashik", accountNumber: "003401234567", ifsc: "ICIC0006789", accountType: "Current", balance: rs(6210000), isPrimary: false, purpose: "Student fee collections (UPI/NEFT)", lastReconciled: "2026-06-30" },
  { bankAccountId: "ba-003", accountName: "SRCOE Development A/c", bankName: "HDFC Bank", branch: "Sharanpur Road, Nashik", accountNumber: "50100234567890", ifsc: "HDFC0001234", accountType: "Current", balance: rs(1287500), isPrimary: false, purpose: "FRC development fund utilisation", lastReconciled: "2026-06-28" },
  { bankAccountId: "ba-004", accountName: "SRCOE Fixed Deposits", bankName: "Bank of Maharashtra", branch: "Nashik Main", accountNumber: "FD-2203456", ifsc: "MAHB0000987", accountType: "FD", balance: rs(15000000), isPrimary: false, purpose: "Corpus — interest bearing", lastReconciled: "2026-06-25" },
];

export interface ReconciliationRow {
  reconciliationId: string;
  bankAccountId: string;
  bankAccountName: string;
  statementDate: string;
  bankRef: string;
  particular: string;
  bankAmount: number; // paise
  systemRef: string | null;
  systemDate: string | null;
  systemAmount: number | null;
  status: "Matched" | "Unmatched" | "Partial";
}

export const MOCK_RECONCILIATION: ReconciliationRow[] = [
  { reconciliationId: "rec-001", bankAccountId: "ba-001", bankAccountName: "SRCOE Operating A/c", statementDate: "2026-06-30", bankRef: "SBI/27062026/118", particular: "NEFT — GreenLeaf Catering", bankAmount: rs(420000), systemRef: "PMT-2026-0109", systemDate: "2026-06-25", systemAmount: rs(420000), status: "Matched" },
  { reconciliationId: "rec-002", bankAccountId: "ba-001", bankAccountName: "SRCOE Operating A/c", statementDate: "2026-06-30", bankRef: "SBI/27062026/119", particular: "Rent — Trimbak Road godown", bankAmount: rs(75000), systemRef: "PMT-2026-0113", systemDate: "2026-06-26", systemAmount: rs(75000), status: "Matched" },
  { reconciliationId: "rec-003", bankAccountId: "ba-001", bankAccountName: "SRCOE Operating A/c", statementDate: "2026-06-30", bankRef: "SBI/28062026/121", particular: "Debit — annual locker charges", bankAmount: rs(2360), systemRef: null, systemDate: null, systemAmount: null, status: "Unmatched" },
  { reconciliationId: "rec-004", bankAccountId: "ba-001", bankAccountName: "SRCOE Operating A/c", statementDate: "2026-06-30", bankRef: "SBI/29062026/125", particular: "NEFT — TechServe IT (partial)", bankAmount: rs(200000), systemRef: "PMT-2026-0118", systemDate: "2026-06-29", systemAmount: rs(446040), status: "Partial" },
  { reconciliationId: "rec-005", bankAccountId: "ba-002", bankAccountName: "SRCOE Fee Collection A/c", statementDate: "2026-06-30", bankRef: "ICICI/30062026/221", particular: "UPI collections — June", bankAmount: rs(1864000), systemRef: "RCPT-2026-03xx (batch)", systemDate: "2026-06-30", systemAmount: rs(1864000), status: "Matched" },
  { reconciliationId: "rec-006", bankAccountId: "ba-002", bankAccountName: "SRCOE Fee Collection A/c", statementDate: "2026-06-30", bankRef: "ICICI/30062026/222", particular: "NEFT — hosteller mess advance", bankAmount: rs(640000), systemRef: "RCPT-2026-0231", systemDate: "2026-05-02", systemAmount: rs(640000), status: "Matched" },
  { reconciliationId: "rec-007", bankAccountId: "ba-002", bankAccountName: "SRCOE Fee Collection A/c", statementDate: "2026-06-30", bankRef: "ICICI/30062026/224", particular: "Chargeback — UPI 406988123456", bankAmount: rs(20000), systemRef: null, systemDate: null, systemAmount: null, status: "Unmatched" },
];

// ── Taxation ────────────────────────────────────────────────────────────

export interface GstReturnPeriod {
  period: string; // e.g. "Jun 2026"
  filing: "GSTR-1" | "GSTR-3B" | "GSTR-9" | "ITC";
  status: "Filed" | "Not Filed" | "Due Soon" | "Upcoming";
  dueDate: string;
  filedOn: string | null;
  outwardTaxable: number; // paise
  outwardTax: number;
  inwardItc: number;
  netPayable: number;
  lateFee?: number;
}

export const MOCK_GST_RETURNS: GstReturnPeriod[] = [
  { period: "Apr 2026", filing: "GSTR-3B", status: "Filed", dueDate: "2026-05-20", filedOn: "2026-05-18", outwardTaxable: rs(4820000), outwardTax: rs(254000), inwardItc: rs(182000), netPayable: rs(72000) },
  { period: "Apr 2026", filing: "GSTR-1", status: "Filed", dueDate: "2026-05-11", filedOn: "2026-05-10", outwardTaxable: rs(4820000), outwardTax: rs(254000), inwardItc: 0, netPayable: 0 },
  { period: "May 2026", filing: "GSTR-3B", status: "Filed", dueDate: "2026-06-20", filedOn: "2026-06-19", outwardTaxable: rs(5340000), outwardTax: rs(281000), inwardItc: rs(198000), netPayable: rs(83000) },
  { period: "May 2026", filing: "GSTR-1", status: "Filed", dueDate: "2026-06-11", filedOn: "2026-06-09", outwardTaxable: rs(5340000), outwardTax: rs(281000), inwardItc: 0, netPayable: 0 },
  { period: "Jun 2026", filing: "GSTR-3B", status: "Due Soon", dueDate: "2026-07-20", filedOn: null, outwardTaxable: rs(5610000), outwardTax: rs(296000), inwardItc: rs(211000), netPayable: rs(85000) },
  { period: "Jun 2026", filing: "GSTR-1", status: "Not Filed", dueDate: "2026-07-11", filedOn: null, outwardTaxable: rs(5610000), outwardTax: rs(296000), inwardItc: 0, netPayable: 0 },
];

export interface TdsDeductionRow {
  deductionId: string;
  vendorName: string;
  pan: string;
  section: string;
  rate: number;
  tdsAmount: number; // paise
  paymentRef: string;
  paymentDate: string;
  challanStatus: "Pending" | "Deposited" | "Filing Pending";
  quarter: string;
  form16a: boolean;
}

export const MOCK_TDS_DEDUCTIONS: TdsDeductionRow[] = [
  { deductionId: "tds-001", vendorName: "TechServe IT Solutions", pan: "AAJCT3322P", section: "194J", rate: 10, tdsAmount: rs(49560), paymentRef: "PMT-2026-0118", paymentDate: "2026-06-29", challanStatus: "Deposited", quarter: "Q1 FY 26-27", form16a: true },
  { deductionId: "tds-002", vendorName: "GreenLeaf Catering Services", pan: "AALPG5566R", section: "194C", rate: 1, tdsAmount: rs(25200), paymentRef: "PMT-2026-0121", paymentDate: "2026-07-02", challanStatus: "Pending", quarter: "Q1 FY 26-27", form16a: false },
  { deductionId: "tds-003", vendorName: "ABC Scientific Supplies", pan: "AAFCB1234K", section: "194C", rate: 1, tdsAmount: rs(11210), paymentRef: "PMT-2026-0123", paymentDate: "2026-07-06", challanStatus: "Pending", quarter: "Q1 FY 26-27", form16a: false },
  { deductionId: "tds-004", vendorName: "Nashik Stationery Mart", pan: "AAJPN7788Q", section: "194C", rate: 1, tdsAmount: rs(1392), paymentRef: "PMT-2026-0115", paymentDate: "2026-07-05", challanStatus: "Deposited", quarter: "Q1 FY 26-27", form16a: true },
  { deductionId: "tds-005", vendorName: "Legal Associates LLP", pan: "AATFA2345M", section: "194J", rate: 10, tdsAmount: rs(120000), paymentRef: "PMT-2026-0102", paymentDate: "2026-05-10", challanStatus: "Deposited", quarter: "Q1 FY 26-27", form16a: true },
  { deductionId: "tds-006", vendorName: "Sharma Electricals & Fittings", pan: "AAJPS8899L", section: "194C", rate: 1, tdsAmount: rs(2124), paymentRef: "PMT-2026-0124", paymentDate: "2026-07-08", challanStatus: "Filing Pending", quarter: "Q1 FY 26-27", form16a: false },
];

// ── Compliance calendar ─────────────────────────────────────────────────

export interface ComplianceItem {
  id: string;
  title: string;
  dueDate: string;
  category: "GST" | "TDS" | "Income Tax" | "Statutory" | "AISHE" | "NAAC" | "FRC";
  status: "Filed" | "Due Soon" | "Overdue" | "Upcoming";
  filedOn?: string;
  responsible: string;
}

export const MOCK_COMPLIANCE: ComplianceItem[] = [
  { id: "cmp-01", title: "GSTR-3B for June 2026", dueDate: "2026-07-20", category: "GST", status: "Due Soon", responsible: "Rajesh Iyer" },
  { id: "cmp-02", title: "GSTR-1 for June 2026", dueDate: "2026-07-11", category: "GST", status: "Overdue", responsible: "Rajesh Iyer" },
  { id: "cmp-03", title: "TDS challan deposit — Q1 FY 26-27", dueDate: "2026-07-07", category: "TDS", status: "Overdue", responsible: "Meena Kulkarni" },
  { id: "cmp-04", title: "TDS return (24Q) for Q1 FY 26-27", dueDate: "2026-07-31", category: "TDS", status: "Due Soon", responsible: "Meena Kulkarni" },
  { id: "cmp-05", title: "GSTR-3B for July 2026", dueDate: "2026-08-20", category: "GST", status: "Upcoming", responsible: "Rajesh Iyer" },
  { id: "cmp-06", title: "Income Tax — Form 16 issue to employees", dueDate: "2026-07-15", category: "Income Tax", status: "Filed", filedOn: "2026-07-12", responsible: "Meena Kulkarni" },
  { id: "cmp-07", title: "AISHE data submission 2026-27", dueDate: "2026-11-30", category: "AISHE", status: "Upcoming", responsible: "Registrar" },
  { id: "cmp-08", title: "NAAC SSR preparation milestone — draft v1", dueDate: "2026-08-15", category: "NAAC", status: "Upcoming", responsible: "IQAC Coordinator" },
  { id: "cmp-09", title: "FRC quarterly fee utilisation report (Q1)", dueDate: "2026-07-25", category: "FRC", status: "Due Soon", responsible: "Finance Controller" },
  { id: "cmp-10", title: "PF/ESI monthly challan — June", dueDate: "2026-07-15", category: "Statutory", status: "Filed", filedOn: "2026-07-13", responsible: "HR Admin" },
  { id: "cmp-11", title: "Professional tax — monthly return", dueDate: "2026-07-10", category: "Statutory", status: "Overdue", responsible: "HR Admin" },
  { id: "cmp-12", title: "GST Annual Return GSTR-9 (FY 25-26)", dueDate: "2026-12-31", category: "GST", status: "Upcoming", responsible: "Rajesh Iyer" },
];

// ── AISHE / NAAC (Reports) ──────────────────────────────────────────────

export interface AisheProgram {
  program: string;
  level: "UG" | "PG" | "Research" | "Diploma";
  intake: number;
  admitted: number;
  male: number;
  female: number;
  faculty: number;
  phd: number;
  minority: number;
}

export const MOCK_AISHE: AisheProgram[] = [
  { program: "B.Tech (Computer Science)", level: "UG", intake: 180, admitted: 176, male: 108, female: 68, faculty: 24, phd: 11, minority: 14 },
  { program: "B.Tech (Mechanical)", level: "UG", intake: 120, admitted: 112, male: 97, female: 15, faculty: 18, phd: 8, minority: 9 },
  { program: "B.Com", level: "UG", intake: 240, admitted: 228, male: 96, female: 132, faculty: 16, phd: 5, minority: 22 },
  { program: "B.Sc (Computer Science)", level: "UG", intake: 120, admitted: 114, male: 62, female: 52, faculty: 14, phd: 6, minority: 11 },
  { program: "MBA", level: "PG", intake: 60, admitted: 54, male: 29, female: 25, faculty: 12, phd: 7, minority: 4 },
  { program: "M.Sc (Mathematics)", level: "PG", intake: 45, admitted: 41, male: 18, female: 23, faculty: 9, phd: 6, minority: 3 },
  { program: "Ph.D (All Faculty)", level: "Research", intake: 30, admitted: 26, male: 15, female: 11, faculty: 0, phd: 26, minority: 2 },
];

export interface NaacMetric {
  criterion: string;
  key: string;
  value: string;
  target: string;
  status: "On Track" | "At Risk" | "Completed";
  trend: number[]; // 4-year
}

export const MOCK_NAAC_METRICS: NaacMetric[] = [
  { criterion: "Criterion 1", key: "Research grants (₹ Cr)", value: "1.85", target: "2.50", status: "At Risk", trend: [0.9, 1.2, 1.5, 1.85] },
  { criterion: "Criterion 1", key: "Grants per faculty (₹ L)", value: "4.6", target: "5.0", status: "On Track", trend: [2.8, 3.4, 4.1, 4.6] },
  { criterion: "Criterion 2", key: "Consultancy income (₹ L)", value: "94", target: "120", status: "At Risk", trend: [40, 62, 78, 94] },
  { criterion: "Criterion 3", key: "Budget allocation to research (%)", value: "12.4", target: "15.0", status: "On Track", trend: [8.2, 9.5, 11.0, 12.4] },
  { criterion: "Criterion 4", key: "Scholarship coverage (%)", value: "38.2", target: "40.0", status: "On Track", trend: [22, 28, 34, 38.2] },
  { criterion: "Criterion 5", key: "ESG initiatives (count)", value: "14", target: "12", status: "Completed", trend: [4, 7, 10, 14] },
  { criterion: "Criterion 6", key: "Student progression to PG (%)", value: "22.5", target: "25.0", status: "At Risk", trend: [14, 17, 20, 22.5] },
];

// ── Users (Settings) ────────────────────────────────────────────────────

export interface UserMock {
  userId: string;
  name: string;
  email: string;
  role: string;
  department: string | null;
  permissions: number;
  lastLogin: string;
  isActive: boolean;
}

export const MOCK_USERS: UserMock[] = [
  { userId: "usr-001", name: "Dr. Meena Kulkarni", email: "meena.kulkarni@srcoe.edu.in", role: "Accountant", department: "Finance", permissions: 28, lastLogin: "2026-07-06 09:14", isActive: true },
  { userId: "usr-002", name: "Rajesh Iyer", email: "rajesh.iyer@srcoe.edu.in", role: "Accountant", department: "Tax & Compliance", permissions: 26, lastLogin: "2026-07-06 08:45", isActive: true },
  { userId: "usr-003", name: "Prof. S. Deshpande", email: "registrar@srcoe.edu.in", role: "Registrar", department: "Administration", permissions: 34, lastLogin: "2026-07-05 17:20", isActive: true },
  { userId: "usr-004", name: "Dr. Anil Pawar", email: "anil.pawar@srcoe.edu.in", role: "CFO", department: "Finance", permissions: 41, lastLogin: "2026-07-06 10:02", isActive: true },
  { userId: "usr-005", name: "Priya Deshmukh", email: "priya.deshmukh@srcoe.edu.in", role: "Accountant", department: "Student Accounts", permissions: 22, lastLogin: "2026-07-04 14:33", isActive: true },
  { userId: "usr-006", name: "Vikram Gaikwad", email: "vikram.gaikwad@srcoe.edu.in", role: "Auditor", department: null, permissions: 12, lastLogin: "2026-07-01 11:11", isActive: false },
  { userId: "usr-007", name: "Sara Khan", email: "sara.khan@srcoe.edu.in", role: "HOD", department: "Computer Science", permissions: 9, lastLogin: "2026-07-06 09:40", isActive: true },
];

// ── Helper ──────────────────────────────────────────────────────────────

/** Simulated network latency for API-shaped hooks. */
export function mockFetch<T>(data: T, delay = 350): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(data), delay));
}

export function findStudent(idOrEnrollment: string): Student | undefined {
  const q = idOrEnrollment.trim().toLowerCase();
  return MOCK_STUDENTS.find(
    (s) => s.student_id === q || s.enrollment.toLowerCase() === q || s.name.toLowerCase().includes(q)
  );
}
