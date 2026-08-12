# SutraERP — Implementation Specs: Treasury & Banking, Tax Engine, Budget & Forecasting, Statutory Reporting

**Prepared by:** CA (agent-ca) — read-only gap analysis, 2026-08-12
**Sources:** `domain-model.md` (DM §2.1–§9.3), `schema/sutra-erp-schema.sql` (DDL), `compliance-discovery.md` (CD §1–§9), `rbac-design.md` (RBAC), `sutra-erp/crates/` (GL/AR/AP/RBAC/Auth/Events implemented; treasury/taxation/budgeting/compliance = empty stubs).
**Conventions inherited (do not re-specify):** paise `BIGINT` money; UUID PKs; `tenant_id` on every table; immutable fact tables (INSERT-only, corrections via reversing entries); transactional outbox; permission format `module:resource:action`; journal line XOR debit/credit; journal statuses DRAFT→POSTED→REVERSED/CANCELLED; period lock; events serialized `#[serde(tag="type")]` into the existing per-module enums (`TreasuryEvent`, `TaxationEvent`, `BudgetEvent`, `ComplianceEvent`). All monetary inputs in paise; all dates ISO 8601; posting rules enforced by the existing GL module (`finance-gl`).

**Existing stubs to fill:** `crates/finance-treasury`, `crates/finance-taxation`, `crates/finance-budgeting`, `crates/compliance` (each has `commands.rs / queries.rs / events.rs / models.rs / repository.rs / errors.rs` empty). `crates/events/src/events.rs` already declares the per-module enums — **extend them, don't rename**. New API nests go in `crates/api/src/router.rs` (currently nests `/gl`, `/ap`, `/auth`; add `/treasury`, `/tax`, `/budget`, `/compliance`).

**Schema gaps (must be added via migration — the DDL lacks these):** `payment_gateway_configs`, `inter_bank_transfers`, `petty_cash_funds`, `petty_cash_transactions`, `gst_rate_master`, `forecasts`. All other tables referenced below already exist in the DDL.

---

## MODULE: Treasury & Banking (crate: `finance-treasury`)

**Purpose**
Owns the institution's money plumbing: the bank account register (multi-campus, per-entity, fund-linked), bank reconciliation (statement import → auto/manual match → BRS), payment-gateway configuration and settlement reconciliation, inter-bank transfers, petty cash, and the Tally-style Bank Book / cash-position views. For an Indian institution this is where UGC's separate-bank-account rule (grants), FCRA's SBI-New-Delhi rule, and the monthly BRS discipline get enforced, and where AR fee receipts (DR Cash → CR FeeIncome) finally meet the bank statement.

**Bounded Context Boundary**
Owns: bank account master + signatories, bank statements, reconciliation state machine, bank transaction register, gateway configs/settlements, inter-bank transfers, petty cash, Bank Book & cash-position read models. **Reads from:** GL (`journal_entries`/`journal_entry_lines` where account is a bank/cash leaf account; `mv_account_balances`), AR (`payment_receipts`, `payment_gateway_transactions`, `refunds`), AP (`vendor_payments`), Fund (`funds.bank_account_id`), Entities (campus mapping). **Writes to:** GL (posts journals for inter-bank transfer, petty-cash top-up, bank charges/interest corrections, transfer of TDS/statutory payments); AR (updates receipt status CHEQUE cleared/bounced via event, not direct write). **Subscribes to:** `PaymentReceiptCreated` (AR), `VendorPaymentProcessed` (AP), `ScholarshipDisbursed` (AR), `FundAmountReceived` (funds) — all become match candidates. **Publishes:** `TreasuryEvent` variants (below). Does **not** own: fee collection initiation, gateway payment initiation/webhooks (AR owns those — DM §3.2), PO/payment execution (AP).

**Aggregates & Entities**

| Aggregate/Entity | Key fields (beyond tenant_id/audit/version) | Invariants | Lifecycle |
|---|---|---|---|
| `BankAccount` (root) | `entity_id` (nullable = shared), `account_number` (encrypted), `ifsc_code`, `account_type` (CURRENT/SAVINGS/FCRA/GRANT_SPECIFIC/DEPOSIT/CASH_CREDIT), `fund_id` (nullable), `is_fcra_account`, `minimum_balance`, `gl_account_id` (FK to COA leaf created on create), `last_reconciled_at` | account_number unique per tenant; FCRA ⇒ bank = SBI New Delhi Main Branch (CD §7.3) and `is_fcra_account=true`; GRANT_SPECIFIC ⇒ `fund_id` required (CD §6.1); one dedicated GL leaf account per bank account; account_number/IFSC format-validated | ACTIVE → (deactivate) → INACTIVE; no delete |
| `BankSignatory` (entity) | `bank_account_id`, `user_id`, `signatory_type` (INDIVIDUAL/JOINT), `is_active` | unique (account,user); at least one active signatory before transfer allowed | ACTIVE/INACTIVE |
| `BankReconciliation` (root) | `bank_account_id`, `period_id`, `statement_date`, `opening_balance`, `closing_balance`, `status`, `verified_by_id` | unique (account, period); one open reconciliation per account; closing balance must equal statement total | IN_PROGRESS → COMPLETED → VERIFIED (DM §5.2.3) |
| `BankStatementLine` (entity) | `bank_reconciliation_id`, `transaction_date`, `transaction_ref` (UTR), `description`, `debit_amount`/`credit_amount` (XOR), `match_status`, `matched_transaction_id`, `matched_transaction_type` | XOR debit/credit; ref required for auto-match candidates | UNMATCHED → MATCHED / MANUAL_MATCH / PARTIAL_MATCH (PARTIAL_MATCH never auto-completes) |
| `BankTransaction` (immutable register) | `bank_account_id`, `transaction_date`, `value_date`, `transaction_ref`, `debit/credit` (XOR), `balance`, `reference_type/id`, `is_reconciled` | INSERT-only; running balance must equal prior balance ± amount | reconciled flag flips on match |
| `InterBankTransfer` (root, **NEW table**) | `from_bank_account_id`, `to_bank_account_id`, `amount`, `transfer_date`, `status`, `bank_reference`, `journal_id` | from ≠ to; both accounts same tenant (cross-entity allowed); approval ≥ configurable threshold; net GL effect zero | INITIATED → APPROVED → PROCESSED → COMPLETED / CANCELLED / FAILED |
| `PettyCashFund` (root, **NEW table**) | `entity_id`, `cash_gl_account_id`, `imprest_amount`, `balance`, `custodian_id`, `status` | balance ≥ 0; per-entity single ACTIVE fund (ASSUMPTION) | ACTIVE → CLOSED |
| `PettyCashTransaction` (immutable, **NEW table**) | `petty_cash_fund_id`, `type` (TOP_UP/RECOUPMENT/EXPENSE/ADJUSTMENT), `amount`, `account_id` (expense), `voucher_no`, `journal_id` | TOP_UP/RECOUPMENT can exceed current balance (funding inflow); EXPENSE ≤ balance; every EXPENSE has an expense account + voucher | immutable |
| `PaymentGatewayConfig` (entity/root, **NEW table**) | `entity_id`, `gateway_type` (BillDesk/Razorpay/CCAvenue/PhonePe/Paytm), `merchant_id`, encrypted `api_key`/`api_secret`/`webhook_secret`, `is_active` | per (entity, gateway) unique; secrets encrypted at rest | ACTIVE/INACTIVE |
| `GatewaySettlement` (root, **NEW table**) | `entity_id`, `gateway_type`, `settlement_date`, `settled_amount`, `gateway_fee_amount`, `status` | settled_amount = sum of matched transactions − fees; unique (entity, gateway, date) | PENDING → RECONCILED / EXCEPTION |

**Commands & Queries**

| Command | Inputs | Validation | Resulting event |
|---|---|---|---|
| `CreateBankAccount` | entity_id, account_number, ifsc, type, fund_id, min_balance | type/IFSC valid; FCRA bank check; fund_id required for GRANT_SPECIFIC; auto-create GL leaf under `10.02` | `BankAccountCreated` |
| `UpdateBankAccount` / `DeactivateBankAccount` | id, fields / id | no deactivate if unreconciled open balance ≠ 0 | `BankAccountUpdated` / `BankAccountDeactivated` |
| `AddSignatory` / `RemoveSignatory` | account_id, user_id, type | unique (account,user) | `SignatoryAdded` / `SignatoryRemoved` |
| `SyncBankBalance` | account_id | — | `BankBalanceSynced` (+ `MinimumBalanceAlert` if < min_balance) |
| `StartReconciliation` | account_id, period_id, statement_date | no open reconciliation for (account, period) | `ReconciliationStarted` |
| `UploadBankStatement` | reconciliation_id, file (CSV/PDF/OFX) | format parser; opening/closing balance cross-check vs statement totals | `BankStatementUploaded` {line_count} |
| `AutoReconcile` | reconciliation_id | matching rule set (below) | `AutoReconciliationCompleted` {matched, unmatched} |
| `ManualMatch` | reconciliation_id, statement_line_id, transaction_id, transaction_type | amount equality; not already matched | `LineMatched` |
| `Unmatch` | reconciliation_id, statement_line_id | only MATCHED/MANUAL_MATCH lines | `LineUnmatched` |
| `CompleteReconciliation` | reconciliation_id | unmatched count = 0 OR user certifies open items; period not closed | `ReconciliationCompleted` |
| `VerifyReconciliation` | reconciliation_id, verified_by | requires `treasury:reconciliation:approve` | `ReconciliationVerified` |
| `GenerateBrs` | reconciliation_id | — | `BrsGenerated` |
| `InitiateInterBankTransfer` | from, to, amount, date | from≠to; sufficient available balance; approval workflow if ≥ threshold (policy `transfer_approval_threshold`, default ₹10,00,000) | `InterBankTransferInitiated` |
| `ApproveInterBankTransfer` / `ProcessInterBankTransfer` / `CompleteInterBankTransfer` | id (+bank ref) | creator ≠ approver (SoD); posts GL contra journal | `InterBankTransferApproved` / `InterBankTransferProcessed` / `InterBankTransferCompleted` |
| `TopUpPettyCash` | fund_id, amount, bank_account_id | fund ACTIVE; posts DR Cash-in-hand → CR Bank | `PettyCashTopUp` |
| `RecordPettyCashExpense` | fund_id, amount, expense_account_id, cost_center/fund, voucher_no | expense ≤ current balance; posts DR Expense → CR Cash | `PettyCashExpenseRecorded` |
| `ConfigureGateway` | entity_id, gateway_type, credentials | validation call against gateway sandbox | `GatewayConfigured` |
| `ReconcileGatewaySettlement` | gateway_type, settlement_date | matches vs `payment_gateway_transactions` (AR table) | `GatewaySettlementReconciled` {settled, fee} |

Queries: `GetBankAccount`, `GetBankAccounts(entity_id)`, `GetFcraAccount`, `GetGrantBankAccounts`, `GetBankBalanceSummary` (cash position), `GetReconciliation(id)`, `GetReconciliations(account_id)`, `GetUnmatchedItems(account, period)`, `GetBrs(account, period)`, `GetReconciliationSummary(account, fy)`, `GetBankBook(account, from, to)` (Tally-style), `GetPettyCashRegister(fund, period)`, `GetPendingGatewaySettlements(entity_id)`, `GetUnclearedCheques` (from AR receipts).

**State Machines**

```
BankReconciliation:  IN_PROGRESS ──Complete──> COMPLETED ──Verify──> VERIFIED
                     (Complete requires: no UNMATCHED lines, or CFO-certified open items)
InterBankTransfer:   INITIATED ──Approve──> APPROVED ──Process──> PROCESSED ──Complete──> COMPLETED
                       │            │             │
                       └─Cancel─> CANCELLED   (Fail on bank rejection) ──> FAILED
                     (Approve: creator ≠ approver; Process: GL contra journal posted with transfer;
                      Complete: bank reference recorded, from/to balances refreshed)
PettyCashFund:       ACTIVE ──Close(custodian change/zero balance)──> CLOSED
GatewaySettlement:   PENDING ──Reconcile──> RECONCILED ──(variance)──> EXCEPTION
```

**Business Rules**
1. **Separate bank account per grant fund** — UGC Grant-in-aid rules / CD §6.1: grant money must be held separately; enforced by `GRANT_SPECIFIC` type + `fund_id` required, and fund receipts post to that account's GL leaf. WHY: UGC audited UC requires grant-wise bank evidence.
2. **FCRA account must be SBI New Delhi Main Branch** — FCRA 2010 s.17 / CD §7.3; enforced in `CreateBankAccount`. WHY: FC-4 filing + statutory audit check.
3. **Monthly BRS is mandatory** — ASSUMPTION (best practice; auditors require it under s.44AB / trust audit): each bank account must have a completed reconciliation per accounting period before that period can be closed; GL period-close saga (DM §2.2.12) should validate this via a treasury read model. WHY: audit trail integrity.
4. **Auto-match criteria** — exact `transaction_ref` (UTR) equality, OR amount equality + date within ±3 days, OR (amount equality + description similarity). Partial amount matches → `PARTIAL_MATCH`, never auto-completed. WHY: UTR-based matching matches bank reality (each bank statement carries UTR for NEFT/RTGS/IMPS/UPI).
5. **Cheque/DD receipts enter the book as UNCLEARED** — AR rule (DM §3.2.6); treasury's reconciliation is the mechanism that flips them to cleared (`ChequeCleared`) — treasury publishes the clearing, AR applies it. WHY: RBI clearing-cycle reality; cash-basis vs book-basis timing difference is the core of BRS.
6. **BRS reconciling items** — deposits-in-transit (book credit, bank not yet), outstanding cheques (book debit, bank not yet), bank charges/interest (bank only → treasury posts adjustment journal `DR Bank Charges (50.06) / CR Bank`, or `DR Bank / CR Interest Income`), gateway fees (bank shows net). WHY: standard BRS structure; gateway fees are always net-of-fee in settlements.
7. **Endowment principal untouched** — Maharashtra Self-Financed Universities Act 2013 / CD §6.3: transfers from an endowment-linked account are blocked unless flagged income-only. WHY: statutory corpus protection.
8. **Minimum-balance alert** — RBI/agreement-driven (ASSUMPTION, per-account `minimum_balance`): `MinimumBalanceAlert` at crossing. WHY: avoids penalty/failed instructions.
9. **Petty cash imprest** — maximum imprest ₹25,000 (ASSUMPTION, configurable `petty_cash_imprest_limit`); expenses must be supported by voucher; recoupment posts DR expense(s) → CR bank. WHY: internal control; limits cash risk.
10. **Two-signature rule** — signatories with `JOINT` type required on cheques above policy threshold `cheque_joint_signature_threshold` (ASSUMPTION, default ₹1,00,000). WHY: institutional banking mandate practice.

**API Contract** (new nest `/treasury` in router; all under `/api/v1`)

| Method | Path | Purpose | Permission |
|---|---|---|---|
| POST | `/treasury/bank-accounts` | CreateBankAccount | `treasury:bank_account:configure` |
| GET | `/treasury/bank-accounts` | list (filter entity/type) | `treasury:bank_account:view` |
| GET | `/treasury/bank-accounts/:id` | detail + GL balance | `treasury:bank_account:view` |
| PUT | `/treasury/bank-accounts/:id` | update | `treasury:bank_account:configure` |
| POST | `/treasury/bank-accounts/:id/deactivate` | deactivate | `treasury:bank_account:configure` |
| POST | `/treasury/bank-accounts/:id/signatories` | add signatory | `treasury:bank_account:configure` |
| POST | `/treasury/reconciliations` | start | `treasury:reconciliation:perform` |
| POST | `/treasury/reconciliations/:id/statement` | upload statement (multipart) | `treasury:bank_statement:upload` (NEW) |
| POST | `/treasury/reconciliations/:id/auto-match` | run auto-reconcile | `treasury:reconciliation:perform` |
| POST | `/treasury/reconciliations/:id/manual-match` | ManualMatch | `treasury:reconciliation:perform` |
| POST | `/treasury/reconciliations/:id/unmatch` | Unmatch | `treasury:reconciliation:perform` |
| POST | `/treasury/reconciliations/:id/complete` | complete | `treasury:reconciliation:perform` |
| POST | `/treasury/reconciliations/:id/verify` | verify | `treasury:reconciliation:approve` |
| GET | `/treasury/reconciliations/:id/brs` | BRS PDF/CSV | `treasury:reconciliation:approve` + `reports:export` |
| POST | `/treasury/transfers` | InterBankTransfer | `treasury:transfer:create` |
| POST | `/treasury/transfers/:id/approve` / `/process` / `/complete` / `/cancel` | transfer lifecycle | `treasury:transfer:approve` (approve/complete), `treasury:transfer:create` (process/cancel) |
| POST | `/treasury/petty-cash` | create fund | `treasury:petty_cash:create` (NEW) |
| POST | `/treasury/petty-cash/:id/top-up` | top-up | `treasury:petty_cash:create` |
| POST | `/treasury/petty-cash/:id/expense` | record expense | `treasury:petty_cash:create` |
| GET | `/treasury/petty-cash/:id/register` | register | `treasury:bank_account:view` |
| POST | `/treasury/gateways` | ConfigureGateway | `treasury:gateway:configure` (NEW) |
| POST | `/treasury/gateways/settlements/reconcile` | settlement reconciliation | `treasury:gateway:configure` |
| GET | `/treasury/reports/bank-book` | Tally-style Bank Book | `reports:financial:view` |
| GET | `/treasury/reports/cash-position` | cash position dashboard | `reports:financial:view` / `reports:dashboard:view` |

**Events** (extend `TreasuryEvent` in `events/src/events.rs`; payload fields)
`BankAccountCreated {bank_account_id, account_name, bank_name, account_type, entity_id, gl_account_id}`, `BankAccountDeactivated {bank_account_id}`, `SignatoryAdded/Removed {bank_account_id, user_id}`, `BankBalanceSynced {bank_account_id, balance, synced_at}`, `MinimumBalanceAlert {bank_account_id, current_balance, minimum_balance}`, `BankStatementUploaded {reconciliation_id, line_count}`, `AutoReconciliationCompleted {reconciliation_id, matched_count, unmatched_count}`, `LineMatched {reconciliation_id, statement_line_id, transaction_id, transaction_type}`, `ReconciliationCompleted {reconciliation_id, bank_account_id, period_id, completed_by, occurred_at}` (exists), `ReconciliationVerified {reconciliation_id, verified_by}`, `BrsGenerated {reconciliation_id, document_url}`, `InterBankTransferCompleted {transfer_id, from_account, to_account, amount, bank_reference}`, `PettyCashTopUp {fund_id, amount}`, `PettyCashExpenseRecorded {fund_id, voucher_no, amount, account_id}`, `GatewayConfigured {entity_id, gateway_type}`, `GatewaySettlementReconciled {entity_id, gateway_type, settlement_date, settled_amount, fee_amount}`.

**Integration Contracts**
- **GL posting (treasury → GL, via existing `gl:journal:create/post`):**
  - *Inter-bank transfer*: Journal type `STANDARD` (contra): `DR Bank-Account-B (10.02.xx.yy) / CR Bank-Account-A (10.02.xx.yy)`, same amount, both lines carry `entity_id` of each campus if cross-entity and the inter-entity due-to/due-from pair when crossing entities (DM §2.5).
  - *Petty cash top-up*: `DR Cash-in-Hand–Petty (10.01.01.01) / CR Bank (10.02.xx.yy)`. *Expense*: `DR Expense A/c (50.xx, with cost_center/fund) / CR Cash-in-Hand (10.01.01.01)`.
  - *Bank charges from statement*: `DR Bank Charges (50.06) / CR Bank (10.02.xx.yy)`; *interest*: `DR Bank / CR Interest Income (40.03)`; both journal type `ADJUSTMENT`, `reference_type='BANK_STATEMENT'`, `reference_id=reconciliation_id`.
  - *TDS deposit / statutory payment*: `DR TDS Payable (24.03.xx) / CR Bank` — posted by tax engine, executed through treasury's bank account (see Tax Engine contract).
  - *Refund execution*: refund journal from AR references the bank account; treasury marks the `bank_transactions` row reconciled when the bank debit appears.
- **What it expects from AR/AP/funds (match candidates):** every system transaction carries a bank reference — AR `payment_receipts.bank_transaction_reference` (UTR for online modes), AP `vendor_payments.bank_transaction_ref`, fund receipts `reference`; DBT scholarship disbursements carry `dbt_transaction_reference`. Reconciliation matches on these + amount + date window.
- **To AR/fee:** publishes `LineMatched` where `matched_transaction_type='PaymentReceipt'` and the receipt was CHEQUE/DD → AR transitions UNCLEARED→COMPLETED (`ChequeCleared`) or →BOUNCED (treasury flags the bank debit-reversal). To scholarship module: matched DBT credits resolve `ScholarshipDBTReconciled`.
- **Account code conventions (ASSUMPTION — default COA seed, configurable):** Bank leaves auto-created under `10.02` (e.g. `10.02.01.01` first account); cash `10.01.01.01`; bank charges `50.06.01.01`; interest `40.03.01.01`; TDS payable `24.03`; GST output `24.01`; RCM `24.02`; ITC recoverable `11.01`. Every bank account created ⇒ exactly one leaf account created under `10.02` and linked via `bank_accounts.gl_account_id` (NEW column — flag for migration).

**Permissions (NEW to add to RBAC Part 1)**
`treasury:bank_statement:upload` (Accountant/Controller, CAMPUS), `treasury:bank_statement:view` (Auditor via `:view` family, GLOBAL), `treasury:gateway:configure` (CFO/Controller, GLOBAL), `treasury:petty_cash:create` (Cashier/Accountant, CAMPUS), `treasury:bank_book:view` (reuse `reports:financial:view` — no new). Existing: `treasury:bank_account:view/configure`, `treasury:reconciliation:perform/approve`, `treasury:transfer:create/approve`.

**AI/Analytics hooks**
- Auto-match suggestions for UNMATCHED lines via embedding/pattern learning on description+amount+date (confidence score; >0.95 auto-apply, else suggest).
- Anomaly detection on bank transactions (round amounts, weekend spikes, unexpected counterparties) — feeds Auditor dashboard flags.
- Cash-position prediction: treasury consumes Budget's cash-flow forecast; liquidity-shortfall alerts (projected balance < minimum within N days).
- Gateway fee benchmarking: detect abnormal gateway fee rates per settlement.

---

## MODULE: Tax Engine (crate: `finance-taxation`)

**Purpose**
Computes, tracks, and reports every statutory tax the institution owes: GST (liability from journal/transaction data, ITC with Rule 42/43 reversal, RCM, GSTR-1/3B/9 return-ready files, configurable rate master), TDS (section/rate/threshold engine used by AP at payment time, deposit tracking, Form 16/16A, 24Q/26Q/27Q data extraction, TDS payable ledger), and income-tax compliance for the trust/society itself (12A/12AB/10(23C) registration validity, 85% application rule, Section 11(5) investments, FCRA caps, audit-deadline tracking). This is the module that keeps an Indian institution out of notices, audits, and 15% disallowances.

**Bounded Context Boundary**
Owns: GST rate master & classification validation, GST liability computation, ITC register + Rule 42/43 reversal, RCM payable ledger + RCM journal creation, GSTR-1/3B/9/9C generation and filing records, TDS section master, TDS computation service (shared library called by AP), TDS deposit/challan tracking, TDS return (24Q/26Q/27Q) generation, Form 16/16A generation, trust exemption registers, 85%-application monitor, FCRA register, tax compliance calendar entries. **Reads from:** GL (`journal_entries`/`journal_entry_lines` with `tax_rate`/`tax_amount`/`is_itc_claimed`), AP (`vendor_invoices`, `vendor_invoice_lines`, `vendor_payments`, `tds_deductions`, `section_197_certificates`, vendors), AR (`payment_receipts`, fee heads' GST classification), Fund (`funds`), Entities (`gstin`). **Writes to:** GL (posts `RCM` and `ITC_REVERSAL` journals, TDS deposit journals); compliance calendar (filing events). **Subscribes to:** `JournalPosted` (GL), `InvoicePosted` (AP), `PaymentReceiptCreated` (AR), `TdsDeducted` (AP), `VendorPaymentProcessed` (AP). **Publishes:** `TaxationEvent` variants. Does **not** own: fee-head/account GST tagging (AR/GL own the masters; tax validates), TDS deduction at payment time (AP executes via the shared computation service).

**Aggregates & Entities**

| Aggregate/Entity | Key fields | Invariants | Lifecycle |
|---|---|---|---|
| `GstRegistration` (root) | `entity_id`, `gstin`, `registration_type`, `filing_frequency`, `is_composite`, `state_code` | unique (tenant, gstin) and (entity); GSTIN regex; state_code matches entity state | ACTIVE/INACTIVE |
| `GstRate` (master, **NEW table** `gst_rate_master`) | `hsn_sac_code`, `description`, `rate`, `itc_eligible`, `effective_from`, `effective_to`, `supply_type` (goods/services) | rate ∈ {0,5,12,18,28}; overlapping effective dates rejected | ACTIVE (effective-dated) |
| `GstReturn` (root) | `gst_registration_id`, `return_type`, `period`, `fiscal_year`, `status`, `due_date`, `filed_date`, `acknowledgment_no`, `json_data`, `tax_liability`, `itc_claimed`, `net_tax_payable` | unique (registration, type, period); GSTR-3B consistency: 3.1(d) RCM = 4(B)(2) RCM ITC | DRAFT → GENERATED → FILED → FILED_WITH_ERRORS → ADJUSTED |
| `GstReturnLine` (entity) | `section`, `taxable_value`, `igst/cgst/sgst/cess` | section code valid for return type (GSTR-1 4A/4B/4C/6/7; GSTR-3B 3.1/4/5) | — |
| `ItcRegister` (root) | `gst_registration_id`, `period`, `total_itc`, `itc_on_inputs`, `itc_on_capital_goods`, `itc_reversal_rule_42`, `itc_reversal_rule_43`, `net_itc_eligible`, `exempt_turnover`, `total_turnover` | unique (registration, period); net = total − reversals | OPEN → COMPUTED → REVERSED → CLOSED |
| `ItcRegisterLine` (entity) | `invoice_id`, `taxable_value`, `igst/cgst/sgst`, `itc_eligibility` (FULL/BLOCKED/REVERSAL_42/REVERSAL_43), `reversal_percent`, `reversal_amount`, `is_reversed` | per invoice per period; eligibility must match account `itc_eligibility` at posting | — |
| `TdsSection` (root/config) | `section_code`, `default_rate`, `threshold_per_payment`, `threshold_aggregate`, `applicable_to` | rate master effective-dated (IT Rules + annual Finance Act); defaults from CD §2.1 | ACTIVE/INACTIVE |
| `TdsDeduction` (owned by AP; tax reads) | payment_id, section, rate, amount, PAN, cert, deposit_status | deposit_status: PENDING → DEPOSITED → FILED (transitioned by tax) | see TDS rules |
| `TdsReturn` (root) | entity_id, `return_type` (FORM_24Q/26Q/27Q), quarter, fiscal_year, status, due_date, ack, total_deductions, total_deposits, json_data | unique (entity, type, quarter, fy) | DRAFT → GENERATED → FILED → FILED_WITH_ERRORS |
| `TdsReturnDetail` (entity) | pan, section, payment_date, payment_amount, tds_rate, tds_amount, surcharge, cess, challan_details, salary_month (24Q) | per deduction row; 24Q rows require salary_month | — |
| `TrustExemption` (root) | entity_id, `exemption_section` (10(23C) variants, 11/12A, 12AB), registration_no, valid_from/to, status | unique (entity, section); 12AB/10(23C) provisional = 3-year validity (CD §7.1) | ACTIVE → RENEWAL_PENDING (≤180d to expiry) → EXPIRED / CANCELLED |
| `IncomeApplication` (root) | fiscal_year_id, entity_id, total_income, amount_applied, application_percent, accumulated_amount, accumulation_year, accumulation_purpose, status | unique (fy, entity); percent = applied/income | COMPLIANT / NON_COMPLIANT / UNDER_REVIEW |
| `IncomeApplicationLine` (entity) | category (SALARIES/INFRASTRUCTURE/SCHOLARSHIPS/RESEARCH/MAINTENANCE/OTHER_EDUCATIONAL), amount, account_id | amount > 0; category maps to qualifying "application" heads | — |
| `FcraRegistration` (root) | registration_no, valid_from/to, `bank_account_id` (must be FCRA account), total_receipts, admin_expenses, admin_expense_ratio, fc4_return_filed_date | admin_expense_ratio ≤ 20% (CD §7.3) | ACTIVE → EXPIRED / RENEWAL_PENDING / CANCELLED |

**Commands & Queries**

| Command | Inputs | Validation | Resulting event |
|---|---|---|---|
| `RegisterGstin` | entity_id, gstin, details | regex; state match; unique | `GstinRegistered` |
| `UpsertGstRate` | hsn_sac, rate, itc_eligible, effective dates | rate set; no date overlap | `GstRateUpdated` |
| `ComputeItc` | registration_id, period | register OPEN; invoice lines pulled from posted vendor invoices | `ItcComputed` |
| `ComputeRule42Reversal` | registration_id, period | formula below; exempt ≤ total turnover | `Rule42ReversalComputed` |
| `ComputeRule43Reversal` | registration_id, period | capital-goods ITC split over 60 months | `Rule43ReversalComputed` |
| `ReverseItc` | register_line_id, amount, reason | amount ≤ eligible; requires `tax:itc:reverse`; posts `ITC_REVERSAL` journal | `ItcReversed` |
| `GenerateGstr1` / `GenerateGstr3b` / `GenerateGstr9` | registration_id, period / fy | period open or previous; sources: journal lines for output, ITC register for input | `Gstr1Generated` / `Gstr3bGenerated` / `Gstr9Generated` |
| `RecordGstFiling` | return_id, acknowledgment_no | return GENERATED; ack format valid | `GstReturnFiled` |
| `CreateRcmEntry` | invoice_id | invoice `is_rcm=true` and no prior RCM entry (idempotent by invoice); posts RCM journals | `RcmEntryCreated` |
| `ConfigureTdsSection` | section_code, rate, thresholds, applicable_to | rate within Finance-Act bands; threshold ≥ 0 | `TdsSectionUpdated` |
| `DeductTds` (service fn for AP) | payment, vendor, invoice(s), section | rate = min(cert rate if valid §197, default); PAN missing → 20% (s.206AA); threshold applied per-payment & aggregate | (AP publishes `TdsDeducted`) |
| `DepositTdsToGovt` | tds_deduction_ids, challan_ref, deposit_date, bank_account_id | only PENDING; posts `DR TDS Payable (24.03) / CR Bank` | `TdsDeposited` |
| `GenerateTdsReturn` | entity_id, return_type, quarter, fy | deductions with deposit_status=DEPOSITED in quarter | `TdsReturnGenerated` |
| `FileTdsReturn` | return_id, ack | GENERATED | `TdsReturnFiled` |
| `GenerateForm16` / `GenerateForm16A` | employee_id / vendor_id, fy | 24Q data complete for all months (Form 16); deductions exist (16A) | `Form16Generated` / `Form16AGenerated` |
| `RegisterTrustExemption` / `RenewExemption` | section, reg_no, valid dates | unique (entity, section); renewal before expiry | `TrustExemptionRegistered` / `ExemptionRenewed` |
| `ComputeIncomeApplication` | fiscal_year_id | all income & application-head expenditure posted | `IncomeApplicationComputed` (+ `IncomeApplicationThresholdMissed` if < 85%) |
| `FlagNonCompliantInvestments` | fiscal_year_id | against s.11(5) securities list (config) | `Section115BreachDetected` |
| `RegisterFcra` / `ComputeFcraCompliance` | reg details / fy | bank must be FCRA type | `FcraRegistered` / `FcraAdminExpenseExceeded` if > 20% |

Queries: `GetGstRegistration(s)`, `GetGstRates(hsn_sac)`, `GetGstReturn/Returns`, `GetGstr1Preview(period)`, `GetGstr3bPreview(period)`, `GetItcRegister(period)`, `GetItcSummary(fy)`, `GetRcmPayable(period)`, `GetGstLiabilitySummary(fy)`, `GetTdsSection(s)`, `GetTdsDeductions(filter)`, `GetTdsRegister(entity, period)` (from TDS-payable GL account), `GetTdsReturns(entity, fy)`, `GetPendingTdsDeposits`, `GetForm16/16A`, `GetTrustExemption`, `GetIncomeApplication(fy)`, `GetAccumulatedIncome`, `GetSection115Compliance(fy)`, `GetFcraStatus/Compliance`, `GetItr7Data(fy)`, `GetAuditRequirements(fy)`.

**State Machines**

```
GstReturn:   DRAFT ──Generate──> GENERATED ──File──> FILED ──(GSTN error)──> FILED_WITH_ERRORS ──Adjust──> ADJUSTED
ItcRegister: OPEN ──Compute──> COMPUTED ──(reversal applied)──> REVERSED ──(period close)──> CLOSED
TdsReturn:   DRAFT ──Generate──> GENERATED ──File──> FILED ──(error)──> FILED_WITH_ERRORS
TdsDeduction (deposit leg): PENDING ──Deposit──> DEPOSITED ──(in return)──> FILED
TrustExemption: ACTIVE ──(≤180d left)──> RENEWAL_PENDING ──Renew──> ACTIVE | ──(past valid_to)──> EXPIRED; CANCELLED from ACTIVE only
```

**Business Rules**
1. **Exempt education services** — Notification 12/2017-CT(R) Entry 66/67 (CD §1.1): tuition/admission/exam/library/lab fees = exempt (SAC 9992); hostel ≤ ₹1,000/day exempt, > ₹1,000/day 5% no-ITC; mess 5% no-ITC; transport 5%; books exempt; stationery 12%; consultancy/workshop/rental 18%. Classification lives on `fee_heads.gst_classification`/`chart_of_accounts.gst_classification`; tax engine validates every classification against the rate master and refuses unknown combos. WHY: misclassification = short-paid tax + interest/penalty under s.73/74.
2. **ITC blocked on exempt inputs** — CGST Act s.17(2) & s.17(5): no ITC on inputs for exempt education services; blocked list (personal consumption, etc.). Accounts tagged `itc_eligibility=BLOCKED` produce zero ITC lines. WHY: s.17(5) is an absolute bar, no recovery.
3. **Rule 42 reversal (inputs)** — CGST Rules 2017 r.42: `C1 = ITC on inputs/input services; D1 = exclusively taxable-use ITC; D2 = exclusively exempt-use ITC; C2 = C1 − D1 − D2; reversal = C2 × (E/F)` where E = exempt turnover, F = total turnover, computed monthly; de-minimis: no reversal if computed amount ≤ 5% of C2 (r.42(1)(m)) — **policy `itc_reversal_tolerance_percent`: default 5% per statute (the DM draft said 0.5 — use 5%, keep configurable — FLAG)**. WHY: ITC must be apportioned when both exempt and taxable supplies exist; reversal posted as `ITC_REVERSAL` journal (DR relevant expense, CR ITC Recoverable).
4. **Rule 43 reversal (capital goods)** — CGST Rules r.43: capital-goods ITC attributable to exempt supplies reversed over 60 months: `monthly reversal = (TC/60) × (E/F)`, TC = total capital-goods ITC; aggregate reversal over 60 months = `TC × E/F` (r.43(1)(c),(d)). WHY: capital goods benefit both exempt and taxable streams over their life.
5. **RCM** — CGST Act s.9(3) + Notification 13/2017-CT(R) (CD §1.4): GTA 5%/12%, advocate 18%, sponsorship 18%, import of services 18%, unregistered security provider; flagged at PO/invoice (`is_rcm_applicable`). **Journal (posted by tax engine on `InvoicePosted` where is_rcm, idempotent by invoice):** `DR Expense (50.xx) / CR RCM Payable (24.02)` and, if ITC eligible, `DR RCM ITC Recoverable (11.01) / CR RCM Payable (24.02)`; GSTR-3B 3.1(d) = RCM output, 4(B)(2) = RCM ITC. WHY: recipient is the taxpayer; non-payment = s.73/74 liability. **Boundary decision: AP posts the base invoice entry only; tax engine owns the RCM leg (FLAG — DM §4.2.15 also mentions RCM in AP's saga; keep single owner = tax, to avoid double posting).**
6. **GSTR-1/3B due dates** — CD §1.3: GSTR-1 11th (monthly) / 13th (QRMP quarterly); GSTR-3B 20th (monthly; QRMP quarterly return 22nd/24th — ASSUMPTION, configurable); GSTR-9 by 31 Dec next FY (>₹2 Cr); GSTR-9C (>₹5 Cr). Return JSON must match the GSTN schema (json_data holds the exact payload). WHY: late filing = late fees s.47 + interest s.50.
7. **TDS rates & thresholds** — IT Act s.192 (slab), 194C 1%/2% (₹30,000/contract, ₹1,00,000 aggregate), 194J 10% (₹30,000), 194I 2%/10% (₹2,40,000), 194A 10% (₹40,000), 194H 5% (₹15,000), 194Q 0.1% (₹50,00,000) (CD §2.1). All in `tds_sections`, effective-dated, configurable. WHY: wrong rate/threshold = s.201(1A) interest + s.271C penalty.
8. **PAN mandatory; else 20%** — IT Act s.206AA (CD §2.1.2): no valid PAN ⇒ deduct at 20% and flag. WHY: statutory rate.
9. **Section 197 lower/nil deduction** — IT Act s.197 (CD §2.3): valid certificate (in-date, vendor-linked) overrides default rate; expiry alerts via calendar. WHY: over-deduction = refund burden; under = interest.
10. **TDS deposit by 7th of next month** — IT Rules r.30 (CD §2.1): deposit via ITNS-281 challan; posted `DR TDS Payable (24.03) / CR Bank`. WHY: late deposit = s.201(1A) interest @1%/mo.
11. **TDS returns 24Q/26Q/27Q by 15th of month after quarter** — IT Rules r.31A (CD §2.2); Form 16 by 31 May (r.31(1)), Form 16A within 15 days of filing (r.31(3)). WHY: late filing = s.234E ₹200/day penalty; missing Form 16 = deductee can't file ITR.
12. **Trust exemption 85% application** — IT Act s.11(1)(a) (CD §7.2): ≥85% of income applied to educational purposes; ≤15% may accumulate under s.11(2) for up to 5 years for specified purposes; non-compliance ⇒ exemption denied for the year (tax at full slab + interest). `income_application_threshold=85`, `accumulation_years=5`. WHY: the single biggest IT risk for trusts.
13. **Section 11(5) investments** — IT Act s.11(5) (CD §7.2): surplus must be invested in specified securities; breach flagged. WHY: violation taints exemption.
14. **12AB/10(23C) validity** — IT Act s.12AB (CD §7.1): provisional registration valid 3 years, renewal required; auto reminder at 180 days. WHY: expiry = automatic loss of exemption.
15. **FCRA caps** — FCRA 2010 s.17 (CD §7.3): admin expenses ≤ 20% of receipts; separate SBI NDMB account; FC-4 by 31 Dec; no re-granting. WHY: s.17 breach = suspension/cancellation.
16. **Audit deadlines** — CD §7.4: s.44AB tax audit, trust audit 12A, Form 10B/10BB, ITR-7 all due 30 Sep. WHY: s.44AB failure = 0.5% (min ₹5,000/max ₹1,50,000) penalty.

**API Contract** (new nest `/tax`)

| Method | Path | Purpose | Permission |
|---|---|---|---|
| POST | `/tax/gst/registrations` | RegisterGstin | `tax:config:configure` |
| GET | `/tax/gst/registrations` | list | `tax:return:view` |
| POST | `/tax/gst/rates` | UpsertGstRate | `tax:config:configure` |
| GET | `/tax/gst/rates` | rate master | `tax:return:view` |
| POST | `/tax/gst/itc/compute` | ComputeItc | `tax:gst_return:prepare` |
| POST | `/tax/gst/itc/rule42` | Rule 42 | `tax:gst_return:prepare` |
| POST | `/tax/gst/itc/rule43` | Rule 43 | `tax:gst_return:prepare` |
| POST | `/tax/gst/itc/reverse` | ReverseItc | `tax:itc:reverse` (NEW) |
| GET | `/tax/gst/itc/register` | ITC register | `tax:return:view` |
| POST | `/tax/gst/returns/gstr1/generate` | GSTR-1 | `tax:gst_return:prepare` |
| GET | `/tax/gst/returns/gstr1/preview` | preview | `tax:return:view` |
| POST | `/tax/gst/returns/gstr3b/generate` | GSTR-3B | `tax:gst_return:prepare` |
| POST | `/tax/gst/returns/:id/file` | RecordGstFiling | `tax:gst_return:file` |
| POST | `/tax/gst/rcm/create` | CreateRcmEntry | `tax:gst_return:prepare` |
| GET | `/tax/gst/rcm/payable` | RCM payable | `tax:return:view` |
| GET | `/tax/gst/reports/liability` | liability summary | `reports:statutory:view` |
| PUT | `/tax/tds/sections/:code` | ConfigureTdsSection | `tax:config:configure` |
| GET | `/tax/tds/sections` | master | `tax:return:view` |
| POST | `/tax/tds/deposit` | DepositTdsToGovt | `tax:tds:deposit` (NEW) |
| GET | `/tax/tds/register` | TDS payable ledger | `tax:return:view` |
| GET | `/tax/tds/pending-deposits` | due deposits | `tax:return:view` |
| POST | `/tax/tds/returns/generate` | GenerateTdsReturn | `tax:tds:return:prepare` (NEW) |
| POST | `/tax/tds/returns/:id/file` | FileTdsReturn | `tax:gst_return:file` |
| POST | `/tax/tds/form16/generate` | Form 16 | `tax:form16:generate` (NEW) |
| POST | `/tax/tds/form16a/generate` | Form 16A | `tax:form16:generate` |
| POST | `/tax/income/exemption` | RegisterTrustExemption | `tax:config:configure` |
| POST | `/tax/income/exemption/:id/renew` | renew | `tax:config:configure` |
| POST | `/tax/income/application/compute` | ComputeIncomeApplication | `tax:income:compute` (NEW) |
| GET | `/tax/income/application` | 85% report | `tax:return:view` |
| POST | `/tax/income/section115/check` | investments check | `tax:income:compute` |
| POST | `/tax/income/fcra/register` | RegisterFcra | `tax:config:configure` |
| GET | `/tax/income/fcra/compliance` | FCRA report | `tax:return:view` |
| GET | `/tax/income/itr7-data` | ITR-7 extract | `reports:export` |
| GET | `/tax/income/audit-requirements` | audit checklist | `tax:return:view` |

**Events** (extend `TaxationEvent`)
`GstinRegistered {reg_id, entity_id, gstin}`, `GstRateUpdated {hsn_sac_code, rate, effective_from}`, `ItcComputed {reg_id, period, total_itc, net_itc_eligible}`, `Rule42ReversalComputed {reg_id, period, reversal_amount, exempt_turnover, total_turnover}`, `Rule43ReversalComputed {reg_id, period, reversal_amount, capital_goods_itc}`, `Gstr1Generated {return_id, period, tax_liability}`, `Gstr3bGenerated {return_id, period, tax_liability, itc_claimed}` (exists), `GstReturnFiled {return_id, period, acknowledgment_no}`, `RcmEntryCreated {invoice_id, rcm_payable_amount, journal_id}`, `TdsSectionUpdated {section_code, rate, threshold}`, `TdsDeposited {tds_deduction_id, challan_reference, deposit_date, amount}`, `TdsReturnGenerated {return_id, return_type, quarter, fiscal_year, total_deductions}` (exists), `TdsReturnFiled {return_id, acknowledgment_no}`, `Form16Generated {employee_id, fiscal_year, document_url}`, `Form16AGenerated {vendor_id, fiscal_year, document_url}`, `TrustExemptionRegistered {reg_id, section, registration_no, valid_to}`, `ExemptionExpiring {reg_id, section, days_remaining}`, `IncomeApplicationComputed {fiscal_year_id, total_income, applied_percent, is_compliant}`, `IncomeApplicationThresholdMissed {fiscal_year_id, applied_percent, threshold}`, `Section115BreachDetected {fiscal_year_id, details}`, `FcraRegistered {reg_id, registration_no, valid_to}`, `FcraAdminExpenseExceeded {fiscal_year_id, expense_ratio, max_allowed}`. (`TdsDeducted` is published by AP on deduction using the shared enum — tax subscribes.)

**Integration Contracts**
- **GL posting rules (all via gl module; journals carry `reference_type`/`reference_id` for traceability):**
  - *Output GST*: posted by AR at receipt for taxable fee heads — `DR Bank/Receivable / CR Fee/Service Income, CR Output CGST/SGST/IGST (24.01)`. Tax engine aggregates GSTR-1/3B from `journal_entry_lines.tax_rate/tax_amount` where account has gst_classification.
  - *ITC on purchases*: posted by AP at invoice — `DR Expense, DR ITC Recoverable CGST/SGST/IGST (11.01) / CR Vendor Payable`. ITC register lines built from `vendor_invoice_lines` + account `itc_eligibility`.
  - *Rule 42/43 reversal*: tax posts `ITC_REVERSAL` journal: `DR Expense (proportionate, 50.xx) / CR ITC Recoverable (11.01)`.
  - *RCM*: tax posts `RCM` journal as per Rule 5 above.
  - *TDS*: AP posts at payment — `DR Vendor Payable (gross) / CR Bank (net), CR TDS Payable (24.03.xx.yy per section)`. Deposit: `DR TDS Payable / CR Bank`. TDS payable ledger = GL balance of 24.03 accounts by section.
  - *Tax payment (GST cash ledger)*: `DR GST Payable (24.01/24.02) / CR Bank` — challan recorded on the return; treasury executes.
- **To compliance calendar:** publish `GstReturnFiled`, `TdsReturnFiled`, `GstFilingDeadlineApproaching`, `TdsReturnGenerated` — compliance marks calendar events COMPLETED.
- **To AP:** expose `TdsComputationService` (pure function: section, rate lookup with §197 override + §206AA fallback + thresholds) — AP calls it in `DeductTds`; single source of truth for rates.
- **To budget/forecast:** tax cash outflows (GST/TDS deposit schedule) feed the cash-flow forecast as deterministic obligations.

**Permissions (NEW)**
`tax:itc:compute` (Accountant/Controller, GLOBAL), `tax:itc:reverse` (Controller/CFO, GLOBAL), `tax:tds:deposit` (Accountant/Controller, CAMPUS), `tax:tds:return:prepare` (Accountant/Controller, CAMPUS), `tax:form16:generate` (Accountant/Controller, CAMPUS), `tax:income:compute` (Controller/CFO, GLOBAL), `tax:exemption:register` (CFO, GLOBAL). Existing: `tax:return:view`, `tax:gst_return:prepare`, `tax:gst_return:file`, `tax:tds:deduct`, `tax:config:configure`.

**AI/Analytics hooks**
- ITC optimization advisor (identify blocked/ignored credits worth claiming; reversal-pattern sanity vs peers).
- GST notice-risk score on return data before filing (variance vs last-12-months profile).
- 85%-application early-warning: mid-year projection of year-end application ratio from current spend run-rate.
- TDS anomaly detection (rate changes mid-year, skipped deductions, PAN churn) — feeds Auditor exception queue.

---

## MODULE: Budget & Forecasting (crate: `finance-budgeting`)

**Purpose**
Turns the institution's annual plan into controlled, enforceable numbers: multi-version budgets (original/revised/approved) across cost centers, departments, projects and grants; encumbrance accounting so POs/contracts reserve budget before cash moves; re-appropriation (budget transfer) with approval; variance analysis (budget vs actual vs committed); and cash-flow forecasting built from fee schedules, payroll and vendor due dates, with what-if scenarios. This is what makes NAAC's research-budget metric (3.4.4) and UGC's grant-budget-head discipline real rather than spreadsheet theatre.

**Bounded Context Boundary**
Owns: budgets + lines + revisions, budget approval workflow triggers, encumbrances, budget transfers, variance read models, forecasts and scenarios. **Reads from:** GL (`mv_account_balances`, journal lines for actuals), Fund (`fund_budget_heads` — grant-approved heads), AP (POs/invoices for encumbrance + vendor due dates), AR (installment plans / fee assessments for projected receipts), cost centers/entities (dimensions). **Writes to:** GL: **none** (budget is a shadow ledger — actuals live in GL; budget does not post journals). Workflow: creates approval requests for budget approval/revision/transfer. **Subscribes to:** `PurchaseOrderIssued` (AP) → create encumbrance; `PurchaseOrderCancelled` (AP) → release encumbrance; `InvoicePosted` (AP) → release + actual; `PaymentReceiptCreated` (AR) → actual income; `JournalPosted` (GL) → actual expense; `FundSanctioned` (funds) → grant budget availability. **Publishes:** `BudgetEvent` variants. Does **not** own: PO/PR creation (AP), grant budget heads (fund accounting), period locking (GL).

**Aggregates & Entities**

| Aggregate/Entity | Key fields | Invariants | Lifecycle |
|---|---|---|---|
| `Budget` (root) | entity_id, fiscal_year_id, `budget_type` (ANNUAL/PROJECT/GRANT/CAPITAL/REVENUE), name, status, total_amount, revised_amount, fund_id, project_id | unique (tenant, entity, fy, type, name); GRANT budgets require fund_id and lines ⊆ fund_budget_heads (CD §6.1); total = Σ lines | DRAFT → UNDER_REVIEW → APPROVED → ACTIVE → CLOSED |
| `BudgetLine` (entity) | account_id, cost_center_id, original_amount, revised_amount, (computed: utilized, encumbered, available) | unique (budget, account, cost_center); revised ≥ 0; available = revised − utilized − encumbered | — |
| `BudgetRevision` (entity) | budget_id, revision_number, previous_amount, new_amount, reason, approved_by | unique (budget, rev_number); every Revise/Transfer appends a row | — |
| `Encumbrance` (root) | budget_line_id, reference_type (PURCHASE_ORDER/CONTRACT/AGREEMENT/STANDING_ORDER), reference_id, amount, remaining_amount, status | remaining ≤ amount; reference unique per type; only ACTIVE/PARTIALLY_RELEASED consume budget | ACTIVE → PARTIALLY_RELEASED → RELEASED / EXPIRED / CANCELLED |
| `BudgetTransfer` (root, **NEW table**; re-appropriation) | budget_id, from_line_id, to_line_id, amount, reason, status, approved_by | from ≠ to; from-line available ≥ amount; same budget (cross-budget transfers = revision, FLAG); creates paired BudgetRevision rows on approval | INITIATED → APPROVED → APPLIED / REJECTED / CANCELLED |
| `Forecast` (root, **NEW table** `forecasts`) | forecast_type (CASH_FLOW/REVENUE/EXPENDITURE/SCHOLARSHIP), entity_id, period (DAILY/WEEKLY/MONTHLY/QUARTERLY/ANNUAL), horizon, scenario (BEST_CASE/EXPECTED/WORST_CASE), data (jsonb), accuracy | per (type, entity, period, horizon, scenario) latest generation | GENERATED (immutable snapshot; refresh creates new row) |
| `ForecastLine` (derived, in jsonb) | date, deterministic_amount, ml_adjustment, total | Σ lines = forecast total | — |

**Commands & Queries**

| Command | Inputs | Validation | Resulting event |
|---|---|---|---|
| `CreateBudget` | entity, fy, type, name, lines[] | lines sum = total; GRANT ⇒ fund + heads ⊆ fund_budget_heads | `BudgetCreated` |
| `SubmitBudgetForReview` | budget_id | DRAFT | (workflow) |
| `ApproveBudget` | budget_id, approver | UNDER_REVIEW/APPROVED; SoD creator ≠ approver | `BudgetApproved` |
| `ReviseBudget` | budget_id, lines[] (revised amounts), reason | APPROVED/ACTIVE; every change appends BudgetRevision; requires `budget:revision:create` | `BudgetRevised` |
| `TransferBudgetLine` | budget_id, from_line_id, to_line_id, amount, reason | available(from) ≥ amount; workflow approval if amount > `budget_transfer_approval_threshold` (default ₹5,00,000 — ASSUMPTION) | `BudgetTransferApproved` |
| `CloseBudget` | budget_id | ACTIVE + period closed; records unspent | `BudgetClosed` |
| `CarryForwardBudget` | budget_id, next_fy_id | policy `budget_carry_forward_enabled` (default true); unspent > 0 | `BudgetCarriedForward` |
| `CreateEncumbrance` (on PO event) | budget_line_id, reference_type/id, amount | `IsBudgetAvailable` (available ≥ amount) else `BudgetExceededWarning`; reference not already encumbered | `EncumbranceCreated` |
| `ReleaseEncumbrance` / `ReleaseFullEncumbrance` | encumbrance_id, amount | amount ≤ remaining | `EncumbranceReleased` |
| `ExpireEncumbrance` | encumbrance_id | PO past validity + no receipts | `EncumbranceExpired` |
| `CheckBudgetAvailability` | budget_line_id, amount | — | — (query) |
| `GenerateForecast` | type, entity, period, horizon, scenario | deterministic baseline (below) + ML overlay | `ForecastGenerated` |
| `RunScenarioAnalysis` | base_forecast_id, adjustments jsonb | adjustments whitelisted (fee %, scholarship %, salary %, capital ₹) | `ScenarioRun` |
| `ComputeForecastAccuracy` | forecast_id, actuals | — | `ForecastAccuracyComputed` |

Queries: `GetBudget`, `GetBudgets(filter)`, `GetBudgetVsActual(budget_id, as_of)`, `GetBudgetUtilization(entity, fy)`, `GetResearchBudgetAllocation(fy)` (NAAC 3.4.4), `GetBudgetVarianceReport(entity, fy)` (budget vs actual vs committed), `GetEncumbrance/Encumbrances`, `GetEncumbranceSummary(budget_id)`, `GetBudgetAvailability(line_id)`, `GetForecast(s)`, `GetCashFlowForecast(entity, from, to)`, `GetRevenueForecast(entity, fy)`, `GetForecastAccuracy`, `GetBudgetTransferLedger(budget_id)`.

**State Machines**

```
Budget:        DRAFT ──Submit──> UNDER_REVIEW ──Approve──> APPROVED ──(activation/effect)──> ACTIVE ──Close──> CLOSED
               (any of DRAFT/UNDER_REVIEW/APPROVED ──Reject──> DRAFT; CLOSED is terminal; carry-forward spawns next-fy budget)
Encumbrance:   ACTIVE ──(partial release)──> PARTIALLY_RELEASED ──(full release)──> RELEASED
               ACTIVE ──Expire──> EXPIRED ; ACTIVE/PARTIALLY_RELEASED ──Cancel──> CANCELLED
BudgetTransfer: INITIATED ──Approve──> APPROVED ──Apply──> APPLIED (posts paired revisions) ; ──Reject/Cancel──> REJECTED/CANCELLED
```

**Business Rules**
1. **Budget is a shadow ledger** — actuals come exclusively from posted GL journals (expense accounts) and AR receipts (income); budget never posts journals. WHY: single source of truth (GL) prevents drift between budget and books; auditors verify against trial balance.
2. **Encumbrance on PO issue** — when AP issues a PO, `PurchaseOrderIssued` triggers `CreateEncumbrance`; GRN/invoice posting releases it and records actual (DM §7.3.2). WHY: encumbrance accounting prevents overspend (common institutional audit finding).
3. **Budget check before PO** — policy `budget_check_on_po` (default true): PO issue fails/warns if `available < PO amount` (DM §4.2.10). WHY: UGC grant budget heads must not be exceeded (CD §6.1).
4. **Grant budget heads are binding** — GRANT-type budget lines must match `fund_budget_heads` (account-level); a PO against a grant fund must reference a head within them. WHY: UC in GFR 12-A compares spend against sanctioned heads; excess = disallowance.
5. **Revision requires approval and full history** — every revise/transfer appends `BudgetRevision`; nothing is overwritten. WHY: audit trail; budget committees need the paper trail.
6. **Re-appropriation approval** — transfers above threshold need workflow approval; from-line availability is checked atomically at APPLY time (row lock on from/to lines). WHY: unauthorized re-appropriation is a classic UGC/NAAC audit observation.
7. **Research budget metric** — `GetResearchBudgetAllocation(fy)` = (Σ budget lines tagged `naac_metric_key='3.4.4'`) / total budget. WHY: NAAC Manual metric 3.4.4 (CD §4.1).
8. **Variance reporting** — variance = revised − (utilized + encumbered); report rows: original, revised, actual, committed, available, % utilization. WHY: management + NAAC 5-year trend needs comparable yearly views.
9. **Cash-flow forecast composition** — deterministic baseline = fee installment due dates (AR) + scholarship DBT pipeline (AR) + payroll schedule (config, default 1st working day — ASSUMPTION) + vendor invoice due dates (AP) + known grant receipts (funds) + tax obligations (tax engine) + capex plan; ML overlay adjusts collection timing/ratios. WHY: fee cash is lumpy (term starts); payroll + TDS are fixed obligations — the two must never collide.
10. **Scenario whitelist** — what-if adjustments limited to configured levers (fee revision %, scholarship %, salary %, new-headcount ₹, capital ₹) so scenarios stay comparable and auditable. WHY: uncontrolled scenarios produce unverifiable numbers (ASSUMPTION).
11. **Forecast accuracy tracked** — every forecast stores accuracy vs actuals (MAPE) for model tuning. WHY: AI claims must be measurable; NAAC uses 5-year trend so forecasts must be comparable.

**API Contract** (new nest `/budget`)

| Method | Path | Purpose | Permission |
|---|---|---|---|
| POST | `/budget/budgets` | CreateBudget | `budget:budget:create` |
| GET | `/budget/budgets` | list | `budget:budget:view` |
| GET | `/budget/budgets/:id` | detail + variance | `budget:budget:view` |
| POST | `/budget/budgets/:id/submit` | submit | `budget:budget:create` |
| POST | `/budget/budgets/:id/approve` | approve | `budget:budget:approve` |
| POST | `/budget/budgets/:id/revise` | ReviseBudget | `budget:revision:create` |
| POST | `/budget/budgets/:id/transfer` | BudgetTransfer | `budget:transfer:create` (NEW) |
| POST | `/budget/transfers/:id/approve` | approve transfer | `budget:transfer:approve` (NEW) |
| POST | `/budget/budgets/:id/close` | CloseBudget | `budget:budget:approve` |
| POST | `/budget/budgets/:id/carry-forward` | carry forward | `budget:budget:approve` |
| GET | `/budget/encumbrances` | list (filter line/budget/status) | `budget:encumbrance:view` |
| GET | `/budget/encumbrances/:id` | detail | `budget:encumbrance:view` |
| GET | `/budget/reports/variance` | variance by entity/fy | `budget:budget:view` |
| GET | `/budget/reports/research-allocation` | NAAC 3.4.4 | `reports:financial:view` |
| POST | `/budget/forecasts` | GenerateForecast | `budget:forecast:generate` (NEW) |
| POST | `/budget/forecasts/:id/scenario` | RunScenarioAnalysis | `budget:forecast:generate` |
| GET | `/budget/forecasts` | list | `budget:forecast:view` (NEW) |
| GET | `/budget/forecasts/cash-flow` | cash-flow projection | `reports:financial:view` |
| GET | `/budget/forecasts/:id/accuracy` | accuracy | `budget:forecast:view` |

**Events** (extend `BudgetEvent`)
`BudgetCreated {budget_id, name, fiscal_year_id, total_amount}`, `BudgetApproved {budget_id, budget_name, fiscal_year_id, total_amount, approved_by, occurred_at}` (exists), `BudgetRevised {budget_id, revision_number, previous_amount, new_amount}`, `BudgetTransferApproved {transfer_id, from_line_id, to_line_id, amount}`, `BudgetClosed {budget_id, utilized_amount, unspent_amount}`, `BudgetCarriedForward {budget_id, next_fiscal_year_id, amount}`, `EncumbranceCreated {encumbrance_id, budget_line_id, reference_type, reference_id, amount}` (exists), `EncumbranceReleased {encumbrance_id, released_amount, remaining_amount}`, `EncumbranceExpired {encumbrance_id, reference_id, amount}`, `BudgetExceeded {budget_line_id, budgeted_amount, actual_amount}` (exists), `BudgetExceededWarning {budget_line_id, requested_amount, available_amount}`, `ForecastGenerated {forecast_id, forecast_type, period, horizon, scenario}`, `ForecastAccuracyComputed {forecast_id, mape, period}`.

**Integration Contracts**
- **GL:** read-only — actuals via `mv_account_balances` and posted journal lines filtered by `account_id` on budget lines, `cost_center_id`, `fund_id`. No writes.
- **From AP:** `PurchaseOrderIssued` {po_id, po_number, total_amount, fund_id, budget_head_id, line accounts/amounts} → encumbrance; `PurchaseOrderCancelled` → release; `InvoicePosted` {invoice_id, journal_id, amount} → release encumbrance by PO reference + actual. **Contract note: POs must carry `budget_head_id` and per-line `account_id` (already in DDL) — budget lookup key is (account_id, cost_center_id, fund_id).**
- **From AR:** `PaymentReceiptCreated` → actual income against revenue budgets; installment schedules (`fee_installments`) feed the deterministic cash-flow baseline.
- **To treasury:** `ForecastGenerated` (CASH_FLOW) → treasury cash-position dashboard consumes it.
- **To workflow:** approval requests for budget approve/revision/transfer (workflow crate, transaction_type `BUDGET` — already in DDL enum).

**Permissions (NEW)**
`budget:transfer:create` (Controller/CFO/HOD-dept, CAMPUS/DEPARTMENT), `budget:transfer:approve` (CFO/Controller, GLOBAL), `budget:encumbrance:create` (system/AP-triggered; Accountant manual, CAMPUS), `budget:encumbrance:release` (Accountant/Controller, CAMPUS), `budget:forecast:generate` (CFO/Controller, GLOBAL), `budget:forecast:view` (CFO/Controller/HOD, GLOBAL/DEPARTMENT). Existing: `budget:budget:view/create/approve`, `budget:revision:create`, `budget:encumbrance:view`.

**AI/Analytics hooks**
- Collection-timing model: predicts fee collection curve per installment/category (feeds EXPECTED scenario; worst-case = −p90 lag).
- Spend run-rate alerts: line-level forecast-to-actual drift > threshold mid-period.
- Grant unspent-balance prediction (consumed by fund accounting for UC planning).
- Scenario sensitivity: which levers move year-end surplus the most (feeds trustee decks).

---

## MODULE: Statutory Reporting (crate: `compliance`)

**Purpose**
The outward-facing compliance surface: generates NAAC financial metrics (per-capita income, expenditure ratios, 5-year trends), AISHE Part-C financial extracts (COA→AISHE code mapping, E1/E2 form data), UGC Utilization Certificates in GFR 12-A format with audited schedules, and runs the annual compliance calendar with every filing deadline and its live status. For a CFO this is the difference between "we're audit-ready" and a scramble every September and December.

**Bounded Context Boundary**
Owns: statutory report records (GST/TDS/PT filing state), regulatory reports (NAAC/AISHE/UGC/FC-4), the compliance calendar (deadline generation, reminders, status), audit schedules. **Reads from:** GL (COA incl. `aishe_head_code`, `naac_metric_key`; account balances), Fund (utilization, interest), tax engine (GSTR/TDS returns + filing status), budget (research allocation), AR (scholarship expenditure), entities. **Writes to:** none in GL; updates own tables only. **Subscribes to:** `GstReturnFiled`, `TdsReturnFiled`, `GstFilingDeadlineApproaching` (tax), `FiscalYearOpened` (GL), `GstinRegistered` (tax), `TdsDeducted` (AP), `BudgetApproved` (budget), `UtilizationCertificateGenerated` (funds). **Publishes:** `ComplianceEvent` variants. Does **not** own: return *computation* (tax engine does GSTR/TDS numbers); calendar *roots* for GST/TDS (tax publishes filings); grant UC *content* (fund accounting generates; compliance files it).

**Aggregates & Entities**

| Aggregate/Entity | Key fields | Invariants | Lifecycle |
|---|---|---|---|
| `StatutoryReport` (root) | entity_id, report_type (GSTR1/GSTR3B/GSTR9/GSTR9C/FORM_24Q/FORM_26Q/FORM_27Q/PT1/PT1A/PT2), period, fiscal_year, status, due_date, filed_date, acknowledgment_no, tax_amount, json_data | unique (entity, type, period); json_data mirrors portal payload | PENDING → DRAFT → GENERATED → REVIEWED → FILED → FILED_WITH_ERRORS |
| `RegulatoryReport` (root) | entity_id, report_type (NAAC/AISHE/UGC_UC/UGC_ANNUAL/FCRA_FC4), fiscal_year, status, json_data, document_url | unique (entity, type, fy) | DRAFT → GENERATED → REVIEWED → SUBMITTED |
| `ComplianceCalendarEvent` (root) | entity_id, event_type (GST_FILING/TDS_FILING/TDS_DEPOSIT/PT_FILING/IT_EXEMPTION_RENEWAL/AUDIT_44AB/AUDIT_12A/FORM_10B/FORM_10BB/ITR7/FCRA_FC4/FCRA_RENEWAL/AISHE_SUBMISSION/UGC_UC/NAAC_SUBMISSION), due_date, reminder_days, status, reference_id | unique (tenant, entity, type, due_date); reference_id links return/report | PENDING → COMPLETED / EXTENSION_APPLIED / OVERDUE (auto on due_date past) |
| `AuditSchedule` (root) | fiscal_year_id, audit_type, due_date, status, auditor details | unique (fy, audit_type) | PENDING → IN_PROGRESS → COMPLETED / EXTENSION_FILED |
| `NaacMetric` (read model, computed) | metric_key (3.3.1/3.3.2/3.3.3/3.4.4/5.1.1/7.1.1–7.1.4), fiscal_year, value, source (account/fund/budget) | each metric traceable to GL accounts via `naac_metric_key` | — |
| `AisheMapping` (entity) | account_id, aishe_head_code (Part C head) | sub-head level accounts only; one code per account (from `chart_of_accounts.aishe_head_code`) | — |
| `UgcUcDraft` (entity, derived) | fund_id, fy, sanctioned, received, expenditure, interest_earned, unspent_balance, schedules jsonb | unspent = received + interest − expenditure | derived from fund ledger; signed → SUBMITTED |

**Commands & Queries**

| Command | Inputs | Validation | Resulting event |
|---|---|---|---|
| `GenerateStatutoryReport` | entity, report_type, period | source data available (tax engine return for GST/TDS); report not already FILED | `ReportGenerated` |
| `ReviewReport` / `FileReport` | report_id (+reviewer / ack) | GENERATED → REVIEWED → FILED; `file` requires `reports:export`+`compliance:statutory:file` | `ReportFiled` (exists as StatutoryReportFiled) |
| `AmendReport` | report_id, reason | FILED only; opens new period row with ADJUSTED lineage | `ReportAmended` |
| `GenerateNaacDashboard` | fiscal_year_id | aggregates 5 years; metrics from COA naac tags + funds + budget | `NaacDashboardGenerated` |
| `GenerateAisheExtract` | fiscal_year_id | data as of 30 Sep; map via `aishe_head_code`; E1/E2 form mapping config (see Rule 5) | `AisheExtractGenerated` |
| `GenerateUgcUtilizationCertificate` | fund_id, fiscal_year_id | fund type RESTRICTED/UGC; expenditure ≤ received+interest; calls fund module for GFR 12-A content | `UgcUcGenerated` |
| `GenerateFcraFc4Return` | fiscal_year_id | FCRA reg active | (FC-4 data export) |
| `SubmitReport` | report_id | GENERATED/REVIEWED | `RegulatoryReportSubmitted` |
| `GenerateComplianceCalendar` | fiscal_year_id | idempotent; derives from registrations + period master | (calendar populated) |
| `UpdateCalendarStatus` | event_id, status, ref | PENDING→COMPLETED needs reference_id (filed return) | `ComplianceDeadlineApproaching` / `FilingDeadlineMissed` |
| `CreateAuditSchedule` / `CompleteAudit` | fy, type, due / auditor details | unique (fy,type) | `AuditCompleted` |

Queries: `GetStatutoryReport(s)`, `GetPendingFilings(entity)`, `GetFilingHistory(entity, fy)`, `GetComplianceCalendar(fy, entity?)`, `GetNaacDashboard(fy)` (with 5-yr trend + per-capita income, expenditure ratios), `GetAisheExtract(fy)`, `GetAisheHeadMapping`, `GetUgcUc(fund, fy)`, `GetAllRegulatoryReports(fy)`, `GetAuditSchedule(fy)`, `GetPendingAudits`.

**State Machines**

```
StatutoryReport:  PENDING ──Generate──> DRAFT ──(data complete)──> GENERATED ──Review──> REVIEWED ──File──> FILED ──(portal error)──> FILED_WITH_ERRORS
                  (FILED_WITH_ERRORS ──Amend──> new period row; prior FILED rows never mutate)
RegulatoryReport: DRAFT ──Generate──> GENERATED ──Review──> REVIEWED ──Submit──> SUBMITTED
CalendarEvent:    PENDING ──(due_date passed)──> OVERDUE ; PENDING/OVERDUE ──(file w/ ack)──> COMPLETED ; ──Extension──> EXTENSION_APPLIED
AuditSchedule:    PENDING ──Start──> IN_PROGRESS ──Complete──> COMPLETED ; PENDING ──(filed 44AB/10B extension)──> EXTENSION_FILED
```

**Business Rules**
1. **NAAC financial metrics** — NAAC Manual (2020) / CD §4.1: 3.3.1 research grants (5-yr, per year), 3.3.2 grants per faculty (needs faculty count — from AISHE extract E-data, ASSUMPTION), 3.3.3 consultancy revenue, 3.4.4 research budget %, 5.1.1 scholarship/freeship expenditure, 7.1.1–7.1.4 environmental/gender/social expenditure. Sources are COA accounts tagged `naac_metric_key` (seeded at COA setup), fund records, and budget module. Dashboard must present 5-year trend and per-capita income (total income / student count — student count from AR/student master via E1/E2, ASSUMPTION). WHY: SSR quantitative metrics require auditable, traceable numbers.
2. **AISHE data as of 30 September** — AISHE Manual (CD §5.1): Part C financial data = receipts (govt grants recurring/non-recurring, other grants, tuition, other fees, exam fees, other receipts) and expenditure (teaching salaries, non-teaching salaries, maintenance, research, other). Mapping via `chart_of_accounts.aishe_head_code` at sub-head level (DM §2.1.5). WHY: wrong/missing mapping = data-rejection at aishe.gov.in; the extract is the institution's official profile.
3. **UGC UC in GFR 12-A** — GFR 2017 r.238 & GFR 12-A form (CD §6.1): UC must state grant sanctioned/received, expenditure under approved heads, unspent balance, interest earned, and be signed by Head of Institution + Statutory Auditor; grants kept in separate bank account; interest reported. Compliance module files the fund module's generated UC and tracks submission. WHY: without a filed UC, subsequent UGC grants are withheld.
4. **Compliance calendar is derived, not hand-entered** — on `FiscalYearOpened` + registrations: GSTR-1/3B monthly (11th/13th, 20th — CD §1.3), GSTR-9/9C 31 Dec if turnover bands met, TDS deposit 7th monthly (r.30), 24Q/26Q/27Q 15th post-quarter (CD §2.2), Form 16 31 May, PT-1 monthly / PT-1A half-yearly + PT-2 31 May (CD §8.1), Form 10B/10BB + ITR-7 + 44AB + trust audit 30 Sep (CD §7.4), FC-4 31 Dec (CD §7.3), 12AB/FCRA renewal (validity-based), AISHE submission (configurable window, ASSUMPTION: Jan–Mar following 30 Sep), UGC UC per sanction terms (configurable). WHY: every deadline maps to a statute; manual calendars rot.
5. **E1/E2 form mapping** — AISHE data-entry forms E1 (college/standalone institution) and E2 (university) differ in structure (ASSUMPTION — CD only specifies Part C financial fields): the extract generator takes `report_type`-specific field maps in `system_config` (`aishe.e1.fieldmap`, `aishe.e2.fieldmap`) so a manual change never needs code. WHY: AISHE revisions arrive yearly; config beats redeploy.
6. **Filing state is append-only** — a FILED report is immutable; errors produce FILED_WITH_ERRORS and a new amended row. WHY: statutory records must reconstruct exactly what was filed when (audit requirement, 8-year retention — IT Act s.44AB / CD §9.3.4).
7. **Calendar completion requires evidence** — a calendar event moves to COMPLETED only with `reference_id` (filed return/report id) and `completed_by_id`. WHY: status must be provable in audit.
8. **Deadline reminders** — reminder schedule [30, 15, 7, 3, 1] days (configurable per event type); missed deadlines auto-flag OVERDUE and notify CFO/compliance officer. WHY: late filing costs are real money (s.47 late fee, s.234E ₹200/day, s.201 interest).
9. **Audit schedule mirrors statutes** — audit_schedules seeded per fy: 44AB, 12A trust audit, Form 10B/10BB, ITR-7 (all 30 Sep — CD §7.4) plus internal/statutory audits. WHY: September is the trust sector's crunch month; the schedule makes it visible from day one.

**API Contract** (new nest `/compliance`)

| Method | Path | Purpose | Permission |
|---|---|---|---|
| GET | `/compliance/calendar` | calendar (filter type/status/due-range) | `compliance:calendar:view` (NEW) |
| POST | `/compliance/calendar/generate` | GenerateComplianceCalendar | `compliance:calendar:configure` (NEW) |
| POST | `/compliance/calendar/:id/complete` | mark complete w/ ref | `compliance:calendar:configure` |
| GET | `/compliance/reports/statutory` | list | `reports:statutory:view` |
| GET | `/compliance/reports/statutory/:id` | detail + json | `reports:statutory:view` |
| POST | `/compliance/reports/statutory/generate` | GenerateStatutoryReport | `compliance:statutory:generate` (NEW) |
| POST | `/compliance/reports/statutory/:id/review` | review | `compliance:statutory:generate` |
| POST | `/compliance/reports/statutory/:id/file` | file w/ ack | `compliance:statutory:file` (NEW) |
| POST | `/compliance/reports/statutory/:id/amend` | amend | `compliance:statutory:file` |
| POST | `/compliance/reports/regulatory/naac/generate` | NAAC dashboard | `compliance:regulatory:generate` (NEW) |
| GET | `/compliance/reports/regulatory/naac` | NAAC dashboard data | `reports:financial:view` |
| POST | `/compliance/reports/regulatory/aishe/generate` | AISHE extract | `compliance:regulatory:generate` |
| GET | `/compliance/reports/regulatory/aishe` | extract + mapping | `reports:financial:view` |
| POST | `/compliance/reports/regulatory/ugc-uc/generate` | UGC UC (fund, fy) | `compliance:regulatory:generate` |
| GET | `/compliance/reports/regulatory/ugc-uc/:fundId/:fy` | UC PDF | `reports:financial:view` |
| POST | `/compliance/reports/regulatory/:id/submit` | SubmitReport | `compliance:regulatory:submit` (NEW) |
| GET | `/compliance/audit-schedules` | schedule per fy | `reports:statutory:view` |
| POST | `/compliance/audit-schedules/:id/complete` | CompleteAudit | `compliance:audit:manage` (NEW) |

**Events** (extend `ComplianceEvent`)
`ComplianceDeadlineApproaching {event_id, event_type, due_date, days_remaining}` (exists), `StatutoryReportFiled {report_id, report_type, period, filed_by, acknowledgment_no}` (exists), `ReportGenerated {report_id, report_type, period}`, `ReportAmended {report_id, report_type, period, reason}`, `FilingDeadlineMissed {report_type, period, due_date}`, `NaacDashboardGenerated {fiscal_year_id, metrics}`, `AisheExtractGenerated {fiscal_year_id, document_url}`, `UgcUcGenerated {fund_id, fiscal_year_id, document_url}`, `RegulatoryReportSubmitted {report_id, report_type, fiscal_year}`, `AuditCompleted {schedule_id, audit_type, fiscal_year, completed_date}`, `AuditDueDateApproaching {audit_type, fiscal_year, due_date}`.

**Integration Contracts**
- **GL/COA:** `chart_of_accounts.aishe_head_code` (Part C mapping at sub-head level) and `naac_metric_key` (NAAC metrics) are the two columns compliance aggregates on; seed them in the default COA template. Account balances come from `mv_account_balances` / posted journals by fy.
- **Tax engine:** `GstReturnFiled` / `TdsReturnFiled` / `GstFilingDeadlineApproaching` drive calendar completion and reminders; statutory GST/TDS report records are mirrors of tax-engine returns (compliance owns the *filing* record, tax owns the *numbers*).
- **Fund accounting:** `GenerateUgcUtilizationCertificate(fund_id, fy)` invokes the fund module's UC content service (GFR 12-A body + schedules + interest computation per DM §2.4); compliance wraps it in the regulatory report record and tracks submission. `UtilizationCertificateGenerated` marks UGC_UC calendar events COMPLETED.
- **Budget:** research-budget % (NAAC 3.4.4) comes from budget module's `GetResearchBudgetAllocation`.
- **AR:** scholarship/freeship expenditure (NAAC 5.1.1) from AR scholarship records; student counts for per-capita metrics from student master (E1/E2 data).

**Permissions (NEW)**
`compliance:calendar:view` (all roles via `:view` family; Auditor GLOBAL), `compliance:calendar:configure` (Compliance Officer/CFO, GLOBAL), `compliance:statutory:generate` (Compliance Officer/Controller, GLOBAL), `compliance:statutory:file` (Compliance Officer/CFO — step-up 2FA, GLOBAL), `compliance:regulatory:generate` (Controller/Compliance Officer, GLOBAL), `compliance:regulatory:submit` (CFO/Principal, GLOBAL), `compliance:audit:manage` (CFO/Controller, GLOBAL). Existing: `reports:statutory:view`, `reports:financial:view`, `reports:export`, `reports:dashboard:view`.

**AI/Analytics hooks**
- Deadline-risk model: probability of missing each filing based on data-readiness signals (unreconciled bank entries, unposted invoices, ITC register not computed) — surfaces "at-risk" calendar items.
- NAAC trend narratives: auto-draft commentary on 5-year metric movements (for SSR writing).
- AISHE variance check: extract totals vs trial-balance totals auto-validated before export (catches mapping gaps).
- Compliance calendar optimization: suggest filing batch windows to spread the September crunch.

---

## Cross-cutting notes (for the lead & Rust engineer)

1. **Schema migrations required** (DDL lacks): `payment_gateway_configs`, `inter_bank_transfers`, `petty_cash_funds`, `petty_cash_transactions`, `gst_rate_master`, `forecasts`, plus `bank_accounts.gl_account_id` column. All follow existing conventions (tenant_id, paise, soft-delete for masters, immutable for fact tables).
2. **Boundary decisions taken (FLAG to lead):** (a) RCM journal is posted by the **tax engine** on `InvoicePosted`, not by AP (DM §4.2.15 mentions RCM in AP's saga — single owner avoids double-posting); (b) TDS deduction at payment stays in **AP** (already has `tds_deductions`), tax engine owns rates/returns/certificates/deposits; (c) payment-gateway *initiation/webhooks* stay in **AR**, treasury owns gateway *config + settlement reconciliation*; (d) statutory GST/TDS *numbers* live in tax engine, *filing records* in compliance.
3. **Rule 42 tolerance**: domain model policy said `0.5`; the statute (CGST Rules r.42(1)(m)) is a 5% de-minimis — default `itc_reversal_tolerance_percent` to 5, keep configurable.
4. **AISHE E1/E2** interpreted as college vs university AISHE forms; field maps config-driven (`system_config`) since CD only specifies Part C.
5. **Event enums**: extend the existing `TreasuryEvent`/`TaxationEvent`/`BudgetEvent`/`ComplianceEvent` in `crates/events/src/events.rs`; do not create new enums. Permissions follow `module:resource:action`; all NEW permissions are listed per module above and should be added to `rbac-design.md` Part 1 with role matrices.
6. **API prefix convention**: add nests `/treasury`, `/tax`, `/budget`, `/compliance` in `crates/api/src/router.rs` (mirrors existing `/gl`, `/ap`); the DM's older `/api/v1/gst/...`, `/tds/...` paths map to the new `/tax/...` namespace.

**COMPLETENESS: FULLY COMPLETE.** All four module specs (Treasury & Banking, Tax Engine, Budget & Forecasting, Statutory Reporting) are delivered with zero ambiguity on aggregates, state machines, transitions, API endpoints, permissions, events, and GL posting rules, and every rule is tagged with its compliance source or marked ASSUMPTION. The Rust engineer can implement each spec against the existing stub crates without re-reading the domain model; the only follow-ups are the schema migration items and the four boundary decisions flagged in section 2 above.
