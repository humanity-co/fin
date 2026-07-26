# SutraERP — Financial Domain Model

**Version:** 1.0  
**Last Updated:** 2026-07-21  
**Author:** CA (agent-ca)  
**Status:** Draft for review by lead  

> This document is the complete Financial Domain Model for SutraERP — an AI-native Institution Operating System for Indian educational institutions. Every concept traces back to a compliance requirement from the compliance discovery report (`compliance-discovery.md`).

---

## Table of Contents

1. [Domain-Wide Conventions](#1-domain-wide-conventions)
2. [Financial Foundation](#2-financial-foundation)
   - 2.1 General Ledger
   - 2.2 Accounting Periods
   - 2.3 Cost Centers
   - 2.4 Fund Accounting
   - 2.5 Multi-Entity
3. [Accounts Receivable](#3-accounts-receivable)
   - 3.1 Student Fee Management
   - 3.2 Fee Collection
   - 3.3 Concessions & Scholarships
   - 3.4 Refunds
   - 3.5 Security Deposits
4. [Accounts Payable](#4-accounts-payable)
   - 4.1 Vendor Master
   - 4.2 Procurement
   - 4.3 Payments
   - 4.4 Employee Reimbursements
5. [Treasury & Banking](#5-treasury--banking)
   - 5.1 Bank Management
   - 5.2 Bank Reconciliation
   - 5.3 Payment Gateway Integration
6. [Taxation](#6-taxation)
   - 6.1 GST Engine
   - 6.2 TDS Engine
   - 6.3 Income Tax
7. [Budget & Planning](#7-budget--planning)
   - 7.1 Budgeting
   - 7.2 Forecasting
   - 7.3 Encumbrance Accounting
8. [Asset Accounting](#8-asset-accounting)
   - 8.1 Fixed Assets
   - 8.2 Inventory
9. [Compliance & Reporting](#9-compliance--reporting)
   - 9.1 Statutory Reports
   - 9.2 Regulatory Reports
   - 9.3 Audit
10. [Workflow & Approval](#10-workflow--approval)
    - 10.1 Approval Engine
    - 10.2 Document Management
11. [Cross-Cutting Concerns](#11-cross-cutting-concerns)
12. [Appendix: Compliance Mapping](#12-appendix-compliance-mapping)

---

## 1. Domain-Wide Conventions

### 1.1 Identifiers
All entity IDs are `UUID v4`. All aggregate root IDs are `UUID v7` (time-ordered for indexing).

### 1.2 Money
All monetary values use `Money` value object: `{ amount: Decimal(18,2), currency: INR, precision_rounding: HALF_UP }`. Stored in paise (integer) in PostgreSQL for accuracy.

### 1.3 Timestamps
All events use `UtcDateTime` with nanosecond precision. All timestamps are stored in UTC, converted to IST on display.

### 1.4 Multi-Tenancy
Every table has `tenant_id: UUID` as the partition key. Every aggregate root enforces tenant isolation.

### 1.5 Audit Trail
Every entity has `created_at`, `created_by`, `updated_at`, `updated_by`, `version` (optimistic concurrency). All mutations are logged to `audit_log` table.

### 1.6 Soft Delete
No hard deletes. All entities have `deleted_at` and `deleted_by` nullable columns.

### 1.7 Event-Driven Architecture
All domain events are published to an event bus (Redis Streams). Sagas subscribe to events and orchestrate distributed transactions.

### 1.8 CQRS-Ready
Write models (aggregates) are separate from read models (projections). Read models are denormalized for query performance.

### 1.9 Configuration-Driven
All business rules are configurable via `system_config` table — never hardcoded. Policies are stored as JSONB with schema validation.

---

## 2. Financial Foundation

### 2.1 General Ledger

#### 2.1.1 Purpose
The General Ledger (GL) is the core of the double-entry accounting engine. It maintains the Chart of Accounts (COA), processes journal entries, generates trial balance, profit & loss statements, and balance sheets. It must be compliant with Indian accounting standards, GST classification, and educational institution COA requirements.

**Compliance traces:** CD-§9.1 (Compliant COA), CD-§5 (AISHE mapping), CD-§4 (NAAC metrics), CD-§6 (UGC grant tracking)

#### 2.1.2 Business Rules

1. **Double-entry invariant:** Every journal entry must have debit total = credit total. Violation rejects the entry.
2. **COA hierarchy:** COA has 5 levels — Group (1 digit), Sub-Group (2 digits), Head (4 digits), Sub-Head (6 digits), Detailed (8 digits). AISHE mapping at Sub-Head level.
3. **Account type classification:** Every account is one of: Asset, Liability, Equity, Income, Expense. This determines its position in financial statements.
4. **GST classification on accounts:** Every income account must be tagged with a GST classification (Exempt/Taxable with rate). Every expense account must be tagged with ITC eligibility (Full ITC/Blocked/Rule 42/43).
5. **AISHE head mapping:** Every COA head at Sub-Head level maps to an AISHE reporting head (CD-§5.1). Mapping is stored in `coa_aishe_mapping` table.
6. **NAAC head mapping:** Every COA head related to research expenditure, consultancy revenue, scholarship expenditure, and environmental/gender/social initiatives maps to NAAC metric keys (CD-§4.1).
7. **UGC grant mapping:** Grant-specific COA heads are created under each grant fund, prefixed with grant_id (CD-§6.1).
8. **Journal entry immutability:** Once posted, journal entries cannot be edited. Corrections require a reversing entry and a new entry.
9. **Period lock:** Journal entries cannot be posted to a closed accounting period.
10. **Auto-numbering:** Journal entries are auto-numbered per tenant per fiscal year in the format: `INV-{YYYY}-{NNNNNN}`.
11. **Opening balance:** Opening balances for a new fiscal year are auto-generated from closing balances of the prior year.
12. **Multi-currency:** Not supported in v1. All entries are in INR.

#### 2.1.3 Aggregates

**Aggregate Root: `Journal`**
- `JournalId` (UUID)
- `JournalNumber` (string, tenant+fiscal year unique)
- `JournalType` (enum: Standard, Reversing, Adjustment, Opening, Closing, RCM, ITC_Reversal, TDS, Accrual, Prepayment)
- `AccountingPeriodId` (FK to AccountingPeriod)
- `EntityId` (FK to Entity — for multi-campus)
- `FundId` (nullable FK to Fund)
- `CostCenterId` (nullable FK to CostCenter)
- `PostingDate` (date)
- `Description` (string)
- `Status` (enum: Draft, Posted, Reversed, Cancelled)
- `TotalDebit` (Money)
- `TotalCredit` (Money)
- `PostedAt` (datetime, nullable)
- `PostedBy` (UserId, nullable)
- `ReversedById` (JournalId, nullable — self-referential for reversal pairs)
- `AttachmentIds` (UUID[])

**Entity: `JournalLine`**
- `JournalLineId` (UUID)
- `JournalId` (FK)
- `LineNumber` (int)
- `AccountId` (FK to Account)
- `DebitAmount` (Money, nullable)
- `CreditAmount` (Money, nullable)
- `Description` (string)
- `CostCenterId` (nullable FK)
- `FundId` (nullable FK)
- `ProjectId` (nullable FK)
- `ReferenceId` (string, nullable — e.g., invoice number, receipt number)
- `ReferenceType` (string, nullable — e.g., "INVOICE", "RECEIPT", "PAYMENT")
- `TaxRate` (decimal, nullable)
- `TaxAmount` (Money, nullable)
- `IsITCClaimed` (boolean, default false)
- `ITCReversalPercent` (decimal, nullable — for Rule 42/43)

**Aggregate Root: `Account`**
- `AccountId` (UUID)
- `AccountCode` (string, 8-digit)
- `AccountName` (string)
- `AccountType` (enum: Asset, Liability, Equity, Income, Expense)
- `ParentAccountId` (nullable FK — self-referential for hierarchy)
- `Level` (int: 1-5)
- `GstClassification` (enum: Exempt, Taxable_5, Taxable_12, Taxable_18, Taxable_28, Nil)
- `HsnSacCode` (string, nullable)
- `ItcEligibility` (enum: Full, Blocked, Reversal_42_43, Capital_Goods)
- `AisheHeadCode` (string, nullable — FK to AISHE head mapping)
- `IsActive` (boolean)
- `IsSystem` (boolean — cannot be deleted)
- `OpeningBalance` (Money, default 0)
- `CurrentBalance` (Money, computed)

#### 2.1.4 Value Objects

- `Money { amount: Decimal(18,2), currency: INR }` — stored as bigint paise in DB
- `AccountCode` — validated 8-digit string, hierarchical (group-subgroup-head-subhead-detailed)
- `JournalType` — enum string
- `AccountType` — enum string
- `GstClassification` — enum string
- `ItcEligibility` — enum string
- `JournalStatus` — enum string

#### 2.1.5 Commands

| Command | Description |
|---------|-------------|
| `CreateJournal(cmd: CreateJournalCmd)` | Creates a draft journal entry |
| `PostJournal(journalId, postedBy)` | Posts journal — validates debits=credits, checks period open, updates account balances |
| `ReverseJournal(journalId, reason, reversedBy)` | Creates a reversing journal entry, marks original as reversed |
| `CancelJournal(journalId, reason, cancelledBy)` | Cancels a draft journal (soft delete) |
| `CreateAccount(cmd: CreateAccountCmd)` | Creates a new COA account |
| `UpdateAccount(accountId, cmd: UpdateAccountCmd)` | Updates account metadata |
| `DeactivateAccount(accountId)` | Marks account as inactive (no new postings) |
| `MapAisheHead(accountId, aisheHeadCode)` | Maps a COA head to AISHE reporting head |
| `MapNaacMetric(accountId, naacMetricKey)` | Maps a COA head to NAAC metric |

#### 2.1.6 Queries

| Query | Description |
|-------|-------------|
| `GetAccount(accountId)` | Single account with hierarchy |
| `GetAccountByCode(code)` | Account lookup by code |
| `GetAccountTree()` | Full COA hierarchy tree |
| `GetTrialBalance(periodId, entityId?)` | Trial balance with debit/credit totals |
| `GetProfitAndLoss(periodId, entityId?, costCenterId?, fundId?)` | P&L statement |
| `GetBalanceSheet(periodId, entityId?, costCenterId?, fundId?)` | Balance sheet |
| `GetJournal(journalId)` | Single journal with lines |
| `GetJournals(filter: JournalFilter)` | Paginated journal list with filters |
| `GetJournalByNumber(number)` | Journal lookup by number |
| `GetAccountBalance(accountId, asOfDate)` | Account balance at a point in time |
| `GetAisheMappedAccounts()` | All accounts with AISHE mapping |
| `GetNaacMappedAccounts()` | All accounts with NAAC mapping |

#### 2.1.7 Events

| Event | Payload |
|-------|---------|
| `JournalCreated` | `{ journalId, journalNumber, journalType, status, createdBy, createdAt }` |
| `JournalPosted` | `{ journalId, journalNumber, totalDebit, totalCredit, periodId, postedBy, postedAt }` |
| `JournalReversed` | `{ originalJournalId, reversingJournalId, reason, reversedBy, reversedAt }` |
| `JournalCancelled` | `{ journalId, reason, cancelledBy, cancelledAt }` |
| `AccountCreated` | `{ accountId, accountCode, accountName, accountType }` |
| `AccountUpdated` | `{ accountId, changes }` |
| `AccountDeactivated` | `{ accountId, accountCode }` |
| `AisheMappingUpdated` | `{ accountId, aisheHeadCode, previousAisheHeadCode }` |
| `TrialBalanceGenerated` | `{ periodId, generatedAt, generatedBy }` |

#### 2.1.8 State Machine — Journal

```
[Draft] ──Post──> [Posted] ──Reverse──> [Reversed]
   │                                       │
   └──Cancel──> [Cancelled]                └──(links to original)
```

- `Draft`: Can be edited, cancelled, or posted.
- `Posted`: Immutable. Can be reversed.
- `Reversed`: Original entry reversed. Reversing entry is Posted.
- `Cancelled`: Soft-deleted draft. Cannot be posted.

#### 2.1.9 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `journal_auto_numbering_enabled` | true | Auto-generate journal numbers |
| `journal_number_format` | `INV-{YYYY}-{NNNNNN}` | Configurable format |
| `journal_approval_required` | false | Require approval before posting |
| `journal_approval_threshold` | 1000000 | Amount above which approval is required (₹10L) |
| `max_lines_per_journal` | 500 | Maximum lines in a single journal |
| `allow_backdated_posting` | false | Allow posting to past dates (within same period) |
| `backdated_posting_days_limit` | 15 | Max days back for backdated posting |

#### 2.1.10 Specifications

| Specification | Purpose |
|---------------|---------|
| `IsJournalBalanced` | Debits = Credits |
| `IsPeriodOpen` | Accounting period is open for posting |
| `IsAccountActive` | Account is not deactivated |
| `IsAccountLeafNode` | Account has no children (can post to it) |
| `IsAisheMappable` | Account is at Sub-Head level (4 digits) |
| `IsJournalAmountWithinLimit` | Total within approval threshold |

#### 2.1.11 API Contracts

```
POST   /api/v1/accounts                     → CreateAccount
GET    /api/v1/accounts                     → GetAccountTree (filtered)
GET    /api/v1/accounts/:id                 → GetAccount
PUT    /api/v1/accounts/:id                 → UpdateAccount
DELETE /api/v1/accounts/:id                 → DeactivateAccount
POST   /api/v1/accounts/:id/aishe-mapping  → MapAisheHead
POST   /api/v1/accounts/:id/naac-metric    → MapNaacMetric

POST   /api/v1/journals                     → CreateJournal
GET    /api/v1/journals                     → GetJournals (paginated, filterable)
GET    /api/v1/journals/:id                 → GetJournal
POST   /api/v1/journals/:id/post           → PostJournal
POST   /api/v1/journals/:id/reverse        → ReverseJournal
DELETE /api/v1/journals/:id                 → CancelJournal

GET    /api/v1/reports/trial-balance        → GetTrialBalance
GET    /api/v1/reports/profit-and-loss      → GetProfitAndLoss
GET    /api/v1/reports/balance-sheet        → GetBalanceSheet
```

**Request/Response Shapes:**

```jsonc
// POST /api/v1/journals — CreateJournal
{
  "journalType": "Standard",
  "accountingPeriodId": "uuid",
  "entityId": "uuid",
  "fundId": "uuid|null",
  "costCenterId": "uuid|null",
  "postingDate": "2026-07-21",
  "description": "Rent payment for July 2026",
  "lines": [
    {
      "lineNumber": 1,
      "accountId": "uuid",
      "debitAmount": 50000.00,
      "creditAmount": null,
      "description": "Rent expense",
      "costCenterId": "uuid|null",
      "fundId": "uuid|null",
      "referenceId": "INV-2026-001234",
      "referenceType": "INVOICE"
    },
    {
      "lineNumber": 2,
      "accountId": "uuid",
      "debitAmount": null,
      "creditAmount": 50000.00,
      "description": "Payment to landlord",
      "costCenterId": null,
      "fundId": null
    }
  ],
  "attachmentIds": ["uuid1", "uuid2"]
}
```

#### 2.1.12 Integration Contracts

| System | Integration | Direction |
|--------|------------|-----------|
| AISHE Portal | Extract COA → AISHE head mapping as CSV/JSON | Outbound |
| NAAC Portal | Generate NAAC financial metrics report | Outbound |
| GST Portal | Map HSN/SAC codes from COA | Outbound |

#### 2.1.13 Permissions & Roles

| Permission | CFO | Controller | Accountant | Auditor | Registrar |
|------------|:---:|:----------:|:----------:|:-------:|:---------:|
| `account.create` | ✓ | ✓ | ✗ | ✗ | ✗ |
| `account.update` | ✓ | ✓ | ✗ | ✗ | ✗ |
| `account.deactivate` | ✓ | ✓ | ✗ | ✗ | ✗ |
| `account.read` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `journal.create` | ✓ | ✓ | ✓ | ✗ | ✗ |
| `journal.post` | ✓ | ✓ | ✓ | ✗ | ✗ |
| `journal.reverse` | ✓ | ✓ | ✗ | ✗ | ✗ |
| `journal.read` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `report.generate` | ✓ | ✓ | ✓ | ✓ | ✓ |

#### 2.1.14 Validation Rules

| Field | Rule |
|-------|------|
| `accountCode` | Required, 8 digits, unique per tenant, must match hierarchical pattern |
| `accountName` | Required, max 200 chars, unique per parent |
| `journal.lines` | At least 2 lines, max 500 |
| `journal.lines[].debitAmount` | XOR with creditAmount (exactly one must be set) |
| `journal.lines[].accountId` | Must reference active leaf account |
| `journal.totalDebit` | Must equal journal.totalCredit |
| `journal.postingDate` | Must be within an open accounting period |
| `journal.postingDate` | Must not be in future (unless `allow_future_dated_posting` policy is true) |

#### 2.1.15 Compensation Logic

**Saga: PostJournal Saga**
1. Validate journal → fail → return error
2. Lock period → fail → release lock, return error
3. Lock accounts → fail → release period, return error
4. Update ledger balances → fail → reverse all balance updates, release locks, return error
5. Mark journal as posted → fail → reverse balances, release locks, mark journal as draft
6. Release account locks → fail → (manual intervention required — journal is posted but locks may be stale; auto-heal on retry)
7. Release period lock → fail → (auto-heal on retry)
8. Publish `JournalPosted` event → fail → (event will be retried via outbox pattern)

#### 2.1.16 Audit Trail

| Action | Fields Audited |
|--------|---------------|
| CreateAccount | All fields + timestamp + user |
| UpdateAccount | Changed fields (old/new) + timestamp + user |
| DeactivateAccount | AccountId + deactivatedAt + user |
| CreateJournal | All journal fields + lines + timestamp + user |
| PostJournal | JournalId + status change (Draft→Posted) + postedAt + user |
| ReverseJournal | OriginalJournalId + reversingJournalId + reason + user |
| CancelJournal | JournalId + reason + user |

#### 2.1.17 AI Features

- **Anomaly detection:** Detect unusual journal entries (e.g., round amounts, weekend postings, unusual account combinations) using ML-based anomaly scoring
- **Auto-categorization:** Suggest account codes for journal line descriptions using NLP
- **Duplicate detection:** Flag potential duplicate journal entries based on amount, payee, date proximity
- **Trial balance analysis:** Identify unusual fluctuations in account balances vs. historical patterns

---

### 2.2 Accounting Periods

#### 2.2.1 Purpose
Manage fiscal years, accounting periods, period open/close, and year-end processing. Ensures financial data integrity by preventing postings to closed periods.

**Compliance traces:** CD-§7.4 (Audit deadlines), CD-§9.2 (Compliance calendar)

#### 2.2.2 Business Rules

1. **Fiscal year:** April 1 to March 31 (Indian fiscal year). Cannot be changed per tenant.
2. **Periods:** Each fiscal year has 12 monthly periods (April–March) plus optional 13th period for adjustments.
3. **Period states:** Open → Closing → Closed. Only Open periods accept postings.
4. **Year-end close:** All periods must be Closed before year-end processing. Year-end close generates opening balances for the next fiscal year.
5. **Retroactive entries:** Once a period is Closed, no entries can be posted to it. Periods can be reopened only by CFO-level approval.
6. **Period locking:** System auto-locks a period 30 days after period end. Manual override with reason required.
7. **Compliance deadline tracking:** Each period tracks GST filing deadline (20th of next month), TDS filing deadline (15th of month after quarter), and other compliance dates.

#### 2.2.3 Aggregates

**Aggregate Root: `FiscalYear`**
- `FiscalYearId` (UUID)
- `YearCode` (string, e.g., "2026-27")
- `StartDate` (date, April 1)
- `EndDate` (date, March 31)
- `Status` (enum: Open, Closing, Closed)
- `IsCurrentYear` (boolean)
- `ClosedAt` (datetime, nullable)
- `ClosedBy` (UserId, nullable)

**Entity: `AccountingPeriod`**
- `AccountingPeriodId` (UUID)
- `FiscalYearId` (FK)
- `PeriodNumber` (int, 1-13)
- `PeriodName` (string, e.g., "April 2026")
- `StartDate` (date)
- `EndDate` (date)
- `Status` (enum: Open, Closing, Closed)
- `GstFilingDeadline` (date)
- `TdsFilingDeadline` (date)
- `GstFiledDate` (date, nullable)
- `TdsFiledDate` (date, nullable)
- `ClosedAt` (datetime, nullable)
- `ClosedBy` (UserId, nullable)

#### 2.2.4 Value Objects

- `FiscalYearStatus` — enum
- `PeriodStatus` — enum

#### 2.2.5 Commands

| Command | Description |
|---------|-------------|
| `OpenFiscalYear(yearCode)` | Create and open a new fiscal year |
| `ClosePeriod(periodId, closedBy, reason)` | Close an accounting period |
| `ReopenPeriod(periodId, reopenedBy, reason)` | Reopen a closed period (requires CFO approval) |
| `CloseFiscalYear(fiscalYearId, closedBy)` | Close entire fiscal year after all periods closed |
| `GenerateOpeningBalances(fiscalYearId)` | Generate opening balances for next fiscal year |

#### 2.2.6 Queries

| Query | Description |
|-------|-------------|
| `GetCurrentFiscalYear()` | Current fiscal year with periods |
| `GetFiscalYear(yearCode)` | Fiscal year by code |
| `GetFiscalYears()` | List all fiscal years |
| `GetOpenPeriods(asOfDate?)` | All open periods |
| `GetPeriod(periodId)` | Single period details |
| `GetComplianceCalendar(fiscalYearId)` | Compliance deadlines for the year |

#### 2.2.7 Events

| Event | Payload |
|-------|---------|
| `FiscalYearOpened` | `{ fiscalYearId, yearCode, startDate, endDate }` |
| `PeriodClosed` | `{ periodId, periodNumber, fiscalYearId, closedBy, closedAt }` |
| `PeriodReopened` | `{ periodId, periodNumber, fiscalYearId, reason, reopenedBy }` |
| `FiscalYearClosed` | `{ fiscalYearId, yearCode, closedBy, closedAt }` |
| `OpeningBalancesGenerated` | `{ fiscalYearId, fromYearCode, toYearCode, generatedBy }` |

#### 2.2.8 State Machine — AccountingPeriod

```
[Open] ──Close──> [Closing] ──(auto after X days)──> [Closed]
  ^                                                     │
  └───────────────────Reopen─────────────────────────────┘
```

#### 2.2.9 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `period_auto_lock_days` | 30 | Days after period end to auto-lock |
| `allow_period_reopen` | true | Whether period reopening is allowed |
| `period_reopen_approval_required` | true | CFO approval needed for reopen |
| `max_adjustment_periods` | 1 | Number of 13th-period adjustment periods |
| `compliance_deadline_reminder_days` | [7, 3, 1] | Days before deadline to send reminders |

#### 2.2.10 Permissions

| Permission | CFO | Controller | Accountant | Auditor |
|------------|:---:|:----------:|:----------:|:-------:|
| `period.close` | ✓ | ✓ | ✗ | ✗ |
| `period.reopen` | ✓ | ✗ | ✗ | ✗ |
| `fiscalyear.close` | ✓ | ✗ | ✗ | ✗ |
| `fiscalyear.read` | ✓ | ✓ | ✓ | ✓ |

#### 2.2.11 Validation Rules

| Field | Rule |
|-------|------|
| `fiscalYear.startDate` | Must be April 1 |
| `fiscalYear.endDate` | Must be March 31 |
| `period.periodNumber` | 1-13, unique per fiscal year |
| `period.startDate` | Must be 1st of month |
| `period.endDate` | Must be last day of month |
| `closePeriod` | All journals in period must be posted |
| `closeFiscalYear` | All 12 periods must be closed |

#### 2.2.12 Compensation — ClosePeriod Saga

1. Validate period can close → fail → return error
2. Lock all accounts with activity in period → fail → return error
3. Generate period-end reports → fail → release locks, return error
4. Mark period as Closing → fail → release locks, return error
5. Run integrity checks (GL balance = sub-ledger totals) → fail → revert to Open, release locks
6. Mark period as Closed → fail → revert to Open, release locks
7. Release account locks → (auto-heal)
8. Publish `PeriodClosed` event → (retry via outbox)

---

### 2.3 Cost Centers

#### 2.3.1 Purpose
Track costs and revenues by department, campus, project, or any other dimension. Enables granular profitability analysis and budget control.

**Compliance traces:** CD-§4.1 (Research grants per faculty), CD-§6.3 (Development grants)

#### 2.3.2 Business Rules

1. **Cost center hierarchy:** Flat or tree structure (parent-child). Max depth: 5 levels.
2. **Cost center types:** Department, Campus, Project, Activity, Program, Course.
3. **Multi-dimensional:** A single transaction can be split across multiple cost centers.
4. **Budget control:** Cost centers can have budgets. Postings exceeding budget can be warned or blocked (configurable).
5. **Cost center manager:** Each cost center can have a manager who receives budget alerts.

#### 2.3.3 Aggregates

**Aggregate Root: `CostCenter`**
- `CostCenterId` (UUID)
- `Code` (string, tenant-unique)
- `Name` (string)
- `Type` (enum: Department, Campus, Project, Activity, Program, Course)
- `ParentId` (nullable FK)
- `ManagerId` (UserId, nullable)
- `EntityId` (FK to Entity — campus association)
- `IsActive` (boolean)
- `BudgetAmount` (Money, nullable)
- `BudgetPeriod` (enum: Monthly, Quarterly, Annual)

#### 2.3.4 Value Objects

- `CostCenterType` — enum

#### 2.3.5 Commands

| Command | Description |
|---------|-------------|
| `CreateCostCenter(cmd)` | Create a new cost center |
| `UpdateCostCenter(id, cmd)` | Update cost center |
| `DeactivateCostCenter(id)` | Deactivate cost center |
| `AssignCostCenterManager(id, userId)` | Assign manager |
| `SetCostCenterBudget(id, amount, period)` | Set budget |

#### 2.3.6 Queries

| Query | Description |
|-------|-------------|
| `GetCostCenter(id)` | Single cost center |
| `GetCostCenterTree()` | Full hierarchy |
| `GetCostCenterByType(type)` | Cost centers by type |
| `GetCostCenterBudgetVsActual(id, periodId)` | Budget vs actual report |
| `GetCostCenterSummary(entityId, periodId)` | Summary by entity |

#### 2.3.7 Events

| Event | Payload |
|-------|---------|
| `CostCenterCreated` | `{ id, code, name, type }` |
| `CostCenterBudgetSet` | `{ id, amount, period }` |
| `CostCenterManagerAssigned` | `{ id, managerId }` |
| `CostCenterBudgetExceeded` | `{ id, budgetAmount, actualAmount, periodId }` |

#### 2.3.8 Permissions

| Permission | CFO | Controller | Accountant |
|------------|:---:|:----------:|:----------:|
| `costcenter.create` | ✓ | ✓ | ✗ |
| `costcenter.update` | ✓ | ✓ | ✗ |
| `costcenter.budget.set` | ✓ | ✓ | ✗ |
| `costcenter.read` | ✓ | ✓ | ✓ |

---

### 2.4 Fund Accounting

#### 2.4.1 Purpose
Manage grant funds, endowment funds, and restricted/unrestricted funds with full audit trail. Essential for UGC compliance, grant utilization tracking, and Section 11(5) investment compliance.

**Compliance traces:** CD-§6.1 (UGC grants), CD-§6.3 (Endowment), CD-§7.2 (85% application rule, Section 11(5)), CD-§7.3 (FCRA)

#### 2.4.2 Business Rules

1. **Fund types:** Restricted (grant-specific), Unrestricted (general operations), Endowment (corpus), FCRA (foreign contributions), Scholarship (DBT pass-through).
2. **Fund segregation:** Each fund has its own ledger within the GL. All transactions to/from a fund must reference the fund.
3. **Grant budget tracking:** Each grant fund has approved budget heads. Expenditure must be within approved heads.
4. **Utilization Certificate (UC) generation:** For UGC grants, UC must be generated in GFR 12-A format (CD-§6.1). UC tracks: grant amount, expenditure, unspent balance, interest earned.
5. **85% application rule:** Track total income and 85% applied to educational purposes. Flag when threshold is not met (CD-§7.2).
6. **Section 11(5) compliance:** Track investments in specified securities. Flag non-compliant investments (CD-§7.2).
7. **FCRA compliance:** Separate ledger for FCRA funds. Track administrative expenses (≤20% of receipts). No re-granting (CD-§7.3).
8. **Endowment fund:** Principal cannot be touched. Only income can be used for development (CD-§6.3).
9. **Unspent balance tracking:** For grants, unspent balance at year-end is tracked and reported.
10. **Interest tracking:** Interest earned on grant funds must be reported separately.

#### 2.4.3 Aggregates

**Aggregate Root: `Fund`**
- `FundId` (UUID)
- `Code` (string, tenant-unique)
- `Name` (string)
- `FundType` (enum: Restricted, Unrestricted, Endowment, FCRA, Scholarship)
- `Source` (enum: Government_UGC, Government_State, Government_Other, Private, Donation, Internal, FCRA)
- `GrantScheme` (string, nullable — e.g., "SAP", "DRS", "DSA")
- `SanctionOrderNumber` (string, nullable)
- `SanctionDate` (date, nullable)
- `SanctionedAmount` (Money, nullable)
- `ReceivedAmount` (Money, computed)
- `ExpenditureAmount` (Money, computed)
- `UnspentBalance` (Money, computed)
- `InterestEarned` (Money, computed)
- `StartDate` (date, nullable)
- `EndDate` (date, nullable)
- `Status` (enum: Active, Completed, Terminated, Suspended)
- `BankAccountId` (FK to BankAccount, nullable — for separate bank account requirement)
- `IsSection115Compliant` (boolean, computed)
- `FcraRegistrationNumber` (string, nullable)
- `FcraAdminExpenseRatio` (decimal, max 20%)
- `PrincipalAmount` (Money, nullable — for endowment funds)
- `IncomeOnly` (boolean, default false — for endowment funds)

**Entity: `FundBudgetHead`**
- `FundBudgetHeadId` (UUID)
- `FundId` (FK)
- `AccountId` (FK to Account)
- `ApprovedAmount` (Money)
- `UtilizedAmount` (Money, computed)
- `EncumberedAmount` (Money, computed)

#### 2.4.4 Value Objects

- `FundType` — enum
- `FundSource` — enum
- `FundStatus` — enum

#### 2.4.5 Commands

| Command | Description |
|---------|-------------|
| `CreateFund(cmd)` | Create a new fund |
| `UpdateFund(id, cmd)` | Update fund details |
| `SanctionFund(id, sanctionOrderNumber, amount)` | Record grant sanction |
| `ReceiveFundAmount(id, amount, receivedDate, reference)` | Record fund receipt |
| `CloseFund(id, reason)` | Close/completed fund |
| `SetFundBudget(id, budgetHeads[])` | Set approved budget heads |
| `GenerateUtilizationCertificate(id, fiscalYearId)` | Generate GFR 12-A UC |
| `TrackIncomeApplication(fiscalYearId)` | Calculate 85% application ratio |
| `FlagNonCompliantInvestments()` | Check Section 11(5) compliance |

#### 2.4.6 Queries

| Query | Description |
|-------|-------------|
| `GetFund(id)` | Fund details with balances |
| `GetFunds(filter)` | List funds with filters |
| `GetFundUtilization(id, fiscalYearId)` | Utilization report |
| `GetGrantSummary(fiscalYearId)` | All grants summary |
| `GetIncomeApplicationRatio(fiscalYearId)` | 85% application ratio calculation |
| `GetFcraComplianceSummary(fiscalYearId)` | FCRA compliance report |
| `GetUtilizationCertificate(id, fiscalYearId)` | UC in GFR 12-A format |
| `GetEndowmentFunds()` | List all endowment funds |

#### 2.4.7 Events

| Event | Payload |
|-------|---------|
| `FundCreated` | `{ fundId, code, name, fundType, source }` |
| `FundSanctioned` | `{ fundId, sanctionOrderNumber, sanctionedAmount }` |
| `FundAmountReceived` | `{ fundId, amount, receivedDate, reference }` |
| `FundClosed` | `{ fundId, unspentBalance, reason }` |
| `UtilizationCertificateGenerated` | `{ fundId, fiscalYearId, documentUrl }` |
| `IncomeApplicationThresholdMissed` | `{ fiscalYearId, appliedPercent, thresholdPercent }` |
| `Section115ComplianceBreach` | `{ fundId, investmentDetails }` |
| `FcraAdminExpenseExceeded` | `{ fundId, expenseRatio, maxAllowed }` |

#### 2.4.8 State Machine — Fund

```
[Active] ──Complete──> [Completed]
   │                        │
   ├──Terminate──> [Terminated]
   └──Suspend───> [Suspended] ──Resume──> [Active]
```

#### 2.4.9 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `income_application_threshold` | 85 | Minimum % of income to be applied (Section 11) |
| `income_accumulation_years` | 5 | Max years for accumulated income to be applied |
| `fcra_admin_expense_limit` | 20 | Max % for FCRA admin expenses |
| `endowment_principal_untouchable` | true | Endowment principal cannot be spent |
| `uc_auto_generate_at_year_end` | true | Auto-generate UC at fiscal year end |
| `grant_separate_bank_account_required` | true | Grants must have separate bank account |

#### 2.4.10 Specifications

| Specification | Purpose |
|---------------|---------|
| `IsFundActive` | Fund is in Active status |
| `IsWithinBudgetHead` | Transaction is within approved budget head limit |
| `MeetsIncomeApplicationRule` | 85% of income applied to educational purposes |
| `IsSection115Compliant` | Investments are in specified securities |
| `IsFcraAdminExpenseCompliant` | Admin expenses ≤ 20% of FCRA receipts |
| `IsEndowmentPrincipalIntact` | Principal amount has not been touched |

#### 2.4.11 API Contracts

```
POST   /api/v1/funds                     → CreateFund
GET    /api/v1/funds                     → GetFunds
GET    /api/v1/funds/:id                 → GetFund
PUT    /api/v1/funds/:id                 → UpdateFund
POST   /api/v1/funds/:id/sanction       → SanctionFund
POST   /api/v1/funds/:id/receive        → ReceiveFundAmount
POST   /api/v1/funds/:id/close          → CloseFund
POST   /api/v1/funds/:id/budget-heads   → SetFundBudget
GET    /api/v1/funds/:id/utilization    → GetFundUtilization
GET    /api/v1/funds/:id/uc             → GetUtilizationCertificate
GET    /api/v1/reports/income-application → GetIncomeApplicationRatio
```

#### 2.4.12 Permissions

| Permission | CFO | Controller | Accountant | Auditor |
|------------|:---:|:----------:|:----------:|:-------:|
| `fund.create` | ✓ | ✗ | ✗ | ✗ |
| `fund.update` | ✓ | ✓ | ✗ | ✗ |
| `fund.sanction` | ✓ | ✓ | ✗ | ✗ |
| `fund.receive` | ✓ | ✓ | ✓ | ✗ |
| `fund.close` | ✓ | ✗ | ✗ | ✗ |
| `fund.read` | ✓ | ✓ | ✓ | ✓ |
| `fund.uc.generate` | ✓ | ✓ | ✓ | ✗ |
| `fund.uc.view` | ✓ | ✓ | ✓ | ✓ |

#### 2.4.13 Validation Rules

| Field | Rule |
|-------|------|
| `fund.code` | Required, unique per tenant, max 20 chars |
| `fund.fundType` | Required |
| `fund.sanctionedAmount` | Must be positive |
| `fund.receiveAmount` | Must be positive, cannot exceed sanctioned amount |
| `fundBudgetHead.approvedAmount` | Must be positive |
| `fund.endowment.principalAmount` | Required for endowment funds |
| `fund.fcraAdminExpenseRatio` | Max 20 |

#### 2.4.14 Compensation — ReceiveFundAmount Saga

1. Create journal entry for fund receipt → fail → return error
2. Update fund received amount → fail → reverse journal, return error
3. Update bank balance → fail → reverse journal, revert fund amount, return error
4. Publish `FundAmountReceived` event → (retry via outbox)

#### 2.4.15 AI Features

- **Grant utilization prediction:** Predict which grants will have unspent balances at year-end
- **Compliance risk scoring:** Score each fund for compliance risk based on spending patterns
- **Anomalous grant spending:** Detect unusual spending patterns that may indicate misuse

---

### 2.5 Multi-Entity

#### 2.5.1 Purpose
Support multi-campus and multi-institute operations with consolidation. Each entity maintains its own books, but the system supports consolidated reporting across entities.

**Compliance traces:** CD-§9.3 (Multi-GSTIN support) — each campus may have its own GSTIN

#### 2.5.2 Business Rules

1. **Entity topology:** Each entity (campus/institute) has its own complete set of books (COA, journals, ledgers).
2. **Shared COA:** All entities under a tenant share a common COA template, but can customize (add local accounts).
3. **Inter-entity transactions:** Supported via inter-entity journal entries with due-to/due-from accounts.
4. **Consolidation:** Eliminate inter-entity transactions during consolidation. Consolidated financial statements are generated at tenant level.
5. **Multi-GSTIN:** Each entity can have its own GSTIN. GSTR-1/3B filed per GSTIN.
6. **Entity types:** Main Campus, Satellite Campus, Research Center, Skill Center.

#### 2.5.3 Aggregates

**Aggregate Root: `Entity`**
- `EntityId` (UUID)
- `TenantId` (UUID)
- `Code` (string, tenant-unique)
- `Name` (string)
- `Type` (enum: MainCampus, SatelliteCampus, ResearchCenter, SkillCenter, Institute)
- `Gstin` (string, nullable)
- `Pan` (string, nullable)
- `Address` (Address value object)
- `IsActive` (boolean)
- `ParentEntityId` (nullable FK — for hierarchy)
- `ConsolidationGroup` (string, nullable)

#### 2.5.4 Value Objects

- `EntityType` — enum
- `Address { line1, line2, city, state, pincode, country }`

#### 2.5.5 Commands

| Command | Description |
|---------|-------------|
| `CreateEntity(cmd)` | Create a new entity (campus/institute) |
| `UpdateEntity(id, cmd)` | Update entity details |
| `DeactivateEntity(id)` | Deactivate entity |
| `RecordInterEntityTransaction(cmd)` | Record transfer between entities |
| `GenerateConsolidatedReport(periodId)` | Generate consolidated financial statements |

#### 2.5.6 Queries

| Query | Description |
|-------|-------------|
| `GetEntity(id)` | Single entity |
| `GetEntities()` | All entities |
| `GetEntityTree()` | Entity hierarchy |
| `GetConsolidatedTrialBalance(periodId)` | Consolidated trial balance |
| `GetConsolidatedPL(periodId)` | Consolidated P&L |
| `GetConsolidatedBalanceSheet(periodId)` | Consolidated balance sheet |
| `GetInterEntityBalances(periodId)` | Inter-entity due-to/due-from |

#### 2.5.7 Events

| Event | Payload |
|-------|---------|
| `EntityCreated` | `{ entityId, code, name, type }` |
| `EntityDeactivated` | `{ entityId, code }` |
| `InterEntityTransactionRecorded` | `{ fromEntityId, toEntityId, amount, reference }` |
| `ConsolidatedReportGenerated` | `{ periodId, reportType }` |

#### 2.5.8 Permissions

| Permission | CFO | Controller | Accountant |
|------------|:---:|:----------:|:----------:|
| `entity.create` | ✓ | ✗ | ✗ |
| `entity.update` | ✓ | ✓ | ✗ |
| `entity.consolidate` | ✓ | ✓ | ✗ |
| `entity.read` | ✓ | ✓ | ✓ |

---

## 3. Accounts Receivable (AR)

### 3.1 Student Fee Management

#### 3.1.1 Purpose
Define fee structures, templates, rules, and installments for student fee assessment. Handles the complexity of multiple programs, categories, and scholarship-adjusted fee calculation.

**Compliance traces:** CD-§3.3 (Fee structure with scholarship), CD-§8.2 (FRC fee regulation)

#### 3.1.2 Business Rules

1. **Fee structure hierarchy:** Program → Academic Year → Semester/Year → Fee Category → Fee Head.
2. **Fee heads:** Tuition, Development, Examination, Library, Laboratory, Sports, Cultural, Admission, Registration, Hostel, Mess, Transportation, Caution Deposit, etc.
3. **GST classification per fee head:** Each fee head is tagged as exempt or taxable with rate (CD-§1.1).
4. **FRC-approved fee structure:** Fee structure must link to FRC approval order number and date (CD-§8.2).
5. **Fee templates:** Reusable fee templates by program type, year, category.
6. **Installment plans:** Fee can be split into installments (e.g., 2 per year, 4 per semester). Each installment has due date and late fee rules.
7. **Category-based fee variation:** Fee amounts can vary by student category (General, SC, ST, OBC, EWS, PwD, etc.).
8. **Scholarship-adjusted fee display:** Fee structure shows: Gross Fee → Scholarship (expected) → Net Payable (CD-§3.3).
9. **Fee revision:** Fee can be revised year-over-year. Previous year's fee structure is archived.
10. **Late fee:** Configurable late fee as fixed amount or percentage per day of delay.

#### 3.1.3 Aggregates

**Aggregate Root: `FeeStructure`**
- `FeeStructureId` (UUID)
- `EntityId` (FK)
- `ProgramId` (UUID — references academic program)
- `AcademicYear` (string, e.g., "2026-27")
- `SemesterTerm` (string, e.g., "Annual", "Sem-1", "Sem-2")
- `StudentCategory` (enum: General, SC, ST, OBC, EWS, VJNT, SBC, PwD, Other)
- `Name` (string)
- `FrcApprovalOrderNumber` (string, nullable)
- `FrcApprovalDate` (date, nullable)
- `Status` (enum: Draft, Active, Archived)
- `EffectiveFrom` (date)
- `EffectiveTo` (date, nullable)

**Entity: `FeeHead`**
- `FeeHeadId` (UUID)
- `Code` (string, tenant-unique)
- `Name` (string)
- `FeeType` (enum: Tuition, Development, Examination, Library, Laboratory, Sports, Cultural, Admission, Registration, Hostel, Mess, Transportation, CautionDeposit, Other)
- `GstClassification` (enum: Exempt, Taxable_5, Taxable_12, Taxable_18)
- `HsnSacCode` (string, nullable)
- `IsOptional` (boolean)
- `IsRefundable` (boolean)
- `IsMandatory` (boolean)

**Entity: `FeeStructureLine`**
- `FeeStructureLineId` (UUID)
- `FeeStructureId` (FK)
- `FeeHeadId` (FK)
- `Amount` (Money)
- `IsOptional` (boolean, overrides FeeHead)
- `InstallmentAllowed` (boolean)

**Entity: `InstallmentPlan`**
- `InstallmentPlanId` (UUID)
- `FeeStructureId` (FK)
- `Name` (string, e.g., "2 Installments", "4 Installments")
- `NumberOfInstallments` (int)
- `InstallmentDistribution` (jsonb — e.g., `[{"number":1,"percentage":50,"dueDate":"2026-07-15"},{"number":2,"percentage":50,"dueDate":"2026-12-15"}]`)

#### 3.1.4 Value Objects

- `FeeType` — enum
- `StudentCategory` — enum
- `FeeStructureStatus` — enum

#### 3.1.5 Commands

| Command | Description |
|---------|-------------|
| `CreateFeeHead(cmd)` | Create a new fee head |
| `UpdateFeeHead(id, cmd)` | Update fee head |
| `CreateFeeStructure(cmd)` | Create fee structure |
| `ActivateFeeStructure(id)` | Activate fee structure |
| `ArchiveFeeStructure(id)` | Archive old fee structure |
| `CreateInstallmentPlan(cmd)` | Create installment plan |
| `AssessStudentFees(studentId, feeStructureId, installmentPlanId)` | Generate fee assessment for a student |
| `ReviseFeeAssessment(studentId, feeAssessmentId, delta)` | Revise an existing fee assessment |

#### 3.1.6 Queries

| Query | Description |
|-------|-------------|
| `GetFeeHead(id)` | Single fee head |
| `GetFeeHeads()` | All fee heads |
| `GetFeeStructure(id)` | Fee structure with lines |
| `GetFeeStructures(filter)` | Filtered fee structures |
| `GetStudentFeeAssessment(studentId, academicYear)` | Student's fee assessment |
| `GetInstallmentPlans(feeStructureId)` | Available installment plans |
| `GetFeeStructureByProgram(programId, academicYear, category)` | Get applicable fee structure |

#### 3.1.7 Events

| Event | Payload |
|-------|---------|
| `FeeHeadCreated` | `{ feeHeadId, code, name, feeType }` |
| `FeeStructureCreated` | `{ feeStructureId, programId, academicYear, status }` |
| `FeeStructureActivated` | `{ feeStructureId, effectiveFrom }` |
| `FeeStructureArchived` | `{ feeStructureId }` |
| `StudentFeeAssessed` | `{ studentId, feeAssessmentId, totalAmount, installments }` |
| `FeeAssessmentRevised` | `{ studentId, feeAssessmentId, deltaAmount, reason }` |

#### 3.1.8 State Machine — FeeStructure

```
[Draft] ──Activate──> [Active] ──Archive──> [Archived]
                          │
                          └──(new version created)──> [Active] (v2)
```

#### 3.1.9 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `late_fee_type` | percentage | Fixed or percentage |
| `late_fee_percentage_per_day` | 0.1 | 0.1% per day late fee |
| `late_fee_fixed_amount` | 100 | Fixed late fee in ₹ |
| `late_fee_grace_period_days` | 7 | Days after due date before late fee applies |
| `max_installments` | 6 | Maximum number of installments |
| `fee_revision_requires_approval` | true | Fee revision requires CFO approval |
| `scholarship_adjustment_order` | scholarship_first | Order: scholarship, then concession, then net payable |

#### 3.1.10 Specifications

| Specification | Purpose |
|---------------|---------|
| `IsFeeHeadMandatory` | Fee head is mandatory for all students |
| `IsFeeStructureActive` | Fee structure is currently active |
| `IsFeeAssessmentPaid` | All installments are paid |
| `IsLateFeeApplicable` | Current date is past due date plus grace period |
| `IsFrcCompliant` | Fee structure references FRC approval |

#### 3.1.11 API Contracts

```
POST   /api/v1/fee-heads                → CreateFeeHead
GET    /api/v1/fee-heads                → GetFeeHeads
GET    /api/v1/fee-heads/:id            → GetFeeHead
PUT    /api/v1/fee-heads/:id            → UpdateFeeHead

POST   /api/v1/fee-structures           → CreateFeeStructure
GET    /api/v1/fee-structures           → GetFeeStructures
GET    /api/v1/fee-structures/:id       → GetFeeStructure
PUT    /api/v1/fee-structures/:id       → ActivateFeeStructure (status=Active)
POST   /api/v1/fee-structures/:id/archive → ArchiveFeeStructure

POST   /api/v1/installment-plans        → CreateInstallmentPlan
GET    /api/v1/installment-plans        → GetInstallmentPlans

POST   /api/v1/students/:id/fee-assessment → AssessStudentFees
GET    /api/v1/students/:id/fee-assessment  → GetStudentFeeAssessment
PUT    /api/v1/students/:id/fee-assessment/:assessmentId → ReviseFeeAssessment
```

#### 3.1.12 Permissions

| Permission | CFO | Controller | Accountant | Registrar |
|------------|:---:|:----------:|:----------:|:---------:|
| `feehead.create` | ✓ | ✓ | ✗ | ✗ |
| `feehead.update` | ✓ | ✓ | ✗ | ✗ |
| `feestructure.create` | ✓ | ✓ | ✗ | ✓ |
| `feestructure.activate` | ✓ | ✓ | ✗ | ✗ |
| `feestructure.archive` | ✓ | ✓ | ✗ | ✗ |
| `feeassessment.create` | ✓ | ✓ | ✓ | ✓ |
| `feeassessment.revise` | ✓ | ✓ | ✗ | ✗ |
| `feeassessment.read` | ✓ | ✓ | ✓ | ✓ |

#### 3.1.13 Validation Rules

| Field | Rule |
|-------|------|
| `feeHead.code` | Required, unique per tenant, max 10 chars |
| `feeHead.name` | Required, max 100 chars |
| `feeStructureLine.amount` | Must be positive |
| `installmentPlan.percentage` | Sum must equal 100% |
| `installmentPlan.dueDate` | Must be within academic year |
| `feeStructure.frcApprovalOrderNumber` | Required if FRC-fee institution |

#### 3.1.14 AI Features

- **Fee structure optimization:** Suggest optimal fee structure based on historical collection patterns
- **Default prediction:** Predict which students are likely to default on fee payments
- **Late fee revenue forecasting:** Predict late fee revenue based on historical payment patterns

---

### 3.2 Fee Collection

#### 3.2.1 Purpose
Process fee payments from students via multiple channels — online (UPI, NEFT, RTGS, IMPS, Card) and offline (Cash, Cheque, DD, POS). Record receipts, allocate to installments, handle partial payments.

**Compliance traces:** CD-§8.2 (FRC refund rules), CD-§3.3 (DBT reconciliation)

#### 3.2.2 Business Rules

1. **Payment modes:** Cash, Cheque, Demand Draft, NEFT, RTGS, IMPS, UPI, Credit Card, Debit Card, POS, Payment Gateway.
2. **Receipt numbering:** Auto-generated per entity per fiscal year: `RCP-{ENTITY}-{YYYY}-{NNNNNN}`.
3. **Payment allocation:** A single payment can be allocated to multiple installments/fee heads.
4. **Partial payment:** Allowed. Outstanding balance is tracked.
5. **Excess payment:** Recorded as credit note or refund (configurable).
6. **Cheque/DD clearance:** Cheque/DD receipts are recorded as "Uncleared" until cleared. Bounce handling.
7. **Payment gateway integration:** Real-time status updates from gateways (BillDesk, Razorpay, CCAvenue, PhonePe, Paytm).
8. **Scholarship DBT reconciliation:** When DBT payment arrives from MahaDBT, match against student's fee assessment. If already paid by student, generate refund (CD-§3.3).
9. **Receipt cancellation:** Only uncancelable after end of fiscal year. Requires CFO approval.
10. **Payment segregation:** Online payments must match with bank statement during reconciliation.

#### 3.2.3 Aggregates

**Aggregate Root: `PaymentReceipt`**
- `PaymentReceiptId` (UUID)
- `ReceiptNumber` (string, tenant+entity+fiscal year unique)
- `EntityId` (FK to Entity)
- `StudentId` (UUID)
- `FeeAssessmentId` (FK)
- `PaymentMode` (enum: Cash, Cheque, DD, NEFT, RTGS, IMPS, UPI, CreditCard, DebitCard, POS, PaymentGateway)
- `PaymentDate` (datetime)
- `Amount` (Money)
- `Status` (enum: Pending, Completed, Failed, Refunded, Cancelled, Uncleared, Bounced)
- `GatewayPaymentId` (string, nullable — for PG transactions)
- `GatewayReference` (string, nullable)
- `BankTransactionReference` (string, nullable)
- `ChequeNumber` (string, nullable)
- `ChequeDate` (date, nullable)
- `ChequeBank` (string, nullable)
- `ClearedDate` (date, nullable)
- `Remarks` (string)
- `ReceivedById` (UserId)
- `PaymentJournalId` (FK to Journal, nullable)

**Entity: `PaymentAllocation`**
- `PaymentAllocationId` (UUID)
- `PaymentReceiptId` (FK)
- `InstallmentId` (FK to Installment)
- `FeeHeadId` (FK to FeeHead)
- `AllocatedAmount` (Money)
- `ScholarshipAmount` (Money, default 0)
- `ConcessionAmount` (Money, default 0)

**Entity: `PaymentGatewayTransaction`**
- `PaymentGatewayTransactionId` (UUID)
- `PaymentReceiptId` (FK)
- `Gateway` (enum: BillDesk, Razorpay, CCAvenue, PhonePe, Paytm)
- `GatewayTransactionId` (string, unique)
- `Status` (enum: Initiated, Pending, Success, Failed, Refunded)
- `RequestPayload` (jsonb)
- `ResponsePayload` (jsonb)
- `ErrorCode` (string, nullable)
- `ErrorMessage` (string, nullable)

#### 3.2.4 Value Objects

- `PaymentMode` — enum
- `ReceiptStatus` — enum
- `GatewayType` — enum
- `GatewayTransactionStatus` — enum

#### 3.2.5 Commands

| Command | Description |
|---------|-------------|
| `RecordPaymentReceipt(cmd)` | Record a payment receipt |
| `AllocatePayment(receiptId, allocations[])` | Allocate payment to installments/fee heads |
| `ClearCheque(receiptId, clearedDate)` | Mark cheque as cleared |
| `BounceCheque(receiptId, reason)` | Mark cheque as bounced |
| `CancelReceipt(receiptId, reason, cancelledBy)` | Cancel a receipt |
| `RefundReceipt(receiptId, amount, reason)` | Process refund against receipt |
| `ReconcileScholarshipDBT(studentId, dbtAmount, dbtDate, scheme)` | Reconcile DBT scholarship payment |
| `InitiateGatewayPayment(cmd)` | Initiate payment gateway transaction |
| `UpdateGatewayPaymentStatus(transactionId, status, payload)` | Update from gateway webhook |

#### 3.2.6 Queries

| Query | Description |
|-------|-------------|
| `GetPaymentReceipt(receiptId)` | Receipt with allocations |
| `GetReceiptByNumber(number)` | Receipt lookup |
| `GetStudentReceipts(studentId, academicYear)` | All receipts for a student |
| `GetDailyCollection(entityId, date)` | Daily collection summary |
| `GetPaymentStatistics(filter)` | Aggregated payment stats |
| `GetUnclearedCheques(entityId)` | All uncleared cheques |
| `GetGatewayTransaction(transactionId)` | Gateway transaction details |
| `GetPendingReconciliation()` | Payments pending bank reconciliation |
| `GetScholarshipDBTReconciliation(scheme, period)` | DBT reconciliation report |

#### 3.2.7 Events

| Event | Payload |
|-------|---------|
| `PaymentReceiptCreated` | `{ receiptId, receiptNumber, studentId, amount, paymentMode }` |
| `PaymentAllocated` | `{ receiptId, allocations }` |
| `ChequeCleared` | `{ receiptId, chequeNumber, clearedDate }` |
| `ChequeBounced` | `{ receiptId, chequeNumber, reason }` |
| `ReceiptCancelled` | `{ receiptId, reason, cancelledBy }` |
| `ReceiptRefunded` | `{ receiptId, refundAmount, refundReceiptId }` |
| `GatewayPaymentInitiated` | `{ transactionId, gateway, amount }` |
| `GatewayPaymentSuccess` | `{ transactionId, gatewayTransactionId, receiptId }` |
| `GatewayPaymentFailed` | `{ transactionId, errorCode, errorMessage }` |
| `ScholarshipDBTReconciled` | `{ studentId, dbtAmount, scheme, receiptId }` |
| `DailyCollectionSettled` | `{ entityId, date, totalAmount, modeWiseBreakdown }` |

#### 3.2.8 State Machine — PaymentReceipt

```
[Pending] ──Complete──> [Completed] ──Refund──> [Refunded]
   │                        │
   ├──Fail──> [Failed]      └──Cancel──> [Cancelled]
   │
   └──(Cheque/DD)──> [Uncleared] ──Clear──> [Completed]
                                   │
                                   └──Bounce──> [Bounced] ──(re-present or reverse)
```

#### 3.2.9 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `cheque_clearance_days` | 3 | Default days for cheque clearance |
| `cheque_bounce_penalty` | 500 | Penalty for bounced cheque |
| `excess_payment_action` | credit_note | Credit note or refund |
| `receipt_cancellation_requires_approval_above` | 50000 | Amount above which CFO approval needed |
| `auto_allocate_payment` | true | Auto-allocate to oldest outstanding installment |
| `payment_gateway_timeout_seconds` | 300 | Payment gateway timeout |

#### 3.2.10 Specifications

| Specification | Purpose |
|---------------|---------|
| `IsPaymentAllocated` | Payment has been allocated to installments |
| `IsReceiptCancellable` | Receipt can be cancelled (not refunded, not end-of-year) |
| `IsChequeClearable` | Cheque is within validity period |
| `IsPaymentWithinAssessment` | Payment amount does not exceed total assessment |
| `IsScholarshipDBTMatched` | DBT amount matches expected scholarship amount |

#### 3.2.11 API Contracts

```
POST   /api/v1/payments/receipts             → RecordPaymentReceipt
GET    /api/v1/payments/receipts             → GetStudentReceipts (filtered)
GET    /api/v1/payments/receipts/:id         → GetPaymentReceipt
POST   /api/v1/payments/receipts/:id/allocate → AllocatePayment
POST   /api/v1/payments/receipts/:id/clear   → ClearCheque
POST   /api/v1/payments/receipts/:id/bounce  → BounceCheque
POST   /api/v1/payments/receipts/:id/cancel  → CancelReceipt
POST   /api/v1/payments/receipts/:id/refund  → RefundReceipt

POST   /api/v1/payments/gateway/initiate     → InitiateGatewayPayment
POST   /api/v1/payments/gateway/webhook      → UpdateGatewayPaymentStatus

GET    /api/v1/payments/daily-collection     → GetDailyCollection
GET    /api/v1/payments/uncleared-cheques    → GetUnclearedCheques
GET    /api/v1/payments/reports/statistics   → GetPaymentStatistics

POST   /api/v1/scholarships/dbt-reconcile    → ReconcileScholarshipDBT
GET    /api/v1/scholarships/dbt-reconciliation → GetScholarshipDBTReconciliation
```

#### 3.2.12 Permissions

| Permission | CFO | Controller | Accountant | Cashier | Registrar |
|------------|:---:|:----------:|:----------:|:-------:|:---------:|
| `payment.receipt.create` | ✓ | ✓ | ✓ | ✓ | ✗ |
| `payment.receipt.cancel` | ✓ | ✓ | ✗ | ✗ | ✗ |
| `payment.receipt.refund` | ✓ | ✓ | ✗ | ✗ | ✗ |
| `payment.cheque.clear` | ✓ | ✓ | ✓ | ✗ | ✗ |
| `payment.cheque.bounce` | ✓ | ✓ | ✓ | ✗ | ✗ |
| `payment.receipt.read` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `payment.gateway.initiate` | ✓ | ✓ | ✓ | ✓ | ✗ |
| `payment.scholarship.reconcile` | ✓ | ✓ | ✓ | ✗ | ✓ |

#### 3.2.13 Validation Rules

| Field | Rule |
|-------|------|
| `receipt.amount` | Must be positive |
| `receipt.paymentMode` | Required |
| `receipt.chequeNumber` | Required if mode is Cheque/DD |
| `receipt.paymentDate` | Must not be in future |
| `allocation.allocatedAmount` | Sum of allocations must equal receipt amount |
| `allocation.allocatedAmount` | Must not exceed installment balance |
| `cancelReceipt` | Receipt must be in Completed or Uncleared status |
| `refundReceipt` | Receipt must be in Completed status |

#### 3.2.14 Compensation — RecordPaymentReceipt Saga

1. Create receipt record → fail → return error
2. Allocate payment to installments → fail → delete receipt, return error
3. Create payment journal entry → fail → reverse allocations, delete receipt, return error
4. Update fee assessment balance → fail → reverse journal, reverse allocations, delete receipt, return error
5. Publish `PaymentReceiptCreated` → (retry via outbox)

#### 3.2.15 AI Features

- **Payment pattern analysis:** Identify patterns in student payment behavior
- **Optimal payment reminder timing:** Predict best time to send payment reminders
- **Fraud detection:** Flag unusual payment patterns (rapid multiple payments, unusual amounts)
- **Collection forecasting:** Predict daily/weekly collection amounts

---

### 3.3 Concessions & Scholarships

#### 3.3.1 Purpose
Manage student concessions (fee waivers) and scholarships (government and private). Integrate with MahaDBT for Maharashtra state scholarships. Track DBT reconciliation.

**Compliance traces:** CD-§3 (Maharashtra Scholarships), CD-§3.3 (MahaDBT integration), CD-§4.1 (Scholarship expenditure for NAAC)

#### 3.3.2 Business Rules

1. **Concession types:** Merit-based, Sports, Cultural, Staff-dependent, Management quota, Need-based.
2. **Scholarship schemes:** Must be configurable (scheme name, provider, amount, eligibility criteria, validity period).
3. **MahaDBT integration:** Student applies on MahaDBT portal → Institute verifies → Department sanctions → DBT to student's Aadhaar-linked account (CD-§3.2).
4. **Scholarship display in fee structure:** Fee structure shows: Gross Fee → Expected Scholarship → Net Payable (CD-§3.3).
5. **DBT reconciliation:** When DBT arrives, reconcile against student's fee assessment. If student has already paid full fee, generate refund. If student has paid partial, adjust outstanding.
6. **Concession approval:** Concessions require approval workflow (amount-based delegation).
7. **Scholarship audit trail:** Every scholarship action timestamped and user-attributed (CD-§3.3).
8. **Documentation:** Scholarship register, fee receipts, bank statements, Aadhaar verification stored (CD-§3.3).
9. **Scheme-wise reconciliation:** Reports showing scheme-wise disbursement, DBT received, and outstanding.
10. **Eligibility validation:** Scholarship eligibility based on caste, income, category, academic performance.

#### 3.3.3 Aggregates

**Aggregate Root: `Concession`**
- `ConcessionId` (UUID)
- `StudentId` (UUID)
- `FeeAssessmentId` (FK)
- `ConcessionType` (enum: Merit, Sports, Cultural, StaffDependent, Management, NeedBased, Other)
- `ConcessionPercent` (decimal, 0-100)
- `ConcessionAmount` (Money, computed)
- `ApprovedById` (UserId)
- `ApprovalDate` (datetime)
- `SanctionOrderNumber` (string, nullable)
- `ValidFrom` (date)
- `ValidTo` (date)
- `Status` (enum: Applied, Approved, Rejected, Expired)
- `Remarks` (string)

**Aggregate Root: `Scholarship`**
- `ScholarshipId` (UUID)
- `StudentId` (UUID)
- `SchemeId` (FK to ScholarshipScheme)
- `FeeAssessmentId` (FK)
- `ApplicationReference` (string — MahaDBT reference)
- `ExpectedAmount` (Money)
- `SanctionedAmount` (Money, nullable)
- `DisbursedAmount` (Money, nullable)
- `DbtDate` (datetime, nullable)
- `DbtTransactionReference` (string, nullable)
- `Status` (enum: Applied, Verified, Sanctioned, Disbursed, PartiallyDisbursed, Rejected, Closed)
- `AppliedDate` (datetime)
- `VerifiedDate` (datetime, nullable)
- `SanctionedDate` (datetime, nullable)
- `DisbursedDate` (datetime, nullable)
- `VerifiedById` (UserId, nullable)
- `SanctionedById` (UserId, nullable)
- `Remarks` (string)

**Entity: `ScholarshipScheme`**
- `ScholarshipSchemeId` (UUID)
- `Code` (string, unique)
- `Name` (string)
- `Provider` (enum: CentralGovernment, StateGovernment, Private, Trust, Other)
- `State` (string, nullable — for state-specific schemes)
- `SchemeType` (enum: TuitionFee, Maintenance, FullTuition, TuitionPlusMaintenance, LumpSum)
- `MaxAmount` (Money)
- `EligibilityCriteria` (jsonb — configurable rules)
- `IsActive` (boolean)
- `RequiresAadhaar` (boolean)
- `RequiresBankAccount` (boolean)
- `RequiresIncomeCertificate` (boolean)
- `RequiresCasteCertificate` (boolean)

#### 3.3.4 Value Objects

- `ConcessionType` — enum
- `ConcessionStatus` — enum
- `ScholarshipStatus` — enum
- `ScholarshipProvider` — enum
- `SchemeType` — enum

#### 3.3.5 Commands

| Command | Description |
|---------|-------------|
| `CreateScholarshipScheme(cmd)` | Create a scholarship scheme |
| `UpdateScholarshipScheme(id, cmd)` | Update scheme |
| `ApplyScholarship(studentId, schemeId, feeAssessmentId)` | Apply for scholarship |
| `VerifyScholarship(scholarshipId, verifiedById)` | Institute verification on MahaDBT |
| `RecordScholarshipSanction(scholarshipId, sanctionedAmount, sanctionedById)` | Record sanction |
| `RecordScholarshipDisbursement(scholarshipId, dbtAmount, dbtDate, transactionRef)` | Record DBT receipt |
| `ReconcileScholarship(scholarshipId, receiptId)` | Reconcile DBT with fee payment |
| `ApproveConcession(cmd)` | Approve a concession |
| `RejectScholarship(scholarshipId, reason)` | Reject scholarship application |
| `CloseScholarship(scholarshipId, reason)` | Close scholarship record |

#### 3.3.6 Queries

| Query | Description |
|-------|-------------|
| `GetScholarshipScheme(id)` | Scheme details |
| `GetScholarshipSchemes(filter)` | All schemes with filters |
| `GetStudentScholarships(studentId)` | All scholarships for a student |
| `GetScholarship(scholarshipId)` | Scholarship details with full history |
| `GetScholarshipReconciliation(schemeId, period)` | Scheme-wise DBT reconciliation |
| `GetPendingVerification(entityId)` | Scholarships pending institute verification |
| `GetConcession(studentId, feeAssessmentId)` | Student's concession |
| `GetScholarshipReport(filter)` | Scholarship summary report |
| `GetMahaDBTStatus(scholarshipId)` | Real-time MahaDBT status |

#### 3.3.7 Events

| Event | Payload |
|-------|---------|
| `ScholarshipSchemeCreated` | `{ schemeId, code, name, provider }` |
| `ScholarshipApplied` | `{ scholarshipId, studentId, schemeId, expectedAmount }` |
| `ScholarshipVerified` | `{ scholarshipId, verifiedById, verifiedAt }` |
| `ScholarshipSanctioned` | `{ scholarshipId, sanctionedAmount, sanctionedById }` |
| `ScholarshipDisbursed` | `{ scholarshipId, dbtAmount, dbtDate, transactionRef }` |
| `ScholarshipReconciled` | `{ scholarshipId, receiptId, reconciledAmount }` |
| `ScholarshipRejected` | `{ scholarshipId, reason }` |
| `ConcessionApproved` | `{ concessionId, studentId, amount, approvedBy }` |
| `ConcessionExpired` | `{ concessionId, studentId, concessionType }` |

#### 3.3.8 State Machine — Scholarship

```
[Applied] ──Verify──> [Verified] ──Sanction──> [Sanctioned] ──Disburse──> [Disbursed]
   │                    │                           │                         │
   └──Reject──> [Rejected]  └──Reject──> [Rejected]  │                         │
                                                      └──Reject──> [Rejected]  │
                                                                               └──Close──> [Closed]
```

#### 3.3.9 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `scholarship_auto_verify` | false | Auto-verify scholarship if all criteria met |
| `scholarship_verification_days` | 15 | Days to verify after application |
| `scholarship_disbursement_tracking` | true | Enable DBT disbursement tracking |
| `concession_approval_threshold` | 50000 | Amount above which CFO approval needed |
| `concession_max_percent` | 100 | Maximum concession percentage |
| `scholarship_adjustment_on_payment` | adjust_outstanding | How to handle DBT after fee payment (adjust_outstanding / refund) |

#### 3.3.10 Specifications

| Specification | Purpose |
|---------------|---------|
| `MeetsScholarshipCriteria` | Student meets all eligibility criteria for a scheme |
| `IsScholarshipVerifiable` | All required documents are uploaded |
| `IsWithinMahaDBTWindow` | Application is within MahaDBT submission window |
| `IsConcessionWithinLimit` | Concession amount/percent within policy limits |
| `HasAadhaarLinkedBank` | Student has Aadhaar-linked bank account for DBT |
| `IsDbtReconcilable` | DBT can be reconciled against fee assessment |

#### 3.3.11 API Contracts

```
POST   /api/v1/scholarship-schemes          → CreateScholarshipScheme
GET    /api/v1/scholarship-schemes          → GetScholarshipSchemes
GET    /api/v1/scholarship-schemes/:id      → GetScholarshipScheme
PUT    /api/v1/scholarship-schemes/:id      → UpdateScholarshipScheme

POST   /api/v1/scholarships/apply          → ApplyScholarship
GET    /api/v1/scholarships                → GetScholarshipReport (filtered)
GET    /api/v1/scholarships/:id            → GetScholarship
POST   /api/v1/scholarships/:id/verify     → VerifyScholarship
POST   /api/v1/scholarships/:id/sanction   → RecordScholarshipSanction
POST   /api/v1/scholarships/:id/disburse   → RecordScholarshipDisbursement
POST   /api/v1/scholarships/:id/reconcile  → ReconcileScholarship
POST   /api/v1/scholarships/:id/reject     → RejectScholarship
POST   /api/v1/scholarships/:id/close      → CloseScholarship

GET    /api/v1/scholarships/reports/reconciliation → GetScholarshipReconciliation
GET    /api/v1/scholarships/pending-verification  → GetPendingVerification

POST   /api/v1/concessions                 → ApproveConcession
GET    /api/v1/concessions                 → GetConcession
```

#### 3.3.12 Permissions

| Permission | CFO | Controller | Accountant | Registrar |
|------------|:---:|:----------:|:----------:|:---------:|
| `scholarship.scheme.create` | ✓ | ✓ | ✗ | ✗ |
| `scholarship.scheme.update` | ✓ | ✓ | ✗ | ✗ |
| `scholarship.apply` | ✗ | ✗ | ✗ | ✓ |
| `scholarship.verify` | ✗ | ✓ | ✗ | ✓ |
| `scholarship.sanction` | ✓ | ✓ | ✗ | ✗ |
| `scholarship.disburse.record` | ✓ | ✓ | ✓ | ✗ |
| `scholarship.reconcile` | ✓ | ✓ | ✓ | ✓ |
| `scholarship.reject` | ✓ | ✓ | ✗ | ✓ |
| `scholarship.read` | ✓ | ✓ | ✓ | ✓ |
| `concession.approve` | ✓ | ✓ | ✗ | ✓ |
| `concession.read` | ✓ | ✓ | ✓ | ✓ |

#### 3.3.13 Validation Rules

| Field | Rule |
|-------|------|
| `scholarship.expectedAmount` | Must be positive, ≤ scheme.maxAmount |
| `scholarship.sanctionedAmount` | Must be ≤ expectedAmount |
| `scholarship.dbtAmount` | Must be ≤ sanctionedAmount |
| `concession.concessionPercent` | 0-100 |
| `concession.validFrom` | Must be before validTo |
| `scholarshipVerification` | Student must have Aadhaar, bank account, caste/income certificates |
| `dbtReconciliation` | DBT amount + student payment ≤ total assessment |

#### 3.3.14 Compensation — ScholarshipDisbursement Saga

1. Record DBT receipt → fail → return error
2. Update scholarship status to Disbursed → fail → delete DBT record, return error
3. If student has paid fee: create credit note/refund → fail → revert status, delete DBT record, return error
4. Update fee assessment balance → fail → revert credit note, revert status, delete DBT, return error
5. Create journal entry for DBT receipt → fail → revert all, return error
6. Publish `ScholarshipDisbursed` → (retry via outbox)

#### 3.3.15 AI Features

- **Scholarship eligibility prediction:** Predict which students are eligible for which schemes
- **Disbursement delay prediction:** Flag scholarships likely to be delayed
- **Fraud detection:** Detect duplicate scholarship applications across schemes
- **Reconciliation automation:** Auto-match DBT transactions to scholarship records

---

### 3.4 Refunds

#### 3.4.1 Purpose
Process full or partial refunds to students/depositors. Handle FRC-compliant refund rules, credit notes, and refund journeys.

**Compliance traces:** CD-§8.2 (FRC refund policy), CD-§3.3 (Refund when DBT arrives after fee payment)

#### 3.4.2 Business Rules

1. **FRC refund rules:** Fee refund as per Maharashtra FRC guidelines — percentage of fee refundable based on withdrawal date (configurable: e.g., 100% before start, 80% after 1 month, 50% after 2 months, 0% after 3 months).
2. **Refund reasons:** Course withdrawal, Scholarship DBT arrival after fee payment, Excess payment, Overpayment, Fee revision, Institution decision.
3. **Refund modes:** Same as payment modes (NEFT, RTGS, Cheque, Cash). Refund must go back to original source where possible.
4. **Credit notes:** Can be issued instead of cash refund. Applicable for future fee adjustments.
5. **Refund approval:** Amount-based approval workflow. Higher amounts require CFO/Trustee approval.
6. **Refund against scholarship DBT:** If DBT arrives after student has paid full fee, refund the equivalent amount to student.
7. **Security deposit refund:** Separate process with forfeiture rules.

#### 3.4.3 Aggregates

**Aggregate Root: `Refund`**
- `RefundId` (UUID)
- `RefundNumber` (string, tenant+entity+fiscal year unique)
- `EntityId` (FK)
- `StudentId` (UUID, nullable)
- `SourceReceiptId` (FK to PaymentReceipt, nullable)
- `RefundType` (enum: FeeRefund, ScholarshipAdjustment, ExcessPayment, DepositRefund, Other)
- `RefundMode` (enum: NEFT, RTGS, Cheque, Cash, CreditNote)
- `Amount` (Money)
- `Reason` (string)
- `FrcRefundPercent` (decimal, nullable — if FRC-based)
- `Status` (enum: Initiated, Approved, Processed, Completed, Failed, Cancelled)
- `ApprovedById` (UserId, nullable)
- `ApprovedAt` (datetime, nullable)
- `ProcessedById` (UserId, nullable)
- `ProcessedAt` (datetime, nullable)
- `BankTransactionReference` (string, nullable)
- `CreditNoteId` (FK, nullable)
- `RefundJournalId` (FK to Journal, nullable)
- `Remarks` (string)

**Entity: `CreditNote`**
- `CreditNoteId` (UUID)
- `CreditNoteNumber` (string, unique)
- `StudentId` (UUID)
- `Amount` (Money)
- `RemainingBalance` (Money)
- `IssueDate` (datetime)
- `ExpiryDate` (date, nullable)
- `Status` (enum: Active, PartiallyUtilized, FullyUtilized, Expired, Cancelled)
- `IssuedAgainst` (string, nullable — receipt ID or refund ID)

#### 3.4.4 Value Objects

- `RefundType` — enum
- `RefundMode` — enum
- `RefundStatus` — enum
- `CreditNoteStatus` — enum

#### 3.4.5 Commands

| Command | Description |
|---------|-------------|
| `InitiateRefund(cmd)` | Start refund process |
| `ApproveRefund(refundId, approvedById)` | Approve refund |
| `ProcessRefund(refundId, processedById, bankRef?)` | Execute refund payment |
| `CompleteRefund(refundId)` | Mark refund as completed |
| `CancelRefund(refundId, reason)` | Cancel refund |
| `IssueCreditNote(cmd)` | Issue a credit note |
| `UtilizeCreditNote(creditNoteId, againstAmount)` | Apply credit note |
| `CalculateFrcRefundAmount(feeAmount, withdrawalDate, programStartDate)` | Calculate FRC-compliant refund amount |

#### 3.4.6 Queries

| Query | Description |
|-------|-------------|
| `GetRefund(refundId)` | Refund details |
| `GetRefunds(filter)` | Filtered refund list |
| `GetStudentRefunds(studentId)` | All refunds for a student |
| `GetCreditNote(creditNoteId)` | Credit note details |
| `GetStudentCreditNotes(studentId)` | All credit notes for a student |
| `GetFrcRefundSchedule()` | FRC refund percentage schedule |
| `GetPendingRefunds(entityId)` | Refunds pending approval/processing |

#### 3.4.7 Events

| Event | Payload |
|-------|---------|
| `RefundInitiated` | `{ refundId, refundNumber, amount, refundType }` |
| `RefundApproved` | `{ refundId, approvedBy, approvedAt }` |
| `RefundProcessed` | `{ refundId, processedBy, bankReference }` |
| `RefundCompleted` | `{ refundId, refundJournalId }` |
| `RefundCancelled` | `{ refundId, reason }` |
| `CreditNoteIssued` | `{ creditNoteId, studentId, amount }` |
| `CreditNoteUtilized` | `{ creditNoteId, utilizedAmount, remainingBalance }` |

#### 3.4.8 State Machine — Refund

```
[Initiated] ──Approve──> [Approved] ──Process──> [Processed] ──Complete──> [Completed]
   │                         │                         │
   └──Cancel──> [Cancelled]  └──Cancel──> [Cancelled]  └──Fail──> [Failed]
```

#### 3.4.9 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `refund_approval_required_above` | 10000 | Amount above which approval needed |
| `refund_second_approval_required_above` | 100000 | Amount above which second approval (CFO) needed |
| `refund_max_days` | 30 | Max days to process refund after initiation |
| `refund_mode_restriction` | original_source | Refund must go to original payment source |
| `frc_refund_policy` | configurable | JSON config of FRC refund percentages by period |
| `credit_note_validity_days` | 365 | Credit note validity period |
| `auto_credit_note_for_excess` | true | Auto-issue credit note for excess payment |

#### 3.4.10 Specifications

| Specification | Purpose |
|---------------|---------|
| `IsFrcRefundCompliant` | Refund amount matches FRC schedule |
| `IsRefundAmountValid` | Refund amount does not exceed paid amount |
| `IsRefundCancellable` | Refund can be cancelled (not yet processed) |
| `IsCreditNoteValid` | Credit note is not expired and has balance |
| `IsRefundWithinApprovalLimit` | Refund amount is within approver's limit |

#### 3.4.11 API Contracts

```
POST   /api/v1/refunds                    → InitiateRefund
GET    /api/v1/refunds                    → GetRefunds
GET    /api/v1/refunds/:id                → GetRefund
POST   /api/v1/refunds/:id/approve       → ApproveRefund
POST   /api/v1/refunds/:id/process       → ProcessRefund
POST   /api/v1/refunds/:id/complete      → CompleteRefund
POST   /api/v1/refunds/:id/cancel        → CancelRefund

POST   /api/v1/credit-notes              → IssueCreditNote
GET    /api/v1/credit-notes              → GetStudentCreditNotes
GET    /api/v1/credit-notes/:id          → GetCreditNote
POST   /api/v1/credit-notes/:id/utilize  → UtilizeCreditNote

GET    /api/v1/refunds/frc-calculator    → CalculateFrcRefundAmount
```

#### 3.4.12 Permissions

| Permission | CFO | Controller | Accountant | Cashier |
|------------|:---:|:----------:|:----------:|:-------:|
| `refund.initiate` | ✓ | ✓ | ✓ | ✗ |
| `refund.approve` | ✓ | ✓ | ✗ | ✗ |
| `refund.approve.high` | ✓ | ✗ | ✗ | ✗ |
| `refund.process` | ✓ | ✓ | ✓ | ✗ |
| `refund.cancel` | ✓ | ✓ | ✗ | ✗ |
| `refund.read` | ✓ | ✓ | ✓ | ✓ |
| `creditnote.issue` | ✓ | ✓ | ✓ | ✗ |
| `creditnote.utilize` | ✓ | ✓ | ✓ | ✓ |

#### 3.4.13 Validation Rules

| Field | Rule |
|-------|------|
| `refund.amount` | Must be positive, ≤ original payment amount |
| `refund.refundMode` | Required |
| `refund.sourceReceiptId` | Required for fee/scholarship refunds |
| `refund.approve` | First approver ≠ second approver (if both required) |
| `creditNote.amount` | Must be positive |
| `creditNote.utilize.amount` | Must be ≤ remaining balance |

#### 3.4.14 Compensation — RefundProcess Saga

1. Initiate refund → fail → return error
2. Approve refund → fail → cancel refund, return error
3. Process payment → fail → mark refund as failed, return error
4. Create journal entry → fail → reverse payment, mark refund as failed, return error
5. Update original receipt status → fail → reverse journal, reverse payment, mark refund as failed, return error
6. Publish `RefundProcessed` → (retry via outbox)

#### 3.4.15 AI Features

- **Refund pattern analysis:** Detect unusual refund patterns (potential fraud)
- **FRC auto-calculation:** Auto-calculate FRC refund amount based on date
- **Refund delay prediction:** Flag refunds likely to exceed processing SLA

---

### 3.5 Security Deposits

#### 3.5.1 Purpose
Manage collection, refund, and forfeiture of security deposits (caution deposits, hostel deposits, library deposits, laboratory deposits).

**Compliance traces:** CD-§8.2 (FRC fee regulation — refund rules apply)

#### 3.5.2 Business Rules

1. **Deposit types:** Caution Deposit, Hostel Deposit, Library Deposit, Lab Deposit, Equipment Deposit.
2. **Deposit collection:** Collected at admission/enrollment.
3. **Deposit refund:** Refundable at course completion, subject to deductions (damages, losses).
4. **Deposit forfeiture:** Under specific conditions (damage, rule violation). Requires approval.
5. **Interest on deposits:** Some deposits earn interest. Configurable interest rate.
6. **Deposit accounting:** Deposits are liabilities on the balance sheet. Not income.
7. **Deposit reconciliation:** Periodically reconcile deposit register with GL balance.

#### 3.5.3 Aggregates

**Aggregate Root: `SecurityDeposit`**
- `SecurityDepositId` (UUID)
- `StudentId` (UUID)
- `DepositType` (enum: Caution, Hostel, Library, Lab, Equipment)
- `Amount` (Money)
- `CollectionDate` (date)
- `ReceiptId` (FK to PaymentReceipt)
- `InterestRate` (decimal, nullable)
- `InterestAccrued` (Money, computed)
- `Status` (enum: Active, Held, Refunded, Forfeited, PartiallyRefunded)
- `RefundDate` (date, nullable)
- `RefundAmount` (Money, nullable)
- `DeductionAmount` (Money, nullable)
- `DeductionReason` (string, nullable)
- `ForfeitureApprovedById` (UserId, nullable)
- `ForfeitureReason` (string, nullable)

#### 3.5.4 Commands

| Command | Description |
|---------|-------------|
| `CollectDeposit(cmd)` | Record deposit collection |
| `RefundDeposit(depositId, amount, deductions[])` | Process deposit refund with deductions |
| `ForfeitDeposit(depositId, reason, approvedBy)` | Forfeit deposit |
| `AccrueInterest(depositId, period)` | Accrue interest on deposit |

#### 3.5.5 Queries

| Query | Description |
|-------|-------------|
| `GetDeposit(depositId)` | Deposit details |
| `GetStudentDeposits(studentId)` | All deposits for a student |
| `GetDepositRegister(entityId, status)` | Deposit register |
| `GetDepositReconciliation(entityId, asOfDate)` | Deposit vs GL balance reconciliation |

#### 3.5.6 Events

| Event | Payload |
|-------|---------|
| `DepositCollected` | `{ depositId, studentId, depositType, amount }` |
| `DepositRefunded` | `{ depositId, refundAmount, deductions }` |
| `DepositForfeited` | `{ depositId, amount, reason, approvedBy }` |
| `DepositInterestAccrued` | `{ depositId, interestAmount, period }` |

#### 3.5.7 Permissions

| Permission | CFO | Controller | Accountant | Cashier |
|------------|:---:|:----------:|:----------:|:-------:|
| `deposit.collect` | ✓ | ✓ | ✓ | ✓ |
| `deposit.refund` | ✓ | ✓ | ✗ | ✗ |
| `deposit.forfeit` | ✓ | ✓ | ✗ | ✗ |
| `deposit.read` | ✓ | ✓ | ✓ | ✓ |

---

## 4. Accounts Payable (AP)

### 4.1 Vendor Master

#### 4.1.1 Purpose
Manage vendor onboarding, verification, and lifecycle. Ensure PAN/GSTIN validation, bank account validation, and compliance with TDS provisions.

**Compliance traces:** CD-§2 (TDS — PAN validation, Section 197 certificates), CD-§1.3 (GST — GSTIN validation)

#### 4.1.2 Business Rules

1. **Vendor categories:** Individual, Proprietorship, Partnership, LLP, Private Limited, Public Limited, Government, Trust, Society, HUF, Others.
2. **PAN verification:** PAN must be validated against Income Tax database. TDS applies if PAN not provided (20% under Section 206AA).
3. **GSTIN verification:** If registered, GSTIN must be validated against GST portal. Composition scheme status tracked.
4. **Bank account validation:** Bank account validated via penny drop or API. Name must match PAN name.
5. **Section 197 certificates:** Lower/Nil deduction certificates stored per vendor, with certificate number, validity period, specified rate, and section. Auto-apply during payment processing. Expiry alerts (CD-§2.3).
6. **Vendor blacklisting:** Vendor can be blacklisted for compliance failures. No new POs to blacklisted vendors.
7. **TDS rate master:** Default TDS rate per section, overridable by Section 197 certificate.
8. **GST withholding:** Where applicable, GST TDS under Section 52.

#### 4.1.3 Aggregates

**Aggregate Root: `Vendor`**
- `VendorId` (UUID)
- `TenantId` (UUID)
- `EntityId` (FK, nullable — campus-specific or shared)
- `VendorCode` (string, tenant-unique)
- `VendorName` (string)
- `VendorType` (enum: Individual, Proprietorship, Partnership, LLP, PrivateLimited, PublicLimited, Government, Trust, Society, HUF, Other)
- `Pan` (Pan value object)
- `PanStatus` (enum: Verified, Unverified, Invalid)
- `Gstin` (Gstin value object, nullable)
- `GstinStatus` (enum: Verified, Unverified, Invalid, NotRegistered)
- `GstCompositionScheme` (boolean, default false)
- `RegistrationType` (enum: Regular, Composition, Unregistered, NonResident)
- `BankAccount` (BankAccount value object)
- `BankValidationStatus` (enum: Verified, Unverified, Failed)
- `Address` (Address value object)
- `ContactPerson` (string)
- `ContactEmail` (string)
- `ContactPhone` (string)
- `PaymentTerms` (int — days, e.g., 30)
- `DefaultTdsSection` (string, nullable — e.g., "194C", "194J", "194I")
- `TdsApplicable` (boolean)
- `TaxApplicable` (boolean — GST)
- `IsActive` (boolean)
- `IsBlacklisted` (boolean)
- `BlacklistReason` (string, nullable)
- `MsmeRegistrationNumber` (string, nullable)
- `MsmeType` (enum: Micro, Small, Medium, nullable)

**Entity: `Section197Certificate`**
- `Section197CertificateId` (UUID)
- `VendorId` (FK)
- `CertificateNumber` (string)
- `Section` (string — e.g., "194C", "194J")
- `SpecifiedRate` (decimal — can be 0 for nil deduction)
- `IssuedBy` (string — Assessing Officer)
- `ValidFrom` (date)
- `ValidTo` (date)
- `IsActive` (boolean)
- `CertificateDocument` (string, URL to stored document)

**Entity: `VendorBankAccount`**
- `VendorBankAccountId` (UUID)
- `VendorId` (FK)
- `AccountNumber` (string, encrypted)
- `IfscCode` (string)
- `BankName` (string)
- `BranchName` (string)
- `AccountType` (enum: Savings, Current, CashCredit)
- `IsPrimary` (boolean)
- `ValidationStatus` (enum: Unverified, Verified, Failed)
- `PennyDropAmount` (Money, nullable)

#### 4.1.4 Value Objects

- `PAN { value: string (10-char pattern) }` — validated with regex: `[A-Z]{5}[0-9]{4}[A-Z]{1}`
- `GSTIN { value: string (15-char pattern) }` — validated with regex: `[0-9]{2}[A-Z]{5}[0-9]{4}[A-Z]{1}[1-9A-Z]{1}Z[0-9A-Z]{1}`
- `IFSC { value: string (11-char pattern) }` — validated with regex: `[A-Z]{4}0[A-Z0-9]{6}`
- `BankAccount { accountNumber (encrypted), ifsc, accountName }`

#### 4.1.5 Commands

| Command | Description |
|---------|-------------|
| `CreateVendor(cmd)` | Onboard new vendor |
| `UpdateVendor(id, cmd)` | Update vendor details |
| `VerifyPan(vendorId)` | Validate PAN against IT database |
| `VerifyGstin(vendorId)` | Validate GSTIN against GST portal |
| `VerifyBankAccount(vendorId)` | Validate bank account (penny drop) |
| `BlacklistVendor(vendorId, reason)` | Blacklist vendor |
| `WhitelistVendor(vendorId)` | Remove blacklist status |
| `AddSection197Certificate(cmd)` | Add Section 197 certificate |
| `ExpireSection197Certificate(certId)` | Mark certificate as expired |
| `DeactivateVendor(vendorId)` | Deactivate vendor |

#### 4.1.6 Queries

| Query | Description |
|-------|-------------|
| `GetVendor(vendorId)` | Vendor details with all verifications |
| `GetVendorByCode(code)` | Vendor lookup by code |
| `GetVendors(filter)` | Filtered vendor list |
| `GetVendorByPan(pan)` | Find vendor by PAN |
| `GetVendorByGstin(gstin)` | Find vendor by GSTIN |
| `GetSection197Certificates(vendorId)` | Active certificates |
| `GetExpiringCertificates(days)` | Certificates expiring within N days |
| `GetVendorsWithExpiredPan()` | Vendors with unverified PAN |
| `GetBlacklistedVendors()` | Blacklisted vendors |

#### 4.1.7 Events

| Event | Payload |
|-------|---------|
| `VendorCreated` | `{ vendorId, vendorCode, vendorName, pan }` |
| `VendorUpdated` | `{ vendorId, changes }` |
| `PanVerified` | `{ vendorId, pan, status }` |
| `GstinVerified` | `{ vendorId, gstin, status }` |
| `BankAccountVerified` | `{ vendorId, accountNumber, status }` |
| `VendorBlacklisted` | `{ vendorId, reason }` |
| `VendorWhitelisted` | `{ vendorId }` |
| `Section197CertificateAdded` | `{ vendorId, section, rate, validTo }` |
| `Section197CertificateExpired` | `{ vendorId, certId, section }` |
| `VendorDeactivated` | `{ vendorId }` |

#### 4.1.8 Permissions

| Permission | CFO | Controller | Accountant |
|------------|:---:|:----------:|:----------:|
| `vendor.create` | ✓ | ✓ | ✓ |
| `vendor.update` | ✓ | ✓ | ✓ |
| `vendor.verify` | ✓ | ✓ | ✓ |
| `vendor.blacklist` | ✓ | ✗ | ✗ |
| `vendor.deactivate` | ✓ | ✓ | ✗ |
| `vendor.section197.add` | ✓ | ✓ | ✗ |
| `vendor.read` | ✓ | ✓ | ✓ |

#### 4.1.9 Validation Rules

| Field | Rule |
|-------|------|
| `vendor.pan` | Required, valid 10-char PAN format |
| `vendor.gstin` | If provided, valid 15-char GSTIN format |
| `vendor.bankAccount.accountNumber` | Required, 9-18 digits |
| `vendor.bankAccount.ifsc` | Required, valid 11-char IFSC |
| `vendor.vendorCode` | Required, unique per tenant |
| `vendor.pan` | Must be unique per tenant (one vendor per PAN) |
| `section197.validTo` | Must be after validFrom |

#### 4.1.10 AI Features

- **Auto-classification:** Suggest vendor category based on name and PAN
- **Duplicate detection:** Flag potential duplicate vendors (name, PAN, GSTIN similarity)
- **Risk scoring:** Score vendors based on compliance history (PAN/GSTIN changes, blacklist history)
- **Certificate expiry prediction:** Predict which vendors need Section 197 renewal

---

### 4.2 Procurement

#### 4.2.1 Purpose
Manage the complete procurement lifecycle: Purchase Requisition → Purchase Order → Goods Receipt Note → Invoice Matching (3-way matching). Ensure compliance with GST, TDS, and institutional procurement policies.

**Compliance traces:** CD-§1.4 (RCM at PO), CD-§2.1 (TDS at payment), CD-§6.1 (UGC grant procurement)

#### 4.2.2 Business Rules

1. **Purchase Requisition (PR):** Internal request for procurement. Requires department head approval. Can be linked to budget/encumbrance.
2. **Purchase Order (PO):** Issued to vendor. Must reference PR. Contains items, quantities, rates, delivery terms, payment terms.
3. **RCM flag at PO:** If the vendor is unregistered under GST, RCM flag is set at PO level. All invoices under this PO are RCM (CD-§1.4).
4. **Goods Receipt Note (GRN):** Records receipt of goods/services. Triggers inventory update and liability recognition.
5. **3-way matching:** PO × GRN × Invoice must match on quantity, rate, and amount. Mismatch triggers workflow.
6. **2-way matching:** For services, PO × Invoice matching (no GRN).
7. **Budget encumbrance:** When PO is issued, budget is encumbered. When GRN is raised, encumbrance is reduced and actual is recorded.
8. **Grant-linked procurement:** If PO is against a grant fund, budget head must be within grant-approved heads.
9. **TDS applicability:** Determined at PO creation based on vendor default section and nature of goods/services.
10. **GST applicability:** Determined at PO creation based on HSN/SAC code and vendor GST registration status.

#### 4.2.3 Aggregates

**Aggregate Root: `PurchaseRequisition`**
- `PurchaseRequisitionId` (UUID)
- `PrNumber` (string, tenant-unique)
- `EntityId` (FK)
- `DepartmentId` (FK to CostCenter)
- `RequestedById` (UserId)
- `ApprovedById` (UserId, nullable)
- `Status` (enum: Draft, Submitted, Approved, Rejected, ConvertedToPO, Cancelled)
- `ExpectedDeliveryDate` (date, nullable)
- `TotalAmount` (Money, estimated)
- `FundId` (FK, nullable)
- `BudgetHeadId` (FK, nullable)
- `Remarks` (string)

**Entity: `PurchaseRequisitionLine`**
- `PurchaseRequisitionLineId` (UUID)
- `PurchaseRequisitionId` (FK)
- `ItemDescription` (string)
- `Quantity` (decimal)
- `EstimatedUnitPrice` (Money)
- `EstimatedTotalAmount` (Money, computed)
- `AccountId` (FK — expense account)
- `CostCenterId` (FK, nullable)
- `HsnSacCode` (string, nullable)

**Aggregate Root: `PurchaseOrder`**
- `PurchaseOrderId` (UUID)
- `PoNumber` (string, tenant+entity+fiscal year unique)
- `EntityId` (FK)
- `VendorId` (FK)
- `PurchaseRequisitionId` (FK, nullable)
- `OrderDate` (date)
- `DeliveryDate` (date, nullable)
- `PaymentTerms` (string)
- `Status` (enum: Draft, Issued, Acknowledged, PartiallyReceived, FullyReceived, Closed, Cancelled)
- `TotalAmount` (Money)
- `TaxAmount` (Money, computed)
- `NetAmount` (Money, computed)
- `IsRcmApplicable` (boolean)
- `TdsSection` (string, nullable)
- `TdsRate` (decimal, nullable)
- `FundId` (FK, nullable)
- `BudgetHeadId` (FK, nullable)
- `EncumberedAmount` (Money, computed)
- `IssuedById` (UserId)
- `ApprovedById` (UserId)

**Entity: `PurchaseOrderLine`**
- `PurchaseOrderLineId` (UUID)
- `PurchaseOrderId` (FK)
- `LineNumber` (int)
- `ItemDescription` (string)
- `HsnSacCode` (string)
- `Quantity` (decimal)
- `UnitPrice` (Money)
- `DiscountPercent` (decimal, nullable)
- `TaxRate` (decimal)
- `TaxType` (enum: GST_Exempt, GST_5, GST_12, GST_18, GST_28, Nil)
- `TotalAmount` (Money, computed)
- `ReceivedQuantity` (decimal, default 0 — updated by GRN)
- `AccountId` (FK)
- `CostCenterId` (FK, nullable)

**Aggregate Root: `GoodsReceiptNote`**
- `GoodsReceiptNoteId` (UUID)
- `GrnNumber` (string, tenant-unique)
- `PurchaseOrderId` (FK)
- `ReceivedDate` (date)
- `ReceivedById` (UserId)
- `Status` (enum: Draft, Completed, Cancelled)
- `Remarks` (string)

**Entity: `GoodsReceiptNoteLine`**
- `GoodsReceiptNoteLineId` (UUID)
- `GoodsReceiptNoteId` (FK)
- `PurchaseOrderLineId` (FK)
- `ReceivedQuantity` (decimal)
- `AcceptedQuantity` (decimal)
- `RejectedQuantity` (decimal)
- `RejectionReason` (string, nullable)

**Aggregate Root: `PurchaseInvoice`**
- `PurchaseInvoiceId` (UUID)
- `InvoiceNumber` (string, vendor's invoice number)
- `InvoiceDate` (date)
- `PurchaseOrderId` (FK)
- `GoodsReceiptNoteId` (FK, nullable)
- `VendorId` (FK)
- `EntityId` (FK)
- `InvoiceAmount` (Money)
- `TaxAmount` (Money)
- `NetAmount` (Money)
- `TdsAmount` (Money, computed)
- `IsRcm` (boolean)
- `RcmPayableAmount` (Money, nullable)
- `Status` (enum: Draft, Matched, Mismatched, Approved, Posted, Cancelled)
- `PaymentStatus` (enum: Unpaid, PartiallyPaid, Paid)
- `DueDate` (date)
- `PostedJournalId` (FK, nullable)
- `ApprovedById` (UserId, nullable)
- `DocumentUrl` (string, nullable)

**Entity: `PurchaseInvoiceLine`**
- `PurchaseInvoiceLineId` (UUID)
- `PurchaseInvoiceId` (FK)
- `PurchaseOrderLineId` (FK, nullable)
- `ItemDescription` (string)
- `Quantity` (decimal)
- `UnitPrice` (Money)
- `TaxRate` (decimal)
- `TaxAmount` (Money)
- `TotalAmount` (Money)
- `AccountId` (FK)
- `CostCenterId` (FK, nullable)

#### 4.2.4 Value Objects

- `PrStatus` — enum
- `PoStatus` — enum
- `GrnStatus` — enum
- `InvoiceStatus` — enum
- `PaymentStatus` — enum
- `TaxType` — enum

#### 4.2.5 Commands

| Command | Description |
|---------|-------------|
| `CreatePurchaseRequisition(cmd)` | Create PR |
| `ApprovePurchaseRequisition(prId, approvedById)` | Approve PR |
| `ConvertPrToPo(prId)` | Convert approved PR to PO |
| `CreatePurchaseOrder(cmd)` | Create PO directly |
| `IssuePurchaseOrder(poId)` | Issue PO to vendor |
| `AcknowledgePurchaseOrder(poId)` | Vendor acknowledges PO |
| `CancelPurchaseOrder(poId, reason)` | Cancel PO |
| `CreateGoodsReceiptNote(cmd)` | Create GRN |
| `CompleteGoodsReceiptNote(grnId)` | Complete GRN, update PO received quantities |
| `CreatePurchaseInvoice(cmd)` | Create purchase invoice |
| `MatchInvoice(invoiceId)` | Run 3-way or 2-way matching |
| `ApproveInvoice(invoiceId, approvedById)` | Approve invoice for payment |
| `PostInvoice(invoiceId)` | Post invoice to GL |
| `CancelInvoice(invoiceId, reason)` | Cancel invoice |

#### 4.2.6 Queries

| Query | Description |
|-------|-------------|
| `GetPurchaseRequisition(prId)` | PR with lines |
| `GetPurchaseRequisitions(filter)` | Filtered PR list |
| `GetPurchaseOrder(poId)` | PO with lines, GRNs, invoices |
| `GetPurchaseOrders(filter)` | Filtered PO list |
| `GetPoByNumber(poNumber)` | PO lookup |
| `GetGoodsReceiptNote(grnId)` | GRN with lines |
| `GetGoodsReceiptNotes(poId)` | All GRNs for a PO |
| `GetPurchaseInvoice(invoiceId)` | Invoice with lines, matching status |
| `GetPurchaseInvoices(filter)` | Filtered invoice list |
| `GetInvoicesPendingMatch()` | Invoices awaiting matching |
| `Get3WayMismatchReport()` | Invoices with 3-way mismatch |
| `GetPoOutstanding(poId)` | Outstanding PO quantity and amount |
| `GetVendorInvoiceHistory(vendorId)` | All invoices from a vendor |

#### 4.2.7 Events

| Event | Payload |
|-------|---------|
| `PurchaseRequisitionCreated` | `{ prId, prNumber, departmentId, totalAmount }` |
| `PurchaseRequisitionApproved` | `{ prId, approvedBy }` |
| `PurchaseOrderCreated` | `{ poId, poNumber, vendorId, totalAmount }` |
| `PurchaseOrderIssued` | `{ poId, poNumber, vendorId }` |
| `PurchaseOrderCancelled` | `{ poId, reason }` |
| `GoodsReceiptNoteCreated` | `{ grnId, grnNumber, poId }` |
| `GoodsReceiptNoteCompleted` | `{ grnId, poId, acceptedItems, rejectedItems }` |
| `PurchaseInvoiceCreated` | `{ invoiceId, invoiceNumber, vendorId, amount }` |
| `InvoiceMatched` | `{ invoiceId, status (matched/mismatched), details }` |
| `InvoiceApproved` | `{ invoiceId, approvedBy }` |
| `InvoicePosted` | `{ invoiceId, journalId }` |
| `InvoiceCancelled` | `{ invoiceId, reason }` |

#### 4.2.8 State Machine — PurchaseOrder

```
[Draft] ──Issue──> [Issued] ──Acknowledge──> [Acknowledged]
                     │                              │
                     │              ┌────────────────┘
                     │              │
                     │         [PartiallyReceived] ──(all received)──> [FullyReceived]
                     │              │                                      │
                     └──Cancel──> [Cancelled]                              └──Close──> [Closed]
```

#### 4.2.9 State Machine — PurchaseInvoice

```
[Draft] ──Match──> [Matched] ──Approve──> [Approved] ──Post──> [Posted] ──Pay──> [Paid]
   │                   │                       │
   └──Cancel──> [Cancelled]  [Mismatched]      └──(reject)──> [Cancelled]
```

#### 4.2.10 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `pr_approval_required` | true | PR requires approval |
| `pr_approval_threshold` | 50000 | Amount above which CFO approval needed for PR |
| `po_approval_required` | true | PO requires approval |
| `po_approval_threshold` | 100000 | Amount above which CFO approval needed for PO |
| `3_way_matching_enabled` | true | Enable 3-way matching for goods |
| `2_way_matching_enabled` | true | Enable 2-way matching for services |
| `matching_tolerance_percent` | 5 | Tolerance % for price/quantity match |
| `matching_tolerance_amount` | 1000 | Tolerance amount for match |
| `auto_approve_invoice_up_to` | 5000 | Auto-approve invoices below this amount |
| `rcm_auto_detect` | true | Auto-detect RCM based on vendor registration |
| `budget_check_on_po` | true | Check budget before issuing PO |
| `encumbrance_enabled` | true | Enable encumbrance accounting |

#### 4.2.11 Specifications

| Specification | Purpose |
|---------------|---------|
| `Is3WayMatch` | PO × GRN × Invoice match on quantity, rate, amount |
| `Is2WayMatch` | PO × Invoice match on amount |
| `IsWithinMatchTolerance` | Match variance within tolerance limits |
| `IsBudgetAvailable` | Sufficient budget available |
| `IsWithinGrantBudgetHead` | PO is within grant-approved budget head |
| `IsRcmApplicable` | RCM applies based on vendor registration status |
| `IsInvoicePostable` | Invoice is approved and not yet posted |

#### 4.2.12 API Contracts

```
POST   /api/v1/purchase-requisitions        → CreatePurchaseRequisition
GET    /api/v1/purchase-requisitions        → GetPurchaseRequisitions
GET    /api/v1/purchase-requisitions/:id    → GetPurchaseRequisition
POST   /api/v1/purchase-requisitions/:id/approve → ApprovePurchaseRequisition
POST   /api/v1/purchase-requisitions/:id/convert  → ConvertPrToPo

POST   /api/v1/purchase-orders              → CreatePurchaseOrder
GET    /api/v1/purchase-orders              → GetPurchaseOrders
GET    /api/v1/purchase-orders/:id          → GetPurchaseOrder
POST   /api/v1/purchase-orders/:id/issue    → IssuePurchaseOrder
POST   /api/v1/purchase-orders/:id/cancel   → CancelPurchaseOrder

POST   /api/v1/goods-receipt-notes          → CreateGoodsReceiptNote
GET    /api/v1/goods-receipt-notes/:id      → GetGoodsReceiptNote
POST   /api/v1/goods-receipt-notes/:id/complete → CompleteGoodsReceiptNote

POST   /api/v1/purchase-invoices            → CreatePurchaseInvoice
GET    /api/v1/purchase-invoices            → GetPurchaseInvoices
GET    /api/v1/purchase-invoices/:id        → GetPurchaseInvoice
POST   /api/v1/purchase-invoices/:id/match  → MatchInvoice
POST   /api/v1/purchase-invoices/:id/approve → ApproveInvoice
POST   /api/v1/purchase-invoices/:id/post   → PostInvoice
POST   /api/v1/purchase-invoices/:id/cancel → CancelInvoice

GET    /api/v1/procurement/reports/3-way-mismatch → Get3WayMismatchReport
GET    /api/v1/procurement/reports/pending-invoices → GetInvoicesPendingMatch
```

#### 4.2.13 Permissions

| Permission | CFO | Controller | Accountant | Auditor |
|------------|:---:|:----------:|:----------:|:-------:|
| `pr.create` | ✓ | ✓ | ✓ | ✗ |
| `pr.approve` | ✓ | ✓ | ✗ | ✗ |
| `po.create` | ✓ | ✓ | ✓ | ✗ |
| `po.issue` | ✓ | ✓ | ✓ | ✗ |
| `po.cancel` | ✓ | ✓ | ✗ | ✗ |
| `grn.create` | ✓ | ✓ | ✓ | ✗ |
| `invoice.create` | ✓ | ✓ | ✓ | ✗ |
| `invoice.match` | ✓ | ✓ | ✓ | ✗ |
| `invoice.approve` | ✓ | ✓ | ✗ | ✗ |
| `invoice.post` | ✓ | ✓ | ✓ | ✗ |
| `invoice.cancel` | ✓ | ✓ | ✗ | ✗ |
| `procurement.read` | ✓ | ✓ | ✓ | ✓ |

#### 4.2.14 Validation Rules

| Field | Rule |
|-------|------|
| `pr.expectedDeliveryDate` | Must be in future |
| `po.orderDate` | Must be today or past |
| `po.deliveryDate` | Must be after orderDate |
| `poLine.quantity` | Must be positive |
| `poLine.unitPrice` | Must be positive |
| `grnLine.receivedQuantity` | Must be positive |
| `grnLine.acceptedQuantity` | Must be ≤ receivedQuantity |
| `invoice.invoiceAmount` | Must be positive |
| `invoiceMatch` | Quantity variance ≤ tolerance |
| `invoiceMatch` | Total amount variance ≤ tolerance |

#### 4.2.15 Compensation — PostInvoice Saga

1. Create journal entry for invoice → fail → return error
2. Update vendor balance → fail → reverse journal, return error
3. Update PO received amount → fail → reverse vendor balance, reverse journal, return error
4. If RCM: create RCM payable journal entry → fail → reverse all, return error
5. Update invoice status to Posted → fail → reverse all entries, return error
6. Publish `InvoicePosted` → (retry via outbox)

#### 4.2.16 AI Features

- **PO item classification:** Auto-classify line items to correct account codes based on description
- **Price variance detection:** Flag POs with unit prices significantly different from historical or market rates
- **Invoice matching automation:** Auto-match invoices with high confidence score, flag only exceptions
- **Procurement delay prediction:** Predict which POs will miss delivery dates
- **Fraud detection:** Detect duplicate invoices, inflated quantities, unusual vendor patterns

---

### 4.3 Payments

#### 4.3.1 Purpose
Execute vendor payments, manage TDS deduction, handle GST RCM payments, and schedule payments. Integrate with banking for payment execution.

**Compliance traces:** CD-§1.4 (RCM payable), CD-§2 (TDS deduction), CD-§2.3 (Section 197 lower rate)

#### 4.3.2 Business Rules

1. **Payment scheduling:** Payments can be scheduled for future dates. Batch processing supported.
2. **TDS deduction:** TDS deducted at time of payment (or credit, whichever is earlier). Section 197 certificate applied if available.
3. **TDS calculation:** Based on TDS section, rate, and applicable threshold. Lower/Nil rate from Section 197 certificate applied before default rate.
4. **RCM payment:** RCM liability paid separately. Entry in GSTR-3B as both output and input.
5. **Payment modes:** NEFT, RTGS, IMPS, Cheque, DD, Cash (for small amounts).
6. **Payment approval:** Amount-based approval workflow.
7. **Payment reference:** Each payment creates a journal entry. Payment reference linked to invoices.
8. **Partial payment:** Supported. Outstanding balance tracked per invoice.
9. **Advance payment:** Can be made against PO. Adjusted against future invoices.
10. **Payment cancellation:** Can be cancelled if not yet processed by bank.

#### 4.3.3 Aggregates

**Aggregate Root: `Payment`**
- `PaymentId` (UUID)
- `PaymentNumber` (string, tenant+entity+fiscal year unique)
- `EntityId` (FK)
- `VendorId` (FK)
- `PaymentType` (enum: VendorPayment, RCMPayment, TDSDeposit, Advance, Refund, Other)
- `PaymentMode` (enum: NEFT, RTGS, IMPS, Cheque, DD, Cash)
- `PaymentDate` (date)
- `Amount` (Money)
- `TdsAmount` (Money, computed)
- `NetAmount` (Money, computed)
- `Status` (enum: Initiated, Approved, Scheduled, Processed, Completed, Failed, Cancelled)
- `BankAccountId` (FK)
- `BankTransactionReference` (string, nullable)
- `ChequeNumber` (string, nullable)
- `ChequeDate` (date, nullable)
- `ApprovedById` (UserId, nullable)
- `ProcessedById` (UserId, nullable)
- `PaymentJournalId` (FK, nullable)
- `Remarks` (string)

**Entity: `PaymentAllocation`**
- `PaymentAllocationId` (UUID)
- `PaymentId` (FK)
- `InvoiceId` (FK to PurchaseInvoice)
- `AllocatedAmount` (Money)
- `TdsAmount` (Money)
- `NetAllocated` (Money, computed)

**Entity: `TdsDeduction`**
- `TdsDeductionId` (UUID)
- `PaymentId` (FK)
- `Section` (string, e.g., "194C", "194J")
- `Rate` (decimal)
- `TdsAmount` (Money)
- `PanOfDeductee` (string)
- `Section197CertificateId` (FK, nullable)
- `TdsDepositStatus` (enum: Pending, Deposited, Filed)
- `TdsDepositDate` (date, nullable)
- `TdsReturnFiledDate` (date, nullable)
- `TdsJournalId` (FK, nullable)

#### 4.3.4 Commands

| Command | Description |
|---------|-------------|
| `InitiatePayment(cmd)` | Create payment record |
| `ApprovePayment(paymentId, approvedById)` | Approve payment |
| `SchedulePayment(paymentId, scheduledDate)` | Schedule payment for future |
| `ProcessPayment(paymentId, processedById, bankRef?)` | Execute payment |
| `CompletePayment(paymentId)` | Mark payment as completed |
| `CancelPayment(paymentId, reason)` | Cancel payment |
| `DepositTds(tdsDeductionId, depositDate, challanRef)` | Record TDS deposit to government |
| `GenerateTdsChallan(tdsDeductions[])` | Generate TDS deposit challan (ITNS 281) |

#### 4.3.5 Queries

| Query | Description |
|-------|-------------|
| `GetPayment(paymentId)` | Payment with allocations and TDS |
| `GetPayments(filter)` | Filtered payment list |
| `GetVendorPayments(vendorId)` | All payments to vendor |
| `GetPendingPayments(entityId)` | Payments awaiting approval/processing |
| `GetTdsDeductions(filter)` | TDS deduction register |
| `GetTdsDepositSchedule()` | TDS due for deposit |
| `GetTdsChallanSummary(period)` | TDS challan summary |
| `GetPaymentSchedule(entityId, fromDate, toDate)` | Payment schedule |

#### 4.3.6 Events

| Event | Payload |
|-------|---------|
| `PaymentInitiated` | `{ paymentId, paymentNumber, vendorId, amount }` |
| `PaymentApproved` | `{ paymentId, approvedBy }` |
| `PaymentScheduled` | `{ paymentId, scheduledDate }` |
| `PaymentProcessed` | `{ paymentId, bankReference, processedBy }` |
| `PaymentCompleted` | `{ paymentId, journalId }` |
| `PaymentCancelled` | `{ paymentId, reason }` |
| `PaymentFailed` | `{ paymentId, error }` |
| `TdsDeducted` | `{ paymentId, section, tdsAmount, pan }` |
| `TdsDeposited` | `{ tdsDeductionId, challanRef, depositDate }` |

#### 4.3.7 State Machine — Payment

```
[Initiated] ──Approve──> [Approved] ──Schedule──> [Scheduled] ──Process──> [Processed] ──Complete──> [Completed]
   │                         │                         │                         │
   └──Cancel──> [Cancelled]  └──Cancel──> [Cancelled]  └──Cancel──> [Cancelled]  └──Fail──> [Failed]
```

#### 4.3.8 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `payment_approval_required_above` | 25000 | Amount above which approval needed |
| `payment_second_approval_required_above` | 500000 | Amount above which second approval needed |
| `payment_batch_processing_enabled` | true | Enable batch payment processing |
| `payment_schedule_max_days` | 30 | Max days a payment can be scheduled in advance |
| `tds_deposit_due_date` | 7th | TDS deposit due date of next month |
| `auto_compute_tds` | true | Auto-compute TDS based on section and vendor |
| `payment_advance_allowed` | true | Allow advance payments against PO |

#### 4.3.9 Specifications

| Specification | Purpose |
|---------------|---------|
| `IsPaymentWithinApprovalLimit` | Amount within approver's delegation |
| `IsTdsApplicable` | TDS applies based on vendor, section, and amount |
| `IsSection197Applicable` | Lower rate applies from valid certificate |
| `IsPaymentCancellable` | Payment can be cancelled (not yet processed) |
| `IsWithinBudget` | Payment is within available budget |

#### 4.3.10 API Contracts

```
POST   /api/v1/payments                    → InitiatePayment
GET    /api/v1/payments                    → GetPayments
GET    /api/v1/payments/:id                → GetPayment
POST   /api/v1/payments/:id/approve       → ApprovePayment
POST   /api/v1/payments/:id/schedule      → SchedulePayment
POST   /api/v1/payments/:id/process       → ProcessPayment
POST   /api/v1/payments/:id/complete      → CompletePayment
POST   /api/v1/payments/:id/cancel        → CancelPayment

POST   /api/v1/tds/deposit                → DepositTds
GET    /api/v1/tds/deductions             → GetTdsDeductions
GET    /api/v1/tds/deposit-schedule       → GetTdsDepositSchedule
GET    /api/v1/tds/challan-summary        → GetTdsChallanSummary

GET    /api/v1/payments/schedule          → GetPaymentSchedule
```

#### 4.3.11 Permissions

| Permission | CFO | Controller | Accountant | Cashier |
|------------|:---:|:----------:|:----------:|:-------:|
| `payment.initiate` | ✓ | ✓ | ✓ | ✗ |
| `payment.approve` | ✓ | ✓ | ✗ | ✗ |
| `payment.approve.high` | ✓ | ✗ | ✗ | ✗ |
| `payment.process` | ✓ | ✓ | ✓ | ✗ |
| `payment.cancel` | ✓ | ✓ | ✗ | ✗ |
| `payment.read` | ✓ | ✓ | ✓ | ✓ |
| `tds.deposit` | ✓ | ✓ | ✓ | ✗ |
| `tds.read` | ✓ | ✓ | ✓ | ✓ |

#### 4.3.12 Validation Rules

| Field | Rule |
|-------|------|
| `payment.amount` | Must be positive |
| `paymentAllocation.allocatedAmount` | Sum of allocations ≤ payment amount |
| `paymentAllocation.allocatedAmount` | Must not exceed invoice outstanding |
| `payment.paymentDate` | Must not be in past (for scheduling) |
| `payment.cancel` | Payment must be in Initiated, Approved, or Scheduled status |
| `tds.rate` | Must match the section's prescribed rate (or lower with Section 197) |

#### 4.3.13 Compensation — ProcessPayment Saga

1. Validate payment can be processed → fail → return error
2. Deduct TDS (compute and record) → fail → return error
3. Create payment journal entry (net of TDS) → fail → reverse TDS, return error
4. Create TDS liability journal entry → fail → reverse payment journal, reverse TDS, return error
5. Update invoice payment status → fail → reverse all journal entries, reverse TDS, return error
6. Send payment instruction to bank → fail → (manual: bank may or may not have processed; set status to Failed, require manual reconciliation)
7. Mark payment as Processed → fail → (manual intervention: bank may have processed; set status to Processed but with warning)
8. Publish `PaymentProcessed` → (retry via outbox)

#### 4.3.14 AI Features

- **Payment timing optimization:** Suggest optimal payment dates based on cash flow
- **TDS deduction accuracy:** Auto-verify TDS deduction amounts against rules
- **Payment failure prediction:** Flag payments likely to fail (bank issues, account validation)
- **Early payment discount detection:** Identify invoices offering early payment discounts

---

### 4.4 Employee Reimbursements

#### 4.4.1 Purpose
Manage employee expense claims, approvals, and payments. Handle travel, medical, conveyance, and other reimbursements.

**Compliance traces:** CD-§2 (TDS on reimbursements if applicable), CD-§8.1 (PT applicability)

#### 4.4.2 Business Rules

1. **Expense categories:** Travel, Medical, Conveyance, Food, Accommodation, Stationery, Phone/Internet, Other.
2. **Claim limits:** Category-wise limits configurable. Excess requires special approval.
3. **Document requirement:** Receipts required above configurable threshold.
4. **Approval workflow:** Manager → Department Head → Finance (amount-based).
5. **TDS applicability:** Reimbursements are generally not subject to TDS unless they exceed prescribed limits or are in the nature of perquisite.
6. **Payment:** Reimbursed through payroll or separate payment (configurable).
7. **Settlement:** Advance taken for travel must be settled with receipts.

#### 4.4.3 Aggregates

**Aggregate Root: `ExpenseClaim`**
- `ExpenseClaimId` (UUID)
- `ClaimNumber` (string, tenant-unique)
- `EmployeeId` (UUID)
- `EntityId` (FK)
- `ClaimDate` (date)
- `TotalAmount` (Money)
- `Status` (enum: Draft, Submitted, Approved, Rejected, Paid, Cancelled)
- `ApprovedById` (UserId, nullable)
- `PaidById` (UserId, nullable)
- `PaymentId` (FK, nullable)
- `Remarks` (string)

**Entity: `ExpenseClaimLine`**
- `ExpenseClaimLineId` (UUID)
- `ExpenseClaimId` (FK)
- `ExpenseCategory` (enum: Travel, Medical, Conveyance, Food, Accommodation, Stationery, PhoneInternet, Other)
- `ExpenseDate` (date)
- `Description` (string)
- `Amount` (Money)
- `GstApplicable` (boolean)
- `GstAmount` (Money, nullable)
- `DocumentUrl` (string, nullable)
- `AccountId` (FK)
- `CostCenterId` (FK, nullable)
- `FundId` (FK, nullable)

#### 4.4.4 Commands

| Command | Description |
|---------|-------------|
| `CreateExpenseClaim(cmd)` | Create claim |
| `SubmitClaim(claimId)` | Submit for approval |
| `ApproveClaim(claimId, approvedById)` | Approve claim |
| `RejectClaim(claimId, reason)` | Reject claim |
| `PayClaim(claimId, paymentId)` | Mark claim as paid |
| `CancelClaim(claimId, reason)` | Cancel claim |

#### 4.4.5 Queries

| Query | Description |
|-------|-------------|
| `GetExpenseClaim(claimId)` | Claim with lines |
| `GetExpenseClaims(filter)` | Filtered claims |
| `GetEmployeeClaims(employeeId)` | Employee's claims |
| `GetPendingClaims(entityId)` | Claims pending approval |

#### 4.4.6 Events

| Event | Payload |
|-------|---------|
| `ExpenseClaimSubmitted` | `{ claimId, employeeId, amount }` |
| `ExpenseClaimApproved` | `{ claimId, approvedBy }` |
| `ExpenseClaimRejected` | `{ claimId, reason }` |
| `ExpenseClaimPaid` | `{ claimId, paymentId }` |

#### 4.4.7 Permissions

| Permission | CFO | Controller | Accountant | Manager |
|------------|:---:|:----------:|:----------:|:-------:|
| `claim.create` | ✓ | ✓ | ✓ | ✓ |
| `claim.approve` | ✓ | ✓ | ✗ | ✓ |
| `claim.approve.high` | ✓ | ✓ | ✗ | ✗ |
| `claim.pay` | ✓ | ✓ | ✓ | ✗ |
| `claim.read` | ✓ | ✓ | ✓ | ✓ |

---

## 5. Treasury & Banking

### 5.1 Bank Management

#### 5.1.1 Purpose
Manage multiple bank accounts, campus-wise bank mapping, account hierarchies, and bank signatories.

**Compliance traces:** CD-§6.1 (Separate bank account for grants), CD-§7.3 (FCRA — SBI New Delhi Main Branch)

#### 5.1.2 Business Rules

1. **Bank account types:** Current, Savings, FCRA, Grant-specific, Deposit, Cash Credit.
2. **Entity mapping:** A bank account can be linked to one entity (campus) or shared across entities.
3. **FCRA account:** Must be at SBI, New Delhi Main Branch (CD-§7.3). Separate mandatory account.
4. **Grant bank accounts:** Grants must be maintained in separate bank accounts (CD-§6.1).
5. **Signatories:** Each account has authorized signatories with individual or joint signing authority.
6. **Balance tracking:** Real-time balance tracking via API integration where available. Manual update fallback.
7. **Minimum balance:** Track minimum balance requirements. Alert when approaching.

#### 5.1.3 Aggregates

**Aggregate Root: `BankAccount`**
- `BankAccountId` (UUID)
- `TenantId` (UUID)
- `EntityId` (FK, nullable)
- `AccountNumber` (string, encrypted)
- `AccountName` (string)
- `BankName` (string)
- `BranchName` (string)
- `IfscCode` (string)
- `AccountType` (enum: Current, Savings, FCRA, GrantSpecific, Deposit, CashCredit)
- `FundId` (FK, nullable — for grant-specific accounts)
- `IsFcraAccount` (boolean)
- `CurrentBalance` (Money, computed)
- `AvailableBalance` (Money, computed)
- `MinimumBalance` (Money, nullable)
- `IsActive` (boolean)
- `LastReconciledAt` (datetime, nullable)
- `LastSyncAt` (datetime, nullable)

**Entity: `BankSignatory`**
- `BankSignatoryId` (UUID)
- `BankAccountId` (FK)
- `UserId` (UUID)
- `SignatoryType` (enum: Individual, Joint)
- `IsActive` (boolean)

#### 5.1.4 Commands

| Command | Description |
|---------|-------------|
| `CreateBankAccount(cmd)` | Add bank account |
| `UpdateBankAccount(id, cmd)` | Update details |
| `DeactivateBankAccount(id)` | Deactivate account |
| `AddSignatory(accountId, userId, type)` | Add signatory |
| `RemoveSignatory(signatoryId)` | Remove signatory |
| `SyncBankBalance(accountId)` | Sync balance via API |

#### 5.1.5 Queries

| Query | Description |
|-------|-------------|
| `GetBankAccount(accountId)` | Account details |
| `GetBankAccounts(entityId?)` | All accounts |
| `GetFcraAccount()` | FCRA account details |
| `GetGrantBankAccounts()` | Grant-specific accounts |
| `GetBankBalanceSummary()` | Consolidated balance |

#### 5.1.6 Events

| Event | Payload |
|-------|---------|
| `BankAccountCreated` | `{ accountId, accountName, bankName, accountType }` |
| `BankAccountDeactivated` | `{ accountId }` |
| `SignatoryAdded` | `{ accountId, userId }` |
| `SignatoryRemoved` | `{ accountId, userId }` |
| `BankBalanceSynced` | `{ accountId, balance, syncedAt }` |
| `MinimumBalanceAlert` | `{ accountId, currentBalance, minimumBalance }` |

---

### 5.2 Bank Reconciliation

#### 5.2.1 Purpose
Auto-reconcile and manual matching of bank transactions with system records. Generate Bank Reconciliation Statements (BRS).

**Compliance traces:** CD-§3.3 (Scholarship DBT reconciliation), CD-§6.1 (Grant bank reconciliation)

#### 5.2.2 Business Rules

1. **Auto-reconciliation:** Match bank statement lines with system transactions (payment receipts, vendor payments, fund receipts, etc.) based on amount, date, reference number.
2. **Matching criteria:** Amount match, reference match (UTR/transaction ID), approximate date match (within ±3 days).
3. **Partial match:** Flag for manual review.
4. **Unmatched items:** Bank-side unmatched (system missing) and Book-side unmatched (bank missing) are tracked separately.
5. **BRS generation:** Generate BRS per bank account per period.
6. **Reconciliation period:** Monthly reconciliation is mandatory.
7. **Scholarship DBT reconciliation:** Match DBT entries from bank statement with scholarship disbursement records.

#### 5.2.3 Aggregates

**Aggregate Root: `BankReconciliation`**
- `BankReconciliationId` (UUID)
- `BankAccountId` (FK)
- `PeriodId` (FK to AccountingPeriod)
- `StatementDate` (date)
- `OpeningBalance` (Money)
- `ClosingBalance` (Money)
- `Status` (enum: InProgress, Completed, Verified)
- `VerifiedById` (UserId, nullable)
- `CompletedAt` (datetime, nullable)

**Entity: `BankStatementLine`**
- `BankStatementLineId` (UUID)
- `BankReconciliationId` (FK)
- `TransactionDate` (date)
- `TransactionReference` (string, nullable)
- `Description` (string)
- `DebitAmount` (Money, nullable)
- `CreditAmount` (Money, nullable)
- `MatchStatus` (enum: Matched, Unmatched, PartialMatch, ManualMatch)
- `MatchedTransactionId` (UUID, nullable — system transaction ID)
- `MatchedTransactionType` (string, nullable — e.g., "PaymentReceipt", "Payment", "FundReceipt")

#### 5.2.4 Commands

| Command | Description |
|---------|-------------|
| `StartReconciliation(accountId, periodId, statement)` | Start reconciliation |
| `UploadBankStatement(reconciliationId, file)` | Upload bank statement (CSV/PDF/OFX) |
| `AutoReconcile(reconciliationId)` | Run auto-reconciliation |
| `ManualMatch(reconciliationId, statementLineId, transactionId, transactionType)` | Manual match |
| `Unmatch(reconciliationId, statementLineId)` | Unmatch a matched line |
| `CompleteReconciliation(reconciliationId)` | Complete reconciliation |
| `VerifyReconciliation(reconciliationId, verifiedById)` | Verify reconciliation |
| `GenerateBrs(reconciliationId)` | Generate BRS |

#### 5.2.5 Queries

| Query | Description |
|-------|-------------|
| `GetReconciliation(id)` | Reconciliation with details |
| `GetReconciliations(accountId)` | All reconciliations for account |
| `GetUnmatchedItems(accountId, periodId)` | Unmatched items |
| `GetBrs(accountId, periodId)` | BRS report |
| `GetReconciliationSummary(accountId, fiscalYearId)` | Yearly reconciliation summary |

#### 5.2.6 Events

| Event | Payload |
|-------|---------|
| `ReconciliationStarted` | `{ reconciliationId, accountId, periodId }` |
| `BankStatementUploaded` | `{ reconciliationId, lineCount }` |
| `AutoReconciliationCompleted` | `{ reconciliationId, matchedCount, unmatchedCount }` |
| `LineMatched` | `{ reconciliationId, statementLineId, transactionId }` |
| `ReconciliationCompleted` | `{ reconciliationId, accountId, periodId }` |
| `ReconciliationVerified` | `{ reconciliationId, verifiedBy }` |
| `BrsGenerated` | `{ reconciliationId, accountId, periodId }` |

#### 5.2.7 Permissions

| Permission | CFO | Controller | Accountant |
|------------|:---:|:----------:|:----------:|
| `reconciliation.start` | ✓ | ✓ | ✓ |
| `reconciliation.auto` | ✓ | ✓ | ✓ |
| `reconciliation.manual.match` | ✓ | ✓ | ✓ |
| `reconciliation.complete` | ✓ | ✓ | ✓ |
| `reconciliation.verify` | ✓ | ✗ | ✗ |
| `reconciliation.read` | ✓ | ✓ | ✓ |

#### 5.2.8 AI Features

- **Auto-matching suggestions:** Suggest matches for unmatched items based on pattern recognition
- **Anomaly detection:** Flag unusual bank transactions (unexpected large amounts, unusual patterns)
- **Reconciliation accuracy scoring:** Score reconciliation quality based on match rate and manual interventions

---

### 5.3 Payment Gateway Integration

#### 5.3.1 Purpose
Integrate with multiple payment gateways (BillDesk, Razorpay, CCAvenue, PhonePe, Paytm) for online fee collection. Handle webhooks, retries, and reconciliation.

**Compliance traces:** CD-§3.3 (DBT reconciliation), CD-§3.2 (Fee collection)

#### 5.3.2 Business Rules

1. **Gateway-agnostic:** Abstract interface over multiple payment gateways. New gateways can be added via plugin.
2. **Webhook handling:** All gateways send webhooks for payment status updates. Idempotent processing.
3. **Retry logic:** Failed payments can be retried. Configurable retry limits.
4. **Settlement reconciliation:** Daily settlement from gateway must match system records.
5. **Refund through gateway:** Where supported, refunds can be processed through the same gateway.
6. **Gateway fees:** Track gateway transaction fees separately for reconciliation.
7. **Multi-tenant routing:** Different gateways can be configured per entity based on their agreements.

#### 5.3.3 Value Objects

- `GatewayConfig { gatewayType, apiKey (encrypted), apiSecret (encrypted), merchantId, webhookSecret, isActive }`

#### 5.3.4 Commands

| Command | Description |
|---------|-------------|
| `ConfigureGateway(entityId, gatewayType, config)` | Configure gateway |
| `InitiateGatewayPayment(cmd)` | Start payment flow |
| `HandleGatewayWebhook(gatewayType, payload, signature)` | Process webhook |
| `RetryGatewayPayment(transactionId)` | Retry failed payment |
| `ReconcileGatewaySettlement(gatewayType, settlementDate)` | Reconcile daily settlement |

#### 5.3.5 Events

| Event | Payload |
|-------|---------|
| `GatewayConfigured` | `{ entityId, gatewayType }` |
| `GatewayPaymentInitiated` | `{ transactionId, gatewayType, amount }` |
| `GatewayPaymentSuccess` | `{ gatewayTransactionId, receiptId, amount }` |
| `GatewayPaymentFailed` | `{ gatewayTransactionId, errorCode, errorMessage }` |
| `GatewayPaymentRefunded` | `{ gatewayTransactionId, refundAmount, refundId }` |
| `GatewaySettlementReconciled` | `{ gatewayType, settlementDate, settledAmount, feeAmount }` |

---

## 6. Taxation

### 6.1 GST Engine

#### 6.1.1 Purpose
Core GST engine that handles exempt/taxable classification, ITC tracking, Rule 42/43 auto-reversal, RCM handling, and GST return generation (GSTR-1, GSTR-3B, GSTR-9).

**Compliance traces:** CD-§1 (Full GST compliance), CD-§1.2 (ITC Rules 42/43), CD-§1.3 (Filing), CD-§1.4 (RCM), CD-§9.9 (ITC tracking), CD-§9.10 (RCM handling)

#### 6.1.2 Business Rules

1. **Exempt vs taxable classification:** Every fee/service transaction is classified as exempt or taxable based on the fee head's GST classification (CD-§1.1).
2. **ITC tracking at invoice level:** Every purchase invoice tracks ITC eligibility — Full, Blocked, or Reversal 42/43 (CD-§1.2).
3. **Rule 42/43 auto-reversal:** ITC reversal computed based on exempt-to-taxable turnover ratio:
   - **Rule 42 (Inputs):** `ITC Reversal = Total ITC × (Exempt Turnover / Total Turnover)`
   - **Rule 43 (Capital Goods):** Reversal over 60 months (60 months for capital goods, 5 years)
4. **RCM handling:** When flagged at PO/invoice, auto-generate RCM payable entries:
   - RCM output: `Expense A/c Dr. | RCM Payable A/c Cr.`
   - RCM ITC: `RCM ITC A/c Dr. | RCM Payable A/c Cr.` (if ITC eligible)
5. **GST return generation:** Auto-generate GSTR-1 (sales), GSTR-3B (summary) from transaction data.
6. **HSN/SAC code mapping:** Every taxable item/service mapped to HSN (goods) or SAC (services) code.
7. **Multi-GSTIN:** Separate GST returns per GSTIN (per campus/entity) (CD-§9.3).
8. **QRMP scheme:** Support for quarterly return with monthly payment (QRMP) option.
9. **GST composition scheme:** Limited support for vendors on composition scheme.

#### 6.1.3 Aggregates

**Aggregate Root: `GstRegistration`**
- `GstRegistrationId` (UUID)
- `EntityId` (FK)
- `Gstin` (GSTIN value object)
- `TradeName` (string)
- `LegalName` (string)
- `RegistrationType` (enum: Regular, Composition, Unregistered)
- `FilingFrequency` (enum: Monthly, Quarterly)
- `IsComposite` (boolean)
- `Address` (Address)
- `StateCode` (string — 2-digit)
- `IsActive` (boolean)

**Aggregate Root: `GstReturn`**
- `GstReturnId` (UUID)
- `GstRegistrationId` (FK)
- `ReturnType` (enum: GSTR1, GSTR3B, GSTR9, GSTR9C)
- `Period` (string — e.g., "072026" for July 2026)
- `FiscalYear` (string)
- `Status` (enum: Draft, Generated, Filed, FiledWithErrors, Adjusted)
- `DueDate` (date)
- `FiledDate` (date, nullable)
- `FiledBy` (UserId, nullable)
- `AcknowledgmentNumber` (string, nullable)
- `JsonData` (jsonb)
- `TaxLiability` (Money, computed)
- `ItcClaimed` (Money, computed)
- `NetTaxPayable` (Money, computed)

**Entity: `GstReturnLine`**
- `GstReturnLineId` (UUID)
- `GstReturnId` (FK)
- `Section` (string — e.g., "4A", "4B", "4C", "5A", "5B")
- `Description` (string)
- `TaxableValue` (Money)
- `IgstAmount` (Money)
- `CgstAmount` (Money)
- `SgstAmount` (Money)
- `CessAmount` (Money)

**Aggregate Root: `ItcRegister`**
- `ItcRegisterId` (UUID)
- `GstRegistrationId` (FK)
- `Period` (string)
- `Status` (enum: Open, Computed, Reversed, Closed)
- `TotalItc` (Money, computed)
- `ItcOnInputs` (Money, computed)
- `ItcOnCapitalGoods` (Money, computed)
- `ItcReversalRule42` (Money, computed)
- `ItcReversalRule43` (Money, computed)
- `NetItcEligible` (Money, computed)
- `ExemptTurnover` (Money, computed)
- `TotalTurnover` (Money, computed)

**Entity: `ItcRegisterLine`**
- `ItcRegisterLineId` (UUID)
- `ItcRegisterId` (FK)
- `InvoiceId` (FK to PurchaseInvoice)
- `InvoiceNumber` (string)
- `InvoiceDate` (date)
- `VendorGstin` (string)
- `TaxableValue` (Money)
- `Igst` (Money)
- `Cgst` (Money)
- `Sgst` (Money)
- `TotalTax` (Money)
- `ItcEligibility` (enum: Full, Blocked, Reversal_42, Reversal_43)
- `ReversalPercent` (decimal, nullable)
- `ReversalAmount` (Money, nullable)
- `IsReversed` (boolean)

#### 6.1.4 Value Objects

- `GstRate { rate: decimal, type: enum(IGST, CGST_SGST) }`
- `HsnSacCode { code: string, description: string, gstRate: GstRate }`

#### 6.1.5 Commands

| Command | Description |
|---------|-------------|
| `RegisterGstin(entityId, gstin, details)` | Register GSTIN for entity |
| `UpdateGstRegistration(regId, details)` | Update registration |
| `ClassifyTransaction(transactionId, gstClassification, rate)` | Classify GST for transaction |
| `ComputeItc(regId, period)` | Compute ITC for a period |
| `ComputeRule42Reversal(regId, period)` | Auto-compute Rule 42 reversal |
| `ComputeRule43Reversal(regId, period)` | Auto-compute Rule 43 reversal |
| `ReverseItc(registerLineId, amount, reason)` | Manual ITC reversal |
| `GenerateGstr1(regId, period)` | Generate GSTR-1 |
| `GenerateGstr3b(regId, period)` | Generate GSTR-3B |
| `GenerateGstr9(regId, fiscalYear)` | Generate GSTR-9 (annual) |
| `RecordGstFiling(returnId, acknowledgmentNo)` | Record successful filing |
| `CreateRcmEntry(invoiceId)` | Create RCM journal entries |
| `ReconcileGstLiability(regId, period)` | Reconcile tax liability with payments |

#### 6.1.6 Queries

| Query | Description |
|-------|-------------|
| `GetGstRegistration(regId)` | Registration details |
| `GetGstRegistrations(entityId)` | All registrations for entity |
| `GetGstReturn(returnId)` | Return with lines |
| `GetGstReturns(regId, fiscalYear)` | All returns for registration |
| `GetGstr1Preview(regId, period)` | GSTR-1 preview |
| `GetGstr3bPreview(regId, period)` | GSTR-3B preview |
| `GetItcRegister(regId, period)` | ITC register for period |
| `GetItcSummary(regId, fiscalYear)` | ITC summary for year |
| `GetRcmPayable(regId, period)` | RCM payable summary |
| `GetGstLiabilitySummary(regId, fiscalYear)` | GST liability summary |
| `GetGstComplianceCalendar(regId, fiscalYear)` | Filing deadlines |
| `GetGstClassificationForFeeHead(feeHeadId)` | GST classification lookup |

#### 6.1.7 Events

| Event | Payload |
|-------|---------|
| `GstinRegistered` | `{ regId, entityId, gstin }` |
| `TransactionClassified` | `{ transactionId, gstClassification, rate }` |
| `ItcComputed` | `{ regId, period, totalItc, reversalAmount }` |
| `Rule42ReversalComputed` | `{ regId, period, reversalAmount, exemptTurnover, totalTurnover }` |
| `Rule43ReversalComputed` | `{ regId, period, reversalAmount, capitalGoodsItc }` |
| `Gstr1Generated` | `{ returnId, period, taxLiability }` |
| `Gstr3bGenerated` | `{ returnId, period, taxLiability, itcClaimed }` |
| `GstReturnFiled` | `{ returnId, period, acknowledgmentNo }` |
| `RcmEntryCreated` | `{ invoiceId, expenseAccount, rcmPayableAccount, amount }` |
| `GstFilingDeadlineApproaching` | `{ regId, returnType, period, dueDate }` |

#### 6.1.8 State Machine — GstReturn

```
[Draft] ──Generate──> [Generated] ──File──> [Filed] ──(if errors)──> [FiledWithErrors] ──Adjust──> [Adjusted]
```

#### 6.1.9 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `gst_auto_classify_enabled` | true | Auto-classify transactions based on fee head |
| `itc_auto_compute_enabled` | true | Auto-compute ITC monthly |
| `rule_42_43_auto_reversal` | true | Auto-compute Rule 42/43 reversal |
| `gstr1_generation_day` | 10 | Generate GSTR-1 by 10th of month |
| `gstr3b_generation_day` | 18 | Generate GSTR-3B by 18th of month |
| `rcm_auto_create_journal` | true | Auto-create RCM journal entries |
| `gst_deadline_reminder_days` | [7, 3, 1] | Days before deadline for reminders |
| `itc_reversal_tolerance_percent` | 0.5 | Tolerance for ITC reversal calculation |
| `enable_qrmp` | true | Enable quarterly filing with monthly payment |

#### 6.1.10 Specifications

| Specification | Purpose |
|---------------|---------|
| `IsGstExempt` | Transaction is exempt from GST |
| `IsTaxable` | Transaction is taxable under GST |
| `IsItcEligible` | Input tax credit is available |
| `IsItcBlocked` | ITC is blocked under Section 17(5) |
| `IsRcmApplicable` | RCM applies to this transaction |
| `IsRule42Applicable` | Rule 42 reversal applies |
| `IsRule43Applicable` | Rule 43 reversal applies |
| `IsGstFilingDue` | Filing is due within N days |
| `IsGstReturnConsistent` | GSTR-1 and GSTR-3B are consistent |

#### 6.1.11 API Contracts

```
POST   /api/v1/gst/registrations           → RegisterGstin
GET    /api/v1/gst/registrations           → GetGstRegistrations
GET    /api/v1/gst/registrations/:id       → GetGstRegistration
PUT    /api/v1/gst/registrations/:id       → UpdateGstRegistration

POST   /api/v1/gst/itc/compute             → ComputeItc
GET    /api/v1/gst/itc/register            → GetItcRegister
GET    /api/v1/gst/itc/summary             → GetItcSummary
POST   /api/v1/gst/itc/reverse             → ReverseItc

POST   /api/v1/gst/returns/gstr1/generate  → GenerateGstr1
GET    /api/v1/gst/returns/gstr1/preview   → GetGstr1Preview
POST   /api/v1/gst/returns/gstr3b/generate → GenerateGstr3b
GET    /api/v1/gst/returns/gstr3b/preview  → GetGstr3bPreview
POST   /api/v1/gst/returns/gstr9/generate  → GenerateGstr9
POST   /api/v1/gst/returns/:id/file        → RecordGstFiling

POST   /api/v1/gst/rcm/create              → CreateRcmEntry
GET    /api/v1/gst/rcm/payable             → GetRcmPayable

GET    /api/v1/gst/reports/liability       → GetGstLiabilitySummary
GET    /api/v1/gst/reports/compliance-calendar → GetGstComplianceCalendar

POST   /api/v1/gst/classify               → ClassifyTransaction
```

#### 6.1.12 Permissions

| Permission | CFO | Controller | Accountant | Auditor |
|------------|:---:|:----------:|:----------:|:-------:|
| `gst.register` | ✓ | ✓ | ✗ | ✗ |
| `gst.itc.compute` | ✓ | ✓ | ✓ | ✗ |
| `gst.itc.reverse` | ✓ | ✓ | ✗ | ✗ |
| `gst.return.generate` | ✓ | ✓ | ✓ | ✗ |
| `gst.return.file` | ✓ | ✓ | ✗ | ✗ |
| `gst.rcm.create` | ✓ | ✓ | ✓ | ✗ |
| `gst.read` | ✓ | ✓ | ✓ | ✓ |

#### 6.1.13 Validation Rules

| Field | Rule |
|-------|------|
| `gstin` | Valid 15-char GSTIN format |
| `gstin.stateCode` | Must match entity's state |
| `itc.totalItc` | Must equal sum of invoice-level ITC |
| `rule42.exemptTurnover` | Must be ≤ totalTurnover |
| `rule42.reversalPercent` | Must be between 0 and 100 |
| `gstr1.taxableValue` | Must match sum of invoice values for period |
| `gstr3b.taxLiability` | Must match GSTR-1 summary |
| `rcm.expenseAccount` | Must be an expense account |

#### 6.1.14 Compensation — GenerateGstr3b Saga

1. Aggregate all sales data for period → fail → return error
2. Aggregate all purchase data for period → fail → return error
3. Compute ITC eligible → fail → return error
4. Compute Rule 42/43 reversal → fail → return error
5. Compute net tax liability → fail → return error
6. Generate GSTR-3B JSON → fail → return error
7. Save return as Draft → fail → return error
8. Mark return as Generated → fail → revert to draft, return error
9. Publish `Gstr3bGenerated` → (retry via outbox)

#### 6.1.15 AI Features

- **ITC optimization:** Suggest optimal ITC claiming strategy to maximize eligible credit
- **GST notice prediction:** Flag transactions that may trigger GST notice
- **Auto-classification:** Suggest GST classification for new fee heads/services
- **Anomaly detection:** Detect unusual GST patterns (e.g., sudden spike in ITC claims)
- **Return accuracy scoring:** Score return accuracy before filing

---

### 6.2 TDS Engine

#### 6.2.1 Purpose
Handle TDS deduction, deposit, return filing, and Form 16/16A generation. Integrate with TRACES for PAN validation and form generation.

**Compliance traces:** CD-§2 (Full TDS compliance), CD-§2.3 (Section 197), CD-§2.2 (Return filing)

#### 6.2.2 Business Rules

1. **Section-wise deduction:** TDS deducted at prescribed rates under applicable sections (192, 194C, 194J, 194I, 194A, 194H, 194Q, etc.) (CD-§2.1).
2. **Threshold application:** TDS deducted only if payment exceeds section-specific threshold. Thresholds are configurable.
3. **Section 197 certificates:** Lower/Nil deduction rate from Section 197 certificate applied before default rate. Certificate validity checked (CD-§2.3).
4. **PAN validation:** PAN validated against TRACES. If PAN not provided, deduct at 20% (Section 206AA).
5. **TDS deposit:** TDS deposited to government via ITNS 281 challan. Due date: 7th of next month.
6. **Return filing:** 24Q (salary), 26Q (non-salary), 27Q (non-resident) filed quarterly. Due: 15th of month after quarter (CD-§2.2).
7. **Form 16/16A generation:** Form 16 (salary) generated by 31st May. Form 16A (non-salary) within 15 days of return filing (CD-§2.2).
8. **TRACES integration:** Connect to TRACES for PAN validation, challan verification, and Form 16/16A generation.
9. **TDS on salary (Section 192):** Computed per employee based on income tax slab. Monthly deduction.
10. **TDS on non-salary (194C, 194J, etc.):** Deducted at time of payment or credit, whichever is earlier.

#### 6.2.3 Aggregates

**Aggregate Root: `TdsSection`** (system configuration, not per-tenant)
- `TdsSectionId` (UUID)
- `SectionCode` (string — e.g., "194C", "194J")
- `Description` (string)
- `DefaultRate` (decimal)
- `ThresholdPerPayment` (Money, nullable)
- `ThresholdAggregate` (Money, nullable)
- `ApplicableTo` (enum: ResidentIndividual, ResidentOther, NonResident, All)
- `IsActive` (boolean)

**Aggregate Root: `TdsReturn`**
- `TdsReturnId` (UUID)
- `EntityId` (FK)
- `ReturnType` (enum: Form24Q, Form26Q, Form27Q)
- `Quarter` (enum: Q1, Q2, Q3, Q4)
- `FiscalYear` (string)
- `Status` (enum: Draft, Generated, Filed, FiledWithErrors)
- `DueDate` (date)
- `FiledDate` (date, nullable)
- `AcknowledgmentNumber` (string, nullable)
- `TotalDeductions` (Money, computed)
- `TotalDeposits` (Money, computed)
- `JsonData` (jsonb)

**Entity: `TdsDeductionDetail`**
- `TdsDeductionDetailId` (UUID)
- `TdsReturnId` (FK)
- `VendorId` (FK, nullable)
- `EmployeeId` (UUID, nullable)
- `Pan` (string)
- `Section` (string)
- `PaymentDate` (date)
- `PaymentAmount` (Money)
- `TdsRate` (decimal)
- `TdsAmount` (Money)
- `Surcharge` (Money, default 0)
- `Cess` (Money, computed)
- `TotalTds` (Money)
- `ChallanDetails` (jsonb, nullable)
- `SalaryMonth` (int, nullable — for Section 192)

#### 6.2.4 Commands

| Command | Description |
|---------|-------------|
| `ConfigureTdsSection(sectionCode, rate, threshold)` | Update TDS section config |
| `DeductTds(paymentId)` | Compute and deduct TDS on payment |
| `ApplySection197(paymentId, certificateId)` | Apply lower rate from certificate |
| `DepositTdsToGovt(tdsDeductionIds, challanRef, depositDate)` | Record TDS deposit |
| `GenerateTdsReturn(returnType, quarter, fiscalYear)` | Generate TDS return |
| `FileTdsReturn(returnId)` | Mark return as filed |
| `GenerateForm16(employeeId, fiscalYear)` | Generate Form 16 |
| `GenerateForm16A(vendorId, fiscalYear)` | Generate Form 16A |
| `ValidatePanWithTRACES(pan)` | Validate PAN via TRACES |
| `SyncTdsChallanFromTRACES(challanRef)` | Sync challan status from TRACES |

#### 6.2.5 Queries

| Query | Description |
|-------|-------------|
| `GetTdsSection(sectionCode)` | Section details with rate |
| `GetTdsSections()` | All TDS sections |
| `GetTdsReturn(returnId)` | Return with deductions |
| `GetTdsReturns(entityId, fiscalYear)` | All returns for year |
| `GetTdsDeductions(filter)` | TDS deduction register |
| `GetTdsRegister(entityId, period)` | TDS register |
| `GetTdsChallanStatus(challanRef)` | Challan status from TRACES |
| `GetForm16(employeeId, fiscalYear)` | Form 16 data |
| `GetForm16A(vendorId, fiscalYear)` | Form 16A data |
| `GetTdsComplianceCalendar(fiscalYear)` | TDS compliance deadlines |
| `GetPendingTdsDeposits()` | TDS due for deposit |
| `GetSection197Certificates(vendorId, section)` | Active certificates for vendor |

#### 6.2.6 Events

| Event | Payload |
|-------|---------|
| `TdsDeducted` | `{ paymentId, section, tdsAmount, pan }` |
| `Section197Applied` | `{ paymentId, certificateId, originalRate, appliedRate }` |
| `TdsDeposited` | `{ tdsDeductionId, challanRef, depositDate, amount }` |
| `TdsReturnGenerated` | `{ returnId, returnType, quarter, fiscalYear }` |
| `TdsReturnFiled` | `{ returnId, acknowledgmentNo }` |
| `Form16Generated` | `{ employeeId, fiscalYear, documentUrl }` |
| `Form16AGenerated` | `{ vendorId, fiscalYear, documentUrl }` |
| `PanValidationFailed` | `{ pan, reason }` |

#### 6.2.7 State Machine — TdsReturn

```
[Draft] ──Generate──> [Generated] ──File──> [Filed] ──(if errors)──> [FiledWithErrors]
```

#### 6.2.8 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `tds_auto_deduct_enabled` | true | Auto-deduct TDS on payment |
| `tds_deposit_due_day` | 7 | TDS deposit due day of month |
| `tds_return_due_day` | 15 | TDS return due day of month after quarter |
| `form16_generation_date` | 2026-05-15 | Date to generate Form 16 |
| `form16a_generation_days` | 15 | Days after return filing to generate Form 16A |
| `pan_validation_required` | true | Validate PAN before TDS deduction |
| `section206aa_rate` | 20 | Rate when PAN not provided |
| `tds_quarterly_filing_enabled` | true | Enable quarterly TDS filing |

#### 6.2.9 Specifications

| Specification | Purpose |
|---------------|---------|
| `IsTdsApplicable` | TDS applies based on section, amount, and threshold |
| `IsPanProvided` | Vendor has valid PAN on file |
| `IsSection197Applicable` | Valid Section 197 certificate exists |
| `IsTdsDepositDue` | TDS deposit is due for the period |
| `IsTdsReturnDue` | TDS return is due for the quarter |
| `IsForm16Generatable` | All TDS data for employee is complete |

#### 6.2.10 API Contracts

```
GET    /api/v1/tds/sections                → GetTdsSections
GET    /api/v1/tds/sections/:code          → GetTdsSection
PUT    /api/v1/tds/sections/:code          → ConfigureTdsSection

POST   /api/v1/tds/deduct                  → DeductTds
POST   /api/v1/tds/apply-section197        → ApplySection197
POST   /api/v1/tds/deposit                → DepositTdsToGovt

POST   /api/v1/tds/returns/generate        → GenerateTdsReturn
GET    /api/v1/tds/returns                 → GetTdsReturns
GET    /api/v1/tds/returns/:id             → GetTdsReturn
POST   /api/v1/tds/returns/:id/file        → FileTdsReturn

GET    /api/v1/tds/deductions              → GetTdsDeductions
GET    /api/v1/tds/register                → GetTdsRegister

POST   /api/v1/tds/form16/generate         → GenerateForm16
GET    /api/v1/tds/form16                  → GetForm16
POST   /api/v1/tds/form16a/generate        → GenerateForm16A
GET    /api/v1/tds/form16a                 → GetForm16A

POST   /api/v1/tds/pan/validate            → ValidatePanWithTRACES
GET    /api/v1/tds/compliance-calendar     → GetTdsComplianceCalendar
GET    /api/v1/tds/pending-deposits        → GetPendingTdsDeposits
```

#### 6.2.11 Permissions

| Permission | CFO | Controller | Accountant | Auditor |
|------------|:---:|:----------:|:----------:|:-------:|
| `tds.section.configure` | ✓ | ✓ | ✗ | ✗ |
| `tds.deduct` | ✓ | ✓ | ✓ | ✗ |
| `tds.deposit` | ✓ | ✓ | ✓ | ✗ |
| `tds.return.generate` | ✓ | ✓ | ✓ | ✗ |
| `tds.return.file` | ✓ | ✓ | ✗ | ✗ |
| `tds.form16.generate` | ✓ | ✓ | ✓ | ✗ |
| `tds.form16a.generate` | ✓ | ✓ | ✓ | ✗ |
| `tds.read` | ✓ | ✓ | ✓ | ✓ |

#### 6.2.12 Validation Rules

| Field | Rule |
|-------|------|
| `tds.sectionCode` | Must be a valid TDS section |
| `tds.tdsRate` | Must match prescribed rate (or lower with valid Section 197) |
| `tds.pan` | Must be valid PAN format |
| `tds.paymentAmount` | Must be > threshold for TDS to apply |
| `tdsReturn.quarter` | Must be valid quarter (Q1-Q4) |
| `tdsReturn.fiscalYear` | Must be current or previous year |
| `form16.employeeId` | Must have TDS deductions for all 12 months |

#### 6.2.13 Compensation — TdsDeduction Saga

1. Compute TDS amount → fail → return error
2. Apply Section 197 if applicable → fail → (continue with default rate)
3. Check PAN validity → fail → apply 20% rate (Section 206AA), flag for review
4. Record TDS deduction → fail → return error
5. Update payment net amount → fail → reverse TDS deduction, return error
6. Create TDS liability journal entry → fail → reverse payment net amount, reverse TDS deduction, return error
7. Publish `TdsDeducted` → (retry via outbox)

#### 6.2.14 AI Features

- **TDS deduction accuracy check:** Auto-verify TDS deduction amounts against rules
- **Form 16/16A auto-generation:** Auto-generate and distribute forms
- **Section 197 certificate expiry prediction:** Alert before certificate expiry
- **Anomaly detection:** Flag unusual TDS deduction patterns (e.g., rate changes, skipped deductions)

---

### 6.3 Income Tax

#### 6.3.1 Purpose
Track trust exemption compliance under Sections 10(23C), 11, 12A/12AB. Monitor 85% application rule, Section 11(5) investment compliance, and FCRA compliance.

**Compliance traces:** CD-§7 (Full Income Tax compliance), CD-§7.1 (Exemptions), CD-§7.2 (85% rule, Section 11(5)), CD-§7.3 (FCRA), CD-§7.4 (Audit)

#### 6.3.2 Business Rules

1. **Exemption tracking:** Institution's exemption status under Section 10(23C), 11, or 12A/12AB is tracked. Registration validity period with renewal reminders (CD-§7.1).
2. **85% application rule:** At least 85% of total income must be applied to educational purposes during the year. Remaining 15% can be accumulated for up to 5 years for specified purposes (CD-§7.2).
3. **Income application tracking:** Track which expenditures qualify as "applied to educational purposes" — salaries, infrastructure, scholarships, research, etc.
4. **Section 11(5) compliance:** Funds must be invested in specified securities (Post Office, NSC, Govt securities, etc.). Non-compliant investments flagged (CD-§7.2).
5. **Accumulated income tracking:** Income accumulated beyond 15% tracked with year of accumulation. Must be applied within 5 years. Flag if not applied (CD-§7.2).
6. **FCRA compliance:** Separate ledger for FCRA funds. Admin expenses ≤ 20% of receipts. No re-granting. FC-4 return by 31st December (CD-§7.3).
7. **Audit requirements:** Track audit deadlines — 44AB, 12A, Form 10B/10BB, ITR-7 (all by 30th September) (CD-§7.4).
8. **Private benefit prohibition:** Flag any transactions that may benefit trustees/founders/relatives.

#### 6.3.3 Aggregates

**Aggregate Root: `TrustExemption`**
- `TrustExemptionId` (UUID)
- `EntityId` (FK)
- `ExemptionSection` (enum: Section10_23C, Section11_12A, Section12AB, Section10_23C_vi)
- `RegistrationNumber` (string)
- `RegistrationDate` (date)
- `ValidFrom` (date)
- `ValidTo` (date) — 3-year validity for 12AB
- `Status` (enum: Active, Expired, RenewalPending, Cancelled)
- `ApprovingAuthority` (string — e.g., "CCIT", "Commissioner")
- `IsTrust` (boolean)
- `TrustName` (string, nullable)
- `TrustPan` (string, nullable)

**Aggregate Root: `IncomeApplication`**
- `IncomeApplicationId` (UUID)
- `FiscalYearId` (FK)
- `EntityId` (FK)
- `TotalIncome` (Money, computed)
- `AmountApplied` (Money, computed)
- `ApplicationPercent` (decimal, computed)
- `AccumulatedAmount` (Money, computed)
- `AccumulationYear` (int, nullable — year of accumulation)
- `AccumulationPurpose` (string, nullable)
- `Status` (enum: Compliant, NonCompliant, UnderReview)
- `LastComputedAt` (datetime)

**Entity: `IncomeApplicationLine`**
- `IncomeApplicationLineId` (UUID)
- `IncomeApplicationId` (FK)
- `Category` (enum: Salaries, Infrastructure, Scholarships, Research, Maintenance, OtherEducational)
- `Amount` (Money)
- `AccountId` (FK)
- `Description` (string)

**Aggregate Root: `FcraRegistration`**
- `FcraRegistrationId` (UUID)
- `EntityId` (FK)
- `RegistrationNumber` (string)
- `ValidFrom` (date)
- `ValidTo` (date)
- `BankAccountId` (FK, must be SBI New Delhi)
- `Status` (enum: Active, Expired, RenewalPending, Cancelled)
- `TotalReceipts` (Money, computed)
- `AdminExpenses` (Money, computed)
- `AdminExpenseRatio` (decimal, computed)
- `Fc4ReturnFiledDate` (date, nullable)

#### 6.3.4 Commands

| Command | Description |
|---------|-------------|
| `RegisterTrustExemption(cmd)` | Record exemption registration |
| `RenewExemption(regId, newRegNumber, validTo)` | Renew exemption |
| `ComputeIncomeApplication(fiscalYearId)` | Compute 85% application ratio |
| `FlagNonCompliantInvestments(fiscalYearId)` | Check Section 11(5) compliance |
| `TrackAccumulatedIncome(amount, year, purpose)` | Track accumulated income |
| `RegisterFcra(cmd)` | Register FCRA |
| `ComputeFcraCompliance(fiscalYearId)` | Compute FCRA compliance metrics |
| `GenerateForm10B(fiscalYearId)` | Generate Form 10B audit report |
| `GenerateForm10BB(fiscalYearId)` | Generate Form 10BB audit report |
| `GenerateItr7(fiscalYearId)` | Generate ITR-7 data |

#### 6.3.5 Queries

| Query | Description |
|-------|-------------|
| `GetTrustExemption(entityId)` | Current exemption details |
| `GetExemptionHistory(entityId)` | Exemption history |
| `GetIncomeApplication(fiscalYearId)` | 85% application report |
| `GetAccumulatedIncome()` | Accumulated income (5-year tracking) |
| `GetSection115Compliance(fiscalYearId)` | Section 11(5) compliance report |
| `GetFcraStatus(entityId)` | FCRA registration and compliance |
| `GetFcraCompliance(fiscalYearId)` | FCRA compliance report |
| `GetAuditRequirements(fiscalYearId)` | Audit requirements checklist |
| `GetItr7Data(fiscalYearId)` | ITR-7 data extract |
| `GetComplianceCalendar(fiscalYearId)` | All IT compliance deadlines |

#### 6.3.6 Events

| Event | Payload |
|-------|---------|
| `TrustExemptionRegistered` | `{ regId, section, registrationNumber, validTo }` |
| `ExemptionExpiring` | `{ regId, section, validTo, daysRemaining }` |
| `IncomeApplicationComputed` | `{ fiscalYearId, totalIncome, appliedPercent, isCompliant }` |
| `IncomeApplicationThresholdMissed` | `{ fiscalYearId, appliedPercent, threshold }` |
| `Section115BreachDetected` | `{ fiscalYearId, investmentDetails }` |
| `AccumulatedIncomeExpiring` | `{ accumulationYear, amount, purpose }` |
| `FcraRegistered` | `{ regId, registrationNumber, validTo }` |
| `FcraAdminExpenseExceeded` | `{ fiscalYearId, expenseRatio, maxAllowed }` |
| `Fc4ReturnDue` | `{ fiscalYearId, dueDate }` |
| `AuditDueDateApproaching` | `{ auditType, dueDate }` |

#### 6.3.7 Policies

| Policy | Default | Description |
|--------|---------|-------------|
| `income_application_threshold` | 85 | Minimum % of income to be applied |
| `accumulation_years` | 5 | Max years for accumulated income |
| `fcra_admin_expense_limit` | 20 | Max % admin expenses of FCRA receipts |
| `exemption_renewal_reminder_days` | 180 | Days before expiry to send reminder |
| `audit_deadline_reminder_days` | [30, 15, 7, 1] | Reminder schedule for audit deadlines |
| `private_benefit_flag_threshold` | 1000000 | Threshold for flagging trustee-related transactions |

#### 6.3.8 Specifications

| Specification | Purpose |
|---------------|---------|
| `MeetsIncomeApplicationRule` | 85% of income applied to educational purposes |
| `IsSection115Compliant` | Investments are in specified securities |
| `IsFcraAdminExpenseCompliant` | Admin expenses ≤ 20% of FCRA receipts |
| `IsExemptionValid` | Trust exemption registration is valid |
| `IsAccumulationWithinLimit` | Accumulated income is within 15% limit and within 5-year window |
| `IsFcraRegrantingProhibited` | No re-granting of FCRA funds |

#### 6.3.9 API Contracts

```
POST   /api/v1/income-tax/exemption              → RegisterTrustExemption
GET    /api/v1/income-tax/exemption              → GetTrustExemption
PUT    /api/v1/income-tax/exemption/:id/renew    → RenewExemption

POST   /api/v1/income-tax/income-application/compute → ComputeIncomeApplication
GET    /api/v1/income-tax/income-application      → GetIncomeApplication
GET    /api/v1/income-tax/income-application/accumulated → GetAccumulatedIncome

POST   /api/v1/income-tax/section115/check        → FlagNonCompliantInvestments
GET    /api/v1/income-tax/section115/compliance   → GetSection115Compliance

POST   /api/v1/income-tax/fcra/register           → RegisterFcra
GET    /api/v1/income-tax/fcra/status             → GetFcraStatus
GET    /api/v1/income-tax/fcra/compliance         → GetFcraCompliance

GET    /api/v1/income-tax/audit-requirements      → GetAuditRequirements
GET    /api/v1/income-tax/compliance-calendar     → GetComplianceCalendar
GET    /api/v1/income-tax/itr7-data               → GetItr7Data
```

#### 6.3.10 Permissions

| Permission | CFO | Controller | Auditor |
|------------|:---:|:----------:|:-------:|
| `it.exemption.register` | ✓ | ✗ | ✗ |
| `it.exemption.renew` | ✓ | ✗ | ✗ |
| `it.income.compute` | ✓ | ✓ | ✗ |
| `it.section115.check` | ✓ | ✓ | ✗ |
| `it.fcra.register` | ✓ | ✗ | ✗ |
| `it.fcra.compute` | ✓ | ✓ | ✗ |
| `it.read` | ✓ | ✓ | ✓ |

#### 6.3.11 AI Features

- **85% application prediction:** Predict year-end application ratio mid-year
- **Compliance risk scoring:** Overall IT compliance risk score
- **Investment compliance monitoring:** Auto-monitor Section 11(5) compliance
- **Exemption renewal optimization:** Suggest optimal time for renewal applications

---

## 7. Budget & Planning

### 7.1 Budgeting

#### 7.1.1 Purpose
Create and manage department-wise, project-wise, and grant-wise budgets. Track budget vs actual with variance analysis.

**Compliance traces:** CD-§4.1 (NAAC — Budget allocation for research), CD-§6.1 (UGC grant budget)

#### 7.1.2 Business Rules

1. **Budget hierarchy:** Institution → Entity → Department → Cost Center → Account Head.
2. **Budget types:** Annual, Project-specific, Grant-specific, Capital, Revenue.
3. **Budget preparation workflow:** Draft → Review → Approved → Active. Multi-level approval.
4. **Budget revision:** Budgets can be revised (increased/decreased) with approval. Revision history tracked.
5. **Budget vs actual:** Real-time tracking of actual expenditure against budget. Variance reports.
6. **Research budget allocation:** Track % of total budget allocated to research (NAAC metric 3.4.4).
7. **Grant budget:** Grant budgets are created from sanctioned grant amounts. Budget heads match grant-approved heads.
8. **Budget carry-forward:** Unspent budget can be carried forward to next year (configurable).

#### 7.1.3 Aggregates

**Aggregate Root: `Budget`**
- `BudgetId` (UUID)
- `EntityId` (FK)
- `FiscalYearId` (FK)
- `BudgetType` (enum: Annual, Project, Grant, Capital, Revenue)
- `Name` (string)
- `Status` (enum: Draft, UnderReview, Approved, Active, Closed)
- `TotalAmount` (Money)
- `RevisedAmount` (Money, nullable)
- `FundId` (FK, nullable)
- `ProjectId` (UUID, nullable)
- `ApprovedById` (UserId, nullable)
- `ApprovedAt` (datetime, nullable)
- `Remarks` (string)

**Entity: `BudgetLine`**
- `BudgetLineId` (UUID)
- `BudgetId` (FK)
- `AccountId` (FK)
- `CostCenterId` (FK, nullable)
- `OriginalAmount` (Money)
- `RevisedAmount` (Money, nullable)
- `UtilizedAmount` (Money, computed)
- `EncumberedAmount` (Money, computed)
- `AvailableAmount` (Money, computed)

**Entity: `BudgetRevision`**
- `BudgetRevisionId` (UUID)
- `BudgetId` (FK)
- `RevisionNumber` (int)
- `PreviousAmount` (Money)
- `NewAmount` (Money)
- `Reason` (string)
- `ApprovedById` (UserId)
- `ApprovedAt` (datetime)

#### 7.1.4 Commands

| Command | Description |
|---------|-------------|
| `CreateBudget(cmd)` | Create budget |
| `SubmitBudgetForReview(budgetId)` | Submit for approval |
| `ApproveBudget(budgetId, approvedById)` | Approve budget |
| `ReviseBudget(budgetId, lines[], reason)` | Revise budget |
| `CloseBudget(budgetId)` | Close budget at year-end |
| `CarryForwardBudget(budgetId, nextFiscalYearId)` | Carry forward unspent budget |

#### 7.1.5 Queries

| Query | Description |
|-------|-------------|
| `GetBudget(budgetId)` | Budget with lines and revisions |
| `GetBudgets(filter)` | Filtered budgets |
| `GetBudgetVsActual(budgetId, asOfDate)` | Budget vs actual report |
| `GetBudgetUtilization(entityId, fiscalYearId)` | Utilization summary |
| `GetResearchBudgetAllocation(fiscalYearId)` | % allocated to research (NAAC) |
| `GetBudgetVarianceReport(entityId, fiscalYearId)` | Variance analysis |

#### 7.1.6 Events

| Event | Payload |
|-------|---------|
| `BudgetCreated` | `{ budgetId, name, fiscalYearId, totalAmount }` |
| `BudgetApproved` | `{ budgetId, approvedBy }` |
| `BudgetRevised` | `{ budgetId, revisionNumber, previousAmount, newAmount }` |
| `BudgetExceeded` | `{ budgetId, accountId, utilizedAmount, budgetAmount }` |
| `BudgetClosed` | `{ budgetId, utilizedAmount, unspentAmount }` |

#### 7.1.7 Permissions

| Permission | CFO | Controller | Dept Head |
|------------|:---:|:----------:|:---------:|
| `budget.create` | ✓ | ✓ | ✓ |
| `budget.approve` | ✓ | ✓ | ✗ |
| `budget.revise` | ✓ | ✓ | ✓ |
| `budget.close` | ✓ | ✗ | ✗ |
| `budget.read` | ✓ | ✓ | ✓ |

---

### 7.2 Forecasting

#### 7.2.1 Purpose
Predict cash flow, revenue, and expenditure using historical data and AI models.

**Compliance traces:** CD-§4.1 (NAAC 5-year financial metrics), CD-§9.5 (Scholarship integration)

#### 7.2.2 Business Rules

1. **Forecast types:** Cash flow (daily/weekly/monthly), Revenue (fee collection), Expenditure (operational), Scholarship disbursement.
2. **Forecast horizon:** Short-term (1 month), Medium-term (1 year), Long-term (5 years for NAAC).
3. **Data sources:** Historical financial data, fee assessment pipeline, PO pipeline, known payment schedules.
4. **AI models:** ML models for fee collection prediction, expense prediction, cash flow forecasting.
5. **Scenario analysis:** Best case, worst case, expected case scenarios.
6. **Forecast accuracy tracking:** Compare forecast vs actual. Track accuracy metrics.

#### 7.2.3 Aggregates

**Aggregate Root: `Forecast`**
- `ForecastId` (UUID)
- `ForecastType` (enum: CashFlow, Revenue, Expenditure, Scholarship)
- `EntityId` (FK)
- `Period` (enum: Daily, Weekly, Monthly, Quarterly, Annual)
- `Horizon` (enum: ShortTerm, MediumTerm, LongTerm)
- `Scenario` (enum: BestCase, Expected, WorstCase)
- `GeneratedAt` (datetime)
- `GeneratedBy` (string — "system" or userId)
- `Accuracy` (decimal, nullable — %)
- `Data` (jsonb — forecast data points)

#### 7.2.4 Commands

| Command | Description |
|---------|-------------|
| `GenerateForecast(type, period, horizon, scenario)` | Generate forecast |
| `RunScenarioAnalysis(baseForecastId, adjustments)` | Run scenario analysis |
| `ComputeForecastAccuracy(forecastId, actualData)` | Compute accuracy |
| `RefreshForecast(forecastId)` | Refresh with latest data |

#### 7.2.5 Queries

| Query | Description |
|-------|-------------|
| `GetForecast(forecastId)` | Forecast with data points |
| `GetForecasts(type, entityId)` | All forecasts |
| `GetCashFlowForecast(entityId, fromDate, toDate)` | Cash flow projection |
| `GetRevenueForecast(entityId, fiscalYear)` | Revenue projection |
| `GetForecastAccuracy(forecastId)` | Accuracy metrics |

#### 7.2.6 AI Features

- **Automated cash flow prediction:** ML-based cash flow forecasting
- **Fee collection prediction:** Predict fee collection by installment and student category
- **Expenditure anomaly detection:** Flag unusual expenditure patterns
- **Scenario simulation:** What-if analysis for budget changes, fee revisions, scholarship changes

---

### 7.3 Encumbrance Accounting

#### 7.3.1 Purpose
Track commitments (POs, contracts) against budgets. Prevent overspending by reserving budget when commitments are made.

**Compliance traces:** CD-§6.1 (Grant budget control)

#### 7.3.2 Business Rules

1. **Encumbrance creation:** When PO is issued, budget is encumbered (reserved).
2. **Encumbrance release:** When GRN is completed or invoice is posted, encumbrance is released and actual is recorded.
3. **Encumbrance types:** Purchase Order, Contract, Agreement, Standing Order.
4. **Pre-encumbrance:** Optional stage before encumbrance (e.g., when PR is approved).
5. **Budget check:** Before encumbrance, check if budget is available. If not, block or warn (configurable).
6. **Encumbrance expiry:** POs that remain open beyond validity period have encumbrance expired.

#### 7.3.3 Aggregates

**Aggregate Root: `Encumbrance`**
- `EncumbranceId` (UUID)
- `ReferenceType` (enum: PurchaseOrder, Contract, Agreement, StandingOrder)
- `ReferenceId` (UUID)
- `BudgetLineId` (FK)
- `Amount` (Money)
- `RemainingAmount` (Money, computed)
- `Status` (enum: Active, PartiallyReleased, Released, Expired, Cancelled)
- `EncumberedAt` (datetime)
- `ReleasedAt` (datetime, nullable)

#### 7.3.4 Commands

| Command | Description |
|---------|-------------|
| `CreateEncumbrance(cmd)` | Encumber budget |
| `ReleaseEncumbrance(encumbranceId, amount)` | Release part of encumbrance |
| `ReleaseFullEncumbrance(encumbranceId)` | Release full encumbrance |
| `ExpireEncumbrance(encumbranceId)` | Expire stale encumbrance |
| `CheckBudgetAvailability(budgetLineId, amount)` | Check if budget is available |

#### 7.3.5 Queries

| Query | Description |
|-------|-------------|
| `GetEncumbrance(encumbranceId)` | Encumbrance details |
| `GetEncumbrances(budgetLineId)` | All encumbrances on budget line |
| `GetEncumbranceSummary(budgetId)` | Encumbrance summary |
| `GetBudgetAvailability(budgetLineId)` | Available budget (original - actual - encumbered) |

#### 7.3.6 Events

| Event | Payload |
|-------|---------|
| `EncumbranceCreated` | `{ encumbranceId, referenceType, referenceId, amount }` |
| `EncumbranceReleased` | `{ encumbranceId, releasedAmount, remainingAmount }` |
| `EncumbranceExpired` | `{ encumbranceId, referenceId, amount }` |
| `BudgetExceededWarning` | `{ budgetLineId, requestedAmount, availableAmount }` |

---

## 8. Asset Accounting

### 8.1 Fixed Assets

#### 8.1.1 Purpose
Manage fixed asset register, depreciation (SLM/WDV), capitalization, transfer, and disposal.

**Compliance traces:** CD-§1.2 (ITC on capital goods — Rule 43), CD-§6.1 (Assets purchased with grants)

#### 8.1.2 Business Rules

1. **Asset categories:** Land, Building, Furniture, Computer Equipment, Lab Equipment, Library Books, Vehicles, Office Equipment, Software.
2. **Capitalization threshold:** Assets above configurable threshold are capitalized. Below threshold: expense.
3. **Depreciation methods:** Straight Line Method (SLM) and Written Down Value (WDV) supported.
4. **Depreciation rates:** Configurable per asset category. Used for books and tax.
5. **ITC on capital goods:** Tracked separately. Rule 43 reversal applies over 60 months (5 years) (CD-§1.2).
6. **Grant-funded assets:** Assets purchased with grant funds are tracked separately. Depreciation treatment as per grant terms.
7. **Asset tagging:** Each asset tagged with barcode/RFID. Location tracking.
8. **Asset transfer:** Assets can be transferred between departments/campuses.
9. **Asset disposal:** Sale, Scrap, Donation, Theft. Disposal requires approval.
10. **Asset verification:** Periodic physical verification. Variance tracked.

#### 8.1.3 Aggregates

**Aggregate Root: `FixedAsset`**
- `FixedAssetId` (UUID)
- `AssetCode` (string, tenant-unique)
- `AssetCategory` (enum: Land, Building, Furniture, ComputerEquipment, LabEquipment, LibraryBooks, Vehicles, OfficeEquipment, Software)
- `AssetName` (string)
- `Description` (string)
- `PurchaseDate` (date)
- `CapitalizationDate` (date)
- `PurchaseCost` (Money)
- `GstOnPurchase` (Money, nullable)
- `ItcClaimed` (Money, nullable)
- `DepreciationMethod` (enum: SLM, WDV)
- `DepreciationRate` (decimal)
- `UsefulLife` (int — years)
- `SalvageValue` (Money, nullable)
- `AccumulatedDepreciation` (Money, computed)
- `NetBookValue` (Money, computed)
- `CurrentLocation` (string, nullable)
- `DepartmentId` (FK to CostCenter, nullable)
- `CustodianId` (UserId, nullable)
- `FundId` (FK, nullable — if grant-funded)
- `Status` (enum: Active, UnderTransfer, Disposed, WrittenOff, Lost)
- `PurchaseInvoiceId` (FK, nullable)
- `IsCapitalGoods` (boolean)
- `Rule43ReversalMonths` (int, default 60)

**Entity: `AssetDepreciation`**
- `AssetDepreciationId` (UUID)
- `FixedAssetId` (FK)
- `FiscalYearId` (FK)
- `PeriodNumber` (int)
- `DepreciationAmount` (Money)
- `IsPosted` (boolean)
- `PostedJournalId` (FK, nullable)

**Entity: `AssetDisposal`**
- `AssetDisposalId` (UUID)
- `FixedAssetId` (FK)
- `DisposalType` (enum: Sale, Scrap, Donation, Theft, WriteOff)
- `DisposalDate` (date)
- `SaleProceeds` (Money, nullable)
- `ProfitLoss` (Money, computed)
- `ApprovedById` (UserId)
- `Remarks` (string)

#### 8.1.4 Commands

| Command | Description |
|---------|-------------|
| `CapitalizeAsset(cmd)` | Create and capitalize asset |
| `TransferAsset(assetId, newDepartment, newCustodian)` | Transfer asset |
| `DisposeAsset(assetId, disposal)` | Dispose asset |
| `ComputeDepreciation(assetId, fiscalYearId)` | Compute depreciation for period |
| `PostDepreciation(assetDepreciationId)` | Post depreciation journal entry |
| `RevalueAsset(assetId, newValue, reason)` | Revalue asset |
| `VerifyAsset(assetId, verifiedAt, location, condition)` | Physical verification |
| `WriteOffAsset(assetId, reason, approvedBy)` | Write off asset |

#### 8.1.5 Queries

| Query | Description |
|-------|-------------|
| `GetFixedAsset(assetId)` | Asset with depreciation schedule |
| `GetFixedAssets(filter)` | Filtered asset list |
| `GetAssetRegister(entityId)` | Asset register |
| `GetDepreciationSchedule(assetId)` | Depreciation schedule |
| `GetDepreciationSummary(fiscalYearId)` | Total depreciation for year |
| `GetAssetsByCategory(category)` | Assets in category |
| `GetGrantFundedAssets(fundId)` | Assets purchased with grant |
| `GetAssetVerificationReport(entityId)` | Verification status |

#### 8.1.6 Events

| Event | Payload |
|-------|---------|
| `AssetCapitalized` | `{ assetId, assetCode, assetName, purchaseCost }` |
| `AssetTransferred` | `{ assetId, fromDepartment, toDepartment, fromCustodian, toCustodian }` |
| `AssetDisposed` | `{ assetId, disposalType, saleProceeds, profitLoss }` |
| `DepreciationPosted` | `{ assetId, fiscalYearId, amount, journalId }` |
| `AssetVerified` | `{ assetId, verifiedAt, condition }` |
| `AssetWrittenOff` | `{ assetId, reason, netBookValue }` |

#### 8.1.7 Permissions

| Permission | CFO | Controller | Accountant | Auditor |
|------------|:---:|:----------:|:----------:|:-------:|
| `asset.capitalize` | ✓ | ✓ | ✓ | ✗ |
| `asset.transfer` | ✓ | ✓ | ✓ | ✗ |
| `asset.dispose` | ✓ | ✓ | ✗ | ✗ |
| `asset.writeoff` | ✓ | ✗ | ✗ | ✗ |
| `asset.depreciation.post` | ✓ | ✓ | ✓ | ✗ |
| `asset.verify` | ✓ | ✓ | ✓ | ✓ |
| `asset.read` | ✓ | ✓ | ✓ | ✓ |

---

### 8.2 Inventory

#### 8.2.1 Purpose
Manage stock of consumables, stationery, lab materials, and other inventory items. Handle valuation (FIFO/Weighted Average) and stock movements.

**Compliance traces:** CD-§1.1 (GST on stationery at 12%)

#### 8.2.2 Business Rules

1. **Valuation methods:** FIFO and Weighted Average supported. Configurable per item category.
2. **Stock movements:** Purchase, Issue, Transfer, Return, Adjustment, Write-off.
3. **Minimum stock level:** Alerts when stock falls below reorder level.
4. **Inventory categories:** General Stores, Lab Consumables, Stationery, Sports Equipment, Maintenance, Canteen, Books.
5. **GST on inventory:** GST applicable on taxable items (e.g., stationery at 12%).
6. **Physical verification:** Periodic stock count. Variance adjustment.

#### 8.2.3 Aggregates

**Aggregate Root: `InventoryItem`**
- `InventoryItemId` (UUID)
- `ItemCode` (string, tenant-unique)
- `ItemName` (string)
- `Category` (string)
- `UnitOfMeasure` (string — e.g., "Nos", "Kg", "Ltr", "Box")
- `ValuationMethod` (enum: FIFO, WeightedAverage)
- `GstRate` (decimal, nullable)
- `HsnSacCode` (string, nullable)
- `CurrentStock` (decimal, computed)
- `CurrentValuation` (Money, computed)
- `ReorderLevel` (decimal, nullable)
- `ReorderQuantity` (decimal, nullable)
- `IsActive` (boolean)

**Entity: `StockMovement`**
- `StockMovementId` (UUID)
- `InventoryItemId` (FK)
- `MovementType` (enum: Purchase, Issue, TransferIn, TransferOut, Return, Adjustment, WriteOff)
- `ReferenceType` (string, nullable — e.g., "GRN", "IssueSlip", "TransferNote")
- `ReferenceId` (UUID, nullable)
- `Quantity` (decimal)
- `UnitPrice` (Money)
- `TotalAmount` (Money)
- `MovementDate` (datetime)
- `Remarks` (string)

#### 8.2.4 Commands

| Command | Description |
|---------|-------------|
| `CreateInventoryItem(cmd)` | Create item |
| `ReceiveStock(itemId, quantity, unitPrice, grnId)` | Record stock receipt (from GRN) |
| `IssueStock(itemId, quantity, departmentId, reason)` | Issue stock |
| `TransferStock(itemId, quantity, fromLocation, toLocation)` | Transfer between locations |
| `AdjustStock(itemId, quantity, reason)` | Stock adjustment (physical count variance) |
| `ComputeStockValuation(itemId, asOfDate)` | Compute current valuation |
| `SetReorderLevel(itemId, level, quantity)` | Set reorder parameters |

#### 8.2.5 Queries

| Query | Description |
|-------|-------------|
| `GetInventoryItem(itemId)` | Item with stock and valuation |
| `GetInventoryItems(filter)` | Filtered items |
| `GetStockReport(category)` | Stock report |
| `GetStockMovements(itemId, fromDate, toDate)` | Movement history |
| `GetReorderAlerts()` | Items below reorder level |
| `GetStockValuationSummary(asOfDate)` | Total inventory valuation |
| `GetPhysicalVarianceReport(verificationId)` | Variance between system and physical count |

---

## 9. Compliance & Reporting

### 9.1 Statutory Reports

#### 9.1.1 Purpose
Generate and file all statutory returns — GST returns (GSTR-1, GSTR-3B, GSTR-9), TDS returns (24Q, 26Q, 27Q), Professional Tax returns (PT-1, PT-2), and other statutory filings.

**Compliance traces:** CD-§1.3 (GST filing), CD-§2.2 (TDS filing), CD-§8.1 (PT filing)

#### 9.1.2 Business Rules

1. **GST returns:** GSTR-1 (monthly/quarterly), GSTR-3B (monthly), GSTR-9 (annual if >₹2 Cr), GSTR-9C (audit if >₹5 Cr).
2. **TDS returns:** 24Q (salary, quarterly), 26Q (non-salary, quarterly), 27Q (non-resident, quarterly).
3. **Professional Tax returns:** PT-1 (monthly) or PT-1A (half-yearly), PT-2 (annual by 31 May) (CD-§8.1).
4. **Return generation:** Returns are auto-generated from transaction data. Manual adjustments allowed.
5. **Filing workflow:** Draft → Review → Approve → File.
6. **Deadline tracking:** All deadlines tracked with configurable reminders.
7. **Multi-entity filing:** Each entity (GSTIN) files separately.

#### 9.1.3 Aggregates

**Aggregate Root: `StatutoryReport`**
- `StatutoryReportId` (UUID)
- `EntityId` (FK)
- `ReportType` (enum: GSTR1, GSTR3B, GSTR9, GSTR9C, Form24Q, Form26Q, Form27Q, PT1, PT1A, PT2)
- `Period` (string)
- `FiscalYear` (string)
- `Status` (enum: Pending, Draft, Generated, Reviewed, Filed, FiledWithErrors)
- `DueDate` (date)
- `FiledDate` (date, nullable)
- `FiledBy` (UserId, nullable)
- `AcknowledgmentNumber` (string, nullable)
- `TaxAmount` (Money, nullable)
- `JsonData` (jsonb)
- `Remarks` (string)

#### 9.1.4 Commands

| Command | Description |
|---------|-------------|
| `GenerateStatutoryReport(cmd)` | Generate report |
| `ReviewReport(reportId, reviewerId)` | Review report |
| `FileReport(reportId, filedBy, acknowledgmentNo)` | Mark as filed |
| `AmendReport(reportId, reason)` | Amend filed report |
| `GenerateComplianceCalendar(fiscalYearId)` | Generate all deadlines |

#### 9.1.5 Queries

| Query | Description |
|-------|-------------|
| `GetStatutoryReport(reportId)` | Report details |
| `GetStatutoryReports(entityId, fiscalYear)` | All reports for entity |
| `GetComplianceCalendar(fiscalYearId)` | All compliance deadlines |
| `GetPendingFilings(entityId)` | Reports due for filing |
| `GetFilingHistory(entityId, fiscalYear)` | Filing history |

#### 9.1.6 Events

| Event | Payload |
|-------|---------|
| `ReportGenerated` | `{ reportId, reportType, period }` |
| `ReportFiled` | `{ reportId, reportType, period, acknowledgmentNo }` |
| `ReportAmended` | `{ reportId, reportType, period, reason }` |
| `FilingDeadlineApproaching` | `{ reportType, period, dueDate, daysRemaining }` |
| `FilingDeadlineMissed` | `{ reportType, period, dueDate }` |

---

### 9.2 Regulatory Reports

#### 9.2.1 Purpose
Generate financial data extracts for regulatory bodies — NAAC dashboard, AISHE extract, UGC Utilization Certificates, and other regulatory submissions.

**Compliance traces:** CD-§4 (NAAC), CD-§5 (AISHE), CD-§6 (UGC)

#### 9.2.2 Business Rules

1. **NAAC dashboard:** Real-time 5-year financial metrics dashboard showing:
   - Grants received for research (3.3.1)
   - Research grants per faculty (3.3.2)
   - Revenue from consultancy (3.3.3)
   - Budget allocation for research (3.4.4)
   - Scholarship/freeship expenditure (5.1.1)
   - Environmental, gender, social initiative expenditure (7.1.1-4)
2. **AISHE extract:** Annual data extract mapping COA heads to AISHE reporting heads (Part C — Financial Data). Data as of 30th September (CD-§5.1).
3. **UGC Utilization Certificate:** GFR 12-A format — audited, signed by Head + Statutory Auditor. Includes grant amount, expenditure, unspent balance, interest earned (CD-§6.1).
4. **Report generation:** Reports generated from live financial data. Schedulable for periodic generation.

#### 9.2.3 Aggregates

**Aggregate Root: `RegulatoryReport`**
- `RegulatoryReportId` (UUID)
- `EntityId` (FK)
- `ReportType` (enum: NAAC, AISHE, UGC_UC, UGC_Annual, FCRA_FC4)
- `FiscalYear` (string)
- `Status` (enum: Draft, Generated, Reviewed, Submitted)
- `GeneratedAt` (datetime)
- `GeneratedBy` (UserId)
- `SubmittedDate` (date, nullable)
- `JsonData` (jsonb)
- `DocumentUrl` (string, nullable — PDF report)

#### 9.2.4 Commands

| Command | Description |
|---------|-------------|
| `GenerateNaacDashboard(fiscalYearId)` | Generate NAAC financial metrics |
| `GenerateAisheExtract(fiscalYearId)` | Generate AISHE data extract |
| `GenerateUgcUtilizationCertificate(fundId, fiscalYearId)` | Generate GFR 12-A UC |
| `GenerateFcraFc4Return(fiscalYearId)` | Generate FC-4 return data |
| `SubmitReport(reportId)` | Mark as submitted |

#### 9.2.5 Queries

| Query | Description |
|-------|-------------|
| `GetNaacDashboard(fiscalYearId)` | NAAC financial metrics |
| `GetAisheExtract(fiscalYearId)` | AISHE data extract |
| `GetUgcUc(fundId, fiscalYearId)` | UGC Utilization Certificate |
| `GetAllRegulatoryReports(fiscalYearId)` | All reports for year |
| `GetAisheHeadMapping()` | COA to AISHE head mapping |

#### 9.2.6 Events

| Event | Payload |
|-------|---------|
| `NaacDashboardGenerated` | `{ fiscalYearId, metrics }` |
| `AisheExtractGenerated` | `{ fiscalYearId, dataUrl }` |
| `UgcUcGenerated` | `{ fundId, fiscalYearId, documentUrl }` |
| `RegulatoryReportSubmitted` | `{ reportId, reportType, fiscalYear }` |

---

### 9.3 Audit

#### 9.3.1 Purpose
Financial audit trails, internal audit support, external auditor access, and audit schedule management.

**Compliance traces:** CD-§7.4 (Audit requirements — 44AB, 12A, Form 10B/10BB), CD-§9.6 (Full audit trail)

#### 9.3.2 Business Rules

1. **Audit trail:** Every financial transaction is timestamped, user-attributed, with before/after values for changes.
2. **External auditor access:** Read-only access to all financial data, reports, and audit trails.
3. **Audit schedule:** Track audit due dates — Tax Audit (44AB) by 30 Sep, Trust Audit (12A) by 30 Sep, Form 10B/10BB by 30 Sep, ITR-7 by 30 Sep (CD-§7.4).
4. **Audit log retention:** All audit logs retained for 8 years (as per Income Tax Act).
5. **Audit confirmation:** Management representation letters, bank confirmations, and other audit evidence can be stored.
6. **Data export:** Audit data export in standard formats (PDF, CSV, XLSX) for auditor review.

#### 9.3.3 Aggregates

**Aggregate Root: `AuditLog`**
- `AuditLogId` (UUID)
- `TenantId` (UUID)
- `EntityId` (UUID, nullable)
- `UserId` (UUID, nullable)
- `UserRole` (string, nullable)
- `Action` (string — e.g., "CREATE", "UPDATE", "DELETE", "POST", "APPROVE", "REVERSE")
- `ResourceType` (string — e.g., "Journal", "PaymentReceipt", "Vendor", "PurchaseOrder")
- `ResourceId` (UUID)
- `Changes` (jsonb — `{ "field": { "old": "value", "new": "value" } }`)
- `IpAddress` (string, nullable)
- `UserAgent` (string, nullable)
- `CreatedAt` (datetime)

**Aggregate Root: `AuditSchedule`**
- `AuditScheduleId` (UUID)
- `FiscalYearId` (FK)
- `AuditType` (enum: TaxAudit_44AB, TrustAudit_12A, Form10B, Form10BB, ITR7, InternalAudit, StatutoryAudit)
- `DueDate` (date)
- `Status` (enum: Pending, InProgress, Completed, ExtensionFiled)
- `CompletedDate` (date, nullable)
- `AuditorName` (string, nullable)
- `AuditorFirm` (string, nullable)
- `AuditorMembershipNumber` (string, nullable)
- `Remarks` (string)

#### 9.3.4 Commands

| Command | Description |
|---------|-------------|
| `LogAuditEntry(action, resourceType, resourceId, changes, userId)` | Log audit entry |
| `CreateAuditSchedule(fiscalYearId, auditType, dueDate)` | Create audit schedule |
| `CompleteAudit(scheduleId, auditorDetails)` | Mark audit as completed |
| `GrantAuditorAccess(entityId, auditorEmail, validUntil)` | Grant read-only access |
| `RevokeAuditorAccess(accessId)` | Revoke auditor access |
| `ExportAuditTrail(filter)` | Export audit trail data |

#### 9.3.5 Queries

| Query | Description |
|-------|-------------|
| `GetAuditLog(filters)` | Paginated audit log |
| `GetAuditTrailForResource(resourceType, resourceId)` | Full audit trail for resource |
| `GetAuditSchedule(fiscalYearId)` | Audit schedule for year |
| `GetPendingAudits()` | Incomplete audits |
| `GetAuditorAccessList()` | Current auditor access |
| `GetUserAuditSummary(userId, fromDate, toDate)` | User action summary |

#### 9.3.6 Events

| Event | Payload |
|-------|---------|
| `AuditLogCreated` | (internal — not published externally) |
| `AuditCompleted` | `{ scheduleId, auditType, fiscalYear, completedDate }` |
| `AuditorAccessGranted` | `{ entityId, auditorEmail, validUntil }` |
| `AuditorAccessRevoked` | `{ entityId, auditorEmail }` |
| `AuditDueDateApproaching` | `{ auditType, fiscalYear, dueDate }` |

---

## 10. Workflow & Approval

### 10.1 Approval Engine

#### 10.1.1 Purpose
Configurable multi-level approval workflows for financial transactions. Role-based and amount-based delegation with escalation.

#### 10.1.2 Business Rules

1. **Approval workflows:** Configurable per transaction type (PO, Invoice, Payment, Refund, Concession, Budget, etc.).
2. **Multi-level approval:** Up to 5 levels of approval. Each level can have different approvers.
3. **Amount-based delegation:** Approvers have maximum approval limits. Transactions above limit go to next level.
4. **Escalation:** If not approved within time limit, escalate to next level.
5. **Approval conditions:** Additional conditions can be configured (e.g., department head + CFO for grant-funded POs).
6. **Delegation:** Approvers can delegate authority to another user during absence.
7. **Approval notifications:** In-app and email notifications for pending approvals.
8. **Approval history:** Complete history of approvals, rejections, and escalations.

#### 10.1.3 Aggregates

**Aggregate Root: `ApprovalWorkflow`**
- `ApprovalWorkflowId` (UUID)
- `EntityId` (FK)
- `TransactionType` (enum: PurchaseOrder, PurchaseInvoice, Payment, Refund, Concession, Budget, ExpenseClaim, Journal, Vendor)
- `Name` (string)
- `IsActive` (boolean)
- `Levels` (int, 1-5)
- `Config` (jsonb — levels configuration)

**Entity: `ApprovalLevel`**
- `ApprovalLevelId` (UUID)
- `ApprovalWorkflowId` (FK)
- `LevelNumber` (int)
- `MaxAmount` (Money, nullable — null means unlimited)
- `ApproverRole` (enum: DeptHead, FinanceController, CFO, Trustee, Registrar)
- `ApproverUserId` (UserId, nullable — specific user)
- `EscalationHours` (int, nullable — hours before escalation)
- `EscalationToLevel` (int, nullable)

**Aggregate Root: `ApprovalRequest`**
- `ApprovalRequestId` (UUID)
- `WorkflowId` (FK)
- `TransactionType` (enum)
- `TransactionId` (UUID)
- `TransactionNumber` (string)
- `Amount` (Money)
- `CurrentLevel` (int)
- `Status` (enum: Pending, Approved, Rejected, Escalated, Cancelled)
- `RequestedById` (UserId)
- `RequestedAt` (datetime)
- `CompletedAt` (datetime, nullable)

**Entity: `ApprovalDecision`**
- `ApprovalDecisionId` (UUID)
- `ApprovalRequestId` (FK)
- `Level` (int)
- `ApproverId` (UserId)
- `Decision` (enum: Approved, Rejected, ReturnedForModification)
- `Comments` (string, nullable)
- `DecidedAt` (datetime)

#### 10.1.4 Commands

| Command | Description |
|---------|-------------|
| `CreateApprovalWorkflow(cmd)` | Create workflow |
| `UpdateApprovalWorkflow(id, config)` | Update workflow |
| `SubmitForApproval(transactionType, transactionId, amount)` | Create approval request |
| `Approve(requestId, approverId, comments)` | Approve at current level |
| `Reject(requestId, approverId, reason)` | Reject request |
| `ReturnForModification(requestId, approverId, comments)` | Return for changes |
| `Escalate(requestId)` | Escalate to next level |
| `DelegateApproval(userId, delegateTo, validFrom, validTo)` | Delegate approval authority |
| `CancelApprovalRequest(requestId, reason)` | Cancel request |

#### 10.1.5 Queries

| Query | Description |
|-------|-------------|
| `GetApprovalWorkflow(transactionType, entityId)` | Workflow for transaction type |
| `GetApprovalRequest(requestId)` | Request with decisions |
| `GetPendingApprovals(userId)` | Pending approvals for user |
| `GetMyApprovalHistory(userId, filter)` | User's approval history |
| `GetApprovalRequestsForTransaction(transactionType, transactionId)` | All requests |
| `GetEscalationPending()` | Requests pending escalation |

#### 10.1.6 Events

| Event | Payload |
|-------|---------|
| `ApprovalRequestCreated` | `{ requestId, transactionType, transactionId, amount }` |
| `ApprovalRequestApproved` | `{ requestId, level, approverId, transactionType, transactionId }` |
| `ApprovalRequestRejected` | `{ requestId, level, approverId, reason }` |
| `ApprovalRequestEscalated` | `{ requestId, fromLevel, toLevel }` |
| `ApprovalRequestCompleted` | `{ requestId, transactionType, transactionId, finalDecision }` |
| `ApprovalDelegationSet` | `{ userId, delegateTo, validFrom, validTo }` |

---

### 10.2 Document Management

#### 10.2.1 Purpose
Manage document attachments for financial transactions — invoices, receipts, PO attachments, GRN documents, compliance certificates, and audit evidence.

#### 10.2.2 Business Rules

1. **Document types:** Invoice, Receipt, PO, GRN, Contract, Agreement, Certificate, Bank Statement, Audit Report, Others.
2. **Storage:** Documents stored in object storage (S3-compatible). Metadata in PostgreSQL.
3. **Document linking:** Documents can be linked to any financial entity (invoice, payment, vendor, etc.).
4. **Versioning:** Document versioning supported. Old versions retained.
5. **Access control:** Document access follows entity-level permissions.
6. **OCR:** Optional OCR for extracting data from invoices/receipts.
7. **Retention policy:** Documents retained per statutory requirements (8 years for financial records).

#### 10.2.3 Aggregates

**Aggregate Root: `Document`**
- `DocumentId` (UUID)
- `TenantId` (UUID)
- `EntityId` (FK, nullable)
- `DocumentType` (enum: Invoice, Receipt, PO, GRN, Contract, Agreement, Certificate, BankStatement, AuditReport, Other)
- `FileName` (string)
- `FileSize` (bigint)
- `MimeType` (string)
- `StoragePath` (string)
- `Checksum` (string — SHA-256)
- `Version` (int)
- `LinkedEntityType` (string, nullable)
- `LinkedEntityId` (UUID, nullable)
- `UploadedById` (UserId)
- `UploadedAt` (datetime)
- `IsDeleted` (boolean)

#### 10.2.4 Commands

| Command | Description |
|---------|-------------|
| `UploadDocument(cmd)` | Upload and link document |
| `UpdateDocument(id, cmd)` | Update document metadata |
| `DeleteDocument(id)` | Soft delete document |
| `LinkDocument(documentId, entityType, entityId)` | Link to entity |

#### 10.2.5 Queries

| Query | Description |
|-------|-------------|
| `GetDocument(documentId)` | Document metadata |
| `GetDocumentDownloadUrl(documentId)` | Pre-signed download URL |
| `GetDocumentsForEntity(entityType, entityId)` | All documents linked to entity |
| `GetDocumentsByType(type, entityId)` | Documents by type |

---

## 11. Cross-Cutting Concerns

### 11.1 Compliance Calendar

Centralized compliance calendar that tracks all filing deadlines across GST, TDS, PT, Income Tax, FCRA, and audit requirements.

**Implementation:** `ComplianceCalendar` aggregate with events for each deadline type. Configurable reminder intervals. Dashboard showing upcoming deadlines.

### 11.2 Multi-Tenant Configuration

All business rules, policies, rates, and thresholds are stored in a `system_config` table as JSONB with schema validation:

```jsonc
{
  "key": "gst.rate.hostel",
  "value": { "rate": 5, "threshold": 1000, "itcEligible": false },
  "scope": "tenant",
  "isActive": true,
  "validFrom": "2026-04-01",
  "validTo": null
}
```

### 11.3 Event Bus / Outbox Pattern

All domain events are published via transactional outbox pattern:
1. Event written to `event_outbox` table in same DB transaction as aggregate change.
2. Background worker reads from outbox and publishes to Redis Streams.
3. Consumers process events idempotently.
4. Failed events retried with exponential backoff, dead-lettered after max retries.

### 11.4 Saga Orchestration

Sagas are implemented as state machines that subscribe to events and emit compensating commands. Saga state stored in `saga_state` table. Each saga step is idempotent.

### 11.5 CQRS Projections

Read models (projections) are updated asynchronously from domain events. Projections for:
- Trial balance (denormalized account balances)
- Fee assessment status
- Vendor payment history
- Student fee ledger
- Budget utilization
- Compliance dashboard

### 11.6 API Versioning

All APIs are versioned via URL prefix (`/api/v1/`). Breaking changes introduce new version. Old versions are deprecated with 6-month notice.

### 11.7 Error Handling

Standardized error response format:

```jsonc
{
  "error": {
    "code": "INSUFFICIENT_BUDGET",
    "message": "Budget not available for this expenditure",
    "details": {
      "budgetId": "uuid",
      "requestedAmount": 50000,
      "availableAmount": 30000
    },
    "requestId": "uuid",
    "timestamp": "2026-07-21T10:30:00Z"
  }
}
```

---

## 12. Appendix: Compliance Mapping

| Compliance Requirement | Domain Model Reference | Key Aggregates |
|------------------------|----------------------|----------------|
| CD-§1.1 GST Exempt/Taxable | §2.1 GL, §3.1 Fee Management, §6.1 GST Engine | Account.gstClassification, FeeHead.gstClassification, GstRegistration |
| CD-§1.2 ITC Rules 42/43 | §6.1 GST Engine | ItcRegister, Rule42/43 auto-reversal computation |
| CD-§1.3 GST Filing | §9.1 Statutory Reports | GstReturn (GSTR-1, GSTR-3B, GSTR-9) |
| CD-§1.4 RCM | §4.2 Procurement, §6.1 GST Engine | PurchaseOrder.isRcmApplicable, RcmEntry |
| CD-§2.1 TDS Sections | §6.2 TDS Engine | TdsSection, TdsDeduction |
| CD-§2.2 TDS Filing | §9.1 Statutory Reports | TdsReturn (Form 24Q, 26Q, 27Q) |
| CD-§2.3 Section 197 | §4.1 Vendor Master | Section197Certificate |
| CD-§3 Maharashtra Scholarships | §3.3 Concessions & Scholarships | Scholarship, ScholarshipScheme, MahaDBT integration |
| CD-§3.3 DBT Reconciliation | §3.2 Fee Collection, §3.3 Scholarships | DBT reconciliation workflow |
| CD-§4 NAAC | §9.2 Regulatory Reports | NaacDashboard (5-year metrics) |
| CD-§5 AISHE | §2.1 GL, §9.2 Regulatory Reports | AisheHeadMapping, AisheExtract |
| CD-§6 UGC Grants | §2.4 Fund Accounting | Fund, FundBudgetHead, UtilizationCertificate |
| CD-§6.3 Endowment | §2.4 Fund Accounting | Fund (fundType=Endowment, incomeOnly) |
| CD-§7.1 Trust Exemptions | §6.3 Income Tax | TrustExemption (10(23C), 11, 12A/12AB) |
| CD-§7.2 85% Application Rule | §6.3 Income Tax | IncomeApplication (85% threshold) |
| CD-§7.2 Section 11(5) | §6.3 Income Tax | Section115 compliance check |
| CD-§7.3 FCRA | §6.3 Income Tax | FcraRegistration, admin expense ≤20% |
| CD-§7.4 Audit | §9.3 Audit | AuditSchedule (44AB, 12A, Form 10B/10BB) |
| CD-§8.1 Professional Tax | §9.1 Statutory Reports | PT-1, PT-2 returns |
| CD-§8.2 FRC Fee Regulation | §3.1 Fee Management, §3.4 Refunds | FrcApprovalOrderNumber, FrcRefundPercent |
| CD-§8.3 Labour Welfare Fund | §9.1 Statutory Reports | LWF computation (employee ₹12, employer ₹24) |
| CD-§9.1 Compliant COA | §2.1 GL | Account hierarchy (5 levels), AISHE-mappable |
| CD-§9.2 Compliance Calendar | §11.1 Cross-Cutting | ComplianceCalendar |
| CD-§9.3 Multi-GSTIN | §2.5 Multi-Entity, §6.1 GST | Entity.gstin, GstRegistration per entity |
| CD-§9.6 Full Audit Trail | §9.3 Audit | AuditLog (every transaction) |
| CD-§9.7 Role-Based Access | §10.1 Approval Engine, All sections | Permission matrix per bounded context |
| CD-§9.9 ITC Tracking | §6.1 GST Engine | ItcRegisterLine (invoice-level tracking) |
| CD-§9.10 RCM Handling | §4.2 Procurement, §6.1 GST | RCM flag at PO, auto-generate RCM entries |

---

*End of Financial Domain Model*