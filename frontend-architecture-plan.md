# SutraERP — Frontend Architecture & UI Plan

**Prepared by:** Senior Frontend Engineer  
**Based on:** Domain Model v1.0 + Compliance Discovery Report  
**Date:** 2026-07-22

---

## 1. Route Map

### URL Pattern Conventions
- List views: `/:context/:resource` — paginated table with filters
- Detail views: `/:context/:resource/:id` — full entity view with tabs
- Create forms: `/:context/:resource/new`
- Edit forms: `/:context/:resource/:id/edit`
- Action routes: `/:context/:resource/:id/:action`
- Reports: `/:context/reports/:reportName`
- Dashboards: `/:context/dashboard`

### Routes by Bounded Context

#### Core Shell
| Route | Screen |
|-------|--------|
| `/` | CFO Dashboard |
| `/dashboard/cfo` | CFO Dashboard |
| `/dashboard/compliance` | Compliance Calendar |
| `/settings` | System Settings |
| `/settings/users` | User Management |
| `/settings/entities` | Entity Management |

#### General Ledger (`/gl`)
| Route | Screen |
|-------|--------|
| `/gl/accounts` | Chart of Accounts |
| `/gl/accounts/new` | Create Account |
| `/gl/accounts/:id` | Account Detail |
| `/gl/accounts/:id/aishe` | AISHE Mapping |
| `/gl/accounts/:id/naac` | NAAC Mapping |
| `/gl/journals` | Journal List |
| `/gl/journals/new` | Journal Entry |
| `/gl/journals/:id` | Journal Detail |
| `/gl/journals/:id/post` | Post Journal |
| `/gl/journals/:id/reverse` | Reverse Journal |
| `/gl/reports/trial-balance` | Trial Balance |
| `/gl/reports/profit-and-loss` | P&L |
| `/gl/reports/balance-sheet` | Balance Sheet |

#### Accounts Receivable (`/ar`)
| Route | Screen |
|-------|--------|
| `/ar/fee-heads` | Fee Heads |
| `/ar/fee-structures` | Fee Structures |
| `/ar/fee-structures/new` | Create Fee Structure |
| `/ar/installment-plans` | Installment Plans |
| `/ar/students/:id/fees` | Student Fee Account |
| `/ar/students/:id/fees/assess` | Assess Fees |
| `/ar/payments/receipts` | Payment Receipts |
| `/ar/payments/receipts/new` | Record Payment |
| `/ar/payments/receipts/:id` | Receipt Detail |
| `/ar/payments/receipts/:id/allocate` | Allocate Payment |
| `/ar/payments/daily-collection` | Daily Collection |
| `/ar/payments/uncleared-cheques` | Uncleared Cheques |
| `/ar/payments/gateway` | Gateway Transactions |
| `/ar/concessions` | Concessions |
| `/ar/scholarships` | Scholarship List |
| `/ar/scholarships/schemes` | Scholarship Schemes |
| `/ar/scholarships/:id` | Scholarship Detail |
| `/ar/scholarships/:id/verify` | Verify Scholarship |
| `/ar/scholarships/:id/reconcile` | Reconcile DBT |
| `/ar/scholarships/pending-verification` | Pending Verifications |
| `/ar/refunds` | Refund List |
| `/ar/refunds/new` | Initiate Refund |
| `/ar/refunds/:id` | Refund Detail |
| `/ar/credit-notes` | Credit Notes |
| `/ar/deposits` | Security Deposits |

#### Accounts Payable (`/ap`)
| Route | Screen |
|-------|--------|
| `/ap/vendors` | Vendor Master |
| `/ap/vendors/new` | Onboard Vendor |
| `/ap/vendors/:id` | Vendor Detail |
| `/ap/vendors/:id/section197` | Section 197 Certificates |
| `/ap/purchase-requisitions` | PR List |
| `/ap/purchase-requisitions/new` | Create PR |
| `/ap/purchase-requisitions/:id` | PR Detail |
| `/ap/purchase-orders` | PO List |
| `/ap/purchase-orders/new` | Create PO |
| `/ap/purchase-orders/:id` | PO Detail |
| `/ap/purchase-orders/:id/issue` | Issue PO |
| `/ap/purchase-orders/:id/cancel` | Cancel PO |
| `/ap/goods-receipt-notes` | GRN List |
| `/ap/goods-receipt-notes/new` | Create GRN |
| `/ap/goods-receipt-notes/:id` | GRN Detail |
| `/ap/purchase-invoices` | Invoice List |
| `/ap/purchase-invoices/new` | Create Invoice |
| `/ap/purchase-invoices/:id` | Invoice Detail |
| `/ap/purchase-invoices/:id/match` | Match Invoice (3-way) |
| `/ap/purchase-invoices/:id/approve` | Approve Invoice |
| `/ap/payments` | Payment List |
| `/ap/payments/new` | Initiate Payment |
| `/ap/payments/:id` | Payment Detail |
| `/ap/payments/:id/approve` | Approve Payment |
| `/ap/payments/:id/process` | Process Payment |
| `/ap/payments/schedule` | Payment Schedule |
| `/ap/tds/deductions` | TDS Deduction Register |
| `/ap/expense-claims` | Expense Claims |

#### Treasury (`/treasury`)
| Route | Screen |
|-------|--------|
| `/treasury/bank-accounts` | Bank Accounts |
| `/treasury/bank-accounts/new` | Add Bank Account |
| `/treasury/bank-accounts/:id` | Bank Account Detail |
| `/treasury/reconciliation` | Reconciliation List |
| `/treasury/reconciliation/new` | Start Reconciliation |
| `/treasury/reconciliation/:id` | Reconciliation Workspace |
| `/treasury/gateway-config` | Payment Gateway Config |

#### Taxation (`/tax`)
| Route | Screen |
|-------|--------|
| `/tax/gst/registrations` | GST Registrations |
| `/tax/gst/itc/register` | ITC Register |
| `/tax/gst/returns/gstr1` | GSTR-1 |
| `/tax/gst/returns/gstr3b` | GSTR-3B |
| `/tax/gst/returns/gstr9` | GSTR-9 |
| `/tax/gst/rcm/payable` | RCM Payable |
| `/tax/tds/sections` | TDS Sections |
| `/tax/tds/returns` | TDS Returns |
| `/tax/tds/deductions` | TDS Deductions |
| `/tax/tds/form16` | Form 16/16A |
| `/tax/income-tax/exemption` | Income Tax Exemption |
| `/tax/income-tax/income-application` | Income Application (85%) |
| `/tax/income-tax/fcra` | FCRA Compliance |
| `/tax/income-tax/itr7` | ITR-7 Data |

#### Budget (`/budget`), Assets (`/assets`), Reports (`/reports`), Workflow (`/workflow`)
Full route tables in the complete report.

---

## 2. Shared Component Tree

### Core Data Display
- **DataTable<T>** — Generic typed table with sorting, filtering, pagination, column visibility, row selection, sticky header, horizontal scroll
- **MoneyDisplay** — ₹ with Indian number system (lakhs/crores), compact mode "₹1.5L"
- **IndianDate** — DD/MM/YYYY with short/long/relative formats
- **StatusBadge** — Configurable per domain: JournalStatus, PaymentStatus, ApprovalStatus, InvoiceStatus, ScholarshipStatus, ComplianceStatus, FilingStatus
- **ApprovalCard** — Linear approval workflow visualization with level indicators
- **AmountSummary** — Side-by-side debit/credit with balance

### Journal Entry
- **JournalForm** — Dual-list debit/credit layout with auto-balance indicator, React Hook Form + Zod
- **AccountSelector** — Hierarchical COA picker (leaf nodes only), searchable, type badges
- **JournalLineItem** — Single line with inline validation

### Financial Tables
- **TrialBalanceTable** — Collapsible hierarchy, opening/closing balance columns
- **LedgerTable** — Date-filtered transaction history with running balance
- **FeeBreakdown** — Gross Fee → Scholarship → Concession → Net Payable, color-coded
- **VendorSearch** — PAN/GSTIN verification badges

### Procurement
- **POLineItems** — Editable line item grid with auto-calculating totals, RCM indicator
- **MatchingReview** — 3-way visual comparison: PO | GRN | Invoice

### Reconciliation
- **BankReconciliationWorkspace** — Split view with auto-match suggestions, manual match flow, difference analysis
- **ReconciliationProgress** — Progress bar with segment counts

### Dashboards
- **KpiCard** — Key metric with sparkline, trend indicator, drill-through
- **ComplianceTimeline** — Vertical timeline, color-coded by urgency
- **CashFlowChart**, **RevenueTrendChart** — Recharts-based with Indian formatting

### Forms & Inputs
- **MoneyInput**, **PanInput**, **GstinInput**, **DatePicker**, **FileUpload**
- **ConfirmDialog**, **MutationFeedback**, **ErrorBoundary**, **Skeleton** variants

---

## 3. Screen Design Highlights (Top 10 Screens)

### CFO Dashboard
- 4 KPI cards (Revenue, Expenditure, Surplus, Collection Rate)
- Cash flow line chart + revenue by source stacked bar
- Compliance alerts timeline (7 upcoming deadlines)
- Quick stats: Bank balance, pending approvals, unreconciled items, grant utilization

### Chart of Accounts
- Split: 30% tree panel (5-level hierarchy, color-coded by type), 70% detail
- Account detail tabs: Overview (GST/ITC/AISHE/NAAC tags), Transaction History (LedgerTable), AISHE/NAAC Mapping

### Journal Entry
- Dual-panel: Debit lines (left), Credit lines (right)
- Live balance indicator: green "BALANCED ✓" or red pulsing "IMBALANCE: ₹X"
- Journal types: Standard, Reversing, Adjustment, RCM, ITC Reversal, TDS, Accrual, Prepayment, Opening, Closing
- Immutable post confirmation: "Corrections require a reversing entry"

### Fee Collection
- Student lookup → Payment details (mode-specific fields) → Allocation (auto/manual) → Gateway integration → Receipt
- Payment modes: UPI, NEFT, RTGS, IMPS, Card, Cash, Cheque, DD

### Purchase Order
- VendorSearch with verification badges, RCM auto-detection
- POLineItems grid with HSN, tax, RCM indicators
- Budget availability check on issue with override warning

### Bank Reconciliation
- Immersive split workspace: bank statement lines (left) vs system transactions (right)
- Auto-match + manual match flow with difference analysis
- Stat pills: Total, Matched ✓, Unmatched ✗, Partial ⚠

### GST Report (GSTR-3B)
- Entity selector, period picker, tabbed (GSTR-1/3B/9/ITC/RCM)
- Auto-computed from posted invoices, manually overridable with audit trail
- ITC register with Rule 42/43 reversal computation panels

### Scholarship Management
- Lifecycle stepper: Applied → Verified → Sanctioned → Disbursed → Reconciled
- MahaDBT reference, DBT reconciliation, refund initiation for post-payment scholarships
- Pending verification queue with bulk verify

### NAAC Dashboard
- 5-year data toggle, metric cards: Research grants, grants per faculty, consultancy, budget allocation, scholarships, ESG initiatives
- Export for NAAC submission

### Compliance Calendar
- FY timeline with month navigator, swimlane layout
- Color-coded: upcoming=blue, due-this-week=yellow, overdue=red, filed=green

---

## 4. Tech Stack & Patterns

- **Stack:** React 18 + TypeScript, Tailwind CSS, shadcn/ui, React Query, Zustand, React Router v6, React Hook Form + Zod, Recharts, Vite
- **Data Fetching:** React Query with optimistic updates; cache invalidation by context
- **State:** Zustand for UI state (sidebar, theme, filters), React Query for server state, URL params for shareable filters
- **Indian UX:** ₹ with lakhs/crores, DD/MM/YYYY, Monday-start calendars
- **Responsive:** DataTable → horizontal scroll on mobile; JournalEntry → stacked panels; Dashboard → single column
- **Performance:** TanStack Virtual for large tables, code splitting by route, skeleton loaders
