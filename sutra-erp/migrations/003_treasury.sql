-- 003_treasury.sql
-- Treasury & Banking module tables.
--
-- Conventions (inherited from the existing DDL): tenant_id on every table,
-- money as BIGINT paise, UUID PKs, audit columns (created_at/created_by/
-- updated_at/updated_by), soft-delete flags on masters, immutable fact tables
-- (INSERT-only). See 001_rbac.sql / 002_app_users.sql for style.
--
-- NOTE: bank_accounts / bank_signatories / bank_reconciliations /
-- bank_statement_lines / bank_transactions are defined in the base DDL
-- (sutra-erp-schema.sql). This migration only adds what the DDL lacks:
--   * bank_accounts.gl_account_id  (link to the dedicated GL leaf under 10.02)
--   * payment_gateway_configs, inter_bank_transfers, petty_cash_funds,
--     petty_cash_transactions, gateway_settlements
-- and seeds the treasury policy defaults in system_config.

-- ── Link bank accounts to their dedicated GL leaf account ─────────────
-- Every bank account gets exactly one leaf account under 10.02 (Bank
-- Accounts) auto-created on bank-account creation; this column links them.
ALTER TABLE IF EXISTS bank_accounts
    ADD COLUMN IF NOT EXISTS gl_account_id UUID REFERENCES chart_of_accounts(account_id);
CREATE INDEX IF NOT EXISTS idx_bank_accounts_gl_account
    ON bank_accounts (gl_account_id) WHERE gl_account_id IS NOT NULL;

-- ── Payment gateway configurations ────────────────────────────────────
-- Secrets (api_key/api_secret/webhook_secret) are encrypted at the
-- application layer before persistence.
CREATE TABLE IF NOT EXISTS payment_gateway_configs (
    payment_gateway_config_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id                UUID        NOT NULL REFERENCES tenants(id),
    entity_id                UUID,                       -- NULL = shared/central config
    gateway_type             TEXT        NOT NULL CHECK (gateway_type IN ('BILLDESK', 'RAZORPAY', 'CCAVENUE', 'PHONEPE', 'PAYTM')),
    merchant_id              TEXT        NOT NULL,
    api_key                  TEXT        NOT NULL,       -- encrypted at rest
    api_secret               TEXT        NOT NULL,       -- encrypted at rest
    webhook_secret           TEXT,                       -- encrypted at rest
    is_active                BOOLEAN     NOT NULL DEFAULT TRUE,
    version                  INT         NOT NULL DEFAULT 1,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by               UUID,
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by               UUID,
    deleted_at               TIMESTAMPTZ,
    deleted_by               UUID,
    UNIQUE (tenant_id, entity_id, gateway_type)
);
COMMENT ON TABLE payment_gateway_configs IS
    'Payment-gateway credentials per entity. Gateway initiation/webhooks are owned by AR; treasury owns config + settlement reconciliation.';
COMMENT ON COLUMN payment_gateway_configs.api_key IS
    'Gateway API key — encrypted at rest at the application layer.';

-- ── Inter-bank transfers ──────────────────────────────────────────────
-- State machine: INITIATED → APPROVED → PROCESSED → COMPLETED,
--                INITIATED/APPROVED → CANCELLED, PROCESSED → FAILED.
-- The net GL effect is zero (contra journal DR to-account / CR from-account).
CREATE TABLE IF NOT EXISTS inter_bank_transfers (
    inter_bank_transfer_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID        NOT NULL REFERENCES tenants(id),
    from_bank_account_id   UUID        NOT NULL REFERENCES bank_accounts(bank_account_id),
    to_bank_account_id     UUID        NOT NULL REFERENCES bank_accounts(bank_account_id),
    amount                 BIGINT      NOT NULL CHECK (amount > 0),        -- paise
    transfer_date          DATE        NOT NULL,
    status                 TEXT        NOT NULL DEFAULT 'INITIATED'
        CHECK (status IN ('INITIATED', 'APPROVED', 'PROCESSED', 'COMPLETED', 'CANCELLED', 'FAILED')),
    requires_approval      BOOLEAN     NOT NULL DEFAULT FALSE,  -- amount >= transfer_approval_threshold
    bank_reference         TEXT,                                 -- UTR / bank reference on completion
    journal_id             UUID        REFERENCES journal_entries(journal_id),
    initiated_by_id        UUID        NOT NULL,
    approved_by_id         UUID,
    processed_by_id        UUID,
    failure_reason         TEXT,
    version                INT         NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by             UUID,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             UUID,
    CHECK (from_bank_account_id <> to_bank_account_id)
);
CREATE INDEX IF NOT EXISTS idx_ibt_from_account ON inter_bank_transfers (tenant_id, from_bank_account_id);
CREATE INDEX IF NOT EXISTS idx_ibt_status       ON inter_bank_transfers (status) WHERE status IN ('INITIATED', 'APPROVED', 'PROCESSED');

-- ── Petty cash funds ──────────────────────────────────────────────────
-- Per-entity single ACTIVE fund (policy assumption). Imprest capped by
-- policy petty_cash_imprest_limit (default ₹25,000).
CREATE TABLE IF NOT EXISTS petty_cash_funds (
    petty_cash_fund_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        NOT NULL REFERENCES tenants(id),
    entity_id          UUID        NOT NULL,
    fund_name          TEXT        NOT NULL,
    cash_gl_account_id UUID        NOT NULL REFERENCES chart_of_accounts(account_id),
    imprest_amount     BIGINT      NOT NULL CHECK (imprest_amount >= 0),   -- paise
    balance            BIGINT      NOT NULL DEFAULT 0 CHECK (balance >= 0),-- paise
    custodian_id       UUID        NOT NULL,
    status             TEXT        NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'CLOSED')),
    version            INT         NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         UUID,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by         UUID,
    deleted_at         TIMESTAMPTZ,
    deleted_by         UUID
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_petty_cash_active_per_entity
    ON petty_cash_funds (tenant_id, entity_id) WHERE status = 'ACTIVE';

-- ── Petty cash transactions (immutable fact table — INSERT only) ─────
CREATE TABLE IF NOT EXISTS petty_cash_transactions (
    petty_cash_txn_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID        NOT NULL REFERENCES tenants(id),
    petty_cash_fund_id  UUID        NOT NULL REFERENCES petty_cash_funds(petty_cash_fund_id),
    transaction_type    TEXT        NOT NULL CHECK (transaction_type IN ('TOP_UP', 'RECOUPMENT', 'EXPENSE', 'ADJUSTMENT')),
    amount              BIGINT      NOT NULL CHECK (amount > 0),           -- paise
    account_id          UUID,                                             -- expense account for EXPENSE/RECOUPMENT
    voucher_no          TEXT,
    narration           TEXT,
    journal_id          UUID        REFERENCES journal_entries(journal_id),
    transaction_date    DATE        NOT NULL,
    recorded_by_id      UUID        NOT NULL,
    version             INT         NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by          UUID
);
CREATE INDEX IF NOT EXISTS idx_pct_fund_date ON petty_cash_transactions (tenant_id, petty_cash_fund_id, transaction_date);

-- ── Gateway settlements ───────────────────────────────────────────────
-- PENDING → RECONCILED / EXCEPTION. Unique per (entity, gateway, date).
CREATE TABLE IF NOT EXISTS gateway_settlements (
    gateway_settlement_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(id),
    entity_id             UUID        NOT NULL,
    gateway_type          TEXT        NOT NULL CHECK (gateway_type IN ('BILLDESK', 'RAZORPAY', 'CCAVENUE', 'PHONEPE', 'PAYTM')),
    settlement_date       DATE        NOT NULL,
    settled_amount        BIGINT      NOT NULL,                             -- paise (net of gateway fees)
    gateway_fee_amount    BIGINT      NOT NULL DEFAULT 0,                   -- paise
    matched_amount        BIGINT      NOT NULL DEFAULT 0,                   -- sum of matched gateway transactions
    status                TEXT        NOT NULL DEFAULT 'PENDING'
        CHECK (status IN ('PENDING', 'RECONCILED', 'EXCEPTION')),
    exception_reason      TEXT,
    reconciled_by_id      UUID,
    reconciled_at         TIMESTAMPTZ,
    version               INT         NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by            UUID,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by            UUID,
    UNIQUE (tenant_id, entity_id, gateway_type, settlement_date)
);
COMMENT ON TABLE gateway_settlements IS
    'Per-day gateway settlement reconciliation. settled_amount must equal matched transactions minus fees.';

-- ── Treasury policy defaults (configurable — never hardcoded) ─────────
-- Values are JSONB per the system_config schema. GLOBAL rows (tenant_id NULL)
-- serve as defaults; a tenant row overrides.
INSERT INTO system_config (tenant_id, config_key, config_value, scope, description)
VALUES
    (NULL, 'treasury.transfer_approval_threshold',   '100000000', 'GLOBAL', 'Transfers at/above this amount (paise, default ₹10,00,000) require approval workflow'),
    (NULL, 'treasury.petty_cash_imprest_limit',      '2500000',   'GLOBAL', 'Maximum petty-cash imprest per fund (paise, default ₹25,000)'),
    (NULL, 'treasury.cheque_joint_signature_threshold', '10000000', 'GLOBAL', 'Cheques at/above this amount (paise, default ₹1,00,000) require JOINT signatories'),
    (NULL, 'treasury.fcra_bank_ifsc',                '"SBIN0000691"', 'GLOBAL', 'FCRA 2010 s.17 — FCRA accounts must be at SBI New Delhi Main Branch (IFSC)'),
    (NULL, 'treasury.fcra_bank_name',                '"State Bank of India"', 'GLOBAL', 'FCRA bank name'),
    (NULL, 'treasury.fcra_branch_name',              '"New Delhi Main Branch"', 'GLOBAL', 'FCRA branch name'),
    (NULL, 'treasury.endowment_transfer_blocked',    'true',      'GLOBAL', 'Block transfers out of endowment-linked accounts unless income_only (Maharashtra Self-Financed Universities Act 2013)'),
    (NULL, 'treasury.auto_match_utr_exact',          'true',      'GLOBAL', 'Auto-match rule 1: exact transaction_ref (UTR) equality'),
    (NULL, 'treasury.auto_match_amount_date_window_days', '3',     'GLOBAL', 'Auto-match rule 2: amount equality + date within ±N days'),
    (NULL, 'treasury.auto_match_description_similarity', 'true',  'GLOBAL', 'Auto-match rule 3: amount equality + description similarity'),
    (NULL, 'treasury.bank_gl_parent_codes',          '["10.02","10.02.01","1110","1100"]', 'GLOBAL', 'Candidate COA codes (highest priority first) under which bank GL leaves are created'),
    (NULL, 'treasury.cash_gl_code',                  '"10.01.01.01"', 'GLOBAL', 'Default cash-in-hand GL code used for petty-cash journals')
ON CONFLICT (tenant_id, config_key, valid_from) DO NOTHING;
