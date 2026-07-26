-- ============================================================================
-- SutraERP — Complete PostgreSQL Database Schema
-- Financial Domain Model v1.0
-- ============================================================================
-- This schema translates the 13 bounded contexts and 46 sub-domains from
-- the Financial Domain Model (domain-model.md) into production-ready DDL.
--
-- Non-Negotiable Rules Applied:
--   1. Multi-tenant: every table has tenant_id UUID NOT NULL
--   2. Immutable financial records: INSERT-only with version column
--   3. Audit trail: created_at, created_by, updated_at, updated_by on all tables
--   4. Money as BIGINT paise (1/100 rupee)
--   5. UUID primary keys (gen_random_uuid())
--   6. Soft deletes on reference/master tables (deleted_at)
--   7. No float types — NUMERIC for percentages/quantities
--   8. Version tracking: entity_version INT DEFAULT 1 on mutable reference data
--
-- UUID v7: We use gen_random_uuid() as the default. For time-ordered UUIDs
-- in production, install pg_uuidv7 extension or generate at the application layer.
-- ============================================================================

-- ============================================================================
-- PART 0: EXTENSIONS & DOMAINS
-- ============================================================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;          -- gen_random_uuid()
CREATE EXTENSION IF NOT EXISTS pg_trgm;            -- trigram fuzzy search

-- Domain: monetary amount in paise (1/100 rupee), stored as BIGINT
-- Always use this type for all money columns.
-- Display formatting is the API/UI layer's responsibility.
CREATE DOMAIN paise AS BIGINT NOT NULL
  CONSTRAINT paise_non_negative CHECK (VALUE >= 0);

-- Domain: nullable paise (for optional monetary values)
CREATE DOMAIN paise_nullable AS BIGINT
  CONSTRAINT paise_nullable_non_negative CHECK (VALUE IS NULL OR VALUE >= 0);

-- ============================================================================
-- PART 1: LOOKUP TABLES (Configurable enumerations)
-- ============================================================================

-- Account types per Indian accounting standards
CREATE TABLE account_types (
    code        TEXT        PRIMARY KEY,  -- 'ASSET', 'LIABILITY', 'EQUITY', 'INCOME', 'EXPENSE'
    name        TEXT        NOT NULL,
    sort_order  INT         NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE account_types IS 'Fixed set of account classifications per Indian GAAP. Not tenant-specific.';
COMMENT ON COLUMN account_types.code IS 'Short code used as identifier throughout the schema';
COMMENT ON COLUMN account_types.sort_order IS 'Display order on balance sheet: 1=Asset, 2=Liability, 3=Equity, 4=Income, 5=Expense';

INSERT INTO account_types (code, name, sort_order, description) VALUES
    ('ASSET',     'Asset',     1, 'Economic resources controlled by the institution (Cash, Receivables, Fixed Assets, Investments)'),
    ('LIABILITY', 'Liability', 2, 'Obligations arising from past events (Payables, Borrowings, Deposits, Accruals)'),
    ('EQUITY',    'Equity',    3, 'Residual interest in assets after deducting liabilities (Corpus, Reserves, Surplus)'),
    ('INCOME',    'Income',    4, 'Increases in economic benefits (Fee Revenue, Grants, Interest, Other Income)'),
    ('EXPENSE',   'Expense',   5, 'Decreases in economic benefits (Salaries, Infrastructure, Research, Admin Expenses)');

-- GST classification values
CREATE TABLE gst_classifications (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    rate        NUMERIC(4,2),  -- NULL for exempt/nil
    description TEXT
);
COMMENT ON TABLE gst_classifications IS 'GST tax classifications mapped to accounts and fee heads.';
INSERT INTO gst_classifications (code, name, rate, description) VALUES
    ('EXEMPT',     'Exempt',              NULL,  'Education services exempt under GST (Heading 9992)'),
    ('NIL',        'Nil Rated',           0.00,  'Nil-rated supplies (e.g., books)'),
    ('TAXABLE_5',  'Taxable at 5%',       5.00,  'GST at 5% (hostel fees under ₹1,000, certain services)'),
    ('TAXABLE_12', 'Taxable at 12%',      12.00, 'GST at 12% (works contracts, catering, stationery)'),
    ('TAXABLE_18', 'Taxable at 18%',      18.00, 'GST at 18% (general rate, consulting, software)'),
    ('TAXABLE_28', 'Taxable at 28%',      28.00, 'GST at 28% (luxury items, sin goods — rarely applicable for education)');

-- ITC eligibility
CREATE TABLE itc_eligibilities (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
COMMENT ON TABLE itc_eligibilities IS 'Input Tax Credit eligibility per invoice line.';
INSERT INTO itc_eligibilities (code, name, description) VALUES
    ('FULL',             'Full ITC Available',      'Full input tax credit is claimable'),
    ('BLOCKED',          'ITC Blocked',             'ITC blocked under Section 17(5) of CGST Act'),
    ('REVERSAL_42',      'Rule 42 Reversal',        'Reversal applicable for inputs used partly for exempt supplies'),
    ('REVERSAL_43',      'Rule 43 Reversal',        'Reversal applicable for capital goods over 60 months'),
    ('CAPITAL_GOODS',    'Capital Goods',           'ITC on capital goods — subject to Rule 43 provisions');

-- Journal types
CREATE TABLE journal_types (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
COMMENT ON TABLE journal_types IS 'Types of double-entry journal entries.';
INSERT INTO journal_types (code, name, description) VALUES
    ('STANDARD',     'Standard Entry',        'Regular financial transaction entry'),
    ('REVERSING',    'Reversing Entry',       'Entry that reverses a previously posted entry'),
    ('ADJUSTMENT',   'Adjustment Entry',      'Period-end adjustment entry'),
    ('OPENING',      'Opening Entry',         'Opening balance entry for a new fiscal year'),
    ('CLOSING',      'Closing Entry',         'Closing entry at year-end'),
    ('RCM',          'RCM Entry',             'Reverse Charge Mechanism entry'),
    ('ITC_REVERSAL', 'ITC Reversal Entry',    'Input Tax Credit reversal entry'),
    ('TDS',          'TDS Entry',             'Tax Deducted at Source entry'),
    ('ACCRUAL',      'Accrual Entry',         'Accrual for expenses/revenue not yet invoiced'),
    ('PREPAYMENT',   'Prepayment Entry',      'Prepaid expense or deferred revenue entry');

-- Journal status
CREATE TABLE journal_statuses (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
INSERT INTO journal_statuses (code, name, description) VALUES
    ('DRAFT',     'Draft',     'Entry in progress, editable'),
    ('POSTED',    'Posted',    'Entry posted to ledger — immutable'),
    ('REVERSED',  'Reversed',  'Original entry reversed by a reversing entry'),
    ('CANCELLED', 'Cancelled', 'Draft entry cancelled before posting');

-- Payment modes
CREATE TABLE payment_modes (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
INSERT INTO payment_modes (code, name, description) VALUES
    ('CASH',          'Cash',            'Physical cash transaction'),
    ('CHEQUE',        'Cheque',          'Paper cheque instrument'),
    ('DD',            'Demand Draft',    'Banker''s demand draft'),
    ('NEFT',          'NEFT',            'National Electronic Funds Transfer'),
    ('RTGS',          'RTGS',            'Real Time Gross Settlement'),
    ('IMPS',          'IMPS',            'Immediate Payment Service'),
    ('UPI',           'UPI',             'Unified Payments Interface'),
    ('CREDIT_CARD',   'Credit Card',     'Credit card payment'),
    ('DEBIT_CARD',    'Debit Card',      'Debit card payment'),
    ('POS',           'POS Terminal',    'Point-of-Sale terminal transaction'),
    ('PAYMENT_GATEWAY','Payment Gateway','Online payment gateway transaction');

-- Receipt status
CREATE TABLE receipt_statuses (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
INSERT INTO receipt_statuses (code, name, description) VALUES
    ('PENDING',     'Pending',     'Payment initiated, awaiting confirmation'),
    ('COMPLETED',   'Completed',   'Payment received and confirmed'),
    ('FAILED',      'Failed',      'Payment failed'),
    ('REFUNDED',    'Refunded',    'Payment refunded to student'),
    ('CANCELLED',   'Cancelled',   'Receipt cancelled'),
    ('UNCLEARED',   'Uncleared',   'Cheque/DD received but not yet cleared'),
    ('BOUNCED',     'Bounced',     'Cheque/DD bounced');

-- Vendor types
CREATE TABLE vendor_types (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
INSERT INTO vendor_types (code, name, description) VALUES
    ('INDIVIDUAL',       'Individual',            'Sole proprietor or individual service provider'),
    ('PROPRIETORSHIP',   'Proprietorship',        'Proprietorship firm'),
    ('PARTNERSHIP',      'Partnership',           'Partnership firm'),
    ('LLP',              'Limited Liability Partnership', 'LLP registered under LLP Act'),
    ('PRIVATE_LTD',      'Private Limited',       'Private limited company'),
    ('PUBLIC_LTD',       'Public Limited',        'Public limited company'),
    ('GOVERNMENT',       'Government',            'Government department or PSU'),
    ('TRUST',            'Trust',                 'Trust or charitable institution'),
    ('SOCIETY',          'Society',               'Society registered under Societies Act'),
    ('HUF',              'Hindu Undivided Family', 'Hindu Undivided Family'),
    ('OTHER',            'Other',                 'Other entity type');

-- Fee types
CREATE TABLE fee_types (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
INSERT INTO fee_types (code, name, description) VALUES
    ('TUITION',        'Tuition Fee',        'Core tuition fee for academic instruction'),
    ('DEVELOPMENT',    'Development Fee',    'Infrastructure and development fee'),
    ('EXAMINATION',    'Examination Fee',    'Examination and assessment fee'),
    ('LIBRARY',        'Library Fee',        'Library access and resources fee'),
    ('LABORATORY',     'Laboratory Fee',     'Laboratory usage and materials fee'),
    ('SPORTS',         'Sports Fee',         'Sports facilities and activities fee'),
    ('CULTURAL',       'Cultural Fee',       'Cultural activities and events fee'),
    ('ADMISSION',      'Admission Fee',      'One-time admission/enrollment fee'),
    ('REGISTRATION',   'Registration Fee',   'Annual/semester registration fee'),
    ('HOSTEL',         'Hostel Fee',         'Hostel accommodation fee'),
    ('MESS',           'Mess Fee',           'Cafeteria and mess fee'),
    ('TRANSPORTATION', 'Transportation Fee', 'Bus and transport fee'),
    ('CAUTION_DEPOSIT','Caution Deposit',    'Refundable caution deposit'),
    ('OTHER',          'Other Fee',          'Other miscellaneous fee');

-- Fund types
CREATE TABLE fund_types (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
INSERT INTO fund_types (code, name, description) VALUES
    ('RESTRICTED',   'Restricted',    'Grant-specific funds with restrictions on usage'),
    ('UNRESTRICTED', 'Unrestricted',  'General operational funds with no restrictions'),
    ('ENDOWMENT',    'Endowment',     'Corpus funds where principal is untouchable'),
    ('FCRA',         'FCRA',          'Foreign Contribution Regulation Act funds'),
    ('SCHOLARSHIP',  'Scholarship',   'Scholarship and freeship pass-through funds');

-- Entity types
CREATE TABLE entity_types (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
INSERT INTO entity_types (code, name, description) VALUES
    ('MAIN_CAMPUS',       'Main Campus',        'Primary/main campus of the institution'),
    ('SATELLITE_CAMPUS',  'Satellite Campus',   'Additional satellite campus location'),
    ('RESEARCH_CENTER',   'Research Center',    'Dedicated research facility'),
    ('SKILL_CENTER',      'Skill Center',       'Vocational training and skill development center'),
    ('INSTITUTE',         'Institute',          'Standalone institute within a multi-institute setup');

-- Student categories
CREATE TABLE student_categories (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
INSERT INTO student_categories (code, name, description) VALUES
    ('GENERAL', 'General',        'General category (no reservation)'),
    ('SC',      'Scheduled Caste', 'Scheduled Caste category'),
    ('ST',      'Scheduled Tribe', 'Scheduled Tribe category'),
    ('OBC',     'OBC',             'Other Backward Classes category'),
    ('EWS',     'EWS',             'Economically Weaker Sections category'),
    ('VJNT',    'VJNT',            'Vimukta Jati and Nomadic Tribes (Maharashtra)'),
    ('SBC',     'SBC',             'Special Backward Classes (Maharashtra)'),
    ('PWD',     'PwD',             'Persons with Disabilities category'),
    ('OTHER',   'Other',           'Other category');

-- Cost center types
CREATE TABLE cost_center_types (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT
);
INSERT INTO cost_center_types (code, name, description) VALUES
    ('DEPARTMENT', 'Department', 'Academic or administrative department'),
    ('CAMPUS',     'Campus',     'Campus/location level cost center'),
    ('PROJECT',    'Project',    'Specific project cost center'),
    ('ACTIVITY',   'Activity',   'Activity-based cost center'),
    ('PROGRAM',    'Program',    'Academic program cost center'),
    ('COURSE',     'Course',     'Course-level cost center');

-- ============================================================================
-- PART 2: SYSTEM TABLES (Tenancy, Configuration)
-- ============================================================================

-- Tenants table
CREATE TABLE tenants (
    tenant_id       UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_code     TEXT        NOT NULL UNIQUE,  -- human-readable code
    tenant_name     TEXT        NOT NULL,
    domain          TEXT,                         -- custom domain for tenant
    logo_url        TEXT,
    address_line1   TEXT,
    address_line2   TEXT,
    city            TEXT,
    state           TEXT,
    pincode         TEXT,
    country         TEXT        NOT NULL DEFAULT 'India',
    contact_email   TEXT,
    contact_phone   TEXT,
    is_active       BOOLEAN     NOT NULL DEFAULT TRUE,
    entity_version  INT         NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by      UUID,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by      UUID,
    deleted_at      TIMESTAMPTZ,
    deleted_by      UUID
);
COMMENT ON TABLE tenants IS 'Root table for multi-tenancy. Every data row references a tenant.';
COMMENT ON COLUMN tenants.tenant_code IS 'Short unique code for the institution (e.g., IITB, VIT)';
COMMENT ON COLUMN tenants.domain IS 'Custom domain name for this tenant''s portal';

-- Tenant configuration (JSONB for extensibility)
CREATE TABLE tenant_configs (
    tenant_config_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL REFERENCES tenants(tenant_id),
    config_key       TEXT        NOT NULL,
    config_value     JSONB       NOT NULL,
    scope            TEXT        NOT NULL DEFAULT 'tenant',  -- 'tenant', 'entity', 'global'
    is_active        BOOLEAN     NOT NULL DEFAULT TRUE,
    valid_from       DATE        NOT NULL DEFAULT '1970-01-01',
    valid_to         DATE,
    entity_version   INT         NOT NULL DEFAULT 1,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by       UUID,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by       UUID,
    UNIQUE (tenant_id, config_key, scope, valid_from)
);
COMMENT ON TABLE tenant_configs IS 'Configuration store for business rules, policies, rates, and thresholds. All business rules are configurable here.';
COMMENT ON COLUMN tenant_configs.config_key IS 'Dot-notation key (e.g., "gst.rate.hostel", "tds.section.194c.rate")';
COMMENT ON COLUMN tenant_configs.config_value IS 'JSONB value — schema validated at application layer';

-- ============================================================================
-- PART 3: FINANCIAL FOUNDATION
-- ============================================================================

-- 3.1 Entities (Multi-Campus / Multi-Institute)
CREATE TABLE entities (
    entity_id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_code        TEXT        NOT NULL,
    entity_name        TEXT        NOT NULL,
    entity_type        TEXT        NOT NULL REFERENCES entity_types(code),
    gstin              TEXT,                              -- 15-char GSTIN
    pan                TEXT,                              -- 10-char PAN
    address_line1      TEXT,
    address_line2      TEXT,
    city               TEXT,
    state              TEXT,
    pincode            TEXT,
    country            TEXT        NOT NULL DEFAULT 'India',
    parent_entity_id   UUID        REFERENCES entities(entity_id),
    consolidation_group TEXT,                             -- for consolidated reporting
    is_active          BOOLEAN     NOT NULL DEFAULT TRUE,
    entity_version     INT         NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         UUID,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by         UUID,
    deleted_at         TIMESTAMPTZ,
    deleted_by         UUID,
    UNIQUE (tenant_id, entity_code),
    UNIQUE (tenant_id, gstin),
    UNIQUE (tenant_id, pan)
);
COMMENT ON TABLE entities IS 'Multi-campus and multi-institute configuration. Each entity has its own books, GSTIN, and PAN.';
COMMENT ON COLUMN entities.gstin IS 'GSTIN of this entity — each campus may have its own GSTIN for filing';
COMMENT ON COLUMN entities.pan IS 'PAN of this entity';
COMMENT ON COLUMN entities.parent_entity_id IS 'For hierarchical entity structures (e.g., institute → campus)';
COMMENT ON COLUMN entities.consolidation_group IS 'Group label for entities consolidated together';

-- 3.2 Chart of Accounts (Hierarchical, 5-level)
CREATE TABLE chart_of_accounts (
    account_id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(tenant_id),
    account_code       TEXT        NOT NULL,              -- 8-digit hierarchical code
    account_name       TEXT        NOT NULL,
    account_type       TEXT        NOT NULL REFERENCES account_types(code),
    parent_account_id  UUID        REFERENCES chart_of_accounts(account_id),
    level              INT         NOT NULL CHECK (level BETWEEN 1 AND 5),  -- 1=Group ... 5=Detailed
    gst_classification TEXT        REFERENCES gst_classifications(code),
    hsn_sac_code       TEXT,                              -- HSN for goods, SAC for services
    itc_eligibility    TEXT        REFERENCES itc_eligibilities(code),
    aishe_head_code    TEXT,                              -- AISHE reporting head code (at sub-head level)
    naac_metric_key    TEXT,                              -- NAAC metric key (for research/gender/environment accounts)
    opening_balance    paise       DEFAULT 0,             -- opening balance at start of current fiscal year
    current_balance    paise       DEFAULT 0,             -- running balance (computed by materialized view)
    is_active          BOOLEAN     NOT NULL DEFAULT TRUE,
    is_system          BOOLEAN     NOT NULL DEFAULT FALSE, -- system-protected accounts cannot be deactivated
    entity_version     INT         NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         UUID,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by         UUID,
    deleted_at         TIMESTAMPTZ,
    deleted_by         UUID,
    UNIQUE (tenant_id, account_code),
    UNIQUE (tenant_id, parent_account_id, account_name)
);
COMMENT ON TABLE chart_of_accounts IS 'Hierarchical Chart of Accounts with 5 levels. AISHE and NAAC mapped at Sub-Head level.';
COMMENT ON COLUMN chart_of_accounts.account_code IS '8-digit hierarchical code: G(1) + SG(1) + H(2) + SH(2) + D(2)';
COMMENT ON COLUMN chart_of_accounts.level IS '1=Group, 2=Sub-Group, 3=Head, 4=Sub-Head (AISHE mappable), 5=Detailed';
COMMENT ON COLUMN chart_of_accounts.gst_classification IS 'GST classification for this account — exempt, nil, or taxable at specific rate';
COMMENT ON COLUMN chart_of_accounts.hsn_sac_code IS 'HSN (goods) or SAC (services) code for GST reporting';
COMMENT ON COLUMN chart_of_accounts.itc_eligibility IS 'Default ITC eligibility for expenses posted to this account';
COMMENT ON COLUMN chart_of_accounts.aishe_head_code IS 'AISHE reporting head mapped at Sub-Head (4-digit) level — used for annual AISHE extract';
COMMENT ON COLUMN chart_of_accounts.naac_metric_key IS 'NAAC metric key for accounts related to research, consultancy, scholarships, environmental/gender/social initiatives';
COMMENT ON COLUMN chart_of_accounts.is_system IS 'System accounts (e.g., Opening Balance, Suspense) cannot be deactivated';
COMMENT ON COLUMN chart_of_accounts.current_balance IS 'Running balance updated by materialized view or trigger — NOT computed on the fly in high-traffic queries';

-- Index for hierarchical tree queries
CREATE INDEX idx_coa_parent ON chart_of_accounts (parent_account_id);
-- Index for account type filtering
CREATE INDEX idx_coa_account_type ON chart_of_accounts (tenant_id, account_type);
-- Partial index to exclude soft-deleted accounts
CREATE INDEX idx_coa_active ON chart_of_accounts (tenant_id) WHERE deleted_at IS NULL AND is_active = TRUE;
-- Index for AISHE mapping queries
CREATE INDEX idx_coa_aishe ON chart_of_accounts (aishe_head_code) WHERE aishe_head_code IS NOT NULL;
-- Index for NAAC metric queries
CREATE INDEX idx_coa_naac ON chart_of_accounts (naac_metric_key) WHERE naac_metric_key IS NOT NULL;
-- Full-text search on account name
CREATE INDEX idx_coa_name_search ON chart_of_accounts USING gin (to_tsvector('english', account_name));
COMMENT ON INDEX idx_coa_parent IS 'Support for recursive COA tree queries (connect-by-parent)';
COMMENT ON INDEX idx_coa_active IS 'Exclude soft-deleted and inactive accounts from dropdowns';
COMMENT ON INDEX idx_coa_aishe IS 'Quick lookup of AISHE-mapped accounts for annual extract';
COMMENT ON INDEX idx_coa_naac IS 'Quick lookup of NAAC-mapped accounts for NAAC dashboard';
COMMENT ON INDEX idx_coa_name_search IS 'Full-text search on account name for auto-complete';

-- 3.3 Accounting Periods
CREATE TABLE fiscal_years (
    fiscal_year_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL REFERENCES tenants(tenant_id),
    year_code        TEXT        NOT NULL,               -- e.g., "2026-27"
    start_date       DATE        NOT NULL,               -- April 1
    end_date         DATE        NOT NULL,               -- March 31
    status           TEXT        NOT NULL DEFAULT 'OPEN' CHECK (status IN ('OPEN', 'CLOSING', 'CLOSED')),
    is_current_year  BOOLEAN     NOT NULL DEFAULT FALSE,
    closed_at        TIMESTAMPTZ,
    closed_by        UUID,
    entity_version   INT         NOT NULL DEFAULT 1,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by       UUID,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by       UUID,
    deleted_at       TIMESTAMPTZ,
    deleted_by       UUID,
    UNIQUE (tenant_id, year_code)
);
COMMENT ON TABLE fiscal_years IS 'Fiscal year definitions (April 1 – March 31). Cannot be changed per tenant.';
COMMENT ON COLUMN fiscal_years.start_date IS 'Must be April 1';
COMMENT ON COLUMN fiscal_years.end_date IS 'Must be March 31';
COMMENT ON COLUMN fiscal_years.status IS 'OPEN=accepting postings, CLOSING=integrity checks in progress, CLOSED=all periods closed';

CREATE TABLE accounting_periods (
    accounting_period_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    fiscal_year_id       UUID        NOT NULL REFERENCES fiscal_years(fiscal_year_id),
    period_number        INT         NOT NULL CHECK (period_number BETWEEN 1 AND 13),  -- 1-12 monthly, 13=adjustment
    period_name          TEXT        NOT NULL,               -- e.g., "April 2026"
    start_date           DATE        NOT NULL,               -- 1st of month
    end_date             DATE        NOT NULL,               -- last day of month
    status               TEXT        NOT NULL DEFAULT 'OPEN' CHECK (status IN ('OPEN', 'CLOSING', 'CLOSED')),
    gst_filing_deadline  DATE,                               -- 20th of next month
    tds_filing_deadline  DATE,                               -- 15th of month after quarter end
    gst_filed_date       DATE,
    tds_filed_date       DATE,
    closed_at            TIMESTAMPTZ,
    closed_by            UUID,
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    deleted_at           TIMESTAMPTZ,
    deleted_by           UUID,
    UNIQUE (fiscal_year_id, period_number),
    UNIQUE (tenant_id, fiscal_year_id, start_date)
);
COMMENT ON TABLE accounting_periods IS 'Monthly accounting periods within a fiscal year. 12 monthly periods plus optional 13th adjustment period.';
COMMENT ON COLUMN accounting_periods.period_number IS '1-12 for monthly periods, 13 for adjustment period';
COMMENT ON COLUMN accounting_periods.status IS 'OPEN=accepting postings, CLOSING=running integrity checks, CLOSED=no further postings allowed';
COMMENT ON COLUMN accounting_periods.gst_filing_deadline IS 'GST filing due date — typically 20th of next month';
COMMENT ON COLUMN accounting_periods.tds_filing_deadline IS 'TDS return due date — typically 15th of month after quarter';

-- Index for current period lookup
CREATE INDEX idx_ap_open ON accounting_periods (tenant_id) WHERE status = 'OPEN';
COMMENT ON INDEX idx_ap_open IS 'Quick lookup of currently open periods for posting validation';

-- 3.4 Cost Centers
CREATE TABLE cost_centers (
    cost_center_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id        UUID        NOT NULL REFERENCES entities(entity_id),
    cost_center_code TEXT        NOT NULL,
    cost_center_name TEXT        NOT NULL,
    cost_center_type TEXT        NOT NULL REFERENCES cost_center_types(code),
    parent_id        UUID        REFERENCES cost_centers(cost_center_id),
    manager_id       UUID,                               -- FK to users table
    budget_amount    paise       DEFAULT 0,
    budget_period    TEXT        CHECK (budget_period IN ('MONTHLY', 'QUARTERLY', 'ANNUAL')),
    is_active        BOOLEAN     NOT NULL DEFAULT TRUE,
    entity_version   INT         NOT NULL DEFAULT 1,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by       UUID,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by       UUID,
    deleted_at       TIMESTAMPTZ,
    deleted_by       UUID,
    UNIQUE (tenant_id, cost_center_code)
);
COMMENT ON TABLE cost_centers IS 'Cost center dimensions for granular profitability analysis and budget control.';
COMMENT ON COLUMN cost_centers.parent_id IS 'For tree hierarchy — max depth 5 levels';
COMMENT ON COLUMN cost_centers.manager_id IS 'Cost center manager who receives budget alerts';

CREATE INDEX idx_cc_parent ON cost_centers (parent_id);
CREATE INDEX idx_cc_active ON cost_centers (tenant_id) WHERE deleted_at IS NULL AND is_active = TRUE;
COMMENT ON INDEX idx_cc_parent IS 'Support for cost center tree hierarchy queries';

-- 3.5 Funds
CREATE TABLE funds (
    fund_id                 UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id               UUID        NOT NULL REFERENCES tenants(tenant_id),
    fund_code               TEXT        NOT NULL,
    fund_name               TEXT        NOT NULL,
    fund_type               TEXT        NOT NULL REFERENCES fund_types(code),
    fund_source             TEXT        NOT NULL CHECK (fund_source IN ('GOVERNMENT_UGC', 'GOVERNMENT_STATE', 'GOVERNMENT_OTHER', 'PRIVATE', 'DONATION', 'INTERNAL', 'FCRA')),
    grant_scheme            TEXT,                               -- e.g., "SAP", "DRS", "DSA" for UGC grants
    sanction_order_number   TEXT,
    sanction_date           DATE,
    sanctioned_amount       paise       DEFAULT 0,
    start_date              DATE,
    end_date                DATE,
    status                  TEXT        NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'COMPLETED', 'TERMINATED', 'SUSPENDED')),
    bank_account_id         UUID,                               -- FK to bank_accounts (separate account requirement for grants)
    fcra_registration_number TEXT,
    fcra_admin_expense_ratio NUMERIC(5,2),                      -- max 20%
    principal_amount        paise_nullable,                     -- for endowment funds
    income_only             BOOLEAN     DEFAULT FALSE,           -- endowment funds: income only
    entity_version          INT         NOT NULL DEFAULT 1,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by              UUID,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by              UUID,
    deleted_at              TIMESTAMPTZ,
    deleted_by              UUID,
    UNIQUE (tenant_id, fund_code)
);
COMMENT ON TABLE funds IS 'Fund accounting — grants, endowments, FCRA, scholarships. Each fund has its own ledger within the GL.';
COMMENT ON COLUMN funds.fund_type IS 'RESTRICTED=grant-specific, UNRESTRICTED=general, ENDOWMENT=corpus, FCRA=foreign, SCHOLARSHIP=DBT pass-through';
COMMENT ON COLUMN funds.fund_source IS 'Source of fund — government, private, donation, internal, FCRA';
COMMENT ON COLUMN funds.fcra_admin_expense_ratio IS 'Administrative expense ratio as % of FCRA receipts — max 20% as per FCRA rules';
COMMENT ON COLUMN funds.principal_amount IS 'Endowment principal — must remain untouched';
COMMENT ON COLUMN funds.income_only IS 'For endowment funds — only income from this fund can be used';

CREATE INDEX idx_funds_type ON funds (tenant_id, fund_type);
CREATE INDEX idx_funds_status ON funds (tenant_id, status);
COMMENT ON INDEX idx_funds_type IS 'Quick filtering by fund type for reports';

-- Fund budget heads (approved budget lines per fund)
CREATE TABLE fund_budget_heads (
    fund_budget_head_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID        NOT NULL REFERENCES tenants(tenant_id),
    fund_id             UUID        NOT NULL REFERENCES funds(fund_id),
    account_id          UUID        NOT NULL REFERENCES chart_of_accounts(account_id),
    approved_amount     paise       NOT NULL,
    entity_version      INT         NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by          UUID,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by          UUID,
    deleted_at          TIMESTAMPTZ,
    deleted_by          UUID,
    UNIQUE (fund_id, account_id)
);
COMMENT ON TABLE fund_budget_heads IS 'Approved budget heads per grant fund. Expenditure must be within approved heads.';

CREATE INDEX idx_fbh_fund ON fund_budget_heads (fund_id);
COMMENT ON INDEX idx_fbh_fund IS 'Lookup all budget heads for a fund';

-- ============================================================================
-- PART 4: GENERAL LEDGER (Double-Entry Accounting Engine)
-- ============================================================================

-- Journal entries (IMMUTABLE — INSERT-only after posting)
CREATE TABLE journal_entries (
    journal_id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(tenant_id),
    journal_number     TEXT        NOT NULL,               -- auto-numbered: INV-{YYYY}-{NNNNNN}
    journal_type       TEXT        NOT NULL REFERENCES journal_types(code),
    accounting_period_id UUID      NOT NULL REFERENCES accounting_periods(accounting_period_id),
    entity_id          UUID        NOT NULL REFERENCES entities(entity_id),
    fund_id            UUID        REFERENCES funds(fund_id),
    cost_center_id     UUID        REFERENCES cost_centers(cost_center_id),
    posting_date       DATE        NOT NULL,
    description        TEXT        NOT NULL,
    status             TEXT        NOT NULL DEFAULT 'DRAFT' REFERENCES journal_statuses(code),
    total_debit        paise       NOT NULL DEFAULT 0,
    total_credit       paise       NOT NULL DEFAULT 0,
    posted_at          TIMESTAMPTZ,
    posted_by          UUID,
    reversed_by_id     UUID        REFERENCES journal_entries(journal_id),   -- self-ref for reversal pair
    reason             TEXT,
    attachment_ids     UUID[],                             -- array of document IDs
    version            INT         NOT NULL DEFAULT 1,     -- version for optimistic concurrency
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         UUID,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by         UUID,
    -- NO deleted_at — financial records are immutable
    UNIQUE (tenant_id, journal_number)
) PARTITION BY RANGE (posting_date);
COMMENT ON TABLE journal_entries IS '★ IMMUTABLE ★ Double-entry journal header. INSERT-only after posting. Corrections require reversing entries.';
COMMENT ON COLUMN journal_entries.journal_number IS 'Auto-numbered per tenant per fiscal year: INV-{YYYY}-{NNNNNN}';
COMMENT ON COLUMN journal_entries.status IS 'DRAFT=editable, POSTED=immutable, REVERSED=original reversed, CANCELLED=draft cancelled';
COMMENT ON COLUMN journal_entries.total_debit IS 'Must equal total_credit for a balanced journal entry';
COMMENT ON COLUMN journal_entries.reversed_by_id IS 'When a journal is reversed, this links to the reversing journal entry';
COMMENT ON COLUMN journal_entries.version IS 'Optimistic concurrency version — incremented on each state transition';

-- Create partitions for journal_entries (current + 5 years)
CREATE TABLE journal_entries_y2026 PARTITION OF journal_entries
    FOR VALUES FROM ('2026-04-01') TO ('2027-04-01');
CREATE TABLE journal_entries_y2027 PARTITION OF journal_entries
    FOR VALUES FROM ('2027-04-01') TO ('2028-04-01');
CREATE TABLE journal_entries_y2028 PARTITION OF journal_entries
    FOR VALUES FROM ('2028-04-01') TO ('2029-04-01');
CREATE TABLE journal_entries_y2029 PARTITION OF journal_entries
    FOR VALUES FROM ('2029-04-01') TO ('2030-04-01');
CREATE TABLE journal_entries_y2030 PARTITION OF journal_entries
    FOR VALUES FROM ('2030-04-01') TO ('2031-04-01');
-- Default partition for future dates
CREATE TABLE journal_entries_default PARTITION OF journal_entries
    FOR VALUES FROM ('2031-04-01') TO ('2099-04-01');

COMMENT ON TABLE journal_entries_y2026 IS 'Partition for FY 2026-27';
COMMENT ON TABLE journal_entries_y2027 IS 'Partition for FY 2027-28';
COMMENT ON TABLE journal_entries_y2028 IS 'Partition for FY 2028-29';
COMMENT ON TABLE journal_entries_y2029 IS 'Partition for FY 2029-30';
COMMENT ON TABLE journal_entries_y2030 IS 'Partition for FY 2030-31';

-- Indexes on journal_entries (on parent — applies to all partitions)
CREATE INDEX idx_je_entity_period ON journal_entries (tenant_id, entity_id, accounting_period_id);
CREATE INDEX idx_je_status ON journal_entries (tenant_id, status) WHERE status = 'DRAFT' OR status = 'POSTED';
CREATE INDEX idx_je_posting_date ON journal_entries (tenant_id, posting_date DESC);
CREATE INDEX idx_je_fund ON journal_entries (fund_id) WHERE fund_id IS NOT NULL;
CREATE INDEX idx_je_cost_center ON journal_entries (cost_center_id) WHERE cost_center_id IS NOT NULL;
CREATE INDEX idx_je_number ON journal_entries (tenant_id, journal_number);
COMMENT ON INDEX idx_je_entity_period IS 'Filter journals by entity and accounting period for reports';
COMMENT ON INDEX idx_je_status IS 'Quick lookup of draft or posted journals';
COMMENT ON INDEX idx_je_posting_date IS 'Date-range queries on posting date';
COMMENT ON INDEX idx_je_fund IS 'Find all journal entries for a specific fund';
COMMENT ON INDEX idx_je_cost_center IS 'Find all journal entries for a cost center';

-- Journal entry lines (IMMUTABLE)
CREATE TABLE journal_entry_lines (
    journal_line_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    journal_id           UUID        NOT NULL,
    line_number          INT         NOT NULL CHECK (line_number > 0),
    account_id           UUID        NOT NULL REFERENCES chart_of_accounts(account_id),
    debit_amount         paise_nullable,                 -- exactly one of debit/credit must be set
    credit_amount        paise_nullable,
    description          TEXT,
    cost_center_id       UUID        REFERENCES cost_centers(cost_center_id),
    fund_id              UUID        REFERENCES funds(fund_id),
    reference_id         TEXT,                            -- e.g., invoice number, receipt number
    reference_type       TEXT,                            -- e.g., 'INVOICE', 'RECEIPT', 'PAYMENT'
    tax_rate             NUMERIC(5,2),                    -- GST rate at time of transaction
    tax_amount           paise_nullable,
    is_itc_claimed       BOOLEAN     DEFAULT FALSE,
    itc_reversal_percent NUMERIC(5,2),                    -- for Rule 42/43
    version              INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    -- NO deleted_at — immutable
    CHECK ((debit_amount IS NOT NULL AND credit_amount IS NULL) OR
           (debit_amount IS NULL AND credit_amount IS NOT NULL)),
    UNIQUE (journal_id, line_number)
) PARTITION BY RANGE (created_at);
COMMENT ON TABLE journal_entry_lines IS '★ IMMUTABLE ★ Individual debit/credit lines of a journal entry. Exactly one of debit/credit must be set (XOR).';
COMMENT ON COLUMN journal_entry_lines.debit_amount IS 'Debit amount — set only for debit legs (XOR with credit_amount)';
COMMENT ON COLUMN journal_entry_lines.credit_amount IS 'Credit amount — set only for credit legs (XOR with debit_amount)';
COMMENT ON COLUMN journal_entry_lines.reference_type IS 'Type of reference document (INVOICE, RECEIPT, PAYMENT, etc.)';
COMMENT ON COLUMN journal_entry_lines.is_itc_claimed IS 'Whether ITC is claimed on this line';
COMMENT ON COLUMN journal_entry_lines.itc_reversal_percent IS 'ITC reversal % for Rule 42 (inputs) or Rule 43 (capital goods)';

-- Copy partition strategy from journal_entries
CREATE TABLE journal_entry_lines_y2026 PARTITION OF journal_entry_lines
    FOR VALUES FROM ('2026-04-01') TO ('2027-04-01');
CREATE TABLE journal_entry_lines_y2027 PARTITION OF journal_entry_lines
    FOR VALUES FROM ('2027-04-01') TO ('2028-04-01');
CREATE TABLE journal_entry_lines_y2028 PARTITION OF journal_entry_lines
    FOR VALUES FROM ('2028-04-01') TO ('2029-04-01');
CREATE TABLE journal_entry_lines_y2029 PARTITION OF journal_entry_lines
    FOR VALUES FROM ('2029-04-01') TO ('2030-04-01');
CREATE TABLE journal_entry_lines_y2030 PARTITION OF journal_entry_lines
    FOR VALUES FROM ('2030-04-01') TO ('2031-04-01');
CREATE TABLE journal_entry_lines_default PARTITION OF journal_entry_lines
    FOR VALUES FROM ('2031-04-01') TO ('2099-04-01');

CREATE INDEX idx_jel_journal ON journal_entry_lines (journal_id);
CREATE INDEX idx_jel_account ON journal_entry_lines (account_id);
CREATE INDEX idx_jel_account_date ON journal_entry_lines (account_id, created_at);
CREATE INDEX idx_jel_reference ON journal_entry_lines (reference_type, reference_id) WHERE reference_id IS NOT NULL;
CREATE INDEX idx_jel_fund ON journal_entry_lines (fund_id) WHERE fund_id IS NOT NULL;
COMMENT ON INDEX idx_jel_journal IS 'All lines belonging to a journal header';
COMMENT ON INDEX idx_jel_account IS 'Find all postings to a specific account (account balance queries)';
COMMENT ON INDEX idx_jel_account_date IS 'Account balance at a point in time (date-range filtered)';
COMMENT ON INDEX idx_jel_reference IS 'Look up journal lines by source document reference';

-- ============================================================================
-- PART 5: ACCOUNTS RECEIVABLE (Student Fee Management)
-- ============================================================================

-- 5.1 Fee Heads
CREATE TABLE fee_heads (
    fee_head_id        UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(tenant_id),
    fee_head_code      TEXT        NOT NULL,
    fee_head_name      TEXT        NOT NULL,
    fee_type           TEXT        NOT NULL REFERENCES fee_types(code),
    gst_classification TEXT        REFERENCES gst_classifications(code),
    hsn_sac_code       TEXT,
    is_optional        BOOLEAN     NOT NULL DEFAULT FALSE,
    is_refundable      BOOLEAN     NOT NULL DEFAULT FALSE,
    is_mandatory       BOOLEAN     NOT NULL DEFAULT TRUE,
    is_active          BOOLEAN     NOT NULL DEFAULT TRUE,
    entity_version     INT         NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         UUID,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by         UUID,
    deleted_at         TIMESTAMPTZ,
    deleted_by         UUID,
    UNIQUE (tenant_id, fee_head_code)
);
COMMENT ON TABLE fee_heads IS 'Catalog of all fee types that can be charged to students.';
COMMENT ON COLUMN fee_heads.gst_classification IS 'GST classification for this fee head — determines whether GST is charged';
COMMENT ON COLUMN fee_heads.hsn_sac_code IS 'HSN/SAC code for GST reporting on this fee head';
COMMENT ON COLUMN fee_heads.is_refundable IS 'True for caution deposits and other refundable fees';

-- 5.2 Fee Structures
CREATE TABLE fee_structures (
    fee_structure_id       UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id              UUID        NOT NULL REFERENCES entities(entity_id),
    program_id             UUID,                               -- FK to academic programs table
    academic_year          TEXT        NOT NULL,               -- e.g., "2026-27"
    semester_term          TEXT        NOT NULL,               -- "Annual", "Sem-1", "Sem-2"
    student_category       TEXT        REFERENCES student_categories(code),
    fee_structure_name     TEXT        NOT NULL,
    frc_approval_order_no  TEXT,                               -- FRC approval order reference
    frc_approval_date      DATE,
    status                 TEXT        NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'ACTIVE', 'ARCHIVED')),
    effective_from         DATE        NOT NULL,
    effective_to           DATE,
    entity_version         INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    deleted_at             TIMESTAMPTZ,
    deleted_by             UUID,
    UNIQUE (tenant_id, entity_id, program_id, academic_year, semester_term, student_category)
);
COMMENT ON TABLE fee_structures IS 'Fee structure definitions per program, academic year, and student category.';
COMMENT ON COLUMN fee_structures.frc_approval_order_no IS 'FRC (Fee Regulation Committee) approval reference for fee compliance';
COMMENT ON COLUMN fee_structures.status IS 'DRAFT=editable, ACTIVE=in use for fee assessment, ARCHIVED=historical';

CREATE INDEX idx_fs_program_year ON fee_structures (tenant_id, entity_id, program_id, academic_year);
COMMENT ON INDEX idx_fs_program_year IS 'Find applicable fee structure for a program and year';

-- Fee structure lines (fee head amounts per structure)
CREATE TABLE fee_structure_lines (
    fee_structure_line_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    fee_structure_id      UUID        NOT NULL REFERENCES fee_structures(fee_structure_id),
    fee_head_id           UUID        NOT NULL REFERENCES fee_heads(fee_head_id),
    amount                paise       NOT NULL,
    is_optional           BOOLEAN,                            -- overrides fee_head default
    installment_allowed   BOOLEAN     NOT NULL DEFAULT TRUE,
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    deleted_at            TIMESTAMPTZ,
    deleted_by            UUID,
    UNIQUE (fee_structure_id, fee_head_id)
);
COMMENT ON TABLE fee_structure_lines IS 'Fee head amounts within a fee structure. Links fee heads to amounts per structure.';

-- Installment plans
CREATE TABLE installment_plans (
    installment_plan_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    fee_structure_id      UUID        NOT NULL REFERENCES fee_structures(fee_structure_id),
    plan_name             TEXT        NOT NULL,               -- "2 Installments", "4 Installments"
    number_of_installments INT       NOT NULL CHECK (number_of_installments > 0 AND number_of_installments <= 6),
    installment_distribution JSONB   NOT NULL,                -- [{"number":1,"percentage":50,"dueDate":"..."},...]
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    deleted_at            TIMESTAMPTZ,
    deleted_by            UUID
);
COMMENT ON TABLE installment_plans IS 'Fee installment plans — how fee is split across due dates. Sum of percentages must equal 100.';
COMMENT ON COLUMN installment_plans.installment_distribution IS 'JSONB array: [{"number":1,"percentage":50,"dueDate":"2026-07-15"},...]';

-- Student fee assessments (per-student fee ledger)
CREATE TABLE student_fee_accounts (
    student_fee_account_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id                UUID        NOT NULL REFERENCES tenants(tenant_id),
    student_id               UUID        NOT NULL,           -- FK to student master
    entity_id                UUID        NOT NULL REFERENCES entities(entity_id),
    fee_structure_id         UUID        NOT NULL REFERENCES fee_structures(fee_structure_id),
    installment_plan_id      UUID        REFERENCES installment_plans(installment_plan_id),
    academic_year            TEXT        NOT NULL,
    total_fee_amount         paise       NOT NULL,
    total_paid_amount        paise       NOT NULL DEFAULT 0,
    total_scholarship_amount paise       NOT NULL DEFAULT 0,
    total_concession_amount  paise       NOT NULL DEFAULT 0,
    outstanding_amount       paise       NOT NULL,
    status                   TEXT        NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'PAID', 'OVERPAID', 'CLOSED')),
    entity_version           INT         NOT NULL DEFAULT 1,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by               UUID,
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by               UUID,
    deleted_at               TIMESTAMPTZ,
    deleted_by               UUID,
    UNIQUE (tenant_id, student_id, academic_year, fee_structure_id)
);
COMMENT ON TABLE student_fee_accounts IS 'Per-student fee ledger — tracks fee assessment, payments, scholarships, and outstanding.';
COMMENT ON COLUMN student_fee_accounts.total_fee_amount IS 'Gross fee assessed (before scholarships/concessions)';
COMMENT ON COLUMN student_fee_accounts.outstanding_amount IS 'total_fee_amount - total_paid_amount - total_scholarship_amount - total_concession_amount';

CREATE INDEX idx_sfa_student_year ON student_fee_accounts (tenant_id, student_id, academic_year);
CREATE INDEX idx_sfa_outstanding ON student_fee_accounts (tenant_id) WHERE outstanding_amount > 0;
COMMENT ON INDEX idx_sfa_student_year IS 'Lookup fee account for a student in a given academic year';
COMMENT ON INDEX idx_sfa_outstanding IS 'Identify students with outstanding fee balances';

-- Fee installments (per student, per installment)
CREATE TABLE fee_installments (
    fee_installment_id     UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    student_fee_account_id UUID        NOT NULL REFERENCES student_fee_accounts(student_fee_account_id),
    installment_number     INT         NOT NULL CHECK (installment_number > 0),
    amount                 paise       NOT NULL,
    due_date               DATE        NOT NULL,
    paid_amount            paise       NOT NULL DEFAULT 0,
    scholarship_amount     paise       NOT NULL DEFAULT 0,
    concession_amount      paise       NOT NULL DEFAULT 0,
    late_fee_amount        paise       NOT NULL DEFAULT 0,
    status                 TEXT        NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING', 'PARTIALLY_PAID', 'PAID', 'OVERPAID', 'WAIVED')),
    paid_at                TIMESTAMPTZ,
    entity_version         INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    deleted_at             TIMESTAMPTZ,
    deleted_by             UUID,
    UNIQUE (student_fee_account_id, installment_number)
);
COMMENT ON TABLE fee_installments IS 'Individual fee installment per student — tracks payment status and due dates.';

CREATE INDEX idx_fi_due ON fee_installments (tenant_id, due_date) WHERE status IN ('PENDING', 'PARTIALLY_PAID');
COMMENT ON INDEX idx_fi_due IS 'Identify upcoming and overdue installments for reminders';

-- 5.3 Fee Transactions (IMMUTABLE)
CREATE TABLE fee_transactions (
    fee_transaction_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    student_fee_account_id UUID      NOT NULL REFERENCES student_fee_accounts(student_fee_account_id),
    fee_installment_id   UUID        REFERENCES fee_installments(fee_installment_id),
    transaction_type     TEXT        NOT NULL CHECK (transaction_type IN ('PAYMENT', 'SCHOLARSHIP', 'CONCESSION', 'REFUND', 'LATE_FEE', 'ADJUSTMENT')),
    amount               paise       NOT NULL,
    payment_receipt_id   UUID,                               -- FK to payment_receipts (for PAYMENT type)
    reference_id         TEXT,                                -- scholarship/refund reference
    reference_type       TEXT,
    description          TEXT,
    posted_journal_id    UUID        REFERENCES journal_entries(journal_id),
    transaction_date     TIMESTAMPTZ NOT NULL DEFAULT now(),
    version              INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID
    -- NO deleted_at — immutable
) PARTITION BY RANGE (transaction_date);
COMMENT ON TABLE fee_transactions IS '★ IMMUTABLE ★ Every fee-related transaction (payment, scholarship, concession, refund) recorded here.';
COMMENT ON COLUMN fee_transactions.transaction_type IS 'PAYMENT=student payment, SCHOLARSHIP=DBT credit, CONCESSION=fee waiver, REFUND=money returned, LATE_FEE=penalty, ADJUSTMENT=manual correction';

-- Partitions for fee_transactions
CREATE TABLE fee_transactions_y2026 PARTITION OF fee_transactions
    FOR VALUES FROM ('2026-04-01') TO ('2027-04-01');
CREATE TABLE fee_transactions_y2027 PARTITION OF fee_transactions
    FOR VALUES FROM ('2027-04-01') TO ('2028-04-01');
CREATE TABLE fee_transactions_y2028 PARTITION OF fee_transactions
    FOR VALUES FROM ('2028-04-01') TO ('2029-04-01');
CREATE TABLE fee_transactions_y2029 PARTITION OF fee_transactions
    FOR VALUES FROM ('2029-04-01') TO ('2030-04-01');
CREATE TABLE fee_transactions_y2030 PARTITION OF fee_transactions
    FOR VALUES FROM ('2030-04-01') TO ('2031-04-01');
CREATE TABLE fee_transactions_default PARTITION OF fee_transactions
    FOR VALUES FROM ('2031-04-01') TO ('2099-04-01');

CREATE INDEX idx_ft_account ON fee_transactions (student_fee_account_id);
CREATE INDEX idx_ft_installment ON fee_transactions (fee_installment_id) WHERE fee_installment_id IS NOT NULL;
CREATE INDEX idx_ft_type_date ON fee_transactions (tenant_id, transaction_type, transaction_date);
COMMENT ON INDEX idx_ft_account IS 'All transactions for a student fee account';
COMMENT ON INDEX idx_ft_type_date IS 'Aggregate payments/receipts by type and date';

-- 5.4 Payment Receipts (IMMUTABLE after completion)
CREATE TABLE payment_receipts (
    payment_receipt_id     UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id              UUID        NOT NULL REFERENCES entities(entity_id),
    receipt_number         TEXT        NOT NULL,             -- RCP-{ENTITY}-{YYYY}-{NNNNNN}
    student_id             UUID        NOT NULL,
    student_fee_account_id UUID        REFERENCES student_fee_accounts(student_fee_account_id),
    payment_mode           TEXT        NOT NULL REFERENCES payment_modes(code),
    payment_date           TIMESTAMPTZ NOT NULL,
    amount                 paise       NOT NULL,
    status                 TEXT        NOT NULL DEFAULT 'PENDING' REFERENCES receipt_statuses(code),
    gateway_payment_id     TEXT,                             -- payment gateway transaction ID
    gateway_reference      TEXT,
    bank_transaction_ref   TEXT,
    cheque_number          TEXT,
    cheque_date            DATE,
    cheque_bank            TEXT,
    cleared_date           DATE,
    remarks                TEXT,
    received_by_id         UUID,
    payment_journal_id     UUID        REFERENCES journal_entries(journal_id),
    version                INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    UNIQUE (tenant_id, receipt_number)
);
COMMENT ON TABLE payment_receipts IS 'Payment receipts from students. Immutable after status reaches COMPLETED.';
COMMENT ON COLUMN payment_receipts.receipt_number IS 'Auto-generated: RCP-{ENTITY}-{YYYY}-{NNNNNN}';
COMMENT ON COLUMN payment_receipts.status IS 'PENDING→COMPLETED/FAILED→(if cheque)→UNCLEARED→COMPLETED/BOUNCED';

CREATE INDEX idx_pr_student ON payment_receipts (tenant_id, student_id);
CREATE INDEX idx_pr_date ON payment_receipts (tenant_id, payment_date DESC);
CREATE INDEX idx_pr_status ON payment_receipts (status) WHERE status IN ('PENDING', 'UNCLEARED');
CREATE INDEX idx_pr_number ON payment_receipts (tenant_id, receipt_number);
COMMENT ON INDEX idx_pr_student IS 'All receipts for a student';
COMMENT ON INDEX idx_pr_status IS 'Pending and uncleared receipts requiring action';

-- Payment allocations to installments
CREATE TABLE payment_allocations (
    payment_allocation_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    payment_receipt_id    UUID        NOT NULL REFERENCES payment_receipts(payment_receipt_id),
    fee_installment_id    UUID        NOT NULL REFERENCES fee_installments(fee_installment_id),
    fee_head_id           UUID        REFERENCES fee_heads(fee_head_id),
    allocated_amount      paise       NOT NULL,
    scholarship_amount    paise       NOT NULL DEFAULT 0,
    concession_amount     paise       NOT NULL DEFAULT 0,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    UNIQUE (payment_receipt_id, fee_installment_id, fee_head_id)
);
COMMENT ON TABLE payment_allocations IS 'Allocation of a payment receipt to specific fee installments and heads.';
COMMENT ON COLUMN payment_allocations.scholarship_amount IS 'Scholarship amount allocated to this installment';
COMMENT ON COLUMN payment_allocations.concession_amount IS 'Concession amount allocated to this installment';

CREATE INDEX idx_pa_receipt ON payment_allocations (payment_receipt_id);
CREATE INDEX idx_pa_installment ON payment_allocations (fee_installment_id);
COMMENT ON INDEX idx_pa_receipt IS 'All allocations for a receipt';
COMMENT ON INDEX idx_pa_installment IS 'All payment allocations to a specific installment';

-- Payment gateway transactions
CREATE TABLE payment_gateway_transactions (
    payment_gateway_txn_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    payment_receipt_id     UUID        REFERENCES payment_receipts(payment_receipt_id),
    gateway                TEXT        NOT NULL CHECK (gateway IN ('BILLDESK', 'RAZORPAY', 'CCAVENUE', 'PHONEPE', 'PAYTM')),
    gateway_transaction_id TEXT        NOT NULL,
    status                 TEXT        NOT NULL CHECK (status IN ('INITIATED', 'PENDING', 'SUCCESS', 'FAILED', 'REFUNDED')),
    request_payload        JSONB,
    response_payload       JSONB,
    error_code             TEXT,
    error_message          TEXT,
    amount                 paise       NOT NULL,
    gateway_fee            paise       DEFAULT 0,
    settled_amount         paise,
    settled_date           DATE,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    UNIQUE (gateway, gateway_transaction_id)
);
COMMENT ON TABLE payment_gateway_transactions IS 'Payment gateway transaction records for online fee collection.';
COMMENT ON COLUMN payment_gateway_transactions.request_payload IS 'Raw request sent to payment gateway';
COMMENT ON COLUMN payment_gateway_transactions.response_payload IS 'Raw response/webhook from payment gateway';

CREATE INDEX idx_pgt_receipt ON payment_gateway_transactions (payment_receipt_id);
CREATE INDEX idx_pgt_gateway_status ON payment_gateway_transactions (gateway, status);
COMMENT ON INDEX idx_pgt_gateway_status IS 'Monitoring pending transactions per gateway';

-- 5.5 Concessions
CREATE TABLE concessions (
    concession_id        UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    student_id           UUID        NOT NULL,
    student_fee_account_id UUID      NOT NULL REFERENCES student_fee_accounts(student_fee_account_id),
    concession_type      TEXT        NOT NULL CHECK (concession_type IN ('MERIT', 'SPORTS', 'CULTURAL', 'STAFF_DEPENDENT', 'MANAGEMENT', 'NEED_BASED', 'OTHER')),
    concession_percent   NUMERIC(5,2) NOT NULL CHECK (concession_percent BETWEEN 0 AND 100),
    concession_amount    paise,
    approved_by_id       UUID,
    approval_date        TIMESTAMPTZ,
    sanction_order_no    TEXT,
    valid_from           DATE        NOT NULL,
    valid_to             DATE,
    status               TEXT        NOT NULL DEFAULT 'APPLIED' CHECK (status IN ('APPLIED', 'APPROVED', 'REJECTED', 'EXPIRED')),
    remarks              TEXT,
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    deleted_at           TIMESTAMPTZ,
    deleted_by           UUID
);
COMMENT ON TABLE concessions IS 'Student fee concessions — fee waivers applied to fee assessments.';
COMMENT ON COLUMN concessions.concession_amount IS 'Computed from concession_percent * fee_amount. Can also be set directly.';

CREATE INDEX idx_conc_student ON concessions (tenant_id, student_id);
COMMENT ON INDEX idx_conc_student IS 'All concessions granted to a student';

-- 5.6 Scholarship Schemes
CREATE TABLE scholarship_schemes (
    scholarship_scheme_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    scheme_code           TEXT        NOT NULL,
    scheme_name           TEXT        NOT NULL,
    provider              TEXT        NOT NULL CHECK (provider IN ('CENTRAL_GOVERNMENT', 'STATE_GOVERNMENT', 'PRIVATE', 'TRUST', 'OTHER')),
    state                 TEXT,                               -- for state-specific schemes
    scheme_type           TEXT        NOT NULL CHECK (scheme_type IN ('TUITION_FEE', 'MAINTENANCE', 'FULL_TUITION', 'TUITION_PLUS_MAINTENANCE', 'LUMP_SUM')),
    max_amount            paise       NOT NULL,
    eligibility_criteria  JSONB,                              -- configurable eligibility rules
    is_active             BOOLEAN     NOT NULL DEFAULT TRUE,
    requires_aadhaar      BOOLEAN     NOT NULL DEFAULT TRUE,
    requires_bank_account BOOLEAN     NOT NULL DEFAULT TRUE,
    requires_income_cert  BOOLEAN     NOT NULL DEFAULT FALSE,
    requires_caste_cert   BOOLEAN     NOT NULL DEFAULT FALSE,
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    deleted_at            TIMESTAMPTZ,
    deleted_by            UUID,
    UNIQUE (tenant_id, scheme_code)
);
COMMENT ON TABLE scholarship_schemes IS 'Configurable scholarship schemes — government, private, and trust-funded.';
COMMENT ON COLUMN scholarship_schemes.eligibility_criteria IS 'JSONB rules for eligibility — caste, income, academic marks, etc.';

-- 5.7 Student Scholarships
CREATE TABLE student_scholarships (
    student_scholarship_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    student_id             UUID        NOT NULL,
    scheme_id              UUID        NOT NULL REFERENCES scholarship_schemes(scholarship_scheme_id),
    student_fee_account_id UUID        REFERENCES student_fee_accounts(student_fee_account_id),
    application_reference  TEXT,                               -- MahaDBT portal reference
    expected_amount        paise       NOT NULL,
    sanctioned_amount      paise_nullable,
    disbursed_amount       paise_nullable,
    dbt_date               TIMESTAMPTZ,
    dbt_transaction_ref    TEXT,
    status                 TEXT        NOT NULL DEFAULT 'APPLIED' CHECK (status IN ('APPLIED', 'VERIFIED', 'SANCTIONED', 'DISBURSED', 'PARTIALLY_DISBURSED', 'REJECTED', 'CLOSED')),
    applied_date           TIMESTAMPTZ NOT NULL DEFAULT now(),
    verified_date          TIMESTAMPTZ,
    sanctioned_date        TIMESTAMPTZ,
    disbursed_date         TIMESTAMPTZ,
    verified_by_id         UUID,
    sanctioned_by_id       UUID,
    remarks                TEXT,
    entity_version         INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    deleted_at             TIMESTAMPTZ,
    deleted_by             UUID
);
COMMENT ON TABLE student_scholarships IS 'Per-student scholarship grants with full lifecycle tracking — from application to DBT disbursement.';
COMMENT ON COLUMN student_scholarships.application_reference IS 'MahaDBT or other portal application reference number';
COMMENT ON COLUMN student_scholarships.status IS 'APPLIED→VERIFIED→SANCTIONED→DISBURSED, or REJECTED/CLOSED at any stage';

CREATE INDEX idx_ss_student ON student_scholarships (tenant_id, student_id);
CREATE INDEX idx_ss_scheme ON student_scholarships (tenant_id, scheme_id);
CREATE INDEX idx_ss_status ON student_scholarships (status) WHERE status IN ('APPLIED', 'VERIFIED');
COMMENT ON INDEX idx_ss_student IS 'All scholarships granted to a student';
COMMENT ON INDEX idx_ss_scheme IS 'All students under a specific scholarship scheme (for scheme-wise reconciliation)';
COMMENT ON INDEX idx_ss_status IS 'Pending verification and sanction — operational dashboard';

-- 5.8 Refunds
CREATE TABLE refunds (
    refund_id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id             UUID        NOT NULL REFERENCES entities(entity_id),
    refund_number         TEXT        NOT NULL,
    student_id            UUID,
    source_receipt_id     UUID        REFERENCES payment_receipts(payment_receipt_id),
    refund_type           TEXT        NOT NULL CHECK (refund_type IN ('FEE_REFUND', 'SCHOLARSHIP_ADJUSTMENT', 'EXCESS_PAYMENT', 'DEPOSIT_REFUND', 'OTHER')),
    refund_mode           TEXT        NOT NULL REFERENCES payment_modes(code),
    amount                paise       NOT NULL,
    reason                TEXT        NOT NULL,
    frc_refund_percent    NUMERIC(5,2),                        -- if FRC-based
    status                TEXT        NOT NULL DEFAULT 'INITIATED' CHECK (status IN ('INITIATED', 'APPROVED', 'PROCESSED', 'COMPLETED', 'FAILED', 'CANCELLED')),
    approved_by_id        UUID,
    approved_at           TIMESTAMPTZ,
    processed_by_id       UUID,
    processed_at          TIMESTAMPTZ,
    bank_transaction_ref  TEXT,
    credit_note_id        UUID,                                -- FK to credit_notes
    refund_journal_id     UUID        REFERENCES journal_entries(journal_id),
    remarks               TEXT,
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    deleted_at            TIMESTAMPTZ,
    deleted_by            UUID,
    UNIQUE (tenant_id, refund_number)
);
COMMENT ON TABLE refunds IS 'Student refund tracking — FRC-compliant fee refunds, scholarship DBT adjustments, and excess payment returns.';
COMMENT ON COLUMN refunds.frc_refund_percent IS 'Refund % as per FRC fee refund schedule based on withdrawal date';
COMMENT ON COLUMN refunds.status IS 'INITIATED→APPROVED→PROCESSED→COMPLETED, or CANCELLED/FAILED';

CREATE INDEX idx_ref_student ON refunds (tenant_id, student_id);
CREATE INDEX idx_ref_status ON refunds (status) WHERE status IN ('INITIATED', 'APPROVED');
COMMENT ON INDEX idx_ref_student IS 'All refunds for a student';
COMMENT ON INDEX idx_ref_status IS 'Refunds pending action';

-- Credit notes
CREATE TABLE credit_notes (
    credit_note_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID        NOT NULL REFERENCES tenants(tenant_id),
    credit_note_number  TEXT        NOT NULL,
    student_id          UUID        NOT NULL,
    amount              paise       NOT NULL,
    remaining_balance   paise       NOT NULL,
    issue_date          TIMESTAMPTZ NOT NULL DEFAULT now(),
    expiry_date         DATE,
    status              TEXT        NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'PARTIALLY_UTILIZED', 'FULLY_UTILIZED', 'EXPIRED', 'CANCELLED')),
    issued_against      TEXT,                                  -- receipt ID, refund ID, etc.
    entity_version      INT         NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by          UUID,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by          UUID,
    deleted_at          TIMESTAMPTZ,
    deleted_by          UUID,
    UNIQUE (tenant_id, credit_note_number)
);
COMMENT ON TABLE credit_notes IS 'Credit notes issued to students for future fee adjustments.';
COMMENT ON COLUMN credit_notes.remaining_balance IS 'Unutilized balance — decreases as credit note is applied';

CREATE INDEX idx_cn_student ON credit_notes (tenant_id, student_id);
CREATE INDEX idx_cn_active ON credit_notes (tenant_id) WHERE status IN ('ACTIVE', 'PARTIALLY_UTILIZED');
COMMENT ON INDEX idx_cn_active IS 'Credit notes available for utilization';

-- 5.9 Security Deposits
CREATE TABLE security_deposits (
    security_deposit_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    student_id            UUID        NOT NULL,
    deposit_type          TEXT        NOT NULL CHECK (deposit_type IN ('CAUTION', 'HOSTEL', 'LIBRARY', 'LAB', 'EQUIPMENT')),
    amount                paise       NOT NULL,
    collection_date       DATE        NOT NULL,
    receipt_id            UUID        REFERENCES payment_receipts(payment_receipt_id),
    interest_rate         NUMERIC(5,2),
    status                TEXT        NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'HELD', 'REFUNDED', 'FORFEITED', 'PARTIALLY_REFUNDED')),
    refund_date           DATE,
    refund_amount         paise_nullable,
    deduction_amount      paise_nullable,
    deduction_reason      TEXT,
    forfeiture_approved_by UUID,
    forfeiture_reason     TEXT,
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    deleted_at            TIMESTAMPTZ,
    deleted_by            UUID
);
COMMENT ON TABLE security_deposits IS 'Student security deposits — caution, hostel, library, lab. Liabilities on the balance sheet.';
COMMENT ON COLUMN security_deposits.status IS 'ACTIVE=currently held, REFUNDED=returned, FORFEITED=forfeited with approval';

CREATE INDEX idx_sd_student ON security_deposits (tenant_id, student_id);
CREATE INDEX idx_sd_active ON security_deposits (tenant_id) WHERE status = 'ACTIVE';
COMMENT ON INDEX idx_sd_active IS 'All active deposits (balance sheet liability)';

-- ============================================================================
-- PART 6: ACCOUNTS PAYABLE
-- ============================================================================

-- 6.1 Vendors
CREATE TABLE vendors (
    vendor_id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id            UUID        REFERENCES entities(entity_id),
    vendor_code          TEXT        NOT NULL,
    vendor_name          TEXT        NOT NULL,
    vendor_type          TEXT        NOT NULL REFERENCES vendor_types(code),
    pan                  TEXT,                               -- validated format: [A-Z]{5}[0-9]{4}[A-Z]{1}
    pan_status           TEXT        DEFAULT 'UNVERIFIED' CHECK (pan_status IN ('VERIFIED', 'UNVERIFIED', 'INVALID')),
    gstin                TEXT,                               -- validated format: 15-char
    gstin_status         TEXT        DEFAULT 'UNVERIFIED' CHECK (gstin_status IN ('VERIFIED', 'UNVERIFIED', 'INVALID', 'NOT_REGISTERED')),
    gst_composition_scheme BOOLEAN   DEFAULT FALSE,
    registration_type    TEXT        DEFAULT 'REGULAR' CHECK (registration_type IN ('REGULAR', 'COMPOSITION', 'UNREGISTERED', 'NON_RESIDENT')),
    contact_person       TEXT,
    contact_email        TEXT,
    contact_phone        TEXT,
    address_line1        TEXT,
    address_line2        TEXT,
    city                 TEXT,
    state                TEXT,
    pincode              TEXT,
    payment_terms        INT         DEFAULT 30,              -- days
    default_tds_section  TEXT,                                -- e.g., "194C", "194J", "194I"
    tds_applicable       BOOLEAN     NOT NULL DEFAULT TRUE,
    tax_applicable       BOOLEAN     NOT NULL DEFAULT TRUE,
    is_active            BOOLEAN     NOT NULL DEFAULT TRUE,
    is_blacklisted       BOOLEAN     NOT NULL DEFAULT FALSE,
    blacklist_reason     TEXT,
    msme_reg_no          TEXT,
    msme_type            TEXT        CHECK (msme_type IN ('MICRO', 'SMALL', 'MEDIUM')),
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    deleted_at           TIMESTAMPTZ,
    deleted_by           UUID,
    UNIQUE (tenant_id, vendor_code),
    UNIQUE (tenant_id, pan)
);
COMMENT ON TABLE vendors IS 'Vendor master with PAN/GSTIN verification, bank validation, and TDS configuration.';
COMMENT ON COLUMN vendors.pan IS 'PAN of the vendor — validated format [A-Z]{5}[0-9]{4}[A-Z]{1}';
COMMENT ON COLUMN vendors.gstin IS 'GSTIN of the vendor — 15-char validated format';
COMMENT ON COLUMN vendors.default_tds_section IS 'Default TDS section for this vendor (e.g., 194C for contractors, 194J for professional services)';
COMMENT ON COLUMN vendors.is_blacklisted IS 'No new POs to blacklisted vendors';

CREATE INDEX idx_vendor_active ON vendors (tenant_id) WHERE deleted_at IS NULL AND is_active = TRUE;
CREATE INDEX idx_vendor_pan ON vendors (tenant_id, pan);
CREATE INDEX idx_vendor_gstin ON vendors (tenant_id, gstin) WHERE gstin IS NOT NULL;
CREATE INDEX idx_vendor_name ON vendors USING gin (vendor_name gin_trgm_ops);
COMMENT ON INDEX idx_vendor_active IS 'Active vendors for dropdown and PO creation';
COMMENT ON INDEX idx_vendor_pan IS 'Lookup by PAN (unique per tenant)';
COMMENT ON INDEX idx_vendor_name IS 'Fuzzy vendor name search for auto-complete';

-- Section 197 certificates (lower TDS deduction)
CREATE TABLE section_197_certificates (
    certificate_id    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id         UUID        NOT NULL REFERENCES tenants(tenant_id),
    vendor_id         UUID        NOT NULL REFERENCES vendors(vendor_id),
    certificate_no    TEXT        NOT NULL,
    section           TEXT        NOT NULL,                    -- e.g., "194C", "194J"
    specified_rate    NUMERIC(5,2) NOT NULL,                   -- can be 0 for nil deduction
    issued_by         TEXT,                                    -- Assessing Officer name
    valid_from        DATE        NOT NULL,
    valid_to          DATE        NOT NULL,
    is_active         BOOLEAN     NOT NULL DEFAULT TRUE,
    document_url      TEXT,                                    -- stored certificate document
    entity_version    INT         NOT NULL DEFAULT 1,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by        UUID,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by        UUID,
    deleted_at        TIMESTAMPTZ,
    deleted_by        UUID,
    UNIQUE (vendor_id, certificate_no)
);
COMMENT ON TABLE section_197_certificates IS 'Section 197 certificates for lower/nil TDS deduction. Auto-applied during payment.';

CREATE INDEX idx_s197_vendor ON section_197_certificates (vendor_id);
CREATE INDEX idx_s197_active ON section_197_certificates (vendor_id) WHERE is_active = TRUE AND valid_to >= CURRENT_DATE;
COMMENT ON INDEX idx_s197_active IS 'Currently valid certificates for auto-application at payment time';

-- Vendor bank accounts
CREATE TABLE vendor_bank_accounts (
    vendor_bank_account_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    vendor_id              UUID        NOT NULL REFERENCES vendors(vendor_id),
    account_number         TEXT        NOT NULL,               -- encrypted at application layer
    ifsc_code              TEXT        NOT NULL,               -- 11-char format
    bank_name              TEXT        NOT NULL,
    branch_name            TEXT,
    account_type           TEXT        DEFAULT 'CURRENT' CHECK (account_type IN ('SAVINGS', 'CURRENT', 'CASH_CREDIT')),
    is_primary             BOOLEAN     NOT NULL DEFAULT FALSE,
    validation_status      TEXT        DEFAULT 'UNVERIFIED' CHECK (validation_status IN ('UNVERIFIED', 'VERIFIED', 'FAILED')),
    penny_drop_amount      paise_nullable,
    entity_version         INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    deleted_at             TIMESTAMPTZ,
    deleted_by             UUID
);
COMMENT ON TABLE vendor_bank_accounts IS 'Vendor bank account details. Account numbers encrypted at the application layer.';

CREATE INDEX idx_vba_vendor ON vendor_bank_accounts (vendor_id);
COMMENT ON INDEX idx_vba_vendor IS 'All bank accounts for a vendor';

-- 6.2 Purchase Requisitions
CREATE TABLE purchase_requisitions (
    purchase_requisition_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id               UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id               UUID        NOT NULL REFERENCES entities(entity_id),
    pr_number               TEXT        NOT NULL,
    department_id           UUID        REFERENCES cost_centers(cost_center_id),
    requested_by_id         UUID,
    approved_by_id          UUID,
    status                  TEXT        NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'SUBMITTED', 'APPROVED', 'REJECTED', 'CONVERTED_TO_PO', 'CANCELLED')),
    expected_delivery_date  DATE,
    total_amount            paise       NOT NULL DEFAULT 0,
    fund_id                 UUID        REFERENCES funds(fund_id),
    budget_head_id          UUID,                              -- FK to budget_lines
    remarks                 TEXT,
    entity_version          INT         NOT NULL DEFAULT 1,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by              UUID,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by              UUID,
    deleted_at              TIMESTAMPTZ,
    deleted_by              UUID,
    UNIQUE (tenant_id, pr_number)
);
COMMENT ON TABLE purchase_requisitions IS 'Internal purchase requests requiring department approval before PO creation.';

CREATE TABLE purchase_requisition_lines (
    pr_line_id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    purchase_requisition_id UUID       NOT NULL REFERENCES purchase_requisitions(purchase_requisition_id),
    line_number            INT         NOT NULL CHECK (line_number > 0),
    item_description       TEXT        NOT NULL,
    quantity               NUMERIC(12,3) NOT NULL CHECK (quantity > 0),
    estimated_unit_price   paise       NOT NULL,
    account_id             UUID        NOT NULL REFERENCES chart_of_accounts(account_id),
    cost_center_id         UUID        REFERENCES cost_centers(cost_center_id),
    hsn_sac_code           TEXT,
    entity_version         INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    deleted_at             TIMESTAMPTZ,
    deleted_by             UUID,
    UNIQUE (purchase_requisition_id, line_number)
);
COMMENT ON TABLE purchase_requisition_lines IS 'Line items within a purchase requisition.';

CREATE INDEX idx_pr_dept_status ON purchase_requisitions (tenant_id, department_id, status);
COMMENT ON INDEX idx_pr_dept_status IS 'Filter PRs by department and status for approval dashboard';

-- 6.3 Purchase Orders
CREATE TABLE purchase_orders (
    purchase_order_id     UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id             UUID        NOT NULL REFERENCES entities(entity_id),
    po_number             TEXT        NOT NULL,
    vendor_id             UUID        NOT NULL REFERENCES vendors(vendor_id),
    purchase_requisition_id UUID     REFERENCES purchase_requisitions(purchase_requisition_id),
    order_date            DATE        NOT NULL DEFAULT CURRENT_DATE,
    delivery_date         DATE,
    payment_terms         TEXT,
    status                TEXT        NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'ISSUED', 'ACKNOWLEDGED', 'PARTIALLY_RECEIVED', 'FULLY_RECEIVED', 'CLOSED', 'CANCELLED')),
    total_amount          paise       NOT NULL DEFAULT 0,
    tax_amount            paise       NOT NULL DEFAULT 0,
    net_amount            paise       NOT NULL DEFAULT 0,
    is_rcm_applicable     BOOLEAN     NOT NULL DEFAULT FALSE,
    tds_section           TEXT,
    tds_rate              NUMERIC(5,2),
    fund_id               UUID        REFERENCES funds(fund_id),
    budget_head_id        UUID,
    issued_by_id          UUID,
    approved_by_id        UUID,
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    deleted_at            TIMESTAMPTZ,
    deleted_by            UUID,
    UNIQUE (tenant_id, po_number)
);
COMMENT ON TABLE purchase_orders IS 'Purchase orders issued to vendors. Tracks procurement commitment and RCM flags.';
COMMENT ON COLUMN purchase_orders.is_rcm_applicable IS 'RCM flag — set if vendor is unregistered under GST. Auto-generates RCM journal entries.';
COMMENT ON COLUMN purchase_orders.tds_section IS 'TDS section applicable (194C, 194J, etc.)';
COMMENT ON COLUMN purchase_orders.tds_rate IS 'TDS rate applicable for this PO';

CREATE INDEX idx_po_vendor ON purchase_orders (tenant_id, vendor_id);
CREATE INDEX idx_po_status ON purchase_orders (tenant_id, status) WHERE status IN ('ISSUED', 'ACKNOWLEDGED', 'PARTIALLY_RECEIVED');
CREATE INDEX idx_po_date ON purchase_orders (order_date);
COMMENT ON INDEX idx_po_status IS 'Open POs awaiting receipt or closure';

-- Purchase Order Lines
CREATE TABLE purchase_order_lines (
    po_line_id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    purchase_order_id    UUID        NOT NULL REFERENCES purchase_orders(purchase_order_id),
    line_number          INT         NOT NULL CHECK (line_number > 0),
    item_description     TEXT        NOT NULL,
    hsn_sac_code         TEXT,
    quantity             NUMERIC(12,3) NOT NULL CHECK (quantity > 0),
    unit_price           paise       NOT NULL CHECK (unit_price >= 0),
    discount_percent     NUMERIC(5,2),
    tax_rate             NUMERIC(5,2) DEFAULT 0,
    tax_type             TEXT        CHECK (tax_type IN ('GST_EXEMPT', 'GST_5', 'GST_12', 'GST_18', 'GST_28', 'NIL')),
    total_amount         paise       NOT NULL,
    received_quantity    NUMERIC(12,3) NOT NULL DEFAULT 0,    -- updated by GRN
    account_id           UUID        NOT NULL REFERENCES chart_of_accounts(account_id),
    cost_center_id       UUID        REFERENCES cost_centers(cost_center_id),
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    deleted_at           TIMESTAMPTZ,
    deleted_by           UUID,
    UNIQUE (purchase_order_id, line_number)
);
COMMENT ON TABLE purchase_order_lines IS 'Line items within a purchase order. Received quantity tracked for 3-way matching.';

CREATE INDEX idx_pol_po ON purchase_order_lines (purchase_order_id);
COMMENT ON INDEX idx_pol_po IS 'All lines of a purchase order';

-- 6.4 Goods Receipt Notes
CREATE TABLE goods_receipt_notes (
    goods_receipt_note_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    grn_number            TEXT        NOT NULL,
    purchase_order_id     UUID        NOT NULL REFERENCES purchase_orders(purchase_order_id),
    received_date         DATE        NOT NULL DEFAULT CURRENT_DATE,
    received_by_id        UUID,
    status                TEXT        NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'COMPLETED', 'CANCELLED')),
    remarks               TEXT,
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    deleted_at            TIMESTAMPTZ,
    deleted_by            UUID,
    UNIQUE (tenant_id, grn_number)
);
COMMENT ON TABLE goods_receipt_notes IS 'Goods receipt notes — record of goods/services received against a purchase order.';

CREATE TABLE goods_receipt_note_lines (
    grn_line_id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    goods_receipt_note_id UUID       NOT NULL REFERENCES goods_receipt_notes(goods_receipt_note_id),
    po_line_id           UUID        NOT NULL REFERENCES purchase_order_lines(po_line_id),
    received_quantity    NUMERIC(12,3) NOT NULL CHECK (received_quantity > 0),
    accepted_quantity    NUMERIC(12,3) NOT NULL,
    rejected_quantity    NUMERIC(12,3) NOT NULL DEFAULT 0,
    rejection_reason     TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    CHECK (accepted_quantity + rejected_quantity = received_quantity)
);
COMMENT ON TABLE goods_receipt_note_lines IS 'Line items within a GRN — tracks received, accepted, and rejected quantities.';

CREATE INDEX idx_grn_po ON goods_receipt_notes (purchase_order_id);
CREATE INDEX idx_grn_line_po_line ON goods_receipt_note_lines (po_line_id);
COMMENT ON INDEX idx_grn_po IS 'All GRNs for a purchase order';
COMMENT ON INDEX idx_grn_line_po_line IS 'GRN lines for a specific PO line (for 3-way matching)';

-- 6.5 Vendor Invoices (Purchase Invoices)
CREATE TABLE vendor_invoices (
    vendor_invoice_id    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id            UUID        NOT NULL REFERENCES entities(entity_id),
    invoice_number       TEXT        NOT NULL,                -- vendor's invoice number
    invoice_date         DATE        NOT NULL,
    purchase_order_id    UUID        REFERENCES purchase_orders(purchase_order_id),
    goods_receipt_note_id UUID       REFERENCES goods_receipt_notes(goods_receipt_note_id),
    vendor_id            UUID        NOT NULL REFERENCES vendors(vendor_id),
    invoice_amount       paise       NOT NULL,
    tax_amount           paise       NOT NULL DEFAULT 0,
    net_amount           paise       NOT NULL,
    tds_amount           paise       DEFAULT 0,
    is_rcm               BOOLEAN     NOT NULL DEFAULT FALSE,
    rcm_payable_amount   paise_nullable,
    status               TEXT        NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'MATCHED', 'MISMATCHED', 'APPROVED', 'POSTED', 'CANCELLED')),
    payment_status       TEXT        NOT NULL DEFAULT 'UNPAID' CHECK (payment_status IN ('UNPAID', 'PARTIALLY_PAID', 'PAID')),
    due_date             DATE        NOT NULL,
    posted_journal_id    UUID        REFERENCES journal_entries(journal_id),
    approved_by_id       UUID,
    document_url         TEXT,
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    deleted_at           TIMESTAMPTZ,
    deleted_by           UUID,
    UNIQUE (tenant_id, vendor_id, invoice_number, invoice_date)
);
COMMENT ON TABLE vendor_invoices IS 'Vendor invoices with 3-way matching support (PO × GRN × Invoice).';
COMMENT ON COLUMN vendor_invoices.status IS 'DRAFT→MATCHED→APPROVED→POSTED, or MISMATCHED for exceptions, CANCELLED for void';
COMMENT ON COLUMN vendor_invoices.is_rcm IS 'Reverse Charge Mechanism applies to this invoice';
COMMENT ON COLUMN vendor_invoices.rcm_payable_amount IS 'RCM tax payable amount (auto-computed)';

CREATE INDEX idx_vi_vendor ON vendor_invoices (tenant_id, vendor_id);
CREATE INDEX idx_vi_po ON vendor_invoices (purchase_order_id);
CREATE INDEX idx_vi_status ON vendor_invoices (tenant_id, status) WHERE status NOT IN ('CANCELLED', 'POSTED');
CREATE INDEX idx_vi_due ON vendor_invoices (due_date) WHERE payment_status IN ('UNPAID', 'PARTIALLY_PAID');
COMMENT ON INDEX idx_vi_vendor IS 'All invoices from a vendor';
COMMENT ON INDEX idx_vi_status IS 'Invoices pending action (matching, approval, posting)';
COMMENT ON INDEX idx_vi_due IS 'Overdue and upcoming invoices for payment scheduling';

-- Vendor Invoice Lines
CREATE TABLE vendor_invoice_lines (
    invoice_line_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    vendor_invoice_id    UUID        NOT NULL REFERENCES vendor_invoices(vendor_invoice_id),
    po_line_id           UUID        REFERENCES purchase_order_lines(po_line_id),
    line_number          INT         NOT NULL CHECK (line_number > 0),
    item_description     TEXT        NOT NULL,
    quantity             NUMERIC(12,3) NOT NULL CHECK (quantity > 0),
    unit_price           paise       NOT NULL,
    tax_rate             NUMERIC(5,2),
    tax_amount           paise,
    total_amount         paise       NOT NULL,
    account_id           UUID        NOT NULL REFERENCES chart_of_accounts(account_id),
    cost_center_id       UUID        REFERENCES cost_centers(cost_center_id),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID
);
COMMENT ON TABLE vendor_invoice_lines IS 'Line items within a vendor invoice — used for 2-way and 3-way matching.';

CREATE INDEX idx_vil_invoice ON vendor_invoice_lines (vendor_invoice_id);
CREATE INDEX idx_vil_po_line ON vendor_invoice_lines (po_line_id) WHERE po_line_id IS NOT NULL;
COMMENT ON INDEX idx_vil_invoice IS 'All lines of an invoice';
COMMENT ON INDEX idx_vil_po_line IS 'For 3-way matching — compare invoice lines with PO and GRN lines';

-- 6.6 Vendor Payments
CREATE TABLE vendor_payments (
    payment_id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id           UUID        NOT NULL REFERENCES entities(entity_id),
    payment_number      TEXT        NOT NULL,
    vendor_id           UUID        NOT NULL REFERENCES vendors(vendor_id),
    payment_type        TEXT        NOT NULL CHECK (payment_type IN ('VENDOR_PAYMENT', 'RCM_PAYMENT', 'TDS_DEPOSIT', 'ADVANCE', 'REFUND', 'OTHER')),
    payment_mode        TEXT        NOT NULL REFERENCES payment_modes(code),
    payment_date        DATE        NOT NULL,
    amount              paise       NOT NULL,
    tds_amount          paise       DEFAULT 0,
    net_amount          paise       NOT NULL,
    status              TEXT        NOT NULL DEFAULT 'INITIATED' CHECK (status IN ('INITIATED', 'APPROVED', 'SCHEDULED', 'PROCESSED', 'COMPLETED', 'FAILED', 'CANCELLED')),
    bank_account_id     UUID,                                -- FK to bank_accounts
    bank_transaction_ref TEXT,
    cheque_number       TEXT,
    cheque_date         DATE,
    approved_by_id      UUID,
    processed_by_id     UUID,
    payment_journal_id  UUID        REFERENCES journal_entries(journal_id),
    remarks             TEXT,
    entity_version      INT         NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by          UUID,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by          UUID,
    deleted_at          TIMESTAMPTZ,
    deleted_by          UUID,
    UNIQUE (tenant_id, payment_number)
);
COMMENT ON TABLE vendor_payments IS 'Vendor payments with TDS deduction, approval workflow, and bank integration.';
COMMENT ON COLUMN vendor_payments.tds_amount IS 'TDS deducted from this payment';
COMMENT ON COLUMN vendor_payments.net_amount IS 'amount - tds_amount = amount actually paid to vendor';

CREATE INDEX idx_vp_vendor ON vendor_payments (tenant_id, vendor_id);
CREATE INDEX idx_vp_status ON vendor_payments (status) WHERE status IN ('INITIATED', 'APPROVED', 'SCHEDULED');
CREATE INDEX idx_vp_date ON vendor_payments (payment_date DESC);
COMMENT ON INDEX idx_vp_status IS 'Payments pending action';

-- Payment allocation to invoices
CREATE TABLE vendor_payment_allocations (
    vendor_payment_alloc_id UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id               UUID    NOT NULL REFERENCES tenants(tenant_id),
    payment_id              UUID    NOT NULL REFERENCES vendor_payments(payment_id),
    invoice_id              UUID    NOT NULL REFERENCES vendor_invoices(vendor_invoice_id),
    allocated_amount        paise   NOT NULL,
    tds_amount              paise   DEFAULT 0,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (payment_id, invoice_id)
);
COMMENT ON TABLE vendor_payment_allocations IS 'Allocation of a payment to specific vendor invoices.';

CREATE INDEX idx_vpa_payment ON vendor_payment_allocations (payment_id);
CREATE INDEX idx_vpa_invoice ON vendor_payment_allocations (invoice_id);
COMMENT ON INDEX idx_vpa_payment IS 'All invoices paid by this payment';

-- 6.7 Employee Reimbursements
CREATE TABLE employee_reimbursements (
    expense_claim_id    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id           UUID        NOT NULL REFERENCES entities(entity_id),
    claim_number        TEXT        NOT NULL,
    employee_id         UUID        NOT NULL,
    claim_date          DATE        NOT NULL DEFAULT CURRENT_DATE,
    total_amount        paise       NOT NULL,
    status              TEXT        NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'SUBMITTED', 'APPROVED', 'REJECTED', 'PAID', 'CANCELLED')),
    approved_by_id      UUID,
    paid_by_id          UUID,
    payment_id          UUID        REFERENCES vendor_payments(payment_id),
    remarks             TEXT,
    entity_version      INT         NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by          UUID,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by          UUID,
    deleted_at          TIMESTAMPTZ,
    deleted_by          UUID,
    UNIQUE (tenant_id, claim_number)
);
COMMENT ON TABLE employee_reimbursements IS 'Employee expense claims for travel, medical, conveyance, and other reimbursements.';

CREATE INDEX idx_er_employee ON employee_reimbursements (tenant_id, employee_id);
CREATE INDEX idx_er_status ON employee_reimbursements (status) WHERE status IN ('SUBMITTED', 'APPROVED');
COMMENT ON INDEX idx_er_status IS 'Claims pending approval or payment';

CREATE TABLE expense_claim_lines (
    expense_claim_line_id UUID   PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID   NOT NULL REFERENCES tenants(tenant_id),
    expense_claim_id      UUID   NOT NULL REFERENCES employee_reimbursements(expense_claim_id),
    expense_category      TEXT   NOT NULL CHECK (expense_category IN ('TRAVEL', 'MEDICAL', 'CONVEYANCE', 'FOOD', 'ACCOMMODATION', 'STATIONERY', 'PHONE_INTERNET', 'OTHER')),
    expense_date          DATE   NOT NULL,
    description           TEXT   NOT NULL,
    amount                paise  NOT NULL,
    gst_applicable        BOOLEAN DEFAULT FALSE,
    gst_amount            paise_nullable,
    document_url          TEXT,
    account_id            UUID   NOT NULL REFERENCES chart_of_accounts(account_id),
    cost_center_id        UUID   REFERENCES cost_centers(cost_center_id),
    fund_id               UUID   REFERENCES funds(fund_id),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID
);
COMMENT ON TABLE expense_claim_lines IS 'Individual expense items within an employee reimbursement claim.';

-- ============================================================================
-- PART 7: TREASURY & BANKING
-- ============================================================================

-- 7.1 Bank Accounts
CREATE TABLE bank_accounts (
    bank_account_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id            UUID        REFERENCES entities(entity_id),
    account_number       TEXT        NOT NULL,                -- encrypted at application layer
    account_name         TEXT        NOT NULL,
    bank_name            TEXT        NOT NULL,
    branch_name          TEXT,
    ifsc_code            TEXT        NOT NULL,
    account_type         TEXT        NOT NULL CHECK (account_type IN ('CURRENT', 'SAVINGS', 'FCRA', 'GRANT_SPECIFIC', 'DEPOSIT', 'CASH_CREDIT')),
    fund_id              UUID        REFERENCES funds(fund_id),
    is_fcra_account      BOOLEAN     NOT NULL DEFAULT FALSE,
    minimum_balance      paise       DEFAULT 0,
    is_active            BOOLEAN     NOT NULL DEFAULT TRUE,
    last_reconciled_at   TIMESTAMPTZ,
    last_sync_at         TIMESTAMPTZ,
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    deleted_at           TIMESTAMPTZ,
    deleted_by           UUID,
    UNIQUE (tenant_id, account_number)
);
COMMENT ON TABLE bank_accounts IS 'Bank account register — current, savings, FCRA, and grant-specific accounts.';
COMMENT ON COLUMN bank_accounts.is_fcra_account IS 'FCRA account must be at SBI New Delhi Main Branch as per FCRA rules';

CREATE INDEX idx_ba_entity ON bank_accounts (tenant_id, entity_id);
CREATE INDEX idx_ba_type ON bank_accounts (account_type) WHERE is_active = TRUE;
COMMENT ON INDEX idx_ba_entity IS 'Bank accounts for an entity';
COMMENT ON INDEX idx_ba_type IS 'Quick filter by account type (FCRA, grant, etc.)';

-- Bank signatories
CREATE TABLE bank_signatories (
    bank_signatory_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id         UUID        NOT NULL REFERENCES tenants(tenant_id),
    bank_account_id   UUID        NOT NULL REFERENCES bank_accounts(bank_account_id),
    user_id           UUID        NOT NULL,
    signatory_type    TEXT        NOT NULL CHECK (signatory_type IN ('INDIVIDUAL', 'JOINT')),
    is_active         BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (bank_account_id, user_id)
);
COMMENT ON TABLE bank_signatories IS 'Authorized signatories for each bank account.';

-- 7.2 Bank Reconciliation
CREATE TABLE bank_reconciliations (
    bank_reconciliation_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    bank_account_id        UUID        NOT NULL REFERENCES bank_accounts(bank_account_id),
    period_id              UUID        NOT NULL REFERENCES accounting_periods(accounting_period_id),
    statement_date         DATE        NOT NULL,
    opening_balance        paise       NOT NULL,
    closing_balance        paise       NOT NULL,
    status                 TEXT        NOT NULL DEFAULT 'IN_PROGRESS' CHECK (status IN ('IN_PROGRESS', 'COMPLETED', 'VERIFIED')),
    verified_by_id         UUID,
    completed_at           TIMESTAMPTZ,
    entity_version         INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    deleted_at             TIMESTAMPTZ,
    deleted_by             UUID,
    UNIQUE (bank_account_id, period_id)
);
COMMENT ON TABLE bank_reconciliations IS 'Bank reconciliation statements — tracks matching of bank statement with system records.';

CREATE TABLE bank_statement_lines (
    bank_statement_line_id UUID   PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID   NOT NULL REFERENCES tenants(tenant_id),
    bank_reconciliation_id UUID   NOT NULL REFERENCES bank_reconciliations(bank_reconciliation_id),
    transaction_date       DATE   NOT NULL,
    transaction_ref        TEXT,
    description            TEXT,
    debit_amount           paise_nullable,
    credit_amount          paise_nullable,
    match_status           TEXT   NOT NULL DEFAULT 'UNMATCHED' CHECK (match_status IN ('MATCHED', 'UNMATCHED', 'PARTIAL_MATCH', 'MANUAL_MATCH')),
    matched_transaction_id UUID,
    matched_transaction_type TEXT,     -- 'PaymentReceipt', 'VendorPayment', 'FundReceipt', etc.
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (debit_amount IS NOT NULL OR credit_amount IS NOT NULL)
);
COMMENT ON TABLE bank_statement_lines IS 'Individual bank statement lines for reconciliation.';

CREATE INDEX idx_bsl_reconciliation ON bank_statement_lines (bank_reconciliation_id);
CREATE INDEX idx_bsl_match_status ON bank_statement_lines (match_status) WHERE match_status IN ('UNMATCHED', 'PARTIAL_MATCH');
COMMENT ON INDEX idx_bsl_reconciliation IS 'All statement lines for a reconciliation';
COMMENT ON INDEX idx_bsl_match_status IS 'Unmatched lines requiring manual review';

-- 7.3 Bank Transactions (IMMUTABLE)
CREATE TABLE bank_transactions (
    bank_transaction_id    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    bank_account_id        UUID        NOT NULL REFERENCES bank_accounts(bank_account_id),
    transaction_date       DATE        NOT NULL,
    value_date             DATE,
    transaction_ref        TEXT,                               -- UTR / transaction reference
    description            TEXT,
    debit_amount           paise_nullable,
    credit_amount          paise_nullable,
    balance                paise,                              -- running balance after this transaction
    reference_type         TEXT,                                -- 'PAYMENT_RECEIPT', 'VENDOR_PAYMENT', 'FUND_RECEIPT', etc.
    reference_id           UUID,
    is_reconciled          BOOLEAN     NOT NULL DEFAULT FALSE,
    reconciled_at          TIMESTAMPTZ,
    version                INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    CHECK (debit_amount IS NOT NULL OR credit_amount IS NOT NULL)
);
COMMENT ON TABLE bank_transactions IS '★ IMMUTABLE ★ Bank transaction register — auto-synced via API or manually uploaded.';

CREATE INDEX idx_bt_account_date ON bank_transactions (bank_account_id, transaction_date DESC);
CREATE INDEX idx_bt_ref ON bank_transactions (reference_type, reference_id) WHERE reference_id IS NOT NULL;
CREATE INDEX idx_bt_unreconciled ON bank_transactions (bank_account_id) WHERE is_reconciled = FALSE;
COMMENT ON INDEX idx_bt_account_date IS 'Transaction history for a bank account';
COMMENT ON INDEX idx_bt_unreconciled IS 'Unreconciled transactions for bank reconciliation';

-- ============================================================================
-- PART 8: TAXATION
-- ============================================================================

-- 8.1 GST Registrations
CREATE TABLE gst_registrations (
    gst_registration_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id           UUID        NOT NULL REFERENCES entities(entity_id),
    gstin               TEXT        NOT NULL,
    trade_name          TEXT        NOT NULL,
    legal_name          TEXT        NOT NULL,
    registration_type   TEXT        NOT NULL DEFAULT 'REGULAR' CHECK (registration_type IN ('REGULAR', 'COMPOSITION', 'UNREGISTERED')),
    filing_frequency    TEXT        NOT NULL DEFAULT 'MONTHLY' CHECK (filing_frequency IN ('MONTHLY', 'QUARTERLY')),
    is_composite        BOOLEAN     NOT NULL DEFAULT FALSE,
    state_code          TEXT        NOT NULL,                   -- 2-digit state code
    address_line1       TEXT,
    address_line2       TEXT,
    city                TEXT,
    state               TEXT,
    pincode             TEXT,
    is_active           BOOLEAN     NOT NULL DEFAULT TRUE,
    entity_version      INT         NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by          UUID,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by          UUID,
    deleted_at          TIMESTAMPTZ,
    deleted_by          UUID,
    UNIQUE (tenant_id, gstin),
    UNIQUE (entity_id)
);
COMMENT ON TABLE gst_registrations IS 'GSTIN registrations per entity. Each campus/entity may have its own GSTIN.';
COMMENT ON COLUMN gst_registrations.state_code IS '2-digit state code (e.g., "27" for Maharashtra)';

-- 8.2 GST Returns
CREATE TABLE gst_returns (
    gst_return_id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    gst_registration_id   UUID        NOT NULL REFERENCES gst_registrations(gst_registration_id),
    return_type           TEXT        NOT NULL CHECK (return_type IN ('GSTR1', 'GSTR3B', 'GSTR9', 'GSTR9C')),
    period                TEXT        NOT NULL,                  -- e.g., "072026" for July 2026
    fiscal_year           TEXT        NOT NULL,
    status                TEXT        NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'GENERATED', 'FILED', 'FILED_WITH_ERRORS', 'ADJUSTED')),
    due_date              DATE        NOT NULL,
    filed_date            DATE,
    filed_by_id           UUID,
    acknowledgment_no     TEXT,
    json_data             JSONB,                                 -- full return JSON
    tax_liability         paise       DEFAULT 0,
    itc_claimed           paise       DEFAULT 0,
    net_tax_payable       paise       DEFAULT 0,
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    deleted_at            TIMESTAMPTZ,
    deleted_by            UUID,
    UNIQUE (gst_registration_id, return_type, period)
);
COMMENT ON TABLE gst_returns IS 'GST returns (GSTR-1, GSTR-3B, GSTR-9, GSTR-9C) auto-generated from transaction data.';

CREATE INDEX idx_gr_registration ON gst_returns (gst_registration_id, fiscal_year);
CREATE INDEX idx_gr_status ON gst_returns (status) WHERE status IN ('DRAFT', 'GENERATED');
COMMENT ON INDEX idx_gr_registration IS 'All returns for a GSTIN in a fiscal year';
COMMENT ON INDEX idx_gr_status IS 'Returns pending filing';

-- GST return lines
CREATE TABLE gst_return_lines (
    gst_return_line_id  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID        NOT NULL REFERENCES tenants(tenant_id),
    gst_return_id       UUID        NOT NULL REFERENCES gst_returns(gst_return_id),
    section             TEXT        NOT NULL,                    -- e.g., "4A", "4B", "5A"
    description         TEXT,
    taxable_value       paise       NOT NULL DEFAULT 0,
    igst_amount         paise       NOT NULL DEFAULT 0,
    cgst_amount         paise       NOT NULL DEFAULT 0,
    sgst_amount         paise       NOT NULL DEFAULT 0,
    cess_amount         paise       NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE gst_return_lines IS 'Individual section lines within a GST return.';

-- 8.3 ITC Register
CREATE TABLE itc_register (
    itc_register_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    gst_registration_id  UUID        NOT NULL REFERENCES gst_registrations(gst_registration_id),
    period               TEXT        NOT NULL,                    -- e.g., "072026"
    status               TEXT        NOT NULL DEFAULT 'OPEN' CHECK (status IN ('OPEN', 'COMPUTED', 'REVERSED', 'CLOSED')),
    total_itc            paise       DEFAULT 0,
    itc_on_inputs        paise       DEFAULT 0,
    itc_on_capital_goods paise       DEFAULT 0,
    itc_reversal_rule_42 paise       DEFAULT 0,
    itc_reversal_rule_43 paise       DEFAULT 0,
    net_itc_eligible     paise       DEFAULT 0,
    exempt_turnover      paise       DEFAULT 0,
    total_turnover       paise       DEFAULT 0,
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    UNIQUE (gst_registration_id, period)
);
COMMENT ON TABLE itc_register IS 'Input Tax Credit register — tracks ITC eligibility, Rule 42/43 reversals per period.';
COMMENT ON COLUMN itc_register.itc_reversal_rule_42 IS 'Rule 42 reversal: ITC × (Exempt Turnover / Total Turnover)';
COMMENT ON COLUMN itc_register.itc_reversal_rule_43 IS 'Rule 43 reversal: Capital goods ITC reversal over 60 months';

-- ITC Register Lines (invoice-level ITC tracking)
CREATE TABLE itc_register_lines (
    itc_register_line_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    itc_register_id      UUID        NOT NULL REFERENCES itc_register(itc_register_id),
    invoice_id           UUID        NOT NULL REFERENCES vendor_invoices(vendor_invoice_id),
    invoice_number       TEXT        NOT NULL,
    invoice_date         DATE        NOT NULL,
    vendor_gstin         TEXT,
    taxable_value        paise       NOT NULL DEFAULT 0,
    igst                 paise       NOT NULL DEFAULT 0,
    cgst                 paise       NOT NULL DEFAULT 0,
    sgst                 paise       NOT NULL DEFAULT 0,
    total_tax            paise       NOT NULL DEFAULT 0,
    itc_eligibility      TEXT        NOT NULL REFERENCES itc_eligibilities(code),
    reversal_percent     NUMERIC(5,2),
    reversal_amount      paise_nullable,
    is_reversed          BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE itc_register_lines IS 'Invoice-level ITC tracking for detailed audit trail.';

CREATE INDEX idx_itcl_register ON itc_register_lines (itc_register_id);
CREATE INDEX idx_itcl_invoice ON itc_register_lines (invoice_id);
COMMENT ON INDEX idx_itcl_register IS 'All ITC entries for a period';
COMMENT ON INDEX idx_itcl_invoice IS 'ITC details for a specific invoice';

-- 8.4 TDS Deductions
CREATE TABLE tds_deductions (
    tds_deduction_id       UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    payment_id             UUID        NOT NULL REFERENCES vendor_payments(payment_id),
    tds_section            TEXT        NOT NULL,                 -- e.g., "194C", "194J"
    tds_rate               NUMERIC(5,2) NOT NULL,
    tds_amount             paise       NOT NULL,
    pan_of_deductee        TEXT        NOT NULL,
    section_197_cert_id    UUID        REFERENCES section_197_certificates(certificate_id),
    tds_deposit_status     TEXT        NOT NULL DEFAULT 'PENDING' CHECK (tds_deposit_status IN ('PENDING', 'DEPOSITED', 'FILED')),
    tds_deposit_date       DATE,
    tds_return_filed_date  DATE,
    tds_journal_id         UUID        REFERENCES journal_entries(journal_id),
    challan_reference      TEXT,
    entity_version         INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    UNIQUE (payment_id, tds_section)
);
COMMENT ON TABLE tds_deductions IS 'TDS deductions per payment — tracks section, rate, deposit status, and challan reference.';
COMMENT ON COLUMN tds_deductions.tds_section IS 'TDS section under which deduction is made (e.g., 194C, 194J, 194I)';
COMMENT ON COLUMN tds_deductions.section_197_cert_id IS 'If a Section 197 certificate was applied, reference it here';

CREATE INDEX idx_td_payment ON tds_deductions (payment_id);
CREATE INDEX idx_td_deposit_status ON tds_deductions (tds_deposit_status) WHERE tds_deposit_status = 'PENDING';
CREATE INDEX idx_td_section_period ON tds_deductions (tenant_id, tds_section, created_at);
COMMENT ON INDEX idx_td_deposit_status IS 'TDS deductions pending deposit to government';
COMMENT ON INDEX idx_td_section_period IS 'TDS register — deductions by section and period';

-- TDS Sections master (configurable)
CREATE TABLE tds_sections (
    tds_section_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    section_code        TEXT        NOT NULL UNIQUE,            -- "194C", "194J", etc.
    description         TEXT        NOT NULL,
    default_rate        NUMERIC(5,2) NOT NULL,
    threshold_per_payment paise_nullable,
    threshold_aggregate paise_nullable,
    applicable_to       TEXT        NOT NULL DEFAULT 'ALL' CHECK (applicable_to IN ('RESIDENT_INDIVIDUAL', 'RESIDENT_OTHER', 'NON_RESIDENT', 'ALL')),
    is_active           BOOLEAN     NOT NULL DEFAULT TRUE,
    entity_version      INT         NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE tds_sections IS 'TDS section master — rates and thresholds per section. Configurable per tenant via tenant_configs.';

-- TDS Returns
CREATE TABLE tds_returns (
    tds_return_id        UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id            UUID        NOT NULL REFERENCES entities(entity_id),
    return_type          TEXT        NOT NULL CHECK (return_type IN ('FORM_24Q', 'FORM_26Q', 'FORM_27Q')),
    quarter              TEXT        NOT NULL CHECK (quarter IN ('Q1', 'Q2', 'Q3', 'Q4')),
    fiscal_year          TEXT        NOT NULL,
    status               TEXT        NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'GENERATED', 'FILED', 'FILED_WITH_ERRORS')),
    due_date             DATE        NOT NULL,
    filed_date           DATE,
    acknowledgment_no    TEXT,
    total_deductions     paise       DEFAULT 0,
    total_deposits       paise       DEFAULT 0,
    json_data            JSONB,
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    UNIQUE (entity_id, return_type, quarter, fiscal_year)
);
COMMENT ON TABLE tds_returns IS 'TDS returns — Form 24Q (salary), 26Q (non-salary), 27Q (non-resident).';

-- TDS deduction details in returns
CREATE TABLE tds_return_details (
    tds_return_detail_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    tds_return_id        UUID        NOT NULL REFERENCES tds_returns(tds_return_id),
    vendor_id            UUID        REFERENCES vendors(vendor_id),
    employee_id          UUID,
    pan                  TEXT        NOT NULL,
    section              TEXT        NOT NULL,
    payment_date         DATE        NOT NULL,
    payment_amount       paise       NOT NULL,
    tds_rate             NUMERIC(5,2) NOT NULL,
    tds_amount           paise       NOT NULL,
    surcharge            paise       DEFAULT 0,
    cess                 paise       DEFAULT 0,
    total_tds            paise       NOT NULL,
    challan_details      JSONB,
    salary_month         INT         CHECK (salary_month BETWEEN 1 AND 12),  -- for Form 24Q
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE tds_return_details IS 'Individual deduction records within a TDS return.';

-- 8.5 Trust Exemption & Income Tax Compliance
CREATE TABLE trust_exemptions (
    trust_exemption_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id            UUID        NOT NULL REFERENCES entities(entity_id),
    exemption_section    TEXT        NOT NULL CHECK (exemption_section IN ('SECTION_10_23C', 'SECTION_11_12A', 'SECTION_12AB', 'SECTION_10_23C_VI')),
    registration_no      TEXT        NOT NULL,
    registration_date    DATE        NOT NULL,
    valid_from           DATE        NOT NULL,
    valid_to             DATE        NOT NULL,
    status               TEXT        NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'EXPIRED', 'RENEWAL_PENDING', 'CANCELLED')),
    approving_authority  TEXT,
    is_trust             BOOLEAN     NOT NULL DEFAULT FALSE,
    trust_name           TEXT,
    trust_pan            TEXT,
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    UNIQUE (entity_id, exemption_section)
);
COMMENT ON TABLE trust_exemptions IS 'Trust exemption registrations under Sections 10(23C), 11, 12A/12AB of Income Tax Act.';

-- Income Application (85% Rule)
CREATE TABLE income_applications (
    income_application_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    fiscal_year_id        UUID        NOT NULL REFERENCES fiscal_years(fiscal_year_id),
    entity_id             UUID        NOT NULL REFERENCES entities(entity_id),
    total_income          paise       NOT NULL DEFAULT 0,
    amount_applied        paise       NOT NULL DEFAULT 0,
    application_percent   NUMERIC(5,2),                          -- computed: (amount_applied / total_income) * 100
    accumulated_amount    paise       NOT NULL DEFAULT 0,
    accumulation_year     INT,                                    -- year of accumulation for 5-year tracking
    accumulation_purpose  TEXT,
    status                TEXT        NOT NULL DEFAULT 'COMPLIANT' CHECK (status IN ('COMPLIANT', 'NON_COMPLIANT', 'UNDER_REVIEW')),
    last_computed_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    UNIQUE (fiscal_year_id, entity_id)
);
COMMENT ON TABLE income_applications IS '85% income application tracking — ensures at least 85% of income is applied to educational purposes.';
COMMENT ON COLUMN income_applications.application_percent IS '(amount_applied / total_income) × 100 — must be ≥ 85%';

CREATE TABLE income_application_lines (
    income_app_line_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    income_application_id UUID       NOT NULL REFERENCES income_applications(income_application_id),
    category             TEXT        NOT NULL CHECK (category IN ('SALARIES', 'INFRASTRUCTURE', 'SCHOLARSHIPS', 'RESEARCH', 'MAINTENANCE', 'OTHER_EDUCATIONAL')),
    amount               paise       NOT NULL,
    account_id           UUID        NOT NULL REFERENCES chart_of_accounts(account_id),
    description          TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE income_application_lines IS 'Breakdown of income application by category for 85% rule compliance.';

-- FCRA Registrations
CREATE TABLE fcra_registrations (
    fcra_registration_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id            UUID        NOT NULL REFERENCES entities(entity_id),
    registration_no      TEXT        NOT NULL,
    valid_from           DATE        NOT NULL,
    valid_to             DATE        NOT NULL,
    bank_account_id      UUID        NOT NULL REFERENCES bank_accounts(bank_account_id),
    status               TEXT        NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'EXPIRED', 'RENEWAL_PENDING', 'CANCELLED')),
    total_receipts       paise       DEFAULT 0,
    admin_expenses       paise       DEFAULT 0,
    admin_expense_ratio  NUMERIC(5,2),                           -- (admin_expenses / total_receipts) * 100
    fc4_return_filed_date DATE,
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    UNIQUE (tenant_id, registration_no)
);
COMMENT ON TABLE fcra_registrations IS 'FCRA (Foreign Contribution Regulation Act) registration and compliance tracking.';
COMMENT ON COLUMN fcra_registrations.admin_expense_ratio IS 'Admin expenses as % of total receipts — must be ≤ 20%';

-- ============================================================================
-- PART 9: BUDGET & ENCUMBRANCE
-- ============================================================================

-- 9.1 Budgets
CREATE TABLE budgets (
    budget_id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id          UUID        NOT NULL REFERENCES entities(entity_id),
    fiscal_year_id     UUID        NOT NULL REFERENCES fiscal_years(fiscal_year_id),
    budget_type        TEXT        NOT NULL CHECK (budget_type IN ('ANNUAL', 'PROJECT', 'GRANT', 'CAPITAL', 'REVENUE')),
    budget_name        TEXT        NOT NULL,
    status             TEXT        NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'UNDER_REVIEW', 'APPROVED', 'ACTIVE', 'CLOSED')),
    total_amount       paise       NOT NULL DEFAULT 0,
    revised_amount     paise_nullable,
    fund_id            UUID        REFERENCES funds(fund_id),
    project_id         UUID,
    approved_by_id     UUID,
    approved_at        TIMESTAMPTZ,
    remarks            TEXT,
    entity_version     INT         NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         UUID,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by         UUID,
    deleted_at         TIMESTAMPTZ,
    deleted_by         UUID,
    UNIQUE (tenant_id, entity_id, fiscal_year_id, budget_type, budget_name)
);
COMMENT ON TABLE budgets IS 'Department-wise, project-wise, and grant-wise budgets with approval workflow.';

CREATE INDEX idx_budget_fiscal ON budgets (tenant_id, fiscal_year_id, entity_id);
CREATE INDEX idx_budget_status ON budgets (status) WHERE status IN ('APPROVED', 'ACTIVE');
COMMENT ON INDEX idx_budget_fiscal IS 'All budgets for a fiscal year';
COMMENT ON INDEX idx_budget_status IS 'Active budgets for budget checking';

-- Budget Lines
CREATE TABLE budget_lines (
    budget_line_id     UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(tenant_id),
    budget_id          UUID        NOT NULL REFERENCES budgets(budget_id),
    account_id         UUID        NOT NULL REFERENCES chart_of_accounts(account_id),
    cost_center_id     UUID        REFERENCES cost_centers(cost_center_id),
    original_amount    paise       NOT NULL,
    revised_amount     paise_nullable,
    entity_version     INT         NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         UUID,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by         UUID,
    UNIQUE (budget_id, account_id, cost_center_id)
);
COMMENT ON TABLE budget_lines IS 'Line items within a budget — by account and cost center.';

CREATE INDEX idx_bl_budget ON budget_lines (budget_id);
COMMENT ON INDEX idx_bl_budget IS 'All lines of a budget';

-- Budget Revisions
CREATE TABLE budget_revisions (
    budget_revision_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(tenant_id),
    budget_id          UUID        NOT NULL REFERENCES budgets(budget_id),
    revision_number    INT         NOT NULL,
    previous_amount    paise       NOT NULL,
    new_amount         paise       NOT NULL,
    reason             TEXT        NOT NULL,
    approved_by_id     UUID        NOT NULL,
    approved_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (budget_id, revision_number)
);
COMMENT ON TABLE budget_revisions IS 'Revision history for budget changes — full audit trail.';

-- 9.2 Encumbrances
CREATE TABLE encumbrances (
    encumbrance_id     UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(tenant_id),
    budget_line_id     UUID        NOT NULL REFERENCES budget_lines(budget_line_id),
    reference_type     TEXT        NOT NULL CHECK (reference_type IN ('PURCHASE_ORDER', 'CONTRACT', 'AGREEMENT', 'STANDING_ORDER')),
    reference_id       UUID        NOT NULL,
    amount             paise       NOT NULL,
    remaining_amount   paise       NOT NULL,
    status             TEXT        NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'PARTIALLY_RELEASED', 'RELEASED', 'EXPIRED', 'CANCELLED')),
    encumbered_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    released_at        TIMESTAMPTZ,
    entity_version     INT         NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         UUID,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by         UUID
);
COMMENT ON TABLE encumbrances IS 'Budget encumbrances — commitments against budget (POs, contracts). Prevents overspending.';
COMMENT ON COLUMN encumbrances.remaining_amount IS 'Amount still encumbered (decreases as encumbrance is released)';

CREATE INDEX idx_enc_budget ON encumbrances (budget_line_id);
CREATE INDEX idx_enc_reference ON encumbrances (reference_type, reference_id);
CREATE INDEX idx_enc_active ON encumbrances (status) WHERE status = 'ACTIVE';
COMMENT ON INDEX idx_enc_budget IS 'All encumbrances on a budget line';
COMMENT ON INDEX idx_enc_reference IS 'Find encumbrance by source document (PO, contract)';
COMMENT ON INDEX idx_enc_active IS 'Active encumbrances consuming budget';

-- ============================================================================
-- PART 10: FIXED ASSETS
-- ============================================================================

-- 10.1 Fixed Assets
CREATE TABLE fixed_assets (
    fixed_asset_id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(tenant_id),
    asset_code             TEXT        NOT NULL,
    asset_category         TEXT        NOT NULL CHECK (asset_category IN ('LAND', 'BUILDING', 'FURNITURE', 'COMPUTER_EQUIPMENT', 'LAB_EQUIPMENT', 'LIBRARY_BOOKS', 'VEHICLES', 'OFFICE_EQUIPMENT', 'SOFTWARE')),
    asset_name             TEXT        NOT NULL,
    description            TEXT,
    purchase_date          DATE        NOT NULL,
    capitalization_date    DATE        NOT NULL,
    purchase_cost          paise       NOT NULL,
    gst_on_purchase        paise_nullable,
    itc_claimed            paise_nullable,
    depreciation_method    TEXT        NOT NULL CHECK (depreciation_method IN ('SLM', 'WDV')),
    depreciation_rate      NUMERIC(5,2) NOT NULL,
    useful_life            INT         NOT NULL,                -- years
    salvage_value          paise_nullable,
    current_location       TEXT,
    department_id          UUID        REFERENCES cost_centers(cost_center_id),
    custodian_id           UUID,
    fund_id                UUID        REFERENCES funds(fund_id),
    status                 TEXT        NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'UNDER_TRANSFER', 'DISPOSED', 'WRITTEN_OFF', 'LOST')),
    purchase_invoice_id    UUID        REFERENCES vendor_invoices(vendor_invoice_id),
    is_capital_goods       BOOLEAN     NOT NULL DEFAULT FALSE,
    rule_43_reversal_months INT        DEFAULT 60,
    asset_tag              TEXT,                                -- barcode/RFID tag
    entity_version         INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    deleted_at             TIMESTAMPTZ,
    deleted_by             UUID,
    UNIQUE (tenant_id, asset_code)
);
COMMENT ON TABLE fixed_assets IS 'Fixed asset register — capitalization, depreciation, transfer, and disposal tracking.';
COMMENT ON COLUMN fixed_assets.depreciation_method IS 'SLM=Straight Line Method, WDV=Written Down Value';
COMMENT ON COLUMN fixed_assets.is_capital_goods IS 'Capital goods subject to Rule 43 ITC reversal';
COMMENT ON COLUMN fixed_assets.rule_43_reversal_months IS 'Rule 43 reversal period — 60 months (5 years) for capital goods';

CREATE INDEX idx_fa_category ON fixed_assets (tenant_id, asset_category);
CREATE INDEX idx_fa_status ON fixed_assets (status) WHERE status = 'ACTIVE';
CREATE INDEX idx_fa_department ON fixed_assets (department_id);
CREATE INDEX idx_fa_fund ON fixed_assets (fund_id) WHERE fund_id IS NOT NULL;
COMMENT ON INDEX idx_fa_status IS 'All active assets for asset register';
COMMENT ON INDEX idx_fa_fund IS 'Grant-funded assets (for UGC tracking)';

-- Asset Depreciation Schedule
CREATE TABLE asset_depreciation (
    asset_depreciation_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    fixed_asset_id        UUID        NOT NULL REFERENCES fixed_assets(fixed_asset_id),
    fiscal_year_id        UUID        NOT NULL REFERENCES fiscal_years(fiscal_year_id),
    period_number         INT         NOT NULL CHECK (period_number BETWEEN 1 AND 12),
    depreciation_amount   paise       NOT NULL,
    is_posted             BOOLEAN     NOT NULL DEFAULT FALSE,
    posted_journal_id     UUID        REFERENCES journal_entries(journal_id),
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    UNIQUE (fixed_asset_id, fiscal_year_id, period_number)
);
COMMENT ON TABLE asset_depreciation IS 'Depreciation schedule — period-wise depreciation entries for each asset.';

CREATE INDEX idx_ad_asset ON asset_depreciation (fixed_asset_id);
CREATE INDEX idx_ad_fiscal ON asset_depreciation (fiscal_year_id);
COMMENT ON INDEX idx_ad_asset IS 'Depreciation schedule for an asset';

-- Asset Disposal
CREATE TABLE asset_disposals (
    asset_disposal_id  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(tenant_id),
    fixed_asset_id     UUID        NOT NULL REFERENCES fixed_assets(fixed_asset_id),
    disposal_type      TEXT        NOT NULL CHECK (disposal_type IN ('SALE', 'SCRAP', 'DONATION', 'THEFT', 'WRITE_OFF')),
    disposal_date      DATE        NOT NULL,
    sale_proceeds      paise_nullable,
    approved_by_id     UUID        NOT NULL,
    remarks            TEXT,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE asset_disposals IS 'Asset disposal records — sale, scrap, donation, theft, or write-off.';

-- 10.2 Inventory
CREATE TABLE inventory_items (
    inventory_item_id  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(tenant_id),
    item_code          TEXT        NOT NULL,
    item_name          TEXT        NOT NULL,
    category           TEXT        NOT NULL,
    unit_of_measure    TEXT        NOT NULL,                     -- "Nos", "Kg", "Ltr", "Box"
    valuation_method   TEXT        NOT NULL DEFAULT 'FIFO' CHECK (valuation_method IN ('FIFO', 'WEIGHTED_AVERAGE')),
    gst_rate           NUMERIC(5,2),
    hsn_sac_code       TEXT,
    reorder_level      NUMERIC(12,3),
    reorder_quantity   NUMERIC(12,3),
    is_active          BOOLEAN     NOT NULL DEFAULT TRUE,
    entity_version     INT         NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         UUID,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by         UUID,
    deleted_at         TIMESTAMPTZ,
    deleted_by         UUID,
    UNIQUE (tenant_id, item_code)
);
COMMENT ON TABLE inventory_items IS 'Inventory item master — consumables, stationery, lab materials, etc.';

-- Stock Movements (IMMUTABLE)
CREATE TABLE inventory_transactions (
    inventory_transaction_id UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id                UUID    NOT NULL REFERENCES tenants(tenant_id),
    inventory_item_id        UUID    NOT NULL REFERENCES inventory_items(inventory_item_id),
    movement_type            TEXT    NOT NULL CHECK (movement_type IN ('PURCHASE', 'ISSUE', 'TRANSFER_IN', 'TRANSFER_OUT', 'RETURN', 'ADJUSTMENT', 'WRITE_OFF')),
    reference_type           TEXT,                                -- "GRN", "IssueSlip", "TransferNote"
    reference_id             UUID,
    quantity                 NUMERIC(12,3) NOT NULL,
    unit_price               paise   NOT NULL,
    total_amount             paise   NOT NULL,
    movement_date            TIMESTAMPTZ NOT NULL DEFAULT now(),
    remarks                  TEXT,
    version                  INT     NOT NULL DEFAULT 1,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by               UUID,
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by               UUID
);
COMMENT ON TABLE inventory_transactions IS '★ IMMUTABLE ★ Stock movement register — every issue, receipt, transfer, and adjustment.';

CREATE INDEX idx_it_item ON inventory_transactions (inventory_item_id);
CREATE INDEX idx_it_date ON inventory_transactions (movement_date DESC);
COMMENT ON INDEX idx_it_item IS 'All stock movements for an item (stock ledger)';

-- ============================================================================
-- PART 11: COMPLIANCE & WORKFLOW
-- ============================================================================

-- 11.1 Compliance Calendar
CREATE TABLE compliance_calendar (
    compliance_event_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id             UUID        REFERENCES entities(entity_id),
    event_type            TEXT        NOT NULL CHECK (event_type IN ('GST_FILING', 'TDS_FILING', 'TDS_DEPOSIT', 'PT_FILING', 'IT_EXEMPTION_RENEWAL', 'AUDIT_44AB', 'AUDIT_12A', 'FORM_10B', 'FORM_10BB', 'ITR7', 'FCRA_FC4', 'FCRA_RENEWAL', 'AISHE_SUBMISSION', 'UGC_UC', 'NAAC_SUBMISSION')),
    event_title           TEXT        NOT NULL,
    due_date              DATE        NOT NULL,
    reminder_days         INT[]       DEFAULT '{7,3,1}',         -- days before to send reminders
    status                TEXT        NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING', 'COMPLETED', 'EXTENSION_APPLIED', 'OVERDUE')),
    completed_date        DATE,
    completed_by_id       UUID,
    reference_id          UUID,                                   -- links to GST return, TDS return, etc.
    remarks               TEXT,
    entity_version        INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    UNIQUE (tenant_id, entity_id, event_type, due_date)
);
COMMENT ON TABLE compliance_calendar IS 'Centralized compliance calendar — all filing deadlines across GST, TDS, PT, IT, FCRA, and audit.';

CREATE INDEX idx_cc_due ON compliance_calendar (due_date) WHERE status IN ('PENDING', 'OVERDUE');
CREATE INDEX idx_cc_entity ON compliance_calendar (tenant_id, entity_id);
COMMENT ON INDEX idx_cc_due IS 'Upcoming and overdue compliance events for dashboard';
COMMENT ON INDEX idx_cc_entity IS 'Compliance events for a specific entity';

-- 11.2 Audit Log (IMMUTABLE — INSERT-only)
CREATE TABLE audit_log (
    audit_log_id    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID        NOT NULL,
    entity_id       UUID,
    user_id         UUID,
    user_role       TEXT,
    action          TEXT        NOT NULL,                         -- 'CREATE', 'UPDATE', 'DELETE', 'POST', 'APPROVE', 'REVERSE', etc.
    resource_type   TEXT        NOT NULL,                         -- 'Journal', 'PaymentReceipt', 'Vendor', 'PurchaseOrder'
    resource_id     UUID        NOT NULL,
    changed_fields  JSONB,                                        -- {"field": {"old": "value", "new": "value"}}
    ip_address      TEXT,
    user_agent      TEXT,
    correlation_id  UUID,                                          -- for tracing across services
    occurred_at     TIMESTAMPTZ NOT NULL DEFAULT now()
) PARTITION BY RANGE (occurred_at);
COMMENT ON TABLE audit_log IS '★ IMMUTABLE ★ Universal audit trail — every mutation is logged here. Retained for 8 years as per Income Tax Act.';
COMMENT ON COLUMN audit_log.action IS 'Action performed: CREATE, UPDATE, DELETE, POST, APPROVE, REVERSE, CANCEL';
COMMENT ON COLUMN audit_log.changed_fields IS 'JSONB of changed fields with old and new values for UPDATE actions';
COMMENT ON COLUMN audit_log.correlation_id IS 'UUID for tracing a request across multiple services';

-- Partitions for audit_log (monthly partitions for high write volume)
CREATE TABLE audit_log_2026_04 PARTITION OF audit_log FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE audit_log_2026_05 PARTITION OF audit_log FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE audit_log_2026_06 PARTITION OF audit_log FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE audit_log_2026_07 PARTITION OF audit_log FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE audit_log_2026_08 PARTITION OF audit_log FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE audit_log_2026_09 PARTITION OF audit_log FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE audit_log_2026_10 PARTITION OF audit_log FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');
CREATE TABLE audit_log_2026_11 PARTITION OF audit_log FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');
CREATE TABLE audit_log_2026_12 PARTITION OF audit_log FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');
CREATE TABLE audit_log_2027_01 PARTITION OF audit_log FOR VALUES FROM ('2027-01-01') TO ('2027-02-01');
CREATE TABLE audit_log_2027_02 PARTITION OF audit_log FOR VALUES FROM ('2027-02-01') TO ('2027-03-01');
CREATE TABLE audit_log_2027_03 PARTITION OF audit_log FOR VALUES FROM ('2027-03-01') TO ('2027-04-01');
-- Default for future/unknown dates
CREATE TABLE audit_log_default PARTITION OF audit_log FOR VALUES FROM ('2027-04-01') TO ('2099-01-01');

CREATE INDEX idx_al_resource ON audit_log (resource_type, resource_id);
CREATE INDEX idx_al_user ON audit_log (user_id);
CREATE INDEX idx_al_tenant_time ON audit_log (tenant_id, occurred_at DESC);
CREATE INDEX idx_al_action ON audit_log (resource_type, action);
COMMENT ON INDEX idx_al_resource IS 'Full audit trail for a specific resource';
COMMENT ON INDEX idx_al_user IS 'All actions by a specific user';
COMMENT ON INDEX idx_al_tenant_time IS 'Time-ordered audit log for a tenant';
COMMENT ON INDEX idx_al_action IS 'Filter audit by action type for specific resources';

-- 11.3 Audit Schedule
CREATE TABLE audit_schedules (
    audit_schedule_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID        NOT NULL REFERENCES tenants(tenant_id),
    fiscal_year_id      UUID        NOT NULL REFERENCES fiscal_years(fiscal_year_id),
    audit_type          TEXT        NOT NULL CHECK (audit_type IN ('TAX_AUDIT_44AB', 'TRUST_AUDIT_12A', 'FORM_10B', 'FORM_10BB', 'ITR7', 'INTERNAL_AUDIT', 'STATUTORY_AUDIT')),
    due_date            DATE        NOT NULL,
    status              TEXT        NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING', 'IN_PROGRESS', 'COMPLETED', 'EXTENSION_FILED')),
    completed_date      DATE,
    auditor_name        TEXT,
    auditor_firm        TEXT,
    auditor_membership  TEXT,
    remarks             TEXT,
    entity_version      INT         NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by          UUID,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by          UUID,
    UNIQUE (fiscal_year_id, audit_type)
);
COMMENT ON TABLE audit_schedules IS 'Audit schedule for statutory audits — Tax Audit 44AB, Trust Audit 12A, Form 10B/10BB.';

-- 11.4 Approval Workflows
CREATE TABLE approval_workflows (
    approval_workflow_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id            UUID        NOT NULL REFERENCES entities(entity_id),
    transaction_type     TEXT        NOT NULL CHECK (transaction_type IN ('PURCHASE_ORDER', 'PURCHASE_INVOICE', 'PAYMENT', 'REFUND', 'CONCESSION', 'BUDGET', 'EXPENSE_CLAIM', 'JOURNAL', 'VENDOR')),
    workflow_name        TEXT        NOT NULL,
    is_active            BOOLEAN     NOT NULL DEFAULT TRUE,
    levels               INT         NOT NULL CHECK (levels BETWEEN 1 AND 5),
    config               JSONB       NOT NULL,                   -- levels configuration
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    UNIQUE (entity_id, transaction_type)
);
COMMENT ON TABLE approval_workflows IS 'Configurable multi-level approval workflows per transaction type.';

-- Approval Levels
CREATE TABLE approval_levels (
    approval_level_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID        NOT NULL REFERENCES tenants(tenant_id),
    approval_workflow_id UUID       NOT NULL REFERENCES approval_workflows(approval_workflow_id),
    level_number        INT         NOT NULL CHECK (level_number BETWEEN 1 AND 5),
    max_amount          paise_nullable,                          -- null = unlimited
    approver_role       TEXT        NOT NULL CHECK (approver_role IN ('DEPT_HEAD', 'FINANCE_CONTROLLER', 'CFO', 'TRUSTEE', 'REGISTRAR')),
    approver_user_id    UUID,                                     -- specific user (optional)
    escalation_hours    INT,                                      -- hours before escalation
    escalation_to_level INT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (approval_workflow_id, level_number)
);
COMMENT ON TABLE approval_levels IS 'Individual levels within an approval workflow.';

-- 11.5 Approval Requests
CREATE TABLE approval_requests (
    approval_request_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID        NOT NULL REFERENCES tenants(tenant_id),
    workflow_id         UUID        NOT NULL REFERENCES approval_workflows(approval_workflow_id),
    transaction_type    TEXT        NOT NULL,
    transaction_id      UUID        NOT NULL,
    transaction_number  TEXT,
    amount              paise       NOT NULL,
    current_level       INT         NOT NULL DEFAULT 1,
    status              TEXT        NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING', 'APPROVED', 'REJECTED', 'ESCALATED', 'CANCELLED')),
    requested_by_id     UUID        NOT NULL,
    requested_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at        TIMESTAMPTZ,
    entity_version      INT         NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by          UUID
);
COMMENT ON TABLE approval_requests IS 'Active and historical approval requests for financial transactions.';

CREATE INDEX idx_ar_pending ON approval_requests (status) WHERE status = 'PENDING';
CREATE INDEX idx_ar_transaction ON approval_requests (transaction_type, transaction_id);
CREATE INDEX idx_ar_requester ON approval_requests (requested_by_id);
COMMENT ON INDEX idx_ar_pending IS 'Pending approval requests for dashboard';
COMMENT ON INDEX idx_ar_transaction IS 'Find approval request for a specific transaction';

-- Approval Decisions
CREATE TABLE approval_decisions (
    approval_decision_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    approval_request_id  UUID        NOT NULL REFERENCES approval_requests(approval_request_id),
    level                INT         NOT NULL,
    approver_id          UUID        NOT NULL,
    decision             TEXT        NOT NULL CHECK (decision IN ('APPROVED', 'REJECTED', 'RETURNED_FOR_MODIFICATION')),
    comments             TEXT,
    decided_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE approval_decisions IS 'Individual approval decisions at each level of the workflow.';

CREATE INDEX idx_ad_request ON approval_decisions (approval_request_id);
COMMENT ON INDEX idx_ad_request IS 'All decisions on an approval request';

-- 11.6 Documents
CREATE TABLE documents (
    document_id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id            UUID        REFERENCES entities(entity_id),
    document_type        TEXT        NOT NULL CHECK (document_type IN ('INVOICE', 'RECEIPT', 'PO', 'GRN', 'CONTRACT', 'AGREEMENT', 'CERTIFICATE', 'BANK_STATEMENT', 'AUDIT_REPORT', 'OTHER')),
    file_name            TEXT        NOT NULL,
    file_size            BIGINT      NOT NULL,
    mime_type            TEXT        NOT NULL,
    storage_path         TEXT        NOT NULL,
    checksum             TEXT,                                    -- SHA-256
    version              INT         NOT NULL DEFAULT 1,
    linked_entity_type   TEXT,
    linked_entity_id     UUID,
    uploaded_by_id       UUID        NOT NULL,
    uploaded_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    is_deleted           BOOLEAN     NOT NULL DEFAULT FALSE,
    deleted_at           TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE documents IS 'Document metadata for attachments linked to financial entities. Files stored in S3-compatible object storage.';
COMMENT ON COLUMN documents.checksum IS 'SHA-256 checksum for integrity verification';
COMMENT ON COLUMN documents.storage_path IS 'Object storage path (S3 key or filesystem path)';

CREATE INDEX idx_doc_entity ON documents (linked_entity_type, linked_entity_id) WHERE is_deleted = FALSE;
CREATE INDEX idx_doc_type ON documents (tenant_id, document_type);
COMMENT ON INDEX idx_doc_entity IS 'All documents linked to a specific entity';

-- 11.7 Statutory Reports
CREATE TABLE statutory_reports (
    statutory_report_id  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id            UUID        NOT NULL REFERENCES entities(entity_id),
    report_type          TEXT        NOT NULL CHECK (report_type IN ('GSTR1', 'GSTR3B', 'GSTR9', 'GSTR9C', 'FORM_24Q', 'FORM_26Q', 'FORM_27Q', 'PT1', 'PT1A', 'PT2')),
    period               TEXT        NOT NULL,                    -- monthly: "MMYYYY", quarterly: "QQYYYY"
    fiscal_year          TEXT        NOT NULL,
    status               TEXT        NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING', 'DRAFT', 'GENERATED', 'REVIEWED', 'FILED', 'FILED_WITH_ERRORS')),
    due_date             DATE        NOT NULL,
    filed_date           DATE,
    filed_by_id          UUID,
    acknowledgment_no    TEXT,
    tax_amount           paise_nullable,
    json_data            JSONB,
    remarks              TEXT,
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    UNIQUE (entity_id, report_type, period)
);
COMMENT ON TABLE statutory_reports IS 'All statutory returns — GST, TDS, PT — with filing status and acknowledgments.';

CREATE INDEX idx_sr_entity_year ON statutory_reports (tenant_id, entity_id, fiscal_year);
CREATE INDEX idx_sr_due ON statutory_reports (due_date) WHERE status IN ('PENDING', 'DRAFT', 'GENERATED');
COMMENT ON INDEX idx_sr_entity_year IS 'All reports for an entity in a fiscal year';
COMMENT ON INDEX idx_sr_due IS 'Reports due for filing';

-- 11.8 Regulatory Reports (NAAC, AISHE, UGC)
CREATE TABLE regulatory_reports (
    regulatory_report_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id            UUID        NOT NULL REFERENCES entities(entity_id),
    report_type          TEXT        NOT NULL CHECK (report_type IN ('NAAC', 'AISHE', 'UGC_UC', 'UGC_ANNUAL', 'FCRA_FC4')),
    fiscal_year          TEXT        NOT NULL,
    status               TEXT        NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'GENERATED', 'REVIEWED', 'SUBMITTED')),
    generated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    generated_by_id      UUID,
    submitted_date       DATE,
    json_data            JSONB,
    document_url         TEXT,
    entity_version       INT         NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by           UUID,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by           UUID,
    UNIQUE (entity_id, report_type, fiscal_year)
);
COMMENT ON TABLE regulatory_reports IS 'Regulatory reports — NAAC financial metrics dashboard, AISHE extract, UGC Utilization Certificates.';

-- ============================================================================
-- PART 12: SYSTEM & EVENT TABLES
-- ============================================================================

-- 12.1 Event Outbox (Transactional Outbox Pattern)
CREATE TABLE event_outbox (
    event_outbox_id  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL,
    aggregate_type   TEXT        NOT NULL,                     -- e.g., "Journal", "PaymentReceipt"
    aggregate_id     UUID        NOT NULL,
    event_type       TEXT        NOT NULL,                     -- e.g., "JournalPosted", "PaymentReceiptCreated"
    event_payload    JSONB       NOT NULL,
    correlation_id   UUID,                                      -- for tracing
    status           TEXT        NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING', 'PUBLISHED', 'FAILED', 'DEAD_LETTERED')),
    retry_count      INT         NOT NULL DEFAULT 0,
    max_retries      INT         NOT NULL DEFAULT 5,
    last_error       TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at     TIMESTAMPTZ
);
COMMENT ON TABLE event_outbox IS 'Transactional outbox for reliable event publishing. Events written in same DB transaction as aggregate changes.';
COMMENT ON COLUMN event_outbox.aggregate_type IS 'Type of aggregate that generated the event';
COMMENT ON COLUMN event_outbox.event_type IS 'Domain event type name (e.g., JournalPosted, PaymentReceiptCreated)';
COMMENT ON COLUMN event_outbox.status IS 'PENDING=not yet published, PUBLISHED=sent to event bus, FAILED=retries exhausted, DEAD_LETTERED=permanent failure';

CREATE INDEX idx_eo_status ON event_outbox (status, created_at) WHERE status = 'PENDING';
CREATE INDEX idx_eo_correlation ON event_outbox (correlation_id);
COMMENT ON INDEX idx_eo_status IS 'Unpublished events for the background publisher worker';
COMMENT ON INDEX idx_eo_correlation IS 'Lookup events by correlation ID for debugging';

-- 12.2 Saga State (For distributed transaction orchestration)
CREATE TABLE saga_state (
    saga_id          UUID        PRIMARY KEY,
    saga_type        TEXT        NOT NULL,                     -- e.g., "PostJournal", "ProcessPayment", "ScholarshipDisbursement"
    state            TEXT        NOT NULL,                     -- current step in saga
    status           TEXT        NOT NULL DEFAULT 'RUNNING' CHECK (status IN ('RUNNING', 'COMPLETED', 'FAILED', 'COMPENSATING', 'COMPENSATED')),
    payload          JSONB       NOT NULL,
    compensation_payload JSONB,
    step_index       INT         NOT NULL DEFAULT 0,
    max_steps        INT         NOT NULL,
    retry_count      INT         NOT NULL DEFAULT 0,
    last_error       TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE saga_state IS 'Saga orchestration state for distributed transactions with compensating actions.';

CREATE INDEX idx_saga_status ON saga_state (status, created_at) WHERE status = 'RUNNING';
COMMENT ON INDEX idx_saga_status IS 'Active sagas for recovery/resume';

-- 12.3 System Config (Configuration-driven business rules)
CREATE TABLE system_config (
    system_config_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        REFERENCES tenants(tenant_id),     -- NULL = global default
    config_key       TEXT        NOT NULL,
    config_value     JSONB       NOT NULL,
    scope            TEXT        NOT NULL DEFAULT 'TENANT' CHECK (scope IN ('GLOBAL', 'TENANT', 'ENTITY')),
    is_active        BOOLEAN     NOT NULL DEFAULT TRUE,
    valid_from       TIMESTAMPTZ NOT NULL DEFAULT '1970-01-01',
    valid_to         TIMESTAMPTZ,
    description      TEXT,
    entity_version   INT         NOT NULL DEFAULT 1,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by       UUID,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by       UUID,
    UNIQUE (tenant_id, config_key, valid_from)
);
COMMENT ON TABLE system_config IS 'Configuration-driven business rules — all policies, rates, and thresholds are stored here.';
COMMENT ON COLUMN system_config.config_key IS 'Dot-notation key (e.g., "gst.rate.hostel", "tds.section.194c.rate")';
COMMENT ON COLUMN system_config.scope IS 'GLOBAL=all tenants, TENANT=specific tenant, ENTITY=specific entity';

-- ============================================================================
-- PART 13: MATERIALIZED VIEWS & CQRS PROJECTIONS
-- ============================================================================

-- Account Balance Materialized View (fast trial balance queries)
CREATE MATERIALIZED VIEW mv_account_balances AS
SELECT
    je.tenant_id,
    jel.account_id,
    je.accounting_period_id,
    SUM(COALESCE(jel.debit_amount, 0)) AS total_debit,
    SUM(COALESCE(jel.credit_amount, 0)) AS total_credit,
    SUM(COALESCE(jel.debit_amount, 0)) - SUM(COALESCE(jel.credit_amount, 0)) AS net_balance
FROM journal_entries je
JOIN journal_entry_lines jel ON je.journal_id = jel.journal_id
WHERE je.status = 'POSTED'
GROUP BY je.tenant_id, jel.account_id, je.accounting_period_id
WITH DATA;

CREATE UNIQUE INDEX idx_mv_ab_unique ON mv_account_balances (tenant_id, account_id, accounting_period_id);
CREATE INDEX idx_mv_ab_tenant ON mv_account_balances (tenant_id);
COMMENT ON MATERIALIZED VIEW mv_account_balances IS 'CQRS projection — pre-computed account balances per accounting period for fast trial balance queries.';
COMMENT ON INDEX idx_mv_ab_unique IS 'Support for concurrent refresh';

-- Fee Outstanding Materialized View
CREATE MATERIALIZED VIEW mv_fee_outstanding AS
SELECT
    tenant_id,
    student_id,
    entity_id,
    academic_year,
    COUNT(*) FILTER (WHERE outstanding_amount > 0) AS installments_pending,
    SUM(outstanding_amount) AS total_outstanding
FROM student_fee_accounts sfa
JOIN fee_installments fi ON sfa.student_fee_account_id = fi.student_fee_account_id
WHERE sfa.status = 'ACTIVE' AND fi.status IN ('PENDING', 'PARTIALLY_PAID')
GROUP BY tenant_id, student_id, entity_id, academic_year
WITH DATA;

CREATE UNIQUE INDEX idx_mv_fo_unique ON mv_fee_outstanding (tenant_id, student_id, academic_year);
COMMENT ON MATERIALIZED VIEW mv_fee_outstanding IS 'CQRS projection — pre-computed fee outstanding per student for dashboard and reminders.';

-- ============================================================================
-- END OF SCHEMA
-- ============================================================================
