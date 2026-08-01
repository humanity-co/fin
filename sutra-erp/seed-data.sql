-- Seed data for ERP

-- Seed Accounts
INSERT INTO chart_of_accounts (account_id, tenant_id, account_code, account_name, account_type, level, is_active, is_system, opening_balance, current_balance, entity_version)
VALUES 
('d5086d4e-1a55-46b7-849c-8519e9514757', '00000000-0000-0000-0000-000000000000', '1000', 'Assets', 'ASSET', 1, true, true, 0, 5000000, 1),
('28ea1cc8-9a3d-4c31-bede-44c116c4f877', '00000000-0000-0000-0000-000000000000', '1100', 'Current Assets', 'ASSET', 2, true, false, 0, 5000000, 1),
('34d47d6a-5c1f-4b07-a5c9-2544e3cb2657', '00000000-0000-0000-0000-000000000000', '1110', 'Bank Accounts', 'ASSET', 3, true, false, 0, 5000000, 1),
('0527376e-e67c-48c6-a67f-ae77b6ff03b5', '00000000-0000-0000-0000-000000000000', '2000', 'Liabilities', 'LIABILITY', 1, true, true, 0, 3000000, 1),
('6f582736-2358-4702-8a4e-1f87ab24a1b0', '00000000-0000-0000-0000-000000000000', '3000', 'Equity', 'EQUITY', 1, true, true, 0, 2000000, 1),
('db933f86-dcc8-422f-934c-68abcb667b96', '00000000-0000-0000-0000-000000000000', '4000', 'Revenue', 'INCOME', 1, true, true, 0, 500000, 1),
('fbb13854-4770-4fc7-bf84-332308cfd29c', '00000000-0000-0000-0000-000000000000', '5000', 'Expenses', 'EXPENSE', 1, true, true, 0, 400000, 1)
ON CONFLICT (account_id) DO NOTHING;

-- Seed Purchase Order
INSERT INTO purchase_orders (purchase_order_id, tenant_id, entity_id, po_number, vendor_id, order_date, delivery_date, payment_terms, status, total_amount, tax_amount, net_amount, entity_version)
VALUES 
('a83852cd-7f02-4ec4-91f9-90610339d1b1', '00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000', 'PO-2026-001', '019fb7ac-8541-75d1-bc7e-4e0708a3024a', '2026-07-31', '2026-08-07', 30, 'ISSUED', 100000, 18000, 118000, 1),
('b49520e5-7977-4df3-b3eb-135ec7b99c8e', '00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000', 'PO-2026-002', '019fb7ac-8541-75d1-bc7e-4e0708a3024a', '2026-07-31', '2026-08-15', 30, 'DRAFT', 250000, 45000, 295000, 1)
ON CONFLICT (purchase_order_id) DO NOTHING;

-- Seed Purchase Order Lines
INSERT INTO purchase_order_lines (po_line_id, tenant_id, purchase_order_id, line_number, item_description, hsn_sac_code, quantity, unit_price, tax_rate, total_amount)
VALUES 
('c86e680a-9e75-4fc1-b1e8-d1aebefd8244', '00000000-0000-0000-0000-000000000000', 'a83852cd-7f02-4ec4-91f9-90610339d1b1', 1, 'Office Laptops', '8471', 10, 10000, 18, 100000),
('d96e680a-9e75-4fc1-b1e8-d1aebefd8244', '00000000-0000-0000-0000-000000000000', 'b49520e5-7977-4df3-b3eb-135ec7b99c8e', 1, 'Server Racks', '8471', 5, 50000, 18, 250000)
ON CONFLICT (po_line_id) DO NOTHING;

-- Seed AR Data
INSERT INTO ar_student_fee_accounts (fee_account_id, tenant_id, student_id, program_id, academic_year_id, total_fee_paise, total_paid_paise, total_concession_paise, total_refund_paise, status)
VALUES 
('e06e680a-9e75-4fc1-b1e8-d1aebefd8244', '00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000', 8500000, 2000000, 0, 0, 'PARTIAL')
ON CONFLICT (fee_account_id) DO NOTHING;

-- Seed Vendors (another one)
INSERT INTO vendors (vendor_id, tenant_id, entity_id, vendor_code, vendor_name, vendor_type, pan, pan_status, gstin, gstin_status, gst_composition_scheme, registration_type, payment_terms, is_active, is_blacklisted, created_by, updated_by, entity_version)
VALUES 
('f06e680a-9e75-4fc1-b1e8-d1aebefd8244', '00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000', 'V002', 'Tech Supplies India Pvt Ltd', 'COMPANY', 'ABCDE1234F', 'VERIFIED', '27ABCDE1234F1Z5', 'VERIFIED', false, 'REGULAR', 30, true, false, '00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000', 1)
ON CONFLICT (vendor_id) DO NOTHING;
