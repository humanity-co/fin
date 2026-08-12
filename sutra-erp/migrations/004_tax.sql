-- 004_tax.sql
-- Tax Engine (crate: finance-taxation) tables.
--
-- Conventions (inherited from 003_treasury.sql and the base DDL):
-- tenant_id on every table, money as BIGINT paise, UUID PKs, audit
-- columns (created_at/created_by/updated_at/updated_by), soft-delete
-- flags on masters, immutable fact tables (INSERT-only), status enums
-- as TEXT + CHECK. See 001_rbac.sql / 002_app_users.sql / 003_treasury.sql.
--
-- NOTE: the base DDL (sutra-erp-schema.sql, PART 8 "Taxation") already
-- defines gst_registrations, gst_returns, gst_return_lines,
-- itc_register, itc_register_lines, tds_deductions, tds_sections,
-- tds_returns, tds_return_details, trust_exemptions,
-- income_applications, income_application_lines and fcra_registrations
-- with the module spec's aggregate columns (verified column-by-column).
-- This migration therefore only adds what the base DDL lacks
-- (mirrors 003_treasury.sql, which likewise did not recreate
-- bank_accounts / journal_entries / chart_of_accounts etc.):
--   * gst_rate_master       — spec: NEW table, effective-dated rate master
--   * tds_deposits          — challan tracking for DepositTdsToGovt
--   * form16_certificates   — Form 16 / Form 16A certificate records
--   * tds_sections          — extended in place to be tenant-scoped and
--                             effective-dated (spec requirement)
--   * tax policy defaults   — seeded in system_config (never hardcoded)

-- ── GST rate master (spec: NEW table) ────────────────────────────────
-- Effective-dated rate master. rate ∈ {0,5,12,18,28}; overlapping
-- effective-date ranges for the same (tenant, hsn_sac_code, supply_type)
-- are rejected at the database level (spec invariant).
CREATE EXTENSION IF NOT EXISTS btree_gist;   -- for the EXCLUDE overlap check
CREATE TABLE IF NOT EXISTS gst_rate_master (
    gst_rate_id    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id      UUID        NOT NULL REFERENCES tenants(tenant_id),
    hsn_sac_code   TEXT        NOT NULL,                     -- HSN (goods) / SAC (services)
    description    TEXT,
    rate           BIGINT      NOT NULL CHECK (rate IN (0, 5, 12, 18, 28)),  -- GST rate %
    itc_eligible   BOOLEAN     NOT NULL DEFAULT FALSE,
    effective_from DATE        NOT NULL,
    effective_to   DATE,                                     -- NULL = open-ended
    supply_type    TEXT        NOT NULL DEFAULT 'SERVICES'
        CHECK (supply_type IN ('GOODS', 'SERVICES')),
    is_active      BOOLEAN     NOT NULL DEFAULT TRUE,
    version        INT         NOT NULL DEFAULT 1,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by     UUID,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by     UUID,
    deleted_at     TIMESTAMPTZ,
    deleted_by     UUID,
    CHECK (effective_to IS NULL OR effective_to > effective_from)
);
COMMENT ON TABLE gst_rate_master IS
    'Effective-dated GST rate master. Rates change by notification (12/2017-CT(R) etc.); '
    'overlapping effective ranges per (tenant, HSN/SAC, supply type) are rejected.';
COMMENT ON COLUMN gst_rate_master.rate IS 'GST rate as a whole percent — only 0, 5, 12, 18, 28 are statutory (goods/services rates).';

-- One rate per (tenant, hsn_sac, supply_type, effective_from) — prevents
-- duplicate starts; the EXCLUDE constraint below rejects overlapping ranges.
CREATE UNIQUE INDEX IF NOT EXISTS uq_gst_rate_master_effective
    ON gst_rate_master (tenant_id, hsn_sac_code, supply_type, effective_from)
    WHERE deleted_at IS NULL;

-- Overlapping-dates rejection (spec invariant). btree_gist required.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_gst_rate_no_overlap') THEN
        ALTER TABLE gst_rate_master ADD CONSTRAINT chk_gst_rate_no_overlap
            EXCLUDE USING gist (
                tenant_id WITH =,
                hsn_sac_code WITH =,
                supply_type WITH =,
                daterange(effective_from, COALESCE(effective_to, 'infinity'::date), '[)') WITH &&
            ) WHERE (deleted_at IS NULL);
    END IF;
END $$;

-- ── TDS deposit challan tracking (spec: DepositTdsToGovt) ─────────────
-- One row per deduction covered by a challan deposit. A single challan may
-- cover several deductions (same challan_reference, one row each), matching
-- the TdsDeposited outbox event which is emitted per deduction.
-- State machine on the AP-owned tds_deductions.deposit_status:
-- PENDING → DEPOSITED → FILED; this table records the DEPOSITED leg.
CREATE TABLE IF NOT EXISTS tds_deposits (
    tds_deposit_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL REFERENCES tenants(tenant_id),
    tds_deduction_id UUID        NOT NULL REFERENCES tds_deductions(tds_deduction_id),
    challan_reference TEXT       NOT NULL,                    -- ITNS-281 challan serial / CIN
    deposit_date     DATE        NOT NULL,
    amount           BIGINT      NOT NULL CHECK (amount > 0), -- paise
    bank_account_id  UUID        REFERENCES bank_accounts(bank_account_id),
    journal_id       UUID        REFERENCES journal_entries(journal_id), -- DR TDS Payable (24.03) / CR Bank
    status           TEXT        NOT NULL DEFAULT 'DEPOSITED'
        CHECK (status IN ('DEPOSITED', 'FILED')),             -- FILED = included in a TDS return
    recorded_by_id   UUID        NOT NULL,
    version          INT         NOT NULL DEFAULT 1,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by       UUID,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by       UUID,
    UNIQUE (tenant_id, challan_reference, tds_deduction_id)
);
CREATE INDEX IF NOT EXISTS idx_td_deposit_deduction
    ON tds_deposits (tenant_id, tds_deduction_id);
COMMENT ON TABLE tds_deposits IS
    'TDS challan deposits (ITNS-281). Deposit posts DR TDS Payable (24.03) / CR Bank via the GL module; '
    'journal_id carries the posted journal for traceability.';

-- ── Form 16 / Form 16A certificate records (spec: GenerateForm16/16A) ─
-- Form 16 = salary TDS (per employee, per FY); Form 16A = non-salary TDS
-- (per vendor/pan, per FY). Exactly one of employee_id / vendor_id must be
-- set, matching the certificate type.
CREATE TABLE IF NOT EXISTS form16_certificates (
    form16_certificate_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(tenant_id),
    entity_id             UUID        NOT NULL REFERENCES entities(entity_id),
    certificate_type      TEXT        NOT NULL
        CHECK (certificate_type IN ('FORM_16', 'FORM_16A')),
    fiscal_year           TEXT        NOT NULL,               -- e.g. '2025-26'
    employee_id           UUID,                               -- Form 16 (salary TDS, s.192)
    vendor_id             UUID        REFERENCES vendors(vendor_id), -- Form 16A (non-salary TDS)
    pan                   TEXT        NOT NULL,
    document_url          TEXT        NOT NULL,
    status                TEXT        NOT NULL DEFAULT 'GENERATED'
        CHECK (status IN ('GENERATED', 'ISSUED', 'REVOKED')),
    generated_by_id       UUID,
    issued_at             TIMESTAMPTZ,
    version               INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    CHECK (
        (certificate_type = 'FORM_16'  AND employee_id IS NOT NULL AND vendor_id IS NULL) OR
        (certificate_type = 'FORM_16A' AND vendor_id IS NOT NULL AND employee_id IS NULL)
    )
);
-- One Form 16 per employee per FY; one Form 16A per vendor per FY.
CREATE UNIQUE INDEX IF NOT EXISTS uq_form16_per_employee
    ON form16_certificates (tenant_id, fiscal_year, employee_id)
    WHERE certificate_type = 'FORM_16';
CREATE UNIQUE INDEX IF NOT EXISTS uq_form16a_per_vendor
    ON form16_certificates (tenant_id, fiscal_year, vendor_id)
    WHERE certificate_type = 'FORM_16A';
COMMENT ON TABLE form16_certificates IS
    'TDS certificates: Form 16 (employees, s.192) and Form 16A (vendors, non-salary). '
    'Due: Form 16 by 31 May (IT Rules r.31(1)); Form 16A within 15 days of return filing (r.31(3)).';

-- ── tds_sections: tenant-scoped, effective-dated extension ────────────
-- The base DDL defines tds_sections WITHOUT tenant_id and WITHOUT effective
-- dating, but the spec requires a per-tenant, effective-dated section/rate
-- master (IT Rules + annual Finance Act). Extend the base table in place;
-- pre-existing rows (if any) map to the nil tenant (GLOBAL defaults).
ALTER TABLE tds_sections ADD COLUMN IF NOT EXISTS
    tenant_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
ALTER TABLE tds_sections ADD COLUMN IF NOT EXISTS
    effective_from DATE NOT NULL DEFAULT '1970-01-01';
ALTER TABLE tds_sections ADD COLUMN IF NOT EXISTS effective_to DATE;
ALTER TABLE tds_sections ADD COLUMN IF NOT EXISTS created_by UUID;
ALTER TABLE tds_sections ADD COLUMN IF NOT EXISTS updated_by UUID;
-- Future inserts must specify the tenant and the effective start explicitly.
ALTER TABLE tds_sections ALTER COLUMN tenant_id DROP DEFAULT;
ALTER TABLE tds_sections ALTER COLUMN effective_from DROP DEFAULT;
-- Replace the base DDL's global UNIQUE(section_code) with per-tenant,
-- effective-dated uniqueness (one rate per section per effective period).
ALTER TABLE tds_sections DROP CONSTRAINT IF EXISTS tds_sections_section_code_key;
CREATE UNIQUE INDEX IF NOT EXISTS uq_tds_sections_tenant_section_effective
    ON tds_sections (tenant_id, section_code, effective_from);
COMMENT ON COLUMN tds_sections.effective_from IS
    'Effective start of this section/rate configuration (Finance Act year start).';
COMMENT ON COLUMN tds_sections.effective_to IS
    'Effective end; NULL = current. New Finance Act rates insert a new row.';

-- ── Tax policy defaults (configurable — never hardcoded) ──────────────
-- Values are JSONB per the system_config schema; GLOBAL rows (tenant_id
-- NULL) are defaults that a tenant row can override.
INSERT INTO system_config (tenant_id, config_key, config_value, scope, description)
VALUES
    (NULL, 'tax.itc_reversal_tolerance_percent', '5',   'GLOBAL', 'Rule 42 de-minimis: no reversal required when computed reversal ≤ 5% of C2 (CGST Rules r.42(1)(m)) — statute default, configurable'),
    (NULL, 'tax.income_application_threshold',   '85',  'GLOBAL', 'Minimum % of income that must be applied to educational purposes (IT Act s.11(1)(a))'),
    (NULL, 'tax.accumulation_years',             '5',   'GLOBAL', 'Maximum period (years) unapplied income may be accumulated under s.11(2)'),
    (NULL, 'tax.fcra_admin_expense_ratio_limit', '20',  'GLOBAL', 'FCRA 2010 s.17 — administrative expenses must be ≤ 20% of FCRA receipts'),
    (NULL, 'tax.tds_deposit_due_day',            '7',   'GLOBAL', 'TDS deposits due by the 7th of the following month (IT Rules r.30)')
ON CONFLICT (tenant_id, config_key, valid_from) DO NOTHING;
