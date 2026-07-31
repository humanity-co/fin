//! Accounts Payable command handlers (CQRS write side).
//!
//! Each command:
//! 1. Validates business rules
//! 2. Performs the mutation within a DB transaction
//! 3. Writes to the outbox for event publishing
//! 4. Integrates with GL for journal creation

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use sutra_finance_gl::{CreateJournalCmd, CreateJournalLineCmd, GlCommandHandler, PostJournalCmd};

use crate::errors::ApError;
use crate::models::vendor::{
    PanStatus, Section197Certificate, Vendor, VendorBankAccount, VendorType,
};
use crate::models::purchase_order::{
    PoStatus, PurchaseOrder, PurchaseOrderLine, TaxType,
};
use crate::models::goods_receipt::{GoodsReceiptNote, GoodsReceiptNoteLine, GrnStatus};
use crate::models::vendor_invoice::{
    InvoiceLine, InvoiceStatus, MatchingStatus, PaymentStatus as InvPaymentStatus, VendorInvoice,
};
use crate::models::vendor_payment::{
    PaymentAllocation, PaymentMode, PaymentType, TdsDeduction, TdsDepositStatus,
    VendorPayment, VpStatus,
};

// ─── Command Definitions ────────────────────────────────────────────────

/// Onboard a new vendor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVendorCmd {
    pub entity_id: Option<Uuid>,
    pub vendor_code: String,
    pub vendor_name: String,
    pub vendor_type: String,
    pub pan: Option<String>,
    pub gstin: Option<String>,
    pub gst_composition_scheme: bool,
    pub registration_type: String,
    pub contact_person: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub pincode: Option<String>,
    pub payment_terms: i32,
    pub default_tds_section: Option<String>,
    pub tds_applicable: bool,
    pub tax_applicable: bool,
    pub msme_reg_number: Option<String>,
    pub msme_type: Option<String>,
}

/// Create a purchase order from a PR or directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseOrderCmd {
    pub entity_id: Uuid,
    pub vendor_id: Uuid,
    pub purchase_requisition_id: Option<Uuid>,
    pub order_date: NaiveDate,
    pub delivery_date: Option<NaiveDate>,
    pub payment_terms: Option<String>,
    pub fund_id: Option<Uuid>,
    pub budget_head_id: Option<Uuid>,
    pub lines: Vec<CreatePoLineCmd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePoLineCmd {
    pub line_number: i32,
    pub item_description: String,
    pub hsn_sac_code: Option<String>,
    pub quantity: f64,
    pub unit_price: f64,
    pub discount_percent: Option<f64>,
    pub tax_rate: Option<f64>,
    pub tax_type: Option<String>,
    pub account_id: Uuid,
    pub cost_center_id: Option<Uuid>,
    pub rcm_applicable: bool,
}

/// Issue a draft purchase order to the vendor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuePurchaseOrderCmd {
    pub purchase_order_id: Uuid,
    pub issued_by_id: Uuid,
}

/// Record goods received against a PO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordGoodsReceiptCmd {
    pub purchase_order_id: Uuid,
    pub received_date: NaiveDate,
    pub received_by_id: Uuid,
    pub remarks: Option<String>,
    pub lines: Vec<GrnLineCmd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrnLineCmd {
    pub po_line_id: Uuid,
    pub received_quantity: f64,
    pub accepted_quantity: f64,
    pub rejected_quantity: f64,
    pub rejection_reason: Option<String>,
}

/// Record a vendor invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordVendorInvoiceCmd {
    pub entity_id: Uuid,
    pub invoice_number: String,
    pub invoice_date: NaiveDate,
    pub purchase_order_id: Option<Uuid>,
    pub goods_receipt_note_id: Option<Uuid>,
    pub vendor_id: Uuid,
    pub due_date: NaiveDate,
    pub is_rcm: bool,
    pub rcm_payable_amount: Option<f64>,
    pub lines: Vec<InvoiceLineCmd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineCmd {
    pub po_line_id: Option<Uuid>,
    pub line_number: i32,
    pub item_description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub tax_rate: Option<f64>,
    pub account_id: Uuid,
    pub cost_center_id: Option<Uuid>,
}

/// Run 3-way matching on an invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchInvoiceCmd {
    pub invoice_id: Uuid,
    pub reviewed_by: Uuid,
    pub accept_mismatch: bool,
    pub tolerance_percent: Option<f64>,
}

/// Post an approved invoice to GL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInvoiceCmd {
    pub invoice_id: Uuid,
    pub posted_by: Uuid,
    pub expense_account_ids: std::collections::HashMap<Uuid, Uuid>, // po_line_id → expense_account_override
    pub accounts_payable_account_id: Uuid,
    pub entity_id: Uuid,
    pub accounting_period_id: Uuid,
}

/// Create a vendor payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVendorPaymentCmd {
    pub entity_id: Uuid,
    pub vendor_id: Uuid,
    pub payment_mode: String,
    pub payment_date: NaiveDate,
    pub amount: f64,
    pub bank_account_id: Option<Uuid>,
    pub cheque_number: Option<String>,
    pub cheque_date: Option<NaiveDate>,
    pub remarks: Option<String>,
    pub allocations: Vec<PaymentAllocationCmd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAllocationCmd {
    pub invoice_id: Uuid,
    pub allocated_amount: f64,
}

/// Process a payment — execute GL postings and TDS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPaymentCmd {
    pub payment_id: Uuid,
    pub processed_by: Uuid,
    pub bank_transaction_ref: Option<String>,
    pub bank_account_id: Uuid,
    pub accounts_payable_account_id: Uuid,
    pub tds_payable_account_id: Uuid,
    pub tds_expense_account_id: Uuid,
    pub entity_id: Uuid,
    pub accounting_period_id: Uuid,
}

// ─── Command Handler ────────────────────────────────────────────────────

pub struct ApCommandHandler {
    pool: PgPool,
}

impl ApCommandHandler {
    pub fn new(pool: PgPool) -> Self {
        ApCommandHandler { pool }
    }

    fn gl_handler(&self) -> GlCommandHandler {
        GlCommandHandler::new(self.pool.clone())
    }

    // ─── Onboard Vendor ────────────────────────────────────────────────

    pub async fn create_vendor(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: CreateVendorCmd,
    ) -> Result<Vendor, ApError> {
        let tid = *tenant_id.as_uuid();

        let vendor_type = VendorType::from_db_str(&cmd.vendor_type);
        let registration_type = crate::models::vendor::RegistrationType::from_db_str(&cmd.registration_type);
        let vendor_id = EntityId::new();
        let audit = AuditInfo::new(created_by);

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO vendors (
                vendor_id, tenant_id, entity_id, vendor_code, vendor_name, vendor_type,
                pan, pan_status, gstin, gstin_status, gst_composition_scheme,
                registration_type, contact_person, contact_email, contact_phone,
                address_line1, address_line2, city, state, pincode,
                payment_terms, default_tds_section, tds_applicable, tax_applicable,
                msme_reg_no, msme_type, is_active, is_blacklisted,
                created_by, created_at, updated_by, updated_at, entity_version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,
                      $16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,TRUE,FALSE,
                      $27,now(),$27,now(),1)"#,
        )
        .bind(vendor_id.as_uuid())
        .bind(tid)
        .bind(cmd.entity_id)
        .bind(&cmd.vendor_code)
        .bind(&cmd.vendor_name)
        .bind(vendor_type.to_db_str())
        .bind(&cmd.pan)
        .bind(PanStatus::Unverified.to_db_str())
        .bind(&cmd.gstin)
        .bind(crate::models::vendor::GstinStatus::Unverified.to_db_str())
        .bind(cmd.gst_composition_scheme)
        .bind(registration_type.to_db_str())
        .bind(&cmd.contact_person)
        .bind(&cmd.contact_email)
        .bind(&cmd.contact_phone)
        .bind(&cmd.address_line1)
        .bind(&cmd.address_line2)
        .bind(&cmd.city)
        .bind(&cmd.state)
        .bind(&cmd.pincode)
        .bind(cmd.payment_terms)
        .bind(&cmd.default_tds_section)
        .bind(cmd.tds_applicable)
        .bind(cmd.tax_applicable)
        .bind(&cmd.msme_reg_number)
        .bind(&cmd.msme_type)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        write_ap_outbox(
            &mut tx,
            tid,
            "Vendor",
            &vendor_id.as_uuid().to_string(),
            "VendorCreated",
            &serde_json::json!({
                "vendor_id": vendor_id.as_uuid().to_string(),
                "vendor_code": cmd.vendor_code,
                "vendor_name": cmd.vendor_name,
                "pan": cmd.pan,
                "occurred_at": Utc::now(),
            }),
        )
        .await?;

        tx.commit().await?;

        info!(tenant_id = %tid, vendor_id = %vendor_id, "Vendor created");

        Ok(Vendor {
            vendor_id,
            tenant_id,
            entity_id: cmd.entity_id,
            vendor_code: cmd.vendor_code,
            vendor_name: cmd.vendor_name,
            vendor_type,
            pan: cmd.pan,
            pan_status: PanStatus::Unverified,
            gstin: cmd.gstin,
            gstin_status: crate::models::vendor::GstinStatus::Unverified,
            gst_composition_scheme: cmd.gst_composition_scheme,
            registration_type,
            contact_person: cmd.contact_person,
            contact_email: cmd.contact_email,
            contact_phone: cmd.contact_phone,
            address_line1: cmd.address_line1,
            address_line2: cmd.address_line2,
            city: cmd.city,
            state: cmd.state,
            pincode: cmd.pincode,
            payment_terms: cmd.payment_terms,
            default_tds_section: cmd.default_tds_section,
            tds_applicable: cmd.tds_applicable,
            tax_applicable: cmd.tax_applicable,
            is_active: true,
            is_blacklisted: false,
            blacklist_reason: None,
            msme_reg_number: cmd.msme_reg_number,
            msme_type: cmd.msme_type.and_then(|s| {
                Some(crate::models::vendor::MsmeType::from_db_str(&s))
            }),
            section_197_certificates: vec![],
            bank_accounts: vec![],
            audit,
        })
    }

    // ─── Create Purchase Order ──────────────────────────────────────────

    pub async fn create_purchase_order(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: CreatePurchaseOrderCmd,
    ) -> Result<PurchaseOrder, ApError> {
        let tid = *tenant_id.as_uuid();

        // Validate vendor exists and is not blacklisted
        let vendor_row = sqlx::query_as::<_, VendorCheckRow>(
            r#"SELECT vendor_id, is_blacklisted, is_active, default_tds_section,
               registration_type, tds_applicable
               FROM vendors
               WHERE vendor_id = $1 AND tenant_id = $2 AND deleted_at IS NULL"#,
        )
        .bind(cmd.vendor_id)
        .bind(tid)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ApError::VendorNotFound(cmd.vendor_id.to_string()))?;

        if vendor_row.is_blacklisted {
            return Err(ApError::VendorBlacklisted);
        }
        if !vendor_row.is_active {
            return Err(ApError::Validation("Vendor is not active".into()));
        }

        let mut tx = self.pool.begin().await?;

        // Generate PO number
        let fy = fiscal_year_from_date(cmd.order_date);
        let seq = next_po_sequence(&mut tx, tid, &fy).await?;
        let po_number = format!("PO-{}-{:06}", fy, seq);

        let po_id = EntityId::new();
        let audit = AuditInfo::new(created_by);

        // Build lines and compute totals
        let mut lines = Vec::with_capacity(cmd.lines.len());
        let mut total_amount = Money::ZERO;
        let mut tax_amount = Money::ZERO;

        // Determine if RCM applies
        let is_rcm = vendor_row.registration_type == "UNREGISTERED"
            || vendor_row.registration_type == "COMPOSITION";

        // TDS section from vendor default (can be overridden per PO)
        let tds_section = vendor_row.default_tds_section.clone();
        let tds_rate = get_default_tds_rate(&tds_section);

        for line_cmd in &cmd.lines {
            let qty = Decimal::from_f64(line_cmd.quantity)
                .ok_or_else(|| ApError::Validation("Invalid quantity".into()))?;
            let up = Money::from_rupees(line_cmd.unit_price);
            let line_total_paise = (qty * Decimal::from(up.as_paise()))
                .round()
                .try_into()
                .map_err(|_| ApError::Validation("Line total overflow".into()))?;
            let line_total = Money::from_paise(line_total_paise);

            let tax_rate = line_cmd.tax_rate.map(|r| Decimal::from_f64(r).unwrap_or_default());
            let discount_pct = line_cmd.discount_percent.map(|d| Decimal::from_f64(d).unwrap_or_default());

            let tax_type = line_cmd.tax_type.as_deref().map(TaxType::from_db_str);

            total_amount += line_total;
            if let Some(tr) = tax_rate {
                if !tr.is_zero() && !is_rcm {
                    let tax_paise = (line_total_paise as f64 * f64::from(tr) / 100.0) as i64;
                    tax_amount += Money::from_paise(tax_paise);
                }
            }

            let po_line_id = Uuid::now_v7();
            lines.push(PurchaseOrderLine {
                po_line_id,
                purchase_order_id: po_id,
                line_number: line_cmd.line_number,
                item_description: line_cmd.item_description.clone(),
                hsn_sac_code: line_cmd.hsn_sac_code.clone(),
                quantity: qty,
                unit_price: up,
                discount_percent: discount_pct,
                tax_rate,
                tax_type,
                total_amount: line_total,
                received_quantity: Decimal::ZERO,
                account_id: line_cmd.account_id,
                cost_center_id: line_cmd.cost_center_id,
                rcm_applicable: line_cmd.rcm_applicable || is_rcm,
            });

            // Persist line
            sqlx::query(
                r#"INSERT INTO purchase_order_lines (
                    po_line_id, tenant_id, purchase_order_id, line_number,
                    item_description, hsn_sac_code, quantity, unit_price,
                    discount_percent, tax_rate, tax_type, total_amount,
                    received_quantity, account_id, cost_center_id,
                    created_by, created_at, updated_by, updated_at
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,now(),$16,now())"#,
            )
            .bind(po_line_id)
            .bind(tid)
            .bind(po_id.as_uuid())
            .bind(line_cmd.line_number)
            .bind(&line_cmd.item_description)
            .bind(&line_cmd.hsn_sac_code)
            .bind(line_cmd.quantity.to_string())
            .bind(up.as_paise())
            .bind(discount_pct.map(|d| d.to_string()))
            .bind(tax_rate.map(|r| r.to_string()))
            .bind(tax_type.map(|t| t.to_db_str().to_string()))
            .bind(line_total.as_paise())
            .bind(Decimal::ZERO.to_string())
            .bind(line_cmd.account_id)
            .bind(line_cmd.cost_center_id)
            .bind(created_by)
            .execute(&mut *tx)
            .await?;
        }

        let net_amount = if is_rcm { total_amount } else { total_amount + tax_amount };

        // Persist PO
        sqlx::query(
            r#"INSERT INTO purchase_orders (
                purchase_order_id, tenant_id, entity_id, po_number,
                vendor_id, purchase_requisition_id, order_date, delivery_date,
                payment_terms, status, total_amount, tax_amount, net_amount,
                is_rcm_applicable, tds_section, tds_rate,
                fund_id, budget_head_id, created_by, created_at, updated_by, updated_at, entity_version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'DRAFT',$10,$11,$12,$13,$14,$15,$16,$17,$18,now(),$18,now(),1)"#,
        )
        .bind(po_id.as_uuid())
        .bind(tid)
        .bind(cmd.entity_id)
        .bind(&po_number)
        .bind(cmd.vendor_id)
        .bind(cmd.purchase_requisition_id)
        .bind(cmd.order_date)
        .bind(cmd.delivery_date)
        .bind(&cmd.payment_terms)
        .bind(total_amount.as_paise())
        .bind(tax_amount.as_paise())
        .bind(net_amount.as_paise())
        .bind(is_rcm)
        .bind(&tds_section)
        .bind(tds_rate.map(|r| r.to_string()))
        .bind(cmd.fund_id)
        .bind(cmd.budget_head_id)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        write_ap_outbox(
            &mut tx, tid, "PurchaseOrder", &po_id.as_uuid().to_string(),
            "PurchaseOrderCreated",
            &serde_json::json!({
                "po_id": po_id.as_uuid().to_string(),
                "po_number": po_number,
                "vendor_id": cmd.vendor_id.to_string(),
                "total_amount": total_amount.as_paise(),
                "occurred_at": Utc::now(),
            }),
        ).await?;

        tx.commit().await?;

        info!(tenant_id = %tid, po_number = %po_number, "Purchase Order created");

        Ok(PurchaseOrder {
            purchase_order_id: po_id,
            tenant_id,
            entity_id: cmd.entity_id,
            po_number,
            vendor_id: cmd.vendor_id,
            purchase_requisition_id: cmd.purchase_requisition_id,
            order_date: cmd.order_date,
            delivery_date: cmd.delivery_date,
            payment_terms: cmd.payment_terms,
            status: PoStatus::Draft,
            total_amount,
            tax_amount,
            net_amount,
            is_rcm_applicable: is_rcm,
            tds_section,
            tds_rate,
            fund_id: cmd.fund_id,
            budget_head_id: cmd.budget_head_id,
            issued_by_id: None,
            approved_by_id: None,
            lines,
            audit,
        })
    }

    // ─── Issue Purchase Order ───────────────────────────────────────────

    pub async fn issue_purchase_order(
        &self,
        tenant_id: TenantId,
        cmd: IssuePurchaseOrderCmd,
    ) -> Result<PurchaseOrder, ApError> {
        let tid = *tenant_id.as_uuid();
        let mut tx = self.pool.begin().await?;

        let po_row = sqlx::query_as::<_, PoRow>(
            r#"SELECT purchase_order_id, tenant_id, entity_id, po_number, vendor_id,
               purchase_requisition_id, order_date, delivery_date, payment_terms,
               status, total_amount, tax_amount, net_amount, is_rcm_applicable,
               tds_section, tds_rate, fund_id, budget_head_id
               FROM purchase_orders
               WHERE purchase_order_id = $1 AND tenant_id = $2 AND deleted_at IS NULL
               FOR UPDATE"#,
        )
        .bind(cmd.purchase_order_id)
        .bind(tid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApError::PONotFound(cmd.purchase_order_id.to_string()))?;

        if po_row.status != "DRAFT" {
            return Err(ApError::Validation(format!(
                "PO must be in DRAFT status to issue, current: {}", po_row.status
            )));
        }

        sqlx::query(
            r#"UPDATE purchase_orders
               SET status = 'ISSUED', issued_by_id = $1, updated_at = now(), entity_version = entity_version + 1
               WHERE purchase_order_id = $2 AND tenant_id = $3"#,
        )
        .bind(cmd.issued_by_id)
        .bind(cmd.purchase_order_id)
        .bind(tid)
        .execute(&mut *tx)
        .await?;

        write_ap_outbox(
            &mut tx, tid, "PurchaseOrder", &cmd.purchase_order_id.to_string(),
            "PurchaseOrderIssued",
            &serde_json::json!({
                "po_id": cmd.purchase_order_id.to_string(),
                "po_number": po_row.po_number,
                "vendor_id": po_row.vendor_id.to_string(),
                "occurred_at": Utc::now(),
            }),
        ).await?;

        tx.commit().await?;

        // Load lines
        let lines = load_po_lines(&self.pool, tid, cmd.purchase_order_id).await?;

        info!(tenant_id = %tid, po_id = %cmd.purchase_order_id, "PO issued");

        Ok(PurchaseOrder {
            purchase_order_id: EntityId::from_uuid(cmd.purchase_order_id.into()),
            tenant_id,
            entity_id: po_row.entity_id,
            po_number: po_row.po_number,
            vendor_id: po_row.vendor_id,
            purchase_requisition_id: po_row.purchase_requisition_id,
            order_date: po_row.order_date,
            delivery_date: po_row.delivery_date,
            payment_terms: po_row.payment_terms,
            status: PoStatus::Issued,
            total_amount: Money::from_paise(po_row.total_amount_paise),
            tax_amount: Money::from_paise(po_row.tax_amount_paise),
            net_amount: Money::from_paise(po_row.net_amount_paise),
            is_rcm_applicable: po_row.is_rcm_applicable,
            tds_section: po_row.tds_section,
            tds_rate: po_row.tds_rate,
            fund_id: po_row.fund_id,
            budget_head_id: po_row.budget_head_id,
            issued_by_id: Some(cmd.issued_by_id),
            approved_by_id: None,
            lines,
            audit: AuditInfo::new(cmd.issued_by_id),
        })
    }

    // ─── Record Goods Receipt ───────────────────────────────────────────

    pub async fn record_goods_receipt(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: RecordGoodsReceiptCmd,
    ) -> Result<GoodsReceiptNote, ApError> {
        let tid = *tenant_id.as_uuid();
        let mut tx = self.pool.begin().await?;

        // Verify PO exists and is in a receivable state
        let po_row = sqlx::query_as::<_, PoRow>(
            r#"SELECT purchase_order_id, tenant_id, entity_id, po_number, vendor_id,
               purchase_requisition_id, order_date, delivery_date, payment_terms,
               status, total_amount, tax_amount, net_amount, is_rcm_applicable,
               tds_section, tds_rate, fund_id, budget_head_id
               FROM purchase_orders
               WHERE purchase_order_id = $1 AND tenant_id = $2 AND deleted_at IS NULL
               FOR UPDATE"#,
        )
        .bind(cmd.purchase_order_id)
        .bind(tid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApError::PONotFound(cmd.purchase_order_id.to_string()))?;

        if po_row.status != "ISSUED" && po_row.status != "ACKNOWLEDGED" && po_row.status != "PARTIALLY_RECEIVED" {
            return Err(ApError::Validation(format!(
                "PO must be ISSUED/PARTIALLY_RECEIVED to record GRN, current: {}",
                po_row.status
            )));
        }

        // Generate GRN number
        let seq = next_grn_sequence(&mut tx, tid).await?;
        let grn_number = format!("GRN-{:06}", seq);

        let grn_id = EntityId::new();
        let audit = AuditInfo::new(created_by);

        sqlx::query(
            r#"INSERT INTO goods_receipt_notes (
                goods_receipt_note_id, tenant_id, grn_number, purchase_order_id,
                received_date, received_by_id, status, remarks,
                created_by, created_at, updated_by, updated_at, entity_version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,now(),$9,now(),1)"#,
        )
        .bind(grn_id.as_uuid())
        .bind(tid)
        .bind(&grn_number)
        .bind(cmd.purchase_order_id)
        .bind(cmd.received_date)
        .bind(cmd.received_by_id)
        .bind("DRAFT")
        .bind(&cmd.remarks)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        let mut grn_lines = Vec::with_capacity(cmd.lines.len());
        let mut all_received = true;

        for line_cmd in &cmd.lines {
            let recvd = Decimal::from_f64(line_cmd.received_quantity)
                .ok_or_else(|| ApError::Validation("Invalid received quantity".into()))?;
            let accepted = Decimal::from_f64(line_cmd.accepted_quantity)
                .ok_or_else(|| ApError::Validation("Invalid accepted quantity".into()))?;
            let rejected = Decimal::from_f64(line_cmd.rejected_quantity)
                .ok_or_else(|| ApError::Validation("Invalid rejected quantity".into()))?;

            let grn_line_id = Uuid::now_v7();

            sqlx::query(
                r#"INSERT INTO goods_receipt_note_lines (
                    grn_line_id, tenant_id, goods_receipt_note_id, po_line_id,
                    received_quantity, accepted_quantity, rejected_quantity, rejection_reason,
                    created_at, created_by, updated_at, updated_by
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,now(),$9,now(),$9)"#,
            )
            .bind(grn_line_id)
            .bind(tid)
            .bind(grn_id.as_uuid())
            .bind(line_cmd.po_line_id)
            .bind(line_cmd.received_quantity.to_string())
            .bind(line_cmd.accepted_quantity.to_string())
            .bind(line_cmd.rejected_quantity.to_string())
            .bind(&line_cmd.rejection_reason)
            .bind(created_by)
            .execute(&mut *tx)
            .await?;

            // Update PO line received quantity
            sqlx::query(
                r#"UPDATE purchase_order_lines
                   SET received_quantity = received_quantity + $1,
                       updated_at = now(), updated_by = $2
                   WHERE po_line_id = $3 AND tenant_id = $4"#,
            )
            .bind(line_cmd.received_quantity.to_string())
            .bind(created_by)
            .bind(line_cmd.po_line_id)
            .bind(tid)
            .execute(&mut *tx)
            .await?;

            grn_lines.push(GoodsReceiptNoteLine {
                grn_line_id,
                goods_receipt_note_id: grn_id,
                po_line_id: line_cmd.po_line_id,
                received_quantity: recvd,
                accepted_quantity: accepted,
                rejected_quantity: rejected,
                rejection_reason: line_cmd.rejection_reason.clone(),
            });
        }

        // Check if all PO lines are now fully received — update PO status
        let all_lines = sqlx::query_as::<_, PoLineSumRow>(
            r#"SELECT quantity, received_quantity FROM purchase_order_lines
               WHERE purchase_order_id = $1 AND tenant_id = $2"#,
        )
        .bind(cmd.purchase_order_id)
        .bind(tid)
        .fetch_all(&mut *tx)
        .await?;

        for line in &all_lines {
            if line.received_qty < line.ordered_qty {
                all_received = false;
                break;
            }
        }

        let new_status = if all_received { "FULLY_RECEIVED" } else { "PARTIALLY_RECEIVED" };
        sqlx::query(
            r#"UPDATE purchase_orders SET status = $1, updated_at = now(), entity_version = entity_version + 1
               WHERE purchase_order_id = $2 AND tenant_id = $3"#,
        )
        .bind(new_status)
        .bind(cmd.purchase_order_id)
        .bind(tid)
        .execute(&mut *tx)
        .await?;

        // Complete GRN
        sqlx::query(
            r#"UPDATE goods_receipt_notes SET status = 'COMPLETED', updated_at = now()
               WHERE goods_receipt_note_id = $1"#,
        )
        .bind(grn_id.as_uuid())
        .execute(&mut *tx)
        .await?;

        write_ap_outbox(
            &mut tx, tid, "GoodsReceiptNote", &grn_id.as_uuid().to_string(),
            "GoodsReceiptNoteCompleted",
            &serde_json::json!({
                "grn_id": grn_id.as_uuid().to_string(),
                "grn_number": grn_number,
                "po_id": cmd.purchase_order_id.to_string(),
                "po_status": new_status,
                "occurred_at": Utc::now(),
            }),
        ).await?;

        tx.commit().await?;

        info!(tenant_id = %tid, grn_number = %grn_number, "GRN recorded");

        Ok(GoodsReceiptNote {
            goods_receipt_note_id: grn_id,
            tenant_id,
            grn_number,
            purchase_order_id: cmd.purchase_order_id,
            received_date: cmd.received_date,
            received_by_id: Some(cmd.received_by_id),
            status: GrnStatus::Completed,
            remarks: cmd.remarks,
            lines: grn_lines,
            audit,
        })
    }

    // ─── Record Vendor Invoice ──────────────────────────────────────────

    pub async fn record_vendor_invoice(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: RecordVendorInvoiceCmd,
    ) -> Result<VendorInvoice, ApError> {
        let tid = *tenant_id.as_uuid();
        let mut tx = self.pool.begin().await?;

        let invoice_id = EntityId::new();
        let audit = AuditInfo::new(created_by);

        let mut lines = Vec::with_capacity(cmd.lines.len());
        let mut total_amount = Money::ZERO;
        let mut tax_amount = Money::ZERO;
        let mut tds_amount = Money::ZERO;

        // Compute TDS if PO is linked
        let (tds_section, tds_rate, is_rcm) = if let Some(po_id) = cmd.purchase_order_id {
            let po = sqlx::query_as::<_, PoTdsRow>(
                r#"SELECT tds_section, tds_rate, is_rcm_applicable
                   FROM purchase_orders
                   WHERE purchase_order_id = $1 AND tenant_id = $2"#,
            )
            .bind(po_id)
            .bind(tid)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| ApError::PONotFound(po_id.to_string()))?;
            (po.tds_section, po.tds_rate, po.is_rcm_applicable)
        } else {
            (None, None, cmd.is_rcm)
        };

        for line_cmd in &cmd.lines {
            let qty = Decimal::from_f64(line_cmd.quantity)
                .ok_or_else(|| ApError::Validation("Invalid quantity".into()))?;
            let up = Money::from_rupees(line_cmd.unit_price);
            let line_total_paise = (qty * Decimal::from(up.as_paise()))
                .round()
                .try_into()
                .map_err(|_| ApError::Validation("Line total overflow".into()))?;
            let line_total = Money::from_paise(line_total_paise);

            let tax_rate = line_cmd.tax_rate.map(|r| Decimal::from_f64(r).unwrap_or_default());
            let line_tax = if let Some(tr) = tax_rate {
                if !tr.is_zero() && !is_rcm {
                    let t = (line_total_paise as f64 * f64::from(tr) / 100.0) as i64;
                    Money::from_paise(t)
                } else {
                    Money::ZERO
                }
            } else {
                Money::ZERO
            };

            total_amount += line_total;
            tax_amount += line_tax;

            let inv_line_id = Uuid::now_v7();

            sqlx::query(
                r#"INSERT INTO vendor_invoice_lines (
                    invoice_line_id, tenant_id, vendor_invoice_id, po_line_id,
                    line_number, item_description, quantity, unit_price,
                    tax_rate, tax_amount, total_amount, account_id, cost_center_id,
                    created_at, created_by, updated_at, updated_by
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,now(),$14,now(),$14)"#,
            )
            .bind(inv_line_id)
            .bind(tid)
            .bind(invoice_id.as_uuid())
            .bind(line_cmd.po_line_id)
            .bind(line_cmd.line_number)
            .bind(&line_cmd.item_description)
            .bind(line_cmd.quantity.to_string())
            .bind(up.as_paise())
            .bind(tax_rate.map(|r| r.to_string()))
            .bind(line_tax.as_paise())
            .bind(line_total.as_paise())
            .bind(line_cmd.account_id)
            .bind(line_cmd.cost_center_id)
            .bind(created_by)
            .execute(&mut *tx)
            .await?;

            lines.push(InvoiceLine {
                invoice_line_id: inv_line_id,
                vendor_invoice_id: invoice_id,
                po_line_id: line_cmd.po_line_id,
                line_number: line_cmd.line_number,
                item_description: line_cmd.item_description.clone(),
                quantity: qty,
                unit_price: up,
                tax_rate,
                tax_amount: Some(line_tax),
                total_amount: line_total,
                account_id: line_cmd.account_id,
                cost_center_id: line_cmd.cost_center_id,
            });
        }

        // Calculate TDS
        if let (Some(section), Some(rate)) = (&tds_section, &tds_rate) {
            let net_for_tds = total_amount + tax_amount;
            let rate_dec = Decimal::from_f64(*rate).unwrap_or_default();
            let tds_paise = (net_for_tds.as_paise() as f64 * f64::from(rate_dec) / 100.0) as i64;
            tds_amount = Money::from_paise(tds_paise);
        }

        let net_amount = total_amount + tax_amount;

        sqlx::query(
            r#"INSERT INTO vendor_invoices (
                vendor_invoice_id, tenant_id, entity_id, invoice_number,
                invoice_date, purchase_order_id, goods_receipt_note_id, vendor_id,
                invoice_amount, tax_amount, net_amount, tds_amount,
                is_rcm, rcm_payable_amount, status, payment_status, due_date,
                created_by, created_at, updated_by, updated_at, entity_version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,'DRAFT','UNPAID',$15,$16,now(),$16,now(),1)"#,
        )
        .bind(invoice_id.as_uuid())
        .bind(tid)
        .bind(cmd.entity_id)
        .bind(&cmd.invoice_number)
        .bind(cmd.invoice_date)
        .bind(cmd.purchase_order_id)
        .bind(cmd.goods_receipt_note_id)
        .bind(cmd.vendor_id)
        .bind(total_amount.as_paise())
        .bind(tax_amount.as_paise())
        .bind(net_amount.as_paise())
        .bind(tds_amount.as_paise())
        .bind(is_rcm)
        .bind(cmd.rcm_payable_amount.map(|a| (a * 100.0) as i64))
        .bind(cmd.due_date)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        write_ap_outbox(
            &mut tx, tid, "VendorInvoice", &invoice_id.as_uuid().to_string(),
            "PurchaseInvoiceCreated",
            &serde_json::json!({
                "invoice_id": invoice_id.as_uuid().to_string(),
                "invoice_number": cmd.invoice_number,
                "vendor_id": cmd.vendor_id.to_string(),
                "amount": net_amount.as_paise(),
                "occurred_at": Utc::now(),
            }),
        ).await?;

        tx.commit().await?;

        Ok(VendorInvoice {
            vendor_invoice_id: invoice_id,
            tenant_id,
            entity_id: cmd.entity_id,
            invoice_number: cmd.invoice_number,
            invoice_date: cmd.invoice_date,
            purchase_order_id: cmd.purchase_order_id,
            goods_receipt_note_id: cmd.goods_receipt_note_id,
            vendor_id: cmd.vendor_id,
            invoice_amount: total_amount,
            tax_amount,
            net_amount,
            tds_amount,
            is_rcm,
            rcm_payable_amount: cmd.rcm_payable_amount.map(Money::from_rupees),
            matching_status: MatchingStatus::Pending,
            status: InvoiceStatus::Draft,
            payment_status: InvPaymentStatus::Unpaid,
            due_date: cmd.due_date,
            posted_journal_id: None,
            approved_by_id: None,
            lines,
            audit,
        })
    }

    // ─── Match Invoice (3-way) ──────────────────────────────────────────

    pub async fn match_invoice(
        &self,
        tenant_id: TenantId,
        cmd: MatchInvoiceCmd,
    ) -> Result<VendorInvoice, ApError> {
        let tid = *tenant_id.as_uuid();
        let tolerance_pct = cmd.tolerance_percent.unwrap_or(5.0);

        let mut tx = self.pool.begin().await?;

        let inv_row = sqlx::query_as::<_, InvoiceRow>(
            r#"SELECT vendor_invoice_id, tenant_id, entity_id, invoice_number,
               invoice_date, purchase_order_id, goods_receipt_note_id, vendor_id,
               invoice_amount, tax_amount, net_amount, tds_amount, is_rcm,
               rcm_payable_amount, status, payment_status, due_date
               FROM vendor_invoices
               WHERE vendor_invoice_id = $1 AND tenant_id = $2 AND deleted_at IS NULL
               FOR UPDATE"#,
        )
        .bind(cmd.invoice_id)
        .bind(tid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApError::InvoiceNotFound(cmd.invoice_id.to_string()))?;

        if inv_row.status != "DRAFT" {
            return Err(ApError::Validation("Invoice must be DRAFT for matching".into()));
        }

        let mut mismatches = Vec::new();

        // If PO linked, do 2-way (PO vs Invoice) or 3-way (PO vs GRN vs Invoice)
        if let Some(po_id) = inv_row.purchase_order_id {
            // Compare invoice lines with PO lines
            let inv_lines = sqlx::query_as::<_, InvLineRow>(
                r#"SELECT po_line_id, quantity, unit_price, total_amount
                   FROM vendor_invoice_lines
                   WHERE vendor_invoice_id = $1 AND tenant_id = $2"#,
            )
            .bind(cmd.invoice_id)
            .bind(tid)
            .fetch_all(&mut *tx)
            .await?;

            for line in &inv_lines {
                if let Some(po_line_id) = line.po_line_id {
                    let po_line = sqlx::query_as::<_, PoLineRow>(
                        r#"SELECT quantity, unit_price, total_amount, received_quantity
                           FROM purchase_order_lines
                           WHERE po_line_id = $1 AND tenant_id = $2"#,
                    )
                    .bind(po_line_id)
                    .bind(tid)
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or_else(|| ApError::Validation(format!("PO line {} not found", po_line_id)))?;

                    // Quantity match
                    let po_qty = po_line.quantity;
                    let inv_qty = line.quantity;
                    if !is_within_tolerance(po_qty, inv_qty, tolerance_pct) {
                        mismatches.push(format!(
                            "Qty mismatch on PO line {}: PO={} vs Inv={}",
                            po_line_id, po_qty, inv_qty
                        ));
                    }

                    // Amount match
                    let po_total = Money::from_paise(po_line.total_amount_paise);
                    let inv_total = Money::from_paise(line.total_amount_paise);
                    if !is_money_within_tolerance(po_total, inv_total, tolerance_pct) {
                        mismatches.push(format!(
                            "Amount mismatch on PO line {}: PO={} vs Inv={}",
                            po_line_id, po_total, inv_total
                        ));
                    }

                    // 3-way: Also compare with GRN if available
                    if inv_row.goods_receipt_note_id.is_some() {
                        let grn_qty = sqlx::query_scalar::<_, Option<String>>(
                            r#"SELECT accepted_quantity FROM goods_receipt_note_lines
                               WHERE po_line_id = $1 AND tenant_id = $2
                               ORDER BY created_at DESC LIMIT 1"#,
                        )
                        .bind(po_line_id)
                        .bind(tid)
                        .fetch_optional(&mut *tx)
                        .await?
                        .and_then(|s| s);

                        if let Some(grn_qty_str) = grn_qty {
                            if let Ok(grn_qty_dec) = grn_qty_str.parse::<f64>() {
                                let grn_dec = Decimal::from_f64(grn_qty_dec).unwrap_or_default();
                                if !is_within_tolerance(grn_dec, inv_qty, tolerance_pct) {
                                    mismatches.push(format!(
                                        "3-way Qty mismatch on PO line {}: GRN={} vs Inv={}",
                                        po_line_id, grn_qty_str, inv_qty
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        let matched = mismatches.is_empty() || cmd.accept_mismatch;
        let new_status = if matched { "MATCHED" } else { "MISMATCHED" };

        sqlx::query(
            r#"UPDATE vendor_invoices
               SET status = $1, updated_at = now(), entity_version = entity_version + 1
               WHERE vendor_invoice_id = $2 AND tenant_id = $3"#,
        )
        .bind(new_status)
        .bind(cmd.invoice_id)
        .bind(tid)
        .execute(&mut *tx)
        .await?;

        write_ap_outbox(
            &mut tx, tid, "VendorInvoice", &cmd.invoice_id.to_string(),
            "InvoiceMatched",
            &serde_json::json!({
                "invoice_id": cmd.invoice_id.to_string(),
                "status": new_status,
                "mismatches": mismatches,
                "occurred_at": Utc::now(),
            }),
        ).await?;

        tx.commit().await?;

        Ok(VendorInvoice {
            vendor_invoice_id: EntityId::from_uuid(cmd.invoice_id.into()),
            tenant_id,
            entity_id: inv_row.entity_id,
            invoice_number: inv_row.invoice_number,
            invoice_date: inv_row.invoice_date,
            purchase_order_id: inv_row.purchase_order_id,
            goods_receipt_note_id: inv_row.goods_receipt_note_id,
            vendor_id: inv_row.vendor_id,
            invoice_amount: Money::from_paise(inv_row.invoice_amount_paise),
            tax_amount: Money::from_paise(inv_row.tax_amount_paise),
            net_amount: Money::from_paise(inv_row.net_amount_paise),
            tds_amount: Money::from_paise(inv_row.tds_amount_paise),
            is_rcm: inv_row.is_rcm,
            rcm_payable_amount: inv_row.rcm_payable_amount_paise.map(Money::from_paise),
            matching_status: if matched { MatchingStatus::Matched } else { MatchingStatus::Mismatch },
            status: InvoiceStatus::from_db_str(new_status),
            payment_status: InvPaymentStatus::from_db_str(&inv_row.payment_status),
            due_date: inv_row.due_date,
            posted_journal_id: None,
            approved_by_id: None,
            lines: vec![],
            audit: AuditInfo::new(cmd.reviewed_by),
        })
    }

    // ─── Post Invoice to GL ─────────────────────────────────────────────

    pub async fn post_invoice(
        &self,
        tenant_id: TenantId,
        cmd: PostInvoiceCmd,
    ) -> Result<VendorInvoice, ApError> {
        let tid = *tenant_id.as_uuid();

        // Load invoice
        let inv_row = sqlx::query_as::<_, InvoiceRow>(
            r#"SELECT vendor_invoice_id, tenant_id, entity_id, invoice_number,
               invoice_date, purchase_order_id, goods_receipt_note_id, vendor_id,
               invoice_amount, tax_amount, net_amount, tds_amount, is_rcm,
               rcm_payable_amount, status, payment_status, due_date
               FROM vendor_invoices
               WHERE vendor_invoice_id = $1 AND tenant_id = $2 AND deleted_at IS NULL
               FOR UPDATE"#,
        )
        .bind(cmd.invoice_id)
        .bind(tid)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ApError::InvoiceNotFound(cmd.invoice_id.to_string()))?;

        if inv_row.status != "MATCHED" && inv_row.status != "APPROVED" {
            return Err(ApError::Validation(format!(
                "Invoice must be MATCHED/APPROVED to post, current: {}",
                inv_row.status
            )));
        }

        // Load invoice lines
        let lines = sqlx::query_as::<_, InvLineRow>(
            r#"SELECT po_line_id, quantity, unit_price, total_amount
               FROM vendor_invoice_lines
               WHERE vendor_invoice_id = $1 AND tenant_id = $2"#,
        )
        .bind(cmd.invoice_id)
        .bind(tid)
        .fetch_all(&self.pool)
        .await?;

        let invoice_amount = Money::from_paise(inv_row.invoice_amount_paise);
        let tax_amount = Money::from_paise(inv_row.tax_amount_paise);
        let total_posting = invoice_amount + tax_amount;

        // ── Create GL Journal: DR Expense accounts → CR Accounts Payable ──
        let mut journal_lines = Vec::new();
        let mut line_num = 1;

        for line in &lines {
            let line_amount = Money::from_paise(line.total_amount_paise);
            // Find the expense account for this line
            let expense_acct = line.po_line_id
                .and_then(|pol_id| cmd.expense_account_ids.get(&pol_id))
                .copied()
                .unwrap_or(cmd.accounts_payable_account_id); // fallback

            journal_lines.push(CreateJournalLineCmd {
                line_number: line_num,
                account_id: expense_acct,
                debit_amount: Some(line_amount),
                credit_amount: None,
                description: Some(format!("Invoice {} — line expense", inv_row.invoice_number)),
                cost_center_id: None,
                fund_id: None,
                reference_id: Some(inv_row.invoice_number.clone()),
                reference_type: Some("VENDOR_INVOICE".to_string()),
            });
            line_num += 1;
        }

        // CR Accounts Payable for the total
        journal_lines.push(CreateJournalLineCmd {
            line_number: line_num,
            account_id: cmd.accounts_payable_account_id,
            debit_amount: None,
            credit_amount: Some(total_posting),
            description: Some(format!("Vendor invoice {}", inv_row.invoice_number)),
            cost_center_id: None,
            fund_id: None,
            reference_id: Some(inv_row.invoice_number.clone()),
            reference_type: Some("VENDOR_INVOICE".to_string()),
        });

        let gl = self.gl_handler();
        let journal_cmd = CreateJournalCmd {
            journal_type: "Standard".to_string(),
            accounting_period_id: cmd.accounting_period_id,
            entity_id: cmd.entity_id,
            fund_id: None,
            cost_center_id: None,
            posting_date: inv_row.invoice_date,
            description: format!("Vendor invoice {} — {}", inv_row.invoice_number, inv_row.vendor_id),
            lines: journal_lines,
            attachment_ids: vec![],
        };

        let journal = gl.create_journal(tenant_id, cmd.posted_by, journal_cmd).await
            .map_err(|e| ApError::Internal(format!("GL journal creation failed: {e}")))?;

        let journal = gl.post_journal(
            tenant_id,
            PostJournalCmd {
                journal_id: *journal.journal_id.as_uuid(),
                posted_by: cmd.posted_by,
            },
        ).await
            .map_err(|e| ApError::Internal(format!("GL journal post failed: {e}")))?;

        let posted_journal_id = *journal.journal_id.as_uuid();

        // Update invoice
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"UPDATE vendor_invoices
               SET status = 'POSTED', posted_journal_id = $1, updated_at = now(), entity_version = entity_version + 1
               WHERE vendor_invoice_id = $2 AND tenant_id = $3"#,
        )
        .bind(posted_journal_id)
        .bind(cmd.invoice_id)
        .bind(tid)
        .execute(&mut *tx)
        .await?;

        write_ap_outbox(
            &mut tx, tid, "VendorInvoice", &cmd.invoice_id.to_string(),
            "InvoicePosted",
            &serde_json::json!({
                "invoice_id": cmd.invoice_id.to_string(),
                "journal_id": posted_journal_id.to_string(),
                "occurred_at": Utc::now(),
            }),
        ).await?;

        tx.commit().await?;

        info!(tenant_id = %tid, invoice_id = %cmd.invoice_id, journal_id = %posted_journal_id, "Invoice posted to GL");

        Ok(VendorInvoice {
            vendor_invoice_id: EntityId::from_uuid(cmd.invoice_id.into()),
            tenant_id,
            entity_id: inv_row.entity_id,
            invoice_number: inv_row.invoice_number,
            invoice_date: inv_row.invoice_date,
            purchase_order_id: inv_row.purchase_order_id,
            goods_receipt_note_id: inv_row.goods_receipt_note_id,
            vendor_id: inv_row.vendor_id,
            invoice_amount,
            tax_amount,
            net_amount: Money::from_paise(inv_row.net_amount_paise),
            tds_amount: Money::from_paise(inv_row.tds_amount_paise),
            is_rcm: inv_row.is_rcm,
            rcm_payable_amount: inv_row.rcm_payable_amount_paise.map(Money::from_paise),
            matching_status: MatchingStatus::from_db_str(&inv_row.status),
            status: InvoiceStatus::Posted,
            payment_status: InvPaymentStatus::from_db_str(&inv_row.payment_status),
            due_date: inv_row.due_date,
            posted_journal_id: Some(posted_journal_id),
            approved_by_id: None,
            lines: vec![],
            audit: AuditInfo::new(cmd.posted_by),
        })
    }

    // ─── Create Vendor Payment ──────────────────────────────────────────

    pub async fn create_vendor_payment(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: CreateVendorPaymentCmd,
    ) -> Result<VendorPayment, ApError> {
        let tid = *tenant_id.as_uuid();
        let mut tx = self.pool.begin().await?;

        // Validate vendor
        let vendor = sqlx::query_as::<_, VendorCheckRow>(
            r#"SELECT vendor_id, is_blacklisted, is_active, default_tds_section,
               registration_type, tds_applicable
               FROM vendors
               WHERE vendor_id = $1 AND tenant_id = $2 AND deleted_at IS NULL"#,
        )
        .bind(cmd.vendor_id)
        .bind(tid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApError::VendorNotFound(cmd.vendor_id.to_string()))?;

        if !vendor.is_active || vendor.is_blacklisted {
            return Err(ApError::VendorBlacklisted);
        }

        let amount = Money::from_rupees(cmd.amount);
        let payment_mode = PaymentMode::from_db_str(&cmd.payment_mode);

        // Generate payment number
        let seq = next_payment_sequence(&mut tx, tid).await?;
        let payment_number = format!("PAY-{:06}", seq);

        let payment_id = EntityId::new();
        let audit = AuditInfo::new(created_by);

        // Calculate TDS: check for Section 197 certificates, apply lower rate
        let tds_section = vendor.default_tds_section.clone();
        let default_tds_rate = get_default_tds_rate(&tds_section);
        let effective_tds_rate = get_effective_tds_rate(&mut tx, tid, cmd.vendor_id, &tds_section, default_tds_rate).await?;

        let tds_amount = if vendor.tds_applicable && effective_tds_rate.is_some() {
            let rate = effective_tds_rate.unwrap_or(Decimal::ZERO);
            let tds_paise = (amount.as_paise() as f64 * f64::from(rate) / 100.0) as i64;
            Money::from_paise(tds_paise)
        } else {
            Money::ZERO
        };

        let net_amount = amount - tds_amount;

        sqlx::query(
            r#"INSERT INTO vendor_payments (
                payment_id, tenant_id, entity_id, payment_number, vendor_id,
                payment_type, payment_mode, payment_date, amount, tds_amount,
                net_amount, status, bank_account_id, cheque_number, cheque_date,
                remarks, created_by, created_at, updated_by, updated_at, entity_version
            ) VALUES ($1,$2,$3,$4,$5,'VENDOR_PAYMENT',$6,$7,$8,$9,$10,'INITIATED',$11,$12,$13,$14,$15,now(),$15,now(),1)"#,
        )
        .bind(payment_id.as_uuid())
        .bind(tid)
        .bind(cmd.entity_id)
        .bind(&payment_number)
        .bind(cmd.vendor_id)
        .bind(payment_mode.to_db_str())
        .bind(cmd.payment_date)
        .bind(amount.as_paise())
        .bind(tds_amount.as_paise())
        .bind(net_amount.as_paise())
        .bind(cmd.bank_account_id)
        .bind(&cmd.cheque_number)
        .bind(cmd.cheque_date)
        .bind(&cmd.remarks)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        // Create allocations
        let mut allocations = Vec::new();
        for alloc_cmd in &cmd.allocations {
            let alloc_amount = Money::from_rupees(alloc_cmd.allocated_amount);
            let alloc_tds = if vendor.tds_applicable && effective_tds_rate.is_some() {
                let rate = effective_tds_rate.unwrap_or(Decimal::ZERO);
                let tds_p = (alloc_amount.as_paise() as f64 * f64::from(rate) / 100.0) as i64;
                Money::from_paise(tds_p)
            } else {
                Money::ZERO
            };

            let alloc_id = Uuid::now_v7();
            sqlx::query(
                r#"INSERT INTO vendor_payment_allocations (
                    vendor_payment_alloc_id, tenant_id, payment_id, invoice_id,
                    allocated_amount, tds_amount, created_at
                ) VALUES ($1,$2,$3,$4,$5,$6,now())"#,
            )
            .bind(alloc_id)
            .bind(tid)
            .bind(payment_id.as_uuid())
            .bind(alloc_cmd.invoice_id)
            .bind(alloc_amount.as_paise())
            .bind(alloc_tds.as_paise())
            .execute(&mut *tx)
            .await?;

            allocations.push(PaymentAllocation {
                vendor_payment_alloc_id: alloc_id,
                payment_id,
                invoice_id: alloc_cmd.invoice_id,
                allocated_amount: alloc_amount,
                tds_amount: alloc_tds,
            });
        }

        write_ap_outbox(
            &mut tx, tid, "VendorPayment", &payment_id.as_uuid().to_string(),
            "PaymentInitiated",
            &serde_json::json!({
                "payment_id": payment_id.as_uuid().to_string(),
                "payment_number": payment_number,
                "vendor_id": cmd.vendor_id.to_string(),
                "amount": amount.as_paise(),
                "tds_amount": tds_amount.as_paise(),
                "occurred_at": Utc::now(),
            }),
        ).await?;

        tx.commit().await?;

        info!(tenant_id = %tid, payment_number = %payment_number, "Payment created");

        Ok(VendorPayment {
            payment_id,
            tenant_id,
            entity_id: cmd.entity_id,
            payment_number,
            vendor_id: cmd.vendor_id,
            payment_type: PaymentType::VendorPayment,
            payment_mode,
            payment_date: cmd.payment_date,
            amount,
            tds_amount,
            net_amount,
            status: VpStatus::Initiated,
            bank_account_id: cmd.bank_account_id,
            bank_transaction_ref: None,
            cheque_number: cmd.cheque_number,
            cheque_date: cmd.cheque_date,
            approved_by_id: None,
            processed_by_id: None,
            payment_journal_id: None,
            remarks: cmd.remarks,
            allocations,
            audit,
        })
    }

    // ─── Process Payment (execute GL + TDS) ─────────────────────────────

    pub async fn process_payment(
        &self,
        tenant_id: TenantId,
        cmd: ProcessPaymentCmd,
    ) -> Result<VendorPayment, ApError> {
        let tid = *tenant_id.as_uuid();

        // Load payment
        let pay_row = sqlx::query_as::<_, PaymentRow>(
            r#"SELECT payment_id, tenant_id, entity_id, payment_number, vendor_id,
               payment_type, payment_mode, payment_date, amount, tds_amount,
               net_amount, status, bank_account_id, cheque_number, cheque_date
               FROM vendor_payments
               WHERE payment_id = $1 AND tenant_id = $2 AND deleted_at IS NULL
               FOR UPDATE"#,
        )
        .bind(cmd.payment_id)
        .bind(tid)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ApError::PaymentNotFound(cmd.payment_id.to_string()))?;

        if pay_row.status != "INITIATED" && pay_row.status != "APPROVED" {
            return Err(ApError::Validation(format!(
                "Payment must be INITIATED/APPROVED, current: {}", pay_row.status
            )));
        }

        let amount = Money::from_paise(pay_row.amount_paise);
        let tds_amount = Money::from_paise(pay_row.tds_amount_paise);
        let net_amount = Money::from_paise(pay_row.net_amount_paise);

        let gl = self.gl_handler();

        // ── Payment Journal: DR Accounts Payable → CR Bank (net of TDS) ──
        let mut journal_lines = vec![
            CreateJournalLineCmd {
                line_number: 1,
                account_id: cmd.accounts_payable_account_id,
                debit_amount: Some(amount),
                credit_amount: None,
                description: Some(format!("Payment {} — DR AP", pay_row.payment_number)),
                cost_center_id: None,
                fund_id: None,
                reference_id: Some(pay_row.payment_number.clone()),
                reference_type: Some("VENDOR_PAYMENT".to_string()),
            },
            CreateJournalLineCmd {
                line_number: 2,
                account_id: cmd.bank_account_id,
                debit_amount: None,
                credit_amount: Some(net_amount),
                description: Some(format!("Payment {} — CR Bank (net of TDS)", pay_row.payment_number)),
                cost_center_id: None,
                fund_id: None,
                reference_id: Some(pay_row.payment_number.clone()),
                reference_type: Some("VENDOR_PAYMENT".to_string()),
            },
        ];

        // TDS entry if TDS was deducted
        let mut tds_journal_id: Option<Uuid> = None;
        if !tds_amount.is_zero() {
            journal_lines.push(CreateJournalLineCmd {
                line_number: 3,
                account_id: cmd.tds_payable_account_id,
                debit_amount: None,
                credit_amount: Some(tds_amount),
                description: Some(format!("TDS deducted — payment {}", pay_row.payment_number)),
                cost_center_id: None,
                fund_id: None,
                reference_id: Some(pay_row.payment_number.clone()),
                reference_type: Some("TDS_DEDUCTION".to_string()),
            });

            // Create a separate TDS expense journal if needed
            let tds_journal_cmd = CreateJournalCmd {
                journal_type: "TDS".to_string(),
                accounting_period_id: cmd.accounting_period_id,
                entity_id: cmd.entity_id,
                fund_id: None,
                cost_center_id: None,
                posting_date: pay_row.payment_date,
                description: format!("TDS deduction — payment {} — {}", pay_row.payment_number, pay_row.vendor_id),
                lines: vec![
                    CreateJournalLineCmd {
                        line_number: 1,
                        account_id: cmd.tds_expense_account_id,
                        debit_amount: Some(tds_amount),
                        credit_amount: None,
                        description: Some("TDS Expense".to_string()),
                        cost_center_id: None,
                        fund_id: None,
                        reference_id: Some(pay_row.payment_number.clone()),
                        reference_type: Some("TDS".to_string()),
                    },
                    CreateJournalLineCmd {
                        line_number: 2,
                        account_id: cmd.tds_payable_account_id,
                        debit_amount: None,
                        credit_amount: Some(tds_amount),
                        description: Some("TDS Payable".to_string()),
                        cost_center_id: None,
                        fund_id: None,
                        reference_id: Some(pay_row.payment_number.clone()),
                        reference_type: Some("TDS".to_string()),
                    },
                ],
                attachment_ids: vec![],
            };

            let tds_j = gl.create_journal(tenant_id, cmd.processed_by, tds_journal_cmd).await
                .map_err(|e| ApError::Internal(format!("TDS journal creation failed: {e}")))?;
            let tds_j = gl.post_journal(tenant_id, PostJournalCmd {
                journal_id: *tds_j.journal_id.as_uuid(),
                posted_by: cmd.processed_by,
            }).await
                .map_err(|e| ApError::Internal(format!("TDS journal post failed: {e}")))?;

            tds_journal_id = Some(*tds_j.journal_id.as_uuid());
        }

        // Create payment journal
        let journal_cmd = CreateJournalCmd {
            journal_type: "Standard".to_string(),
            accounting_period_id: cmd.accounting_period_id,
            entity_id: cmd.entity_id,
            fund_id: None,
            cost_center_id: None,
            posting_date: pay_row.payment_date,
            description: format!("Vendor payment {} — {}", pay_row.payment_number, pay_row.vendor_id),
            lines: journal_lines,
            attachment_ids: vec![],
        };

        let journal = gl.create_journal(tenant_id, cmd.processed_by, journal_cmd).await
            .map_err(|e| ApError::Internal(format!("Payment journal creation failed: {e}")))?;
        let journal = gl.post_journal(tenant_id, PostJournalCmd {
            journal_id: *journal.journal_id.as_uuid(),
            posted_by: cmd.processed_by,
        }).await
            .map_err(|e| ApError::Internal(format!("Payment journal post failed: {e}")))?;

        let payment_journal_id = *journal.journal_id.as_uuid();

        // Update payment
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"UPDATE vendor_payments
               SET status = 'PROCESSED', processed_by_id = $1,
                   bank_transaction_ref = $2, payment_journal_id = $3,
                   updated_at = now(), entity_version = entity_version + 1
               WHERE payment_id = $4 AND tenant_id = $5"#,
        )
        .bind(cmd.processed_by)
        .bind(&cmd.bank_transaction_ref)
        .bind(payment_journal_id)
        .bind(cmd.payment_id)
        .bind(tid)
        .execute(&mut *tx)
        .await?;

        // Update invoice payment statuses
        let allocations = sqlx::query_as::<_, PaymentAllocRow>(
            r#"SELECT invoice_id, allocated_amount
               FROM vendor_payment_allocations
               WHERE payment_id = $1 AND tenant_id = $2"#,
        )
        .bind(cmd.payment_id)
        .bind(tid)
        .fetch_all(&mut *tx)
        .await?;

        for alloc in &allocations {
            // Check if invoice is now fully paid
            let total_allocated: Option<i64> = sqlx::query_scalar(
                r#"SELECT COALESCE(SUM(allocated_amount), 0)
                   FROM vendor_payment_allocations
                   WHERE invoice_id = $1 AND tenant_id = $2"#,
            )
            .bind(alloc.invoice_id)
            .bind(tid)
            .fetch_one(&mut *tx)
            .await?;

            let inv_row = sqlx::query_as::<_, InvAmountRow>(
                r#"SELECT net_amount FROM vendor_invoices
                   WHERE vendor_invoice_id = $1 AND tenant_id = $2"#,
            )
            .bind(alloc.invoice_id)
            .bind(tid)
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(inv) = inv_row {
                let new_ps = if let Some(total) = total_allocated {
                    if total >= inv.net_amount_paise { "PAID" }
                    else if total > 0 { "PARTIALLY_PAID" }
                    else { "UNPAID" }
                } else { "UNPAID" };

                sqlx::query(
                    r#"UPDATE vendor_invoices
                       SET payment_status = $1, status = CASE WHEN $1 = 'PAID' THEN 'PAID' ELSE status END,
                           updated_at = now(), entity_version = entity_version + 1
                       WHERE vendor_invoice_id = $2 AND tenant_id = $3"#,
                )
                .bind(new_ps)
                .bind(alloc.invoice_id)
                .bind(tid)
                .execute(&mut *tx)
                .await?;
            }
        }

        write_ap_outbox(
            &mut tx, tid, "VendorPayment", &cmd.payment_id.to_string(),
            "PaymentProcessed",
            &serde_json::json!({
                "payment_id": cmd.payment_id.to_string(),
                "payment_journal_id": payment_journal_id.to_string(),
                "tds_journal_id": tds_journal_id.map(|id| id.to_string()),
                "bank_reference": cmd.bank_transaction_ref,
                "processed_by": cmd.processed_by.to_string(),
                "occurred_at": Utc::now(),
            }),
        ).await?;

        tx.commit().await?;

        info!(tenant_id = %tid, payment_id = %cmd.payment_id, "Payment processed");

        Ok(VendorPayment {
            payment_id: EntityId::from_uuid(cmd.payment_id.into()),
            tenant_id,
            entity_id: pay_row.entity_id,
            payment_number: pay_row.payment_number,
            vendor_id: pay_row.vendor_id,
            payment_type: PaymentType::from_db_str(&pay_row.payment_type),
            payment_mode: PaymentMode::from_db_str(&pay_row.payment_mode),
            payment_date: pay_row.payment_date,
            amount,
            tds_amount,
            net_amount,
            status: VpStatus::Processed,
            bank_account_id: pay_row.bank_account_id,
            bank_transaction_ref: cmd.bank_transaction_ref,
            cheque_number: pay_row.cheque_number,
            cheque_date: pay_row.cheque_date,
            approved_by_id: None,
            processed_by_id: Some(cmd.processed_by),
            payment_journal_id: Some(payment_journal_id),
            remarks: None,
            allocations: vec![],
            audit: AuditInfo::new(cmd.processed_by),
        })
    }
}

// ─── Helper Functions ───────────────────────────────────────────────────

fn fiscal_year_from_date(date: NaiveDate) -> String {
    let year = date.year();
    let month = date.month();
    if month >= 4 {
        format!("{}-{:02}", year, (year + 1) % 100)
    } else {
        format!("{}-{:02}", year - 1, year % 100)
    }
}

fn get_default_tds_rate(section: &Option<String>) -> Option<Decimal> {
    match section.as_deref() {
        Some("194C") => Some(Decimal::new(1, 0)),    // 1% individual/HUF, 2% others — default 2%
        Some("194J") => Some(Decimal::new(10, 0)),    // 10%
        Some("194I") => Some(Decimal::new(2, 0)),     // 2% for plant & machinery, 10% for other — default 2%
        Some("194H") => Some(Decimal::new(5, 0)),     // 5%
        Some("194A") => Some(Decimal::new(10, 0)),    // 10%
        Some("194D") => Some(Decimal::new(10, 0)),    // 10%
        Some("194G") => Some(Decimal::new(5, 0)),     // 5%
        _ => None,
    }
}

async fn get_effective_tds_rate(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    vendor_id: Uuid,
    section: &Option<String>,
    default_rate: Option<Decimal>,
) -> Result<Option<Decimal>, ApError> {
    if section.is_none() || default_rate.is_none() {
        return Ok(None);
    }

    let section = section.as_deref().unwrap();
    let today = Utc::now().date_naive();

    // Check for active Section 197 certificates
    let cert_rate = sqlx::query_scalar::<_, Option<String>>(
        r#"SELECT specified_rate FROM section_197_certificates
           WHERE vendor_id = $1 AND tenant_id = $2 AND section = $3
           AND is_active = TRUE AND valid_from <= $4 AND valid_to >= $4
           ORDER BY valid_to DESC LIMIT 1"#,
    )
    .bind(vendor_id)
    .bind(tenant_id)
    .bind(section)
    .bind(today)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(rate_str) = cert_rate {
        if let Ok(rate) = rate_str.parse::<f64>() {
            return Ok(Some(Decimal::from_f64(rate).unwrap_or(default_rate.unwrap())));
        }
    }

    Ok(default_rate)
}

fn is_within_tolerance(po_val: Decimal, inv_val: Decimal, tolerance_pct: f64) -> bool {
    if po_val.is_zero() && inv_val.is_zero() {
        return true;
    }
    let po_f64 = f64::from(po_val);
    let inv_f64 = f64::from(inv_val);
    let diff = (po_f64 - inv_f64).abs();
    let tolerance = (po_f64.abs() * tolerance_pct / 100.0).max(0.01);
    diff <= tolerance
}

fn is_money_within_tolerance(po_val: Money, inv_val: Money, tolerance_pct: f64) -> bool {
    if po_val.is_zero() && inv_val.is_zero() {
        return true;
    }
    let po = po_val.as_paise() as f64;
    let inv = inv_val.as_paise() as f64;
    let diff = (po - inv).abs();
    let tolerance = (po.abs() * tolerance_pct / 100.0).max(100.0); // min ₹1 tolerance
    diff <= tolerance
}

// ─── Outbox Writer ──────────────────────────────────────────────────────

async fn write_ap_outbox(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    aggregate_type: &str,
    aggregate_id: &str,
    event_type: &str,
    event_payload: &serde_json::Value,
) -> Result<(), ApError> {
    sqlx::query(
        r#"INSERT INTO event_outbox (
            outbox_id, tenant_id, aggregate_type, aggregate_id,
            event_type, event_payload, status, retry_count, max_retries, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, 'PENDING', 0, 5, now())"#,
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(aggregate_type)
    .bind(aggregate_id)
    .bind(event_type)
    .bind(event_payload)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

// ─── Sequence Generators ────────────────────────────────────────────────

async fn next_po_sequence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    fy: &str,
) -> Result<i64, ApError> {
    let row = sqlx::query_scalar::<_, Option<i64>>(
        r#"SELECT COALESCE(MAX(
               CASE WHEN po_number ~ ('^PO-' || $2 || '-[0-9]+$')
                    THEN (regexp_replace(po_number, '^PO-' || $2 || '-', ''))::bigint
                    ELSE 0 END
           ), 0) + 1
           FROM purchase_orders WHERE tenant_id = $1"#,
    )
    .bind(tenant_id)
    .bind(fy)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.unwrap_or(1))
}

async fn next_grn_sequence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
) -> Result<i64, ApError> {
    let row = sqlx::query_scalar::<_, Option<i64>>(
        r#"SELECT COALESCE(MAX(
               CASE WHEN grn_number ~ '^GRN-[0-9]+$'
                    THEN (regexp_replace(grn_number, '^GRN-', ''))::bigint
                    ELSE 0 END
           ), 0) + 1
           FROM goods_receipt_notes WHERE tenant_id = $1"#,
    )
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.unwrap_or(1))
}

async fn next_payment_sequence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
) -> Result<i64, ApError> {
    let row = sqlx::query_scalar::<_, Option<i64>>(
        r#"SELECT COALESCE(MAX(
               CASE WHEN payment_number ~ '^PAY-[0-9]+$'
                    THEN (regexp_replace(payment_number, '^PAY-', ''))::bigint
                    ELSE 0 END
           ), 0) + 1
           FROM vendor_payments WHERE tenant_id = $1"#,
    )
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.unwrap_or(1))
}

async fn load_po_lines(
    pool: &PgPool,
    tenant_id: Uuid,
    po_id: Uuid,
) -> Result<Vec<PurchaseOrderLine>, ApError> {
    let rows = sqlx::query_as::<_, PoLineDbRow>(
        r#"SELECT po_line_id, line_number, item_description, hsn_sac_code,
           quantity, unit_price, discount_percent, tax_rate, tax_type,
           total_amount, received_quantity, account_id, cost_center_id
           FROM purchase_order_lines
           WHERE purchase_order_id = $1 AND tenant_id = $2
           ORDER BY line_number"#,
    )
    .bind(po_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| PurchaseOrderLine {
        po_line_id: r.po_line_id,
        purchase_order_id: EntityId::from_uuid(po_id.into()),
        line_number: r.line_number,
        item_description: r.item_description,
        hsn_sac_code: r.hsn_sac_code,
        quantity: Decimal::from_str_exact(&r.quantity).unwrap_or_default(),
        unit_price: Money::from_paise(r.unit_price_paise),
        discount_percent: r.discount_percent,
        tax_rate: r.tax_rate,
        tax_type: r.tax_type.map(|s| TaxType::from_db_str(&s)),
        total_amount: Money::from_paise(r.total_amount_paise),
        received_quantity: Decimal::from_str_exact(&r.received_quantity).unwrap_or_default(),
        account_id: r.account_id,
        cost_center_id: r.cost_center_id,
        rcm_applicable: false,
    }).collect())
}

// ─── DB Row Types ────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct VendorCheckRow {
    #[allow(dead_code)]
    vendor_id: Uuid,
    is_blacklisted: bool,
    is_active: bool,
    default_tds_section: Option<String>,
    #[allow(dead_code)]
    registration_type: String,
    tds_applicable: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct PoRow {
    #[allow(dead_code)]
    purchase_order_id: Uuid,
    #[allow(dead_code)]
    tenant_id: Uuid,
    entity_id: Uuid,
    po_number: String,
    vendor_id: Uuid,
    purchase_requisition_id: Option<Uuid>,
    order_date: NaiveDate,
    delivery_date: Option<NaiveDate>,
    payment_terms: Option<String>,
    status: String,
    total_amount_paise: i64,
    tax_amount_paise: i64,
    net_amount_paise: i64,
    is_rcm_applicable: bool,
    tds_section: Option<String>,
    tds_rate: Option<Decimal>,
    fund_id: Option<Uuid>,
    budget_head_id: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
struct PoTdsRow {
    tds_section: Option<String>,
    tds_rate: Option<f64>,
    is_rcm_applicable: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct PoLineSumRow {
    ordered_qty: Decimal,
    received_qty: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
struct InvoiceRow {
    #[allow(dead_code)]
    vendor_invoice_id: Uuid,
    #[allow(dead_code)]
    tenant_id: Uuid,
    entity_id: Uuid,
    invoice_number: String,
    invoice_date: NaiveDate,
    purchase_order_id: Option<Uuid>,
    goods_receipt_note_id: Option<Uuid>,
    vendor_id: Uuid,
    invoice_amount_paise: i64,
    tax_amount_paise: i64,
    net_amount_paise: i64,
    tds_amount_paise: i64,
    is_rcm: bool,
    rcm_payable_amount_paise: Option<i64>,
    status: String,
    payment_status: String,
    due_date: NaiveDate,
}

#[derive(Debug, sqlx::FromRow)]
struct InvLineRow {
    po_line_id: Option<Uuid>,
    quantity: Decimal,
    #[allow(dead_code)]
    unit_price_paise: i64,
    total_amount_paise: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct PoLineRow {
    quantity: Decimal,
    #[allow(dead_code)]
    unit_price_paise: i64,
    total_amount_paise: i64,
    #[allow(dead_code)]
    received_quantity: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
struct PoLineDbRow {
    po_line_id: Uuid,
    line_number: i32,
    item_description: String,
    hsn_sac_code: Option<String>,
    quantity: String,
    unit_price_paise: i64,
    discount_percent: Option<Decimal>,
    tax_rate: Option<Decimal>,
    tax_type: Option<String>,
    total_amount_paise: i64,
    received_quantity: String,
    account_id: Uuid,
    cost_center_id: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
struct PaymentRow {
    #[allow(dead_code)]
    payment_id: Uuid,
    #[allow(dead_code)]
    tenant_id: Uuid,
    entity_id: Uuid,
    payment_number: String,
    vendor_id: Uuid,
    payment_type: String,
    payment_mode: String,
    payment_date: NaiveDate,
    amount_paise: i64,
    tds_amount_paise: i64,
    net_amount_paise: i64,
    status: String,
    bank_account_id: Option<Uuid>,
    cheque_number: Option<String>,
    cheque_date: Option<NaiveDate>,
}

#[derive(Debug, sqlx::FromRow)]
struct PaymentAllocRow {
    invoice_id: Uuid,
    #[allow(dead_code)]
    allocated_amount: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct InvAmountRow {
    net_amount_paise: i64,
}
