//! Accounts Payable API routes — v1.
//!
//! ## Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | POST   | /api/v1/ap/vendors | Onboard vendor |
//! | GET    | /api/v1/ap/vendors | List/search vendors |
//! | GET    | /api/v1/ap/vendors/:id | Get vendor |
//! | POST   | /api/v1/ap/purchase-orders | Create purchase order |
//! | PUT    | /api/v1/ap/purchase-orders/:id/issue | Issue PO |
//! | POST   | /api/v1/ap/goods-receipts | Record GRN |
//! | POST   | /api/v1/ap/invoices | Record vendor invoice |
//! | PUT    | /api/v1/ap/invoices/:id/match | 3-way match review |
//! | PUT    | /api/v1/ap/invoices/:id/post | Post to GL |
//! | POST   | /api/v1/ap/payments | Create vendor payment |
//! | PUT    | /api/v1/ap/payments/:id/process | Process payment |
//! | GET    | /api/v1/ap/tds/deductions | TDS deduction register |

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use sutra_core::{Money, TenantId};
use sutra_finance_ap::{
    ApCommandHandler, CreatePurchaseOrderCmd, CreateVendorCmd,
    CreateVendorPaymentCmd, IssuePurchaseOrderCmd, MatchInvoiceCmd,
    PostInvoiceCmd, ProcessPaymentCmd, RecordGoodsReceiptCmd,
    RecordVendorInvoiceCmd,
};

use crate::state::AppState;

/// Create the AP routes sub-router.
pub fn ap_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/vendors", post(create_vendor).get(list_vendors))
        .route("/vendors/{id}", get(get_vendor))
        .route("/purchase-orders", post(create_purchase_order))
        .route("/purchase-orders/{id}/issue", put(issue_purchase_order))
        .route("/goods-receipts", post(record_goods_receipt))
        .route("/invoices", post(record_vendor_invoice))
        .route("/invoices/{id}/match", put(match_invoice))
        .route("/invoices/{id}/post", put(post_invoice))
        .route("/payments", post(create_vendor_payment))
        .route("/payments/{id}/process", put(process_payment))
        .route("/tds/deductions", get(list_tds_deductions))
}

// ─── Request Types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateVendorRequest {
    #[serde(default)]
    entity_id: Option<Uuid>,
    vendor_code: String,
    vendor_name: String,
    vendor_type: String,
    #[serde(default)]
    pan: Option<String>,
    #[serde(default)]
    gstin: Option<String>,
    #[serde(default)]
    gst_composition_scheme: bool,
    #[serde(default)]
    registration_type: Option<String>,
    #[serde(default)]
    contact_person: Option<String>,
    #[serde(default)]
    contact_email: Option<String>,
    #[serde(default)]
    contact_phone: Option<String>,
    #[serde(default)]
    address_line1: Option<String>,
    #[serde(default)]
    address_line2: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    pincode: Option<String>,
    #[serde(default = "default_payment_terms")]
    payment_terms: i32,
    #[serde(default)]
    default_tds_section: Option<String>,
    #[serde(default = "default_true")]
    tds_applicable: bool,
    #[serde(default = "default_true")]
    tax_applicable: bool,
    #[serde(default)]
    msme_reg_number: Option<String>,
    #[serde(default)]
    msme_type: Option<String>,
}

fn default_payment_terms() -> i32 { 30 }
fn default_true() -> bool { true }

#[derive(Debug, Deserialize)]
struct CreatePoRequest {
    entity_id: Uuid,
    vendor_id: Uuid,
    #[serde(default)]
    purchase_requisition_id: Option<Uuid>,
    #[serde(default = "today")]
    order_date: String,
    #[serde(default)]
    delivery_date: Option<String>,
    #[serde(default)]
    payment_terms: Option<String>,
    #[serde(default)]
    fund_id: Option<Uuid>,
    #[serde(default)]
    budget_head_id: Option<Uuid>,
    lines: Vec<PoLineRequest>,
}

fn today() -> String {
    chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string()
}

#[derive(Debug, Deserialize)]
struct PoLineRequest {
    line_number: i32,
    item_description: String,
    #[serde(default)]
    hsn_sac_code: Option<String>,
    quantity: f64,
    unit_price: f64,
    #[serde(default)]
    discount_percent: Option<f64>,
    #[serde(default)]
    tax_rate: Option<f64>,
    #[serde(default)]
    tax_type: Option<String>,
    account_id: Uuid,
    #[serde(default)]
    cost_center_id: Option<Uuid>,
    #[serde(default)]
    rcm_applicable: bool,
}

#[derive(Debug, Deserialize)]
struct IssuePoRequest {
    issued_by_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct GrnRequest {
    purchase_order_id: Uuid,
    #[serde(default = "today")]
    received_date: String,
    received_by_id: Uuid,
    #[serde(default)]
    remarks: Option<String>,
    lines: Vec<GrnLineRequest>,
}

#[derive(Debug, Deserialize)]
struct GrnLineRequest {
    po_line_id: Uuid,
    received_quantity: f64,
    accepted_quantity: f64,
    rejected_quantity: f64,
    #[serde(default)]
    rejection_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RecordInvoiceRequest {
    entity_id: Uuid,
    invoice_number: String,
    #[serde(default = "today")]
    invoice_date: String,
    #[serde(default)]
    purchase_order_id: Option<Uuid>,
    #[serde(default)]
    goods_receipt_note_id: Option<Uuid>,
    vendor_id: Uuid,
    #[serde(default = "due_date_default")]
    due_date: String,
    #[serde(default)]
    is_rcm: bool,
    #[serde(default)]
    rcm_payable_amount: Option<f64>,
    lines: Vec<InvoiceLineRequest>,
}

fn due_date_default() -> String {
    let d = chrono::Utc::now().date_naive() + chrono::Duration::days(30);
    d.format("%Y-%m-%d").to_string()
}

#[derive(Debug, Deserialize)]
struct InvoiceLineRequest {
    #[serde(default)]
    po_line_id: Option<Uuid>,
    line_number: i32,
    item_description: String,
    quantity: f64,
    unit_price: f64,
    #[serde(default)]
    tax_rate: Option<f64>,
    account_id: Uuid,
    #[serde(default)]
    cost_center_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct MatchInvoiceRequest {
    reviewed_by: Uuid,
    #[serde(default)]
    accept_mismatch: bool,
    #[serde(default = "default_tolerance")]
    tolerance_percent: Option<f64>,
}

fn default_tolerance() -> Option<f64> { Some(5.0) }

#[derive(Debug, Deserialize)]
struct PostInvoiceRequest {
    posted_by: Uuid,
    #[serde(default)]
    expense_account_ids: std::collections::HashMap<Uuid, Uuid>,
    accounts_payable_account_id: Uuid,
    entity_id: Uuid,
    accounting_period_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct CreatePaymentRequest {
    entity_id: Uuid,
    vendor_id: Uuid,
    payment_mode: String,
    #[serde(default = "today")]
    payment_date: String,
    amount: f64,
    #[serde(default)]
    bank_account_id: Option<Uuid>,
    #[serde(default)]
    cheque_number: Option<String>,
    #[serde(default)]
    cheque_date: Option<String>,
    #[serde(default)]
    remarks: Option<String>,
    allocations: Vec<PaymentAllocRequest>,
}

#[derive(Debug, Deserialize)]
struct PaymentAllocRequest {
    invoice_id: Uuid,
    allocated_amount: f64,
}

#[derive(Debug, Deserialize)]
struct ProcessPaymentRequest {
    processed_by: Uuid,
    #[serde(default)]
    bank_transaction_ref: Option<String>,
    bank_account_id: Uuid,
    accounts_payable_account_id: Uuid,
    tds_payable_account_id: Uuid,
    tds_expense_account_id: Uuid,
    entity_id: Uuid,
    accounting_period_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct VendorListQuery {
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    vendor_type: Option<String>,
    #[serde(default)]
    is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TdsFilterQuery {
    #[serde(default)]
    from_date: Option<String>,
    #[serde(default)]
    to_date: Option<String>,
    #[serde(default)]
    section: Option<String>,
}

// ─── Error / OK Helpers ─────────────────────────────────────────────────

fn err_response(status: StatusCode, msg: String) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "success": false, "error": msg, "data": null::<()> })))
}

fn ok_response(data: impl serde::Serialize) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "success": true, "data": data, "error": null::<()> }))
}

fn parse_date(s: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

// ─── Handlers ──────────────────────────────────────────────────────────

/// POST /api/v1/ap/vendors
async fn create_vendor(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateVendorRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ApCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let created_by = Uuid::nil();

    let cmd = CreateVendorCmd {
        entity_id: body.entity_id,
        vendor_code: body.vendor_code,
        vendor_name: body.vendor_name,
        vendor_type: body.vendor_type,
        pan: body.pan,
        gstin: body.gstin,
        gst_composition_scheme: body.gst_composition_scheme,
        registration_type: body.registration_type.unwrap_or_else(|| "REGULAR".into()),
        contact_person: body.contact_person,
        contact_email: body.contact_email,
        contact_phone: body.contact_phone,
        address_line1: body.address_line1,
        address_line2: body.address_line2,
        city: body.city,
        state: body.state,
        pincode: body.pincode,
        payment_terms: body.payment_terms,
        default_tds_section: body.default_tds_section,
        tds_applicable: body.tds_applicable,
        tax_applicable: body.tax_applicable,
        msme_reg_number: body.msme_reg_number,
        msme_type: body.msme_type,
    };

    match handler.create_vendor(tenant_id, created_by, cmd).await {
        Ok(vendor) => Ok(ok_response(vendor)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// GET /api/v1/ap/vendors
async fn list_vendors(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<VendorListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let tid = Uuid::nil();
    let pool = &state.db;

    let mut query_str = String::from(
        r#"SELECT vendor_id, tenant_id, entity_id, vendor_code, vendor_name,
           vendor_type, pan, pan_status, gstin, gstin_status, contact_person,
           contact_email, contact_phone, payment_terms, default_tds_section,
           is_active, is_blacklisted, msme_reg_no, msme_type
           FROM vendors
           WHERE tenant_id = $1 AND deleted_at IS NULL"#,
    );
    let mut params: Vec<String> = vec![];
    let mut idx = 2;

    if let Some(ref s) = filter.search {
        query_str.push_str(&format!(" AND (vendor_name ILIKE ${} OR vendor_code ILIKE ${} OR pan ILIKE ${})", idx, idx+1, idx+2));
        params.push(format!("%{}%", s));
        params.push(format!("%{}%", s));
        params.push(format!("%{}%", s));
        idx += 3;
    }
    if let Some(ref vt) = filter.vendor_type {
        query_str.push_str(&format!(" AND vendor_type = ${}", idx));
        params.push(vt.clone());
        idx += 1;
    }
    if let Some(active) = filter.is_active {
        query_str.push_str(&format!(" AND is_active = ${}", idx));
        params.push(active.to_string());
        idx += 1;
    }

    query_str.push_str(" ORDER BY vendor_name ASC LIMIT 100");

    let mut q = sqlx::query(&query_str).bind(tid);
    for p in &params {
        q = q.bind(p);
    }

    let _rows: Vec<sqlx::postgres::PgRow> = q.fetch_all(pool).await
        .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(ok_response(serde_json::json!({
        "vendors": [],
        "total": 0,
    })))
}

/// GET /api/v1/ap/vendors/:id
async fn get_vendor(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let tid = Uuid::nil();
    let pool = &state.db;

    let vendor = sqlx::query_as::<_, (Uuid, String, String, String, Option<String>, Option<String>, bool, bool)>(
        r#"SELECT vendor_id, vendor_code, vendor_name, vendor_type, pan, gstin, is_active, is_blacklisted
           FROM vendors
           WHERE vendor_id = $1 AND tenant_id = $2 AND deleted_at IS NULL"#,
    )
    .bind(id)
    .bind(tid)
    .fetch_optional(pool)
    .await
    .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| err_response(StatusCode::NOT_FOUND, "Vendor not found".into()))?;

    Ok(ok_response(serde_json::json!({
        "vendor_id": vendor.0,
        "vendor_code": vendor.1,
        "vendor_name": vendor.2,
        "vendor_type": vendor.3,
        "pan": vendor.4,
        "gstin": vendor.5,
        "is_active": vendor.6,
        "is_blacklisted": vendor.7,
    })))
}

/// POST /api/v1/ap/purchase-orders
async fn create_purchase_order(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreatePoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ApCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let created_by = Uuid::nil();

    let order_date = parse_date(&body.order_date)
        .ok_or_else(|| err_response(StatusCode::BAD_REQUEST, "Invalid order_date".into()))?;
    let delivery_date = body.delivery_date.as_deref().and_then(parse_date);

    let lines: Vec<_> = body.lines.into_iter().map(|l| {
        sutra_finance_ap::commands::CreatePoLineCmd {
            line_number: l.line_number,
            item_description: l.item_description,
            hsn_sac_code: l.hsn_sac_code,
            quantity: l.quantity,
            unit_price: l.unit_price,
            discount_percent: l.discount_percent,
            tax_rate: l.tax_rate,
            tax_type: l.tax_type,
            account_id: l.account_id,
            cost_center_id: l.cost_center_id,
            rcm_applicable: l.rcm_applicable,
        }
    }).collect();

    let cmd = CreatePurchaseOrderCmd {
        entity_id: body.entity_id,
        vendor_id: body.vendor_id,
        purchase_requisition_id: body.purchase_requisition_id,
        order_date,
        delivery_date,
        payment_terms: body.payment_terms,
        fund_id: body.fund_id,
        budget_head_id: body.budget_head_id,
        lines,
    };

    match handler.create_purchase_order(tenant_id, created_by, cmd).await {
        Ok(po) => Ok(ok_response(po)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// PUT /api/v1/ap/purchase-orders/:id/issue
async fn issue_purchase_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<IssuePoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ApCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());

    let cmd = IssuePurchaseOrderCmd {
        purchase_order_id: id,
        issued_by_id: body.issued_by_id,
    };

    match handler.issue_purchase_order(tenant_id, cmd).await {
        Ok(po) => Ok(ok_response(po)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// POST /api/v1/ap/goods-receipts
async fn record_goods_receipt(
    State(state): State<Arc<AppState>>,
    Json(body): Json<GrnRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ApCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let created_by = Uuid::nil();

    let received_date = parse_date(&body.received_date)
        .ok_or_else(|| err_response(StatusCode::BAD_REQUEST, "Invalid received_date".into()))?;

    let lines: Vec<_> = body.lines.into_iter().map(|l| {
        sutra_finance_ap::commands::GrnLineCmd {
            po_line_id: l.po_line_id,
            received_quantity: l.received_quantity,
            accepted_quantity: l.accepted_quantity,
            rejected_quantity: l.rejected_quantity,
            rejection_reason: l.rejection_reason,
        }
    }).collect();

    let cmd = RecordGoodsReceiptCmd {
        purchase_order_id: body.purchase_order_id,
        received_date,
        received_by_id: body.received_by_id,
        remarks: body.remarks,
        lines,
    };

    match handler.record_goods_receipt(tenant_id, created_by, cmd).await {
        Ok(grn) => Ok(ok_response(grn)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// POST /api/v1/ap/invoices
async fn record_vendor_invoice(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RecordInvoiceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ApCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let created_by = Uuid::nil();

    let invoice_date = parse_date(&body.invoice_date)
        .ok_or_else(|| err_response(StatusCode::BAD_REQUEST, "Invalid invoice_date".into()))?;
    let due_date = parse_date(&body.due_date)
        .ok_or_else(|| err_response(StatusCode::BAD_REQUEST, "Invalid due_date".into()))?;

    let lines: Vec<_> = body.lines.into_iter().map(|l| {
        sutra_finance_ap::commands::InvoiceLineCmd {
            po_line_id: l.po_line_id,
            line_number: l.line_number,
            item_description: l.item_description,
            quantity: l.quantity,
            unit_price: l.unit_price,
            tax_rate: l.tax_rate,
            account_id: l.account_id,
            cost_center_id: l.cost_center_id,
        }
    }).collect();

    let cmd = RecordVendorInvoiceCmd {
        entity_id: body.entity_id,
        invoice_number: body.invoice_number,
        invoice_date,
        purchase_order_id: body.purchase_order_id,
        goods_receipt_note_id: body.goods_receipt_note_id,
        vendor_id: body.vendor_id,
        due_date,
        is_rcm: body.is_rcm,
        rcm_payable_amount: body.rcm_payable_amount,
        lines,
    };

    match handler.record_vendor_invoice(tenant_id, created_by, cmd).await {
        Ok(invoice) => Ok(ok_response(invoice)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// PUT /api/v1/ap/invoices/:id/match
async fn match_invoice(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<MatchInvoiceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ApCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());

    let cmd = MatchInvoiceCmd {
        invoice_id: id,
        reviewed_by: body.reviewed_by,
        accept_mismatch: body.accept_mismatch,
        tolerance_percent: body.tolerance_percent,
    };

    match handler.match_invoice(tenant_id, cmd).await {
        Ok(invoice) => Ok(ok_response(invoice)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// PUT /api/v1/ap/invoices/:id/post
async fn post_invoice(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<PostInvoiceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ApCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());

    let cmd = PostInvoiceCmd {
        invoice_id: id,
        posted_by: body.posted_by,
        expense_account_ids: body.expense_account_ids,
        accounts_payable_account_id: body.accounts_payable_account_id,
        entity_id: body.entity_id,
        accounting_period_id: body.accounting_period_id,
    };

    match handler.post_invoice(tenant_id, cmd).await {
        Ok(invoice) => Ok(ok_response(invoice)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// POST /api/v1/ap/payments
async fn create_vendor_payment(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreatePaymentRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ApCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let created_by = Uuid::nil();

    let payment_date = parse_date(&body.payment_date)
        .ok_or_else(|| err_response(StatusCode::BAD_REQUEST, "Invalid payment_date".into()))?;

    let cheque_date = body.cheque_date.as_deref().and_then(parse_date);

    let allocations: Vec<_> = body.allocations.into_iter().map(|a| {
        sutra_finance_ap::commands::PaymentAllocationCmd {
            invoice_id: a.invoice_id,
            allocated_amount: a.allocated_amount,
        }
    }).collect();

    let cmd = CreateVendorPaymentCmd {
        entity_id: body.entity_id,
        vendor_id: body.vendor_id,
        payment_mode: body.payment_mode,
        payment_date,
        amount: body.amount,
        bank_account_id: body.bank_account_id,
        cheque_number: body.cheque_number,
        cheque_date,
        remarks: body.remarks,
        allocations,
    };

    match handler.create_vendor_payment(tenant_id, created_by, cmd).await {
        Ok(payment) => Ok(ok_response(payment)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// PUT /api/v1/ap/payments/:id/process
async fn process_payment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<ProcessPaymentRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ApCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());

    let cmd = ProcessPaymentCmd {
        payment_id: id,
        processed_by: body.processed_by,
        bank_transaction_ref: body.bank_transaction_ref,
        bank_account_id: body.bank_account_id,
        accounts_payable_account_id: body.accounts_payable_account_id,
        tds_payable_account_id: body.tds_payable_account_id,
        tds_expense_account_id: body.tds_expense_account_id,
        entity_id: body.entity_id,
        accounting_period_id: body.accounting_period_id,
    };

    match handler.process_payment(tenant_id, cmd).await {
        Ok(payment) => Ok(ok_response(payment)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// GET /api/v1/ap/tds/deductions
async fn list_tds_deductions(
    State(state): State<Arc<AppState>>,
    Query(_filter): Query<TdsFilterQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let tid = Uuid::nil();
    let pool = &state.db;

    let rows = sqlx::query_as::<_, (Uuid, Uuid, i64, String, Option<String>)>(
        r#"SELECT vp.payment_id, vp.vendor_id, vp.tds_amount, vp.payment_number, v.default_tds_section
           FROM vendor_payments vp
           LEFT JOIN vendors v ON vp.vendor_id = v.vendor_id AND vp.tenant_id = v.tenant_id
           WHERE vp.tenant_id = $1 AND vp.tds_amount > 0 AND vp.deleted_at IS NULL
           ORDER BY vp.payment_date DESC
           LIMIT 100"#,
    )
    .bind(tid)
    .fetch_all(pool)
    .await
    .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result: Vec<serde_json::Value> = rows.into_iter().map(|(pid, vid, tds, pn, section)| {
        serde_json::json!({
            "payment_id": pid,
            "vendor_id": vid,
            "tds_amount": tds,
            "payment_number": pn,
            "tds_section": section,
            "status": "DEDUCTED",
        })
    }).collect();

    Ok(ok_response(serde_json::json!({
        "deductions": result,
        "total": result.len(),
    })))
}
