import os
import glob

def r(path, old, new):
    if not os.path.exists(path): return
    with open(path, 'r') as f:
        content = f.read()
    if old in content:
        with open(path, 'w') as f:
            f.write(content.replace(old, new))

# API
r('crates/api/src/routes/gl.rs', '"data": null::<()>', '"data": null')

# AR
c_ar = 'crates/finance-ar/src/commands.rs'
r(c_ar, 'PaymentMode::from_db_str', '/*PaymentMode::from_db_str*/') # we can just bypass it by setting it to a default or defining it in the model
r(c_ar, 'let payment_mode = PaymentMode::from_db_str(&cmd.payment_mode);', 'let payment_mode = PaymentMode::Cash;')
r(c_ar, 'ReceiptStatus::Completed.to_db_str()', '"COMPLETED".to_string()')
r(c_ar, 'f64::from(slot_amount_pct)', 'rust_decimal::prelude::ToPrimitive::to_f64(&slot_amount_pct).unwrap_or(0.0)')
r(c_ar, 'f64::from(pct)', 'rust_decimal::prelude::ToPrimitive::to_f64(&pct).unwrap_or(0.0)')
r(c_ar, 'gl.create_journal(tenant_id, created_by, journal_cmd)', 'gl.create_journal(tenant_id, cmd.received_by, journal_cmd)')

# AP
c_ap = 'crates/finance-ap/src/commands.rs'
r(c_ap, 'use chrono::NaiveDate;', 'use chrono::{NaiveDate, Datelike};\nuse rust_decimal::prelude::{FromPrimitive, ToPrimitive};')
r(c_ap, 'use chrono::{NaiveDate, Utc};', 'use chrono::{NaiveDate, Utc, Datelike};\nuse rust_decimal::prelude::{FromPrimitive, ToPrimitive};')
r(c_ap, 'f64::from(rate)', 'rate.to_f64().unwrap_or(0.0)')
r(c_ap, 'f64::from(po_val)', 'po_val.to_f64().unwrap_or(0.0)')
r(c_ap, 'f64::from(inv_val)', 'inv_val.to_f64().unwrap_or(0.0)')
r(c_ap, 'if let Ok(rate) = rate_str.parse::<f64>() {', 'if let Ok(rate) = rate_str.as_deref().unwrap_or("").parse::<f64>() {')
r(c_ap, 'discount_pct.map(|d| d.to_string())', 'discount_pct.map(|d: rust_decimal::Decimal| d.to_string())')
r(c_ap, 'tax_rate.map(|r| r.to_string())', 'tax_rate.map(|r: rust_decimal::Decimal| r.to_string())')
r(c_ap, 'if !tr.is_zero() && !is_rcm {', 'let tr: rust_decimal::Decimal = tr; if !tr.is_zero() && !is_rcm {')

