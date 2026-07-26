# SutraERP Finance Schema — INDEX.md

## Schema Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        SYSTEM BOUNDED CONTEXTS                      │
│  ┌──────────┐  ┌──────────────┐  ┌────────────┐  ┌──────────────┐  │
│  │ Tenants  │  │  System      │  │ Event      │  │  Saga        │  │
│  │ & Config │  │  Config      │  │ Outbox     │  │  State       │  │
│  └──────────┘  └──────────────┘  └────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                              │
    ┌─────────────────────────┼────────────────────────────┐
    │                         │                            │
    ▼                         ▼                            ▼
┌────────────────────────────────────────────────────────────────────┐
│                  FINANCIAL FOUNDATION                               │
│  ┌───────────┐  ┌──────────────┐  ┌───────────┐  ┌──────────┐     │
│  │ Entities  │  │ Chart of     │  │Accounting │  │  Cost    │     │
│  │(Multi-    │  │ Accounts     │  │ Periods   │  │ Centers  │     │
│  │ Campus)   │  │ (5-level)    │  │(13 periods)│  │          │     │
│  └───────────┘  └──────────────┘  └───────────┘  └──────────┘     │
│  ┌───────────┐  ┌──────────────┐                                   │
│  │  Funds    │  │Fund Budget   │                                   │
│  │(Grants)   │  │ Heads        │                                   │
│  └───────────┘  └──────────────┘                                   │
└────────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌─────────────────┐  ┌─────────────────┐  ┌──────────────────────┐
│ GENERAL LEDGER  │  │ ACCOUNTS        │  │ ACCOUNTS             │
│ (Double-Entry)  │  │ RECEIVABLE      │  │ PAYABLE              │
├─────────────────┤  ├─────────────────┤  ├──────────────────────┤
│ Journal Entries │  │ Fee Structures  │  │ Vendors (PAN/GSTIN)  │
│ Journal Lines   │  │ Fee Heads       │  │ Purchase Requisitions│
│ ┌─────────────┐ │  │ Student Fee     │  │ Purchase Orders      │
│ │ IMMUTABLE   │ │  │   Accounts      │  │ Goods Receipt Notes  │
│ │ INSERT-only │ │  │ Fee Installments│  │ Vendor Invoices      │
│ └─────────────┘ │  │ Payment Receipts│  │   (3-way matching)   │
│ Partitioned by  │  │ Fee Transactions│  │ Vendor Payments      │
│ fiscal year     │  │ Concessions     │  │ Employee Reimb.      │
└─────────────────┘  │ Scholarships    │  └──────────────────────┘
                     │ Refunds         │
                     │ Credit Notes    │
                     │ Security Dep.   │
                     └─────────────────┘

┌─────────────────┐  ┌─────────────────┐  ┌──────────────────────┐
│ TREASURY &      │  │ TAXATION        │  │ BUDGET &             │
│ BANKING         │  │                 │  │ ENCUMBRANCE          │
├─────────────────┤  ├─────────────────┤  ├──────────────────────┤
│ Bank Accounts   │  │ GST Registratns │  │ Budgets              │
│ Bank Signatories│  │ GST Returns     │  │ Budget Lines         │
│ Bank Reconcil.  │  │ ITC Register    │  │ Budget Revisions     │
│ Bank Statement  │  │ TDS Deductions  │  │ Encumbrances         │
│   Lines         │  │ TDS Returns     │  │                      │
│ Bank Transactns │  │ Trust Exemptions│  │                      │
└─────────────────┘  │ Income Applicat │  └──────────────────────┘
                     │ FCRA Register   │
                     └─────────────────┘

┌─────────────────┐  ┌─────────────────┐  ┌──────────────────────┐
│ FIXED ASSETS &  │  │ COMPLIANCE &    │  │ WORKFLOW &           │
│ INVENTORY       │  │ REPORTING       │  │ DOCUMENTS            │
├─────────────────┤  ├─────────────────┤  ├──────────────────────┤
│ Fixed Assets    │  │ Compliance      │  │ Approval Workflows   │
│ Asset Deprec.   │  │   Calendar      │  │ Approval Levels      │
│ Asset Disposals │  │ Audit Log       │  │ Approval Requests    │
│ Inventory Items │  │   (INSERT-only) │  │ Approval Decisions   │
│ Inventory Trans │  │ Audit Schedule  │  │ Documents            │
│   (INSERT-only) │  │ Statutory       │  │                      │
│                 │  │   Reports       │  │                      │
│                 │  │ Regulatory      │  │                      │
│                 │  │   Reports       │  │                      │
└─────────────────┘  └─────────────────┘  └──────────────────────┘
```

## Table Listing by Bounded Context

### System (2 tables)
| Table | Type | Description |
|-------|------|-------------|
| `tenants` | Master | Root multi-tenancy table |
| `tenant_configs` | Config | Business rules, policies, rates |

### Financial Foundation (7 tables)
| Table | Type | Description |
|-------|------|-------------|
| `entities` | Master | Multi-campus/institute setup |
| `chart_of_accounts` | Master | 5-level hierarchical COA |
| `fiscal_years` | Master | April-March fiscal years |
| `accounting_periods` | Transactional | 12-13 monthly periods |
| `cost_centers` | Master | Department/campus/project dimensions |
| `funds` | Master | Grant, endowment, FCRA, scholarship funds |
| `fund_budget_heads` | Master | Approved budget heads per fund |

### General Ledger (2 tables + 2 MV)
| Table | Type | Description |
|-------|------|-------------|
| `journal_entries` | ★ IMMUTABLE ★ | Double-entry journal header (partitioned) |
| `journal_entry_lines` | ★ IMMUTABLE ★ | Debit/credit lines (partitioned) |
| `mv_account_balances` | Materialized View | CQRS projection for fast trial balance |
| `mv_fee_outstanding` | Materialized View | CQRS projection for fee dashboard |

### Accounts Receivable (16 tables)
| Table | Type | Description |
|-------|------|-------------|
| `fee_heads` | Master | Catalog of fee types |
| `fee_structures` | Master | Fee structure definitions |
| `fee_structure_lines` | Transactional | Fee head amounts per structure |
| `installment_plans` | Master | Fee installment plans |
| `student_fee_accounts` | Transactional | Per-student fee ledger |
| `fee_installments` | Transactional | Individual installment tracking |
| `fee_transactions` | ★ IMMUTABLE ★ | All fee-related transactions (partitioned) |
| `payment_receipts` | Transactional | Student payment receipts |
| `payment_allocations` | Transactional | Receipt allocation to installments |
| `payment_gateway_transactions` | Transactional | Gateway transaction records |
| `concessions` | Transactional | Student fee waivers |
| `scholarship_schemes` | Master | Scholarship scheme definitions |
| `student_scholarships` | Transactional | Per-student scholarship grants |
| `refunds` | Transactional | Refund requests and processing |
| `credit_notes` | Transactional | Credit notes for future adjustments |
| `security_deposits` | Transactional | Caution/hostel/lab deposits |

### Accounts Payable (14 tables)
| Table | Type | Description |
|-------|------|-------------|
| `vendors` | Master | Vendor master with PAN/GSTIN |
| `section_197_certificates` | Master | Lower TDS deduction certificates |
| `vendor_bank_accounts` | Master | Vendor bank details (encrypted) |
| `purchase_requisitions` | Transactional | Internal purchase requests |
| `purchase_requisition_lines` | Transactional | PR line items |
| `purchase_orders` | Transactional | Purchase orders |
| `purchase_order_lines` | Transactional | PO line items |
| `goods_receipt_notes` | Transactional | Goods receipt records |
| `goods_receipt_note_lines` | Transactional | GRN line items |
| `vendor_invoices` | Transactional | Purchase invoices (3-way match) |
| `vendor_invoice_lines` | Transactional | Invoice line items |
| `vendor_payments` | Transactional | Vendor payments with TDS |
| `vendor_payment_allocations` | Transactional | Payment-to-invoice allocation |
| `employee_reimbursements` | Transactional | Employee expense claims |

### Treasury & Banking (5 tables)
| Table | Type | Description |
|-------|------|-------------|
| `bank_accounts` | Master | Bank account register |
| `bank_signatories` | Master | Authorized signatories |
| `bank_reconciliations` | Transactional | BRS state machine |
| `bank_statement_lines` | Transactional | Statement lines for reconciliation |
| `bank_transactions` | ★ IMMUTABLE ★ | Bank transaction register |

### Taxation (12 tables)
| Table | Type | Description |
|-------|------|-------------|
| `gst_registrations` | Master | Per-entity GSTIN |
| `gst_returns` | Transactional | GSTR-1, 3B, 9, 9C |
| `gst_return_lines` | Transactional | Return section lines |
| `itc_register` | Transactional | ITC register per period |
| `itc_register_lines` | Transactional | Invoice-level ITC tracking |
| `tds_deductions` | Transactional | Per-payment TDS |
| `tds_sections` | Master | TDS section rates/thresholds |
| `tds_returns` | Transactional | Form 24Q/26Q/27Q |
| `tds_return_details` | Transactional | Individual deduction records |
| `trust_exemptions` | Master | 10(23C)/11/12AB registration |
| `income_applications` | Transactional | 85% rule tracking |
| `fcra_registrations` | Master | FCRA registration & compliance |

### Budget & Encumbrance (5 tables)
| Table | Type | Description |
|-------|------|-------------|
| `budgets` | Transactional | Department/project/grant budgets |
| `budget_lines` | Transactional | Line items |
| `budget_revisions` | Transactional | Revision history |
| `encumbrances` | Transactional | Commitments against budget |

### Fixed Assets & Inventory (5 tables)
| Table | Type | Description |
|-------|------|-------------|
| `fixed_assets` | Master | Asset register |
| `asset_depreciation` | Transactional | Depreciation schedule |
| `asset_disposals` | Transactional | Disposal records |
| `inventory_items` | Master | Item master |
| `inventory_transactions` | ★ IMMUTABLE ★ | Stock movement register |

### Compliance & Workflow (11 tables)
| Table | Type | Description |
|-------|------|-------------|
| `compliance_calendar` | Transactional | All filing deadlines |
| `audit_log` | ★ IMMUTABLE ★ | Universal audit trail (partitioned monthly) |
| `audit_schedules` | Transactional | Audit due dates |
| `approval_workflows` | Config | Workflow definitions |
| `approval_levels` | Config | Level configuration |
| `approval_requests` | Transactional | Active approval requests |
| `approval_decisions` | Transactional | Individual decisions |
| `documents` | Master | Document metadata |
| `statutory_reports` | Transactional | GST/TDS/PT returns |
| `regulatory_reports` | Transactional | NAAC/AISHE/UGC reports |

### System/Event Infrastructure (3 tables)
| Table | Type | Description |
|-------|------|-------------|
| `event_outbox` | Infrastructure | Transactional outbox |
| `saga_state` | Infrastructure | Saga orchestration |
| `system_config` | Config | Global/tenant configuration |

### Lookup Tables (10 tables)
`account_types`, `gst_classifications`, `itc_eligibilities`, `journal_types`,
`journal_statuses`, `payment_modes`, `receipt_statuses`, `vendor_types`,
`fee_types`, `fund_types`, `entity_types`, `student_categories`,
`cost_center_types`

**Total: ~83 tables (data) + 10 lookup tables + 2 materialized views**

---

## Key Design Decisions

### 1. Money as Paise (BIGINT)
All monetary values are stored as `BIGINT` representing the smallest unit of INR (1 paise = 1/100 rupee). This avoids floating-point rounding errors and is consistent with how Indian financial systems work (rupees and paise). The `paise` and `paise_nullable` custom domains enforce non-negative constraint at the database level.

**Trade-off:** Application layer must convert to/from display format (₹1,234.56 ↔ 123456).
**Rationale:** Absolute precision for financial calculations > convenience of decimal types.

### 2. Lookup Tables Over ENUMs
Configurable value sets (fee types, fund types, etc.) use lookup tables rather than native PostgreSQL ENUMs. This allows adding new values without schema migrations.

**Exception:** Simple CHECK constraints are used for status fields that are internal state machine values unlikely to change (e.g., `'DRAFT', 'POSTED', 'REVERSED', 'CANCELLED'` for journal status).

### 3. Partitioning Strategy for High-Volume Tables
Three tables are partitioned:
- **`journal_entries` & `journal_entry_lines`**: Range-partitioned by fiscal year (`posting_date`). Each fiscal year (April–March) gets its own partition. Default partition for future dates.
- **`fee_transactions`**: Range-partitioned by fiscal year (`transaction_date`). Same strategy as journals.
- **`audit_log`**: Range-partitioned monthly for very high write volume. Monthly partitions provide partition pruning for date-scoped queries and simplified retention (drop old partitions after 8 years).

### 4. Immutable Financial Records
Tables marked ★IMMUTABLE★ are INSERT-only:
- **`journal_entries` / `journal_entry_lines`**: Once POSTED, no UPDATE/DELETE. Corrections done via reversing entries.
- **`fee_transactions`**: Once recorded, never modified. Corrections via adjustment transactions.
- **`audit_log`**: Append-only. Never modified under any circumstances.
- **`bank_transactions`**: Append-only bank register.
- **`inventory_transactions`**: Append-only stock movement register.

These tables have `version` (integer) for concurrency control during their mutable lifecycle (e.g., journal transitions from DRAFT→POSTED). Once in terminal state, version is irrelevant.

### 5. CQRS Materialized Views
Two materialized views provide pre-computed read models:
- **`mv_account_balances`**: Trial balance by account and period. Refreshable on-demand or via cron.
- **`mv_fee_outstanding`**: Aggregated fee outstanding by student and academic year for dashboards.

### 6. Soft Deletes
Applied to all reference/master tables (`vendors`, `chart_of_accounts`, `entities`, `fee_heads`, `cost_centers`, etc.) using `deleted_at` and `deleted_by` columns.

**Not applied to:**
- Financial fact tables (journal entries, fee transactions, etc.) — these are immutable
- Audit log — append-only
- Event outbox — autonomous lifecycle
- Saga state — autonomous lifecycle

### 7. Transactional Outbox Pattern
`event_outbox` table stores domain events in the same DB transaction as the aggregate change. A background worker reads from the outbox (filtering `status = 'PENDING'`) and publishes to Redis Streams. Failed events retry up to 5 times before being dead-lettered.

### 8. UUID Primary Keys
All tables use `gen_random_uuid()` (UUID v4) as default. For production with UUID v7 (time-ordered), either install the `pg_uuidv7` extension or generate at the application layer. Time-ordered UUIDs improve B-tree index performance for INSERT-heavy tables.

---

## Performance Considerations

### Hot Tables (Highest Write Throughput)
| Table | Expected Operations | Mitigation |
|-------|-------------------|------------|
| `audit_log` | 10,000+ writes/day per tenant | Monthly partitions, INSERT-only, no UPDATEs |
| `journal_entry_lines` | 5,000+ writes/day | Fiscal year partitions |
| `fee_transactions` | 2,000+ writes/day during fee season | Fiscal year partitions |
| `payment_receipts` | 1,000+ writes/day during fee season | Index on status for pending lookups |
| `event_outbox` | Matches transaction volume | Index on (status, created_at) for publisher |

### Hot Tables (Highest Read Throughput)
| Table | Expected Operations | Mitigation |
|-------|-------------------|------------|
| `chart_of_accounts` | Read on every transaction | Cached in application, active-only partial index |
| `vendors` | Read on every PO/invoice/payment | Active-only partial index, trgm search index |
| `student_fee_accounts` | Read on every fee transaction | Materialized view for outstanding |
| `journal_entry_lines` | Read for GL reports | Materialized view for balance aggregation |

### Recommended Partition Keys
- **`journal_entries` / `journal_entry_lines`**: `posting_date` (range by fiscal year)
- **`fee_transactions`**: `transaction_date` (range by fiscal year)
- **`audit_log`**: `occurred_at` (range by month)
- **`bank_statement_lines`**: `bank_reconciliation_id` (list by reconciliation batch)
- **Other high-volume transaction tables**: Consider partitioning by `tenant_id` if a single tenant has >100M rows

### Index Strategy
- **All foreign keys** are indexed (implicitly via FK constraints or explicitly where not unique)
- **Partial indexes** (`WHERE deleted_at IS NULL`) on soft-delete tables for query performance
- **Trigram indexes** on vendor names and account names for fuzzy search
- **Composite indexes** on common query patterns (e.g., `(tenant_id, entity_id, accounting_period_id)` for journals)

---

## Migration Strategy (Zero-Downtime)

### Phase 1: Baseline (v1.0)
```
CREATE SCHEMA IF NOT EXISTS sutra_finance;
-- Run full DDL in a single transaction
BEGIN;
  -- All CREATE TABLE, CREATE INDEX, etc.
COMMIT;
```

### Phase 2: Data Migration from Legacy Systems
1. Create staging tables in a separate schema
2. Use ETL pipeline (custom or Airbyte) to transform legacy data
3. Validate data integrity (debits = credits, account balance consistency)
4. Bulk INSERT into partitioned tables using `pg_bulkload` or `COPY`
5. Create materialized views after data load

### Phase 3: Schema Evolution (no downtime)
**Strategy:** Expand-only migrations with Dual-Write pattern

1. **Add column**: `ALTER TABLE ... ADD COLUMN ... DEFAULT ...` (PostgreSQL 11+ handles this without table rewrite for non-null defaults)
2. **Rename column**: Add new column → Dual-write → Backfill → Drop old column
3. **Add NOT NULL**: Only after all rows have been backfilled
4. **Change data type**: Add new column with new type → Dual-write → Backfill → Switch reads → Drop old column
5. **Add index**: `CREATE INDEX CONCURRENTLY` — avoids locks
6. **Add constraint**: `ALTER TABLE ... ADD CONSTRAINT ... NOT VALID` → `VALIDATE CONSTRAINT`
7. **Drop index**: `DROP INDEX CONCURRENTLY`
8. **Partition existing table**: Requires careful planning. Options:
   - For small tables (<100M rows): Create partitioned table → INSERT INTO ... SELECT → Rename
   - For large tables: Use pg_partman or create new partitioned table with trigger-based routing for live data, then backfill historically

### Partition Maintenance (Automated)
Use `pg_cron` or application scheduler to:
- Create new partitions before the current one fills
- For `audit_log`: Drop partitions older than 8 years (statutory requirement)
- For fiscal-year partitions: Create next year's partition at year-end
- Refresh materialized views during low-activity windows

### Recommended Tools
- **Flyway** or **Liquibase** for change management
- **pg_partman** for automated partition management
- **pg_cron** for scheduled maintenance (refresh MVs, partition creation)
- **pg_repack** for table bloat recovery without downtime

---

## Non-Rules Coverage

| Rule | Implementation |
|------|---------------|
| Multi-tenant (`tenant_id`) | Every table has `tenant_id UUID NOT NULL` referencing `tenants` |
| Immutable financial records | INSERT-only tables marked ★IMMUTABLE★; no UPDATE/DELETE allowed |
| Audit trail | All tables have created/updated timestamps; `audit_log` captures every mutation |
| Money as BIGINT paise | Custom `paise` domain with CHECK constraint |
| UUID primary keys | All `gen_random_uuid()` default |
| Soft deletes | Reference tables have `deleted_at`/`deleted_by`; fact tables don't |
| No floats | NUMERIC(12,3) for quantities, NUMERIC(5,2) for percentages/rates |
| Version tracking | `entity_version INT DEFAULT 1` on all mutable reference data |
| Double-entry invariant | CHECK constraint: exactly one of debit/credit per line |
| Period lock | Journals reference open accounting periods; application enforces validation |
| CHECK for business rules | Domain-specific constraints (e.g., installment %, quantity > 0) |
| CQRS projections | Materialized views for account balances, fee outstanding |
| Transactional outbox | `event_outbox` table for reliable event publishing |
