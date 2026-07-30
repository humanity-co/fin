//! Accounts Receivable API routes — v1.
//!
//! ## Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | POST   | /api/v1/ar/students/:id/assess-fees | Assess student fees |
//! | POST   | /api/v1/ar/payments | Record fee payment |
//! | GET    | /api/v1/ar/students/:id/fees | Get student fee account |
//! | GET    | /api/v1/ar/payments/receipts | List receipts |
//! | GET    | /api/v1/ar/payments/receipts/:id | Get receipt |
//! | POST   | /api/v1/ar/concessions | Grant concession |
//! | POST   | /api/v1/ar/scholarships | Apply scholarship |
//! | PUT    | /api/v1/ar/scholarships/:id/verify | Verify scholarship |
//! | PUT    | /api/v1/ar/scholarships/:id/disburse | Record DBT disbursement |
//! | GET    | /api/v1/ar/scholarships/pending-verification | Pending verification list |
//! | POST   | /api/v1/ar/refunds | Initiate refund |
//! | PUT    | /api/v1/ar/refunds/:id/process | Process refund |

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use sutra_core::{Money, TenantId};
use sutra_finance_ar::{
    ApplyScholarshipCmd, ArCommandHandler, AssessStudentFeesCmd, GrantConcessionCmd,
    InitiateRefundCmd, ProcessRefundCmd, RecordFeePaymentCmd,
    RecordScholarshipDisbursementCmd, VerifyScholarshipCmd,
};

use crate::state::AppState;

/// Create the AR routes sub-router.
pub fn ar_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/students/{id}/assess-fees", post(assess_student_fees))
        .route("/payments", post(record_fee_payment))
        .route("/students/{id}/fees", get(get_student_fees))
        .route("/payments/receipts", get(list_receipts))
        .route("/payments/receipts/{id}", get(get_receipt))
        .route("/concessions", post(grant_concession))
        .route("/scholarships", post(apply_scholarship))
        .route(
            "/scholarships/pending-verification",
            get(list_pending_verification),
        )
        .route("/scholarships/{id}/verify", put(verify_scholarship))
        .route("/scholarships/{id}/disburse", put(disburse_scholarship))
        .route("/refunds", post(initiate_refund))
        .route("/refunds/{id}/process", put(process_refund))
}

// ─── Request Types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AssessFeesRequest {
    fee_structure_id: Uuid,
    #[serde(default)]
    installment_plan_id: Option<Uuid>,
    academic_year: String,
    #[serde(default)]
    scholarship_expected: Option<f64>,
    #[serde(default)]
    concession_amount: Option<f64>,
    entity_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct RecordPaymentRequest {
    student_id: Uuid,
    student_fee_account_id: Uuid,
    amount: f64,
    payment_mode: String,
    #[serde(default)]
    gateway_transaction_id: Option<String>,
    #[serde(default)]
    bank_transaction_ref: Option<String>,
    #[serde(default)]
    cheque_number: Option<String>,
    #[serde(default)]
    cheque_date: Option<String>,
    entity_id: Uuid,
    bank_account_id: Uuid,
    fee_income_account_ids: std::collections::HashMap<String, Uuid>,
    accounting_period_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct GrantConcessionRequest {
    student_id: Uuid,
    student_fee_account_id: Uuid,
    #[serde(default)]
    fee_head_id: Option<Uuid>,
    concession_type: String,
    value: f64,
    reason: String,
    approved_by: Uuid,
}

#[derive(Debug, Deserialize)]
struct ApplyScholarshipRequest {
    student_id: Uuid,
    scheme_id: Uuid,
    #[serde(default)]
    student_fee_account_id: Option<Uuid>,
    academic_year: String,
    expected_amount: f64,
    #[serde(default)]
    maha_dbt_application_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VerifyScholarshipRequest {
    verified_by: Uuid,
}

#[derive(Debug, Deserialize)]
struct DisburseScholarshipRequest {
    disbursed_amount: f64,
    dbt_transaction_id: String,
    #[serde(default)]
    bank_account_id: Option<Uuid>,
    #[serde(default)]
    fee_income_account_id: Option<Uuid>,
    #[serde(default)]
    accounting_period_id: Option<Uuid>,
    #[serde(default)]
    entity_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct InitiateRefundRequest {
    student_id: Uuid,
    amount: f64,
    refund_reason: String,
    refund_mode: String,
    #[serde(default)]
    linked_payment_id: Option<Uuid>,
    #[serde(default)]
    withdrawal_date: Option<String>,
    #[serde(default)]
    course_start_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProcessRefundRequest {
    processed_by: Uuid,
    #[serde(default)]
    bank_transaction_ref: Option<String>,
    bank_account_id: Uuid,
    fee_income_account_id: Uuid,
    accounting_period_id: Uuid,
    entity_id: Uuid,
}

// ─── Error Helper ──────────────────────────────────────────────────────

fn err_response(
    status: StatusCode,
    msg: String,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({
            "success": false,
            "error": msg,
            "data": null::<()>
        })),
    )
}

fn ok_response(data: impl serde::Serialize) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "success": true,
        "data": data,
        "error": null::<()>
    }))
}

// ─── Handlers ──────────────────────────────────────────────────────────

/// POST /api/v1/ar/students/:id/assess-fees
async fn assess_student_fees(
    State(state): State<Arc<AppState>>,
    Path(student_id): Path<Uuid>,
    Json(body): Json<AssessFeesRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ArCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let created_by = Uuid::nil();

    let cmd = AssessStudentFeesCmd {
        student_id,
        fee_structure_id: body.fee_structure_id,
        installment_plan_id: body.installment_plan_id,
        academic_year: body.academic_year,
        scholarship_expected: body.scholarship_expected.map(Money::from_rupees),
        concession_amount: body.concession_amount.map(Money::from_rupees),
        entity_id: body.entity_id,
    };

    match handler.assess_student_fees(tenant_id, created_by, cmd).await {
        Ok(account) => Ok(ok_response(account)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// POST /api/v1/ar/payments
async fn record_fee_payment(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RecordPaymentRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ArCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let received_by = Uuid::nil();

    let cheque_date = body
        .cheque_date
        .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

    let cmd = RecordFeePaymentCmd {
        student_id: body.student_id,
        student_fee_account_id: body.student_fee_account_id,
        amount: Money::from_rupees(body.amount),
        payment_mode: body.payment_mode,
        payment_date: chrono::Utc::now(),
        gateway_transaction_id: body.gateway_transaction_id,
        bank_transaction_ref: body.bank_transaction_ref,
        cheque_number: body.cheque_number,
        cheque_date,
        received_by,
        entity_id: body.entity_id,
        bank_account_id: body.bank_account_id,
        fee_income_account_ids: body.fee_income_account_ids,
        accounting_period_id: body.accounting_period_id,
    };

    match handler.record_fee_payment(tenant_id, cmd).await {
        Ok(receipt) => Ok(ok_response(receipt)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// GET /api/v1/ar/students/:id/fees
async fn get_student_fees(
    State(state): State<Arc<AppState>>,
    Path(student_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let tid = Uuid::nil();
    let pool = &state.db;

    let accounts = sqlx::query_as::<_, (Uuid, String, i64, i64, i64, i64, i64, String)>(
        r#"SELECT student_fee_account_id, academic_year, gross_fee, scholarship_expected,
           concession_amount, net_payable, total_paid, status
           FROM ar_student_fee_accounts
           WHERE student_id = $1 AND tenant_id = $2 AND deleted_at IS NULL
           ORDER BY created_at DESC"#,
    )
    .bind(student_id)
    .bind(tid)
    .fetch_all(pool)
    .await
    .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result: Vec<serde_json::Value> = accounts
        .into_iter()
        .map(|(id, ay, gf, se, ca, np, tp, st)| {
            serde_json::json!({
                "student_fee_account_id": id,
                "academic_year": ay,
                "gross_fee": gf,
                "scholarship_expected": se,
                "concession_amount": ca,
                "net_payable": np,
                "total_paid": tp,
                "outstanding": np - tp,
                "status": st,
            })
        })
        .collect();

    Ok(ok_response(result))
}

/// GET /api/v1/ar/payments/receipts
async fn list_receipts(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let tid = Uuid::nil();
    let pool = &state.db;

    let receipts = sqlx::query_as::<_, (Uuid, String, Uuid, i64, String, String)>(
        r#"SELECT payment_receipt_id, receipt_number, student_id, amount, payment_mode, status
           FROM ar_payment_receipts
           WHERE tenant_id = $1 AND deleted_at IS NULL
           ORDER BY created_at DESC
           LIMIT 100"#,
    )
    .bind(tid)
    .fetch_all(pool)
    .await
    .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result: Vec<serde_json::Value> = receipts
        .into_iter()
        .map(|(id, rn, sid, amt, pm, st)| {
            serde_json::json!({
                "payment_receipt_id": id,
                "receipt_number": rn,
                "student_id": sid,
                "amount": amt,
                "payment_mode": pm,
                "status": st,
            })
        })
        .collect();

    Ok(ok_response(result))
}

/// GET /api/v1/ar/payments/receipts/:id
async fn get_receipt(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let tid = Uuid::nil();
    let pool = &state.db;

    let receipt = sqlx::query_as::<_, (Uuid, String, Uuid, i64, String, String, Option<Uuid>)>(
        r#"SELECT payment_receipt_id, receipt_number, student_id, amount, payment_mode,
           status, payment_journal_id
           FROM ar_payment_receipts
           WHERE payment_receipt_id = $1 AND tenant_id = $2 AND deleted_at IS NULL"#,
    )
    .bind(id)
    .bind(tid)
    .fetch_optional(pool)
    .await
    .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| err_response(StatusCode::NOT_FOUND, "Receipt not found".into()))?;

    Ok(ok_response(serde_json::json!({
        "payment_receipt_id": receipt.0,
        "receipt_number": receipt.1,
        "student_id": receipt.2,
        "amount": receipt.3,
        "payment_mode": receipt.4,
        "status": receipt.5,
        "payment_journal_id": receipt.6,
    })))
}

/// POST /api/v1/ar/concessions
async fn grant_concession(
    State(state): State<Arc<AppState>>,
    Json(body): Json<GrantConcessionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ArCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());

    let value = rust_decimal::Decimal::from_f64(body.value)
        .ok_or_else(|| err_response(StatusCode::BAD_REQUEST, "Invalid value".into()))?;

    let cmd = GrantConcessionCmd {
        student_id: body.student_id,
        student_fee_account_id: body.student_fee_account_id,
        fee_head_id: body.fee_head_id,
        concession_type: body.concession_type,
        value,
        reason: body.reason,
        approved_by: body.approved_by,
    };

    match handler.grant_concession(tenant_id, cmd).await {
        Ok(concession) => Ok(ok_response(concession)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// POST /api/v1/ar/scholarships
async fn apply_scholarship(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ApplyScholarshipRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ArCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let created_by = Uuid::nil();

    let cmd = ApplyScholarshipCmd {
        student_id: body.student_id,
        scheme_id: body.scheme_id,
        student_fee_account_id: body.student_fee_account_id,
        academic_year: body.academic_year,
        expected_amount: Money::from_rupees(body.expected_amount),
        maha_dbt_application_id: body.maha_dbt_application_id,
    };

    match handler.apply_scholarship(tenant_id, created_by, cmd).await {
        Ok(scholarship) => Ok(ok_response(scholarship)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// PUT /api/v1/ar/scholarships/:id/verify
async fn verify_scholarship(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<VerifyScholarshipRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ArCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());

    let cmd = VerifyScholarshipCmd {
        scholarship_id: id,
        verified_by: body.verified_by,
    };

    match handler.verify_scholarship(tenant_id, cmd).await {
        Ok(scholarship) => Ok(ok_response(scholarship)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// PUT /api/v1/ar/scholarships/:id/disburse
async fn disburse_scholarship(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<DisburseScholarshipRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ArCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let created_by = Uuid::nil();

    let cmd = RecordScholarshipDisbursementCmd {
        scholarship_id: id,
        disbursed_amount: Money::from_rupees(body.disbursed_amount),
        dbt_transaction_id: body.dbt_transaction_id,
        dbt_date: chrono::Utc::now(),
        bank_account_id: body.bank_account_id,
        fee_income_account_id: body.fee_income_account_id,
        accounting_period_id: body.accounting_period_id,
        entity_id: body.entity_id,
    };

    match handler.record_scholarship_disbursement(tenant_id, created_by, cmd).await {
        Ok(scholarship) => Ok(ok_response(scholarship)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// GET /api/v1/ar/scholarships/pending-verification
async fn list_pending_verification(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let tid = Uuid::nil();
    let pool = &state.db;

    let scholarships = sqlx::query_as::<_, (Uuid, Uuid, Uuid, i64, String)>(
        r#"SELECT scholarship_id, student_id, scheme_id, expected_amount, status
           FROM ar_student_scholarships
           WHERE tenant_id = $1 AND status = 'APPLIED' AND deleted_at IS NULL
           ORDER BY created_at ASC
           LIMIT 100"#,
    )
    .bind(tid)
    .fetch_all(pool)
    .await
    .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result: Vec<serde_json::Value> = scholarships
        .into_iter()
        .map(|(sid, stid, scid, ea, s)| {
            serde_json::json!({
                "scholarship_id": sid,
                "student_id": stid,
                "scheme_id": scid,
                "expected_amount": ea,
                "status": s,
            })
        })
        .collect();

    Ok(ok_response(result))
}

/// POST /api/v1/ar/refunds
async fn initiate_refund(
    State(state): State<Arc<AppState>>,
    Json(body): Json<InitiateRefundRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ArCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let created_by = Uuid::nil();

    let withdrawal_date = body
        .withdrawal_date
        .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
    let course_start_date = body
        .course_start_date
        .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

    let cmd = InitiateRefundCmd {
        student_id: body.student_id,
        amount: Money::from_rupees(body.amount),
        refund_reason: body.refund_reason,
        refund_mode: body.refund_mode,
        linked_payment_id: body.linked_payment_id,
        withdrawal_date,
        course_start_date,
    };

    match handler.initiate_refund(tenant_id, created_by, cmd).await {
        Ok(refund) => Ok(ok_response(refund)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// PUT /api/v1/ar/refunds/:id/process
async fn process_refund(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<ProcessRefundRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = ArCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil());

    let cmd = ProcessRefundCmd {
        refund_id: id,
        processed_by: body.processed_by,
        bank_transaction_ref: body.bank_transaction_ref,
        bank_account_id: body.bank_account_id,
        fee_income_account_id: body.fee_income_account_id,
        accounting_period_id: body.accounting_period_id,
        entity_id: body.entity_id,
    };

    match handler.process_refund(tenant_id, cmd).await {
        Ok(refund) => Ok(ok_response(refund)),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}
