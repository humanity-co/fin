CREATE TABLE IF NOT EXISTS accounts (
    account_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    account_code VARCHAR(50) NOT NULL,
    account_name VARCHAR(255) NOT NULL,
    account_type VARCHAR(50) NOT NULL,
    parent_account_id UUID,
    level INTEGER NOT NULL,
    gst_classification VARCHAR(50),
    hsn_sac_code VARCHAR(50),
    itc_eligibility VARCHAR(50),
    aishe_head_code VARCHAR(50),
    naac_metric_key VARCHAR(50),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_system BOOLEAN NOT NULL DEFAULT FALSE,
    opening_balance_paise BIGINT NOT NULL DEFAULT 0,
    current_balance_paise BIGINT NOT NULL DEFAULT 0,
    created_by UUID,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_by UUID,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    entity_version INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS purchase_orders (
    purchase_order_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    entity_id UUID NOT NULL,
    po_number VARCHAR(50) NOT NULL,
    vendor_id UUID NOT NULL,
    purchase_requisition_id UUID,
    order_date DATE NOT NULL,
    delivery_date DATE,
    payment_terms INTEGER,
    status VARCHAR(50) NOT NULL,
    total_amount BIGINT NOT NULL,
    tax_amount BIGINT NOT NULL,
    net_amount BIGINT NOT NULL,
    is_rcm_applicable BOOLEAN,
    tds_section VARCHAR(50),
    tds_rate NUMERIC,
    fund_id UUID,
    budget_head_id UUID,
    created_by UUID,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_by UUID,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    entity_version INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS purchase_order_lines (
    po_line_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    purchase_order_id UUID NOT NULL,
    line_number INTEGER NOT NULL,
    item_description TEXT NOT NULL,
    hsn_sac_code VARCHAR(50),
    quantity NUMERIC NOT NULL,
    unit_price BIGINT NOT NULL,
    discount_percent NUMERIC,
    tax_rate NUMERIC,
    tax_type VARCHAR(50),
    total_amount BIGINT NOT NULL,
    received_quantity NUMERIC,
    account_id UUID,
    cost_center_id UUID,
    created_by UUID,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_by UUID,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- AR Tables (Stubs for Fee Collection)
CREATE TABLE IF NOT EXISTS ar_student_fee_accounts (
    fee_account_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    student_id UUID NOT NULL,
    program_id UUID NOT NULL,
    academic_year_id UUID NOT NULL,
    total_fee_paise BIGINT NOT NULL,
    total_paid_paise BIGINT NOT NULL DEFAULT 0,
    total_concession_paise BIGINT NOT NULL DEFAULT 0,
    total_refund_paise BIGINT NOT NULL DEFAULT 0,
    status VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ar_payment_receipts (
    receipt_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    entity_id UUID NOT NULL,
    receipt_number VARCHAR(50) NOT NULL,
    student_id UUID NOT NULL,
    payment_date DATE NOT NULL,
    payment_mode VARCHAR(50) NOT NULL,
    amount_paise BIGINT NOT NULL,
    reference_number VARCHAR(100),
    bank_name VARCHAR(100),
    status VARCHAR(50) NOT NULL,
    created_by UUID,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ar_fee_installments (
    installment_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    fee_account_id UUID NOT NULL,
    due_date DATE NOT NULL,
    amount_paise BIGINT NOT NULL,
    paid_amount_paise BIGINT NOT NULL DEFAULT 0,
    status VARCHAR(50) NOT NULL
);
