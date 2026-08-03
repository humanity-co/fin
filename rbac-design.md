# SutraERP RBAC Permission System — Design

**Conventions:** Permission = `module:resource:action`. Action hierarchy per resource: `view ⊂ create/update ⊂ approve ⊂ delete`; `configure ⊃ all`; `export` standalone. Read-only roles get only `:view`. All roles are tenant-scoped; every grant carries a **scope** (GLOBAL / CAMPUS / DEPARTMENT / SELF) — a user can hold the same role in multiple scopes (e.g., Accountant@Campus-A + Accountant@Campus-B).

## Part 1 — Permission List (41)

| Module | Permissions |
|---|---|
| **gl** | `gl:coa:view`, `gl:coa:configure`, `gl:journal:view`, `gl:journal:create`, `gl:journal:approve`, `gl:journal:post` (incl. reversal), `gl:period_close:execute` |
| **ar** | `ar:fee_structure:view`, `ar:fee_structure:configure`, `ar:fee_collection:collect`, `ar:fee_collection:view`, `ar:receipt:cancel`, `ar:concession:create`, `ar:scholarship:create`, `ar:scholarship:approve`, `ar:refund:create`, `ar:refund:approve` |
| **ap** | `ap:vendor:create`, `ap:po:create`, `ap:po:approve`, `ap:grn:create`, `ap:invoice:create`, `ap:invoice:approve`, `ap:payment:create`, `ap:payment:approve`, `ap:reimbursement:create`, `ap:reimbursement:approve` |
| **treasury** | `treasury:bank_account:view`, `treasury:bank_account:configure`, `treasury:reconciliation:perform`, `treasury:reconciliation:approve`, `treasury:transfer:create`, `treasury:transfer:approve` |
| **tax** | `tax:return:view`, `tax:gst_return:prepare`, `tax:gst_return:file`, `tax:tds:deduct`, `tax:config:configure` |
| **budget** | `budget:budget:view`, `budget:budget:create`, `budget:budget:approve`, `budget:revision:create`, `budget:encumbrance:view` |
| **reports** | `reports:financial:view`, `reports:statutory:view`, `reports:dashboard:view`, `reports:export` |
| **workflow** | `workflow:approval_queue:view`, `workflow:approval:action`, `workflow:rule:configure`, `workflow:exception:handle` |
| **admin** | `admin:user:manage`, `admin:role:manage`, `admin:config:manage`, `admin:audit_log:view`, `admin:entity:manage` |

## Part 2 — Role Groups → Permissions

| Group | Who | Key Permissions (scope in brackets) |
|---|---|---|
| **Trustee/Mgmt** | Chairman, CEO, Board | All `:view` + `reports:financial/statutory:view`, `reports:export` (GLOBAL). No create/edit. |
| **CFO** | CFO | All finance CRUD + `*:approve`; `gl:coa:configure`, `budget:budget:create+approve`, `treasury:transfer:approve`, `tax:config:configure`, `reports:export`, `workflow:rule:configure` (GLOBAL). |
| **Finance Controller** | Finance Officer, Accounts Officer | All finance `create/update/post/perform`, `reports:financial:view`; **no** `:configure`, no final `:approve` beyond departmental (GLOBAL/CAMPUS). |
| **Accountant** | Accountant | `gl:journal:create/post`, `ar:fee_collection:collect`, `ap:vendor/po/grn/invoice:create`, `ap:payment:create`, `treasury:reconciliation:perform`, `tax:tds:deduct`, `budget:budget:view`; no `:approve`, no `:configure` (CAMPUS). |
| **Cashier** | Cashier, Billing | `ar:fee_collection:collect` + `ar:fee_collection:view` (CAMPUS). |
| **Principal** | Principal, Director | All `:view`; `budget:budget:approve`, `ap:po:approve`, `ar:scholarship:approve`, `ar:refund:approve`, `ap:reimbursement:approve` (CAMPUS). |
| **Registrar** | Registrar | AR student-facing only: `ar:fee_structure:view`, `ar:fee_collection:view`, `ar:concession:create`, `ar:scholarship:create`, `reports:financial:view` (GLOBAL/CAMPUS). |
| **HOD** | HOD | `budget:budget:view` + `budget:encumbrance:view` (DEPARTMENT); `ap:po:create` (dept). |
| **Faculty** | Faculty | `ap:reimbursement:create` (SELF only), grants view of own grant utilization (SELF). |
| **Student/Parent** | Student, Parent | `ar:fee_collection:view` (SELF), `ar:scholarship:view` (SELF), receipts/payments history (SELF). |
| **Auditor** | Internal, External, Tax | **ALL** `:view` + `reports:statutory:view`, `reports:export`, `gl:ledger:view`, `admin:audit_log:view`; zero create/approve/configure (GLOBAL). |
| **Procurement** | Purchase Officer | `ap:vendor:create`, `ap:po:create`, `ap:grn:create`, `ap:invoice:create`; **no** `ap:payment:*`, no approve (CAMPUS). |
| **Compliance** | Compliance Officer | `tax:return:view`, `tax:gst_return:prepare`, `reports:statutory:view`, `reports:financial:view`, `budget:budget:view` (GLOBAL). |
| **IT Admin** | ERP/Sys Admin | `admin:user:manage`, `admin:role:manage`, `admin:config:manage`, `admin:audit_log:view`, `admin:entity:manage`, `workflow:rule:configure` (GLOBAL). **Zero** finance data permissions — cannot be granted finance roles (SoD rule). |

**Enforcement rules:**
1. **Separation of duties** — creator ≠ approver enforced at workflow layer
2. Every request → permission check + scope filter (user's scope_ids injected into queries)
3. System roles (Cashier, Auditor) immutable — grants only via IT Admin, always audit-logged
4. Sensitive actions (post, file, approve) require step-up (2FA/OTP)

## Part 3 — DB Tables

```sql
CREATE TABLE permissions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code         TEXT NOT NULL UNIQUE,
    module       TEXT NOT NULL,
    resource     TEXT NOT NULL,
    action       TEXT NOT NULL,
    description  TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE roles (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID NOT NULL REFERENCES tenants(id),
    code         TEXT NOT NULL,
    name         TEXT NOT NULL,
    is_system    BOOLEAN NOT NULL DEFAULT FALSE,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by   UUID,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by   UUID,
    UNIQUE (tenant_id, code)
);

CREATE TABLE role_permissions (
    role_id         UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id   UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    granted_by      UUID,
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE user_roles (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID NOT NULL REFERENCES tenants(id),
    user_id      UUID NOT NULL REFERENCES app_users(id),
    role_id      UUID NOT NULL REFERENCES roles(id),
    scope_type   TEXT NOT NULL CHECK (scope_type IN ('GLOBAL','CAMPUS','DEPARTMENT','SELF')),
    scope_id     UUID,
    valid_from   TIMESTAMPTZ NOT NULL DEFAULT now(),
    valid_to     TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by   UUID,
    UNIQUE (user_id, role_id, scope_type, scope_id, valid_from)
);
```

## Part 4 — Dashboard KPIs per Role

- **CFO**: Cash position + bank balances vs. projections, receivable DSO & fee collection % vs. target, budget utilization by fund, compliance calendar (GST/TDS due-in-days)
- **Principal**: Budget utilization % vs. approved, pending approval queue (PO/scholarship/refund) with age, department-wise expense vs. allocation
- **Accountant**: Today's collections vs. expectations, pending vendor invoices & PO-to-invoice gaps, unreconciled bank entries count, journal entries awaiting posting
- **HOD**: Department budget remaining vs. encumbrance, pending POs, grant/consumable utilization vs. sanction
- **Student**: Own fee balance, next installment due date/amount, scholarship/concession status, payment & receipt history
- **Auditor**: Exception/flag count (duplicate payments, stale receivables), unreconciled accounts, trial-balance variance, sample completion vs. audit plan
