//! General Ledger API routes — v1.
//!
//! ## Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | POST   | /api/v1/gl/journals | Create a draft journal entry |
//! | POST   | /api/v1/gl/journals/:id/post | Post a draft journal |
//! | POST   | /api/v1/gl/journals/:id/reverse | Reverse a posted journal |
//! | GET    | /api/v1/gl/journals | List journals (paginated) |
//! | GET    | /api/v1/gl/journals/:id | Get journal by ID |
//! | GET    | /api/v1/gl/trial-balance | Get trial balance |
//! | GET    | /api/v1/gl/accounts | Get Chart of Accounts (tree) |
//! | GET    | /api/v1/gl/accounts/:id/ledger | Get account ledger |

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use sutra_core::TenantId;
use sutra_finance_gl::{
    CreateJournalCmd, CreateJournalLineCmd, GlCommandHandler, GlQueryHandler,
    JournalFilter, PostJournalCmd, ReverseJournalCmd, TrialBalanceQuery,
};

use crate::state::AppState;

/// Create the GL routes sub-router.
pub fn gl_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/journals", post(create_journal).get(list_journals))
        .route("/journals/{id}", get(get_journal_by_id))
        .route("/journals/{id}/post", post(post_journal))
        .route("/journals/{id}/reverse", post(reverse_journal))
        .route("/trial-balance", get(get_trial_balance))
        .route("/accounts", get(get_chart_of_accounts))
        .route("/accounts/{id}/ledger", get(get_account_ledger))
}

// ─── Request/Response Types ──────────────────────────────────────────

/// Request body for creating a journal.
#[derive(Debug, Deserialize)]
struct CreateJournalRequest {
    journal_type: String,
    accounting_period_id: Uuid,
    entity_id: Uuid,
    #[serde(default)]
    fund_id: Option<Uuid>,
    #[serde(default)]
    cost_center_id: Option<Uuid>,
    posting_date: String,
    description: String,
    lines: Vec<CreateJournalLineRequest>,
    #[serde(default)]
    attachment_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
struct CreateJournalLineRequest {
    line_number: i32,
    account_id: Uuid,
    #[serde(default)]
    debit_amount: Option<f64>,
    #[serde(default)]
    credit_amount: Option<f64>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    cost_center_id: Option<Uuid>,
    #[serde(default)]
    fund_id: Option<Uuid>,
    #[serde(default)]
    reference_id: Option<String>,
    #[serde(default)]
    reference_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PostJournalRequest {
    posted_by: Uuid,
}

#[derive(Debug, Deserialize)]
struct ReverseJournalRequest {
    reason: String,
    reversed_by: Uuid,
}

#[derive(Debug, Deserialize)]
struct JournalListQuery {
    #[serde(default)]
    entity_id: Option<Uuid>,
    #[serde(default)]
    accounting_period_id: Option<Uuid>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    journal_type: Option<String>,
    #[serde(default)]
    from_date: Option<String>,
    #[serde(default)]
    to_date: Option<String>,
    #[serde(default)]
    page: Option<u32>,
    #[serde(default)]
    per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TrialBalanceRequest {
    period_id: Uuid,
    #[serde(default)]
    entity_id: Option<Uuid>,
    #[serde(default)]
    cost_center_id: Option<Uuid>,
    #[serde(default)]
    fund_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct LedgerQuery {
    #[serde(default)]
    from_date: Option<String>,
    #[serde(default)]
    to_date: Option<String>,
}

// ─── Generic API Response ────────────────────────────────────────────

/// Standard API response wrapper.
#[derive(Debug, serde::Serialize)]
struct ApiResponse<T: serde::Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: serde::Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

fn err_response(status: StatusCode, msg: String) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({
            "success": false,
            "error": msg,
            "data": null
        })),
    )
}

// ─── Handlers ────────────────────────────────────────────────────────

/// POST /api/v1/gl/journals
async fn create_journal(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateJournalRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = GlCommandHandler::new(state.db.clone());

    // Parse posting date
    let posting_date = chrono::NaiveDate::parse_from_str(&body.posting_date, "%Y-%m-%d")
        .map_err(|e| err_response(StatusCode::BAD_REQUEST, format!("Invalid posting_date: {e}")))?;

    // Convert lines
    let lines: Vec<CreateJournalLineCmd> = body
        .lines
        .into_iter()
        .map(|l| CreateJournalLineCmd {
            line_number: l.line_number,
            account_id: l.account_id,
            debit_amount: l.debit_amount.map(|d| sutra_core::Money::from_rupees(d)),
            credit_amount: l.credit_amount.map(|d| sutra_core::Money::from_rupees(d)),
            description: l.description,
            cost_center_id: l.cost_center_id,
            fund_id: l.fund_id,
            reference_id: l.reference_id,
            reference_type: l.reference_type,
        })
        .collect();

    let cmd = CreateJournalCmd {
        journal_type: body.journal_type,
        accounting_period_id: body.accounting_period_id,
        entity_id: body.entity_id,
        fund_id: body.fund_id,
        cost_center_id: body.cost_center_id,
        posting_date,
        description: body.description,
        lines,
        attachment_ids: body.attachment_ids,
    };

    // TODO: Extract real tenant_id from auth context
    let tenant_id = TenantId::from_uuid(Uuid::nil());
    let created_by = Uuid::nil(); // TODO: From auth context

    match handler.create_journal(tenant_id, created_by, cmd).await {
        Ok(journal) => Ok(Json(serde_json::to_value(journal).unwrap())),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// POST /api/v1/gl/journals/:id/post
async fn post_journal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<PostJournalRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = GlCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil()); // TODO: From auth

    let cmd = PostJournalCmd {
        journal_id: id,
        posted_by: body.posted_by,
    };

    match handler.post_journal(tenant_id, cmd).await {
        Ok(journal) => Ok(Json(serde_json::to_value(journal).unwrap())),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// POST /api/v1/gl/journals/:id/reverse
async fn reverse_journal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReverseJournalRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = GlCommandHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil()); // TODO: From auth

    let cmd = ReverseJournalCmd {
        journal_id: id,
        reason: body.reason,
        reversed_by: body.reversed_by,
    };

    match handler.reverse_journal(tenant_id, cmd).await {
        Ok((original, reversal)) => Ok(Json(serde_json::json!({
            "original": original,
            "reversal": reversal,
        }))),
        Err(e) => Err(err_response(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// GET /api/v1/gl/journals
async fn list_journals(
    State(state): State<Arc<AppState>>,
    Query(query): Query<JournalListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = GlQueryHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil()); // TODO: From auth

    let filter = JournalFilter {
        entity_id: query.entity_id,
        accounting_period_id: query.accounting_period_id,
        status: query.status,
        journal_type: query.journal_type,
        from_date: query.from_date,
        to_date: query.to_date,
        page: query.page,
        per_page: query.per_page,
    };

    match handler.list_journals(tenant_id, filter).await {
        Ok(response) => Ok(Json(serde_json::to_value(response).unwrap())),
        Err(e) => Err(err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )),
    }
}

/// GET /api/v1/gl/journals/:id
async fn get_journal_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = GlQueryHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil()); // TODO: From auth

    match handler.get_journal_by_id(tenant_id, id).await {
        Ok(Some(journal)) => Ok(Json(serde_json::to_value(journal).unwrap())),
        Ok(None) => Err(err_response(StatusCode::NOT_FOUND, "Journal not found".into())),
        Err(e) => Err(err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )),
    }
}

/// GET /api/v1/gl/trial-balance
async fn get_trial_balance(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TrialBalanceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = GlQueryHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil()); // TODO: From auth

    let tb_query = TrialBalanceQuery {
        period_id: query.period_id,
        entity_id: query.entity_id,
        cost_center_id: query.cost_center_id,
        fund_id: query.fund_id,
    };

    match handler.get_trial_balance(tenant_id, tb_query).await {
        Ok(rows) => Ok(Json(serde_json::to_value(rows).unwrap())),
        Err(e) => Err(err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )),
    }
}

/// GET /api/v1/gl/accounts
async fn get_chart_of_accounts(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = GlQueryHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil()); // TODO: From auth

    match handler.get_chart_of_accounts(tenant_id).await {
        Ok(tree) => Ok(Json(serde_json::to_value(tree).unwrap())),
        Err(e) => Err(err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )),
    }
}

/// GET /api/v1/gl/accounts/:id/ledger
async fn get_account_ledger(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<LedgerQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let handler = GlQueryHandler::new(state.db.clone());
    let tenant_id = TenantId::from_uuid(Uuid::nil()); // TODO: From auth

    let from_date = query
        .from_date
        .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
    let to_date = query
        .to_date
        .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

    match handler
        .get_account_ledger(tenant_id, id, from_date, to_date)
        .await
    {
        Ok(entries) => Ok(Json(serde_json::to_value(entries).unwrap())),
        Err(e) => Err(err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )),
    }
}
