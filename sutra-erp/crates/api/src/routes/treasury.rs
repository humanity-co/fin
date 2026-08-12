//! Treasury & Banking API routes — v1.
//!
//! All endpoints live under `/api/v1/treasury`. Permission codes follow the
//! spec's API table; new permissions (`treasury:bank_statement:upload`,
//! `treasury:bank_statement:view`, `treasury:gateway:configure`,
//! `treasury:petty_cash:create`) are seeded in the RBAC seed function.
//!
//! Money is transported as integer paise everywhere — never floats.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use sutra_core::TenantId;
use sutra_finance_treasury::{
    AddSignatoryCmd, ConfigureGatewayCmd, CreateBankAccountCmd,
    InitiateInterBankTransferCmd, ManualMatchCmd, RecordPettyCashExpenseCmd,
    ReconcileGatewaySettlementCmd, StartReconciliationCmd, SyncBankBalanceCmd,
    TopUpPettyCashCmd, TreasuryCommandHandler, TreasuryQueryHandler,
    UpdateBankAccountCmd, UploadBankStatementCmd,
};
use sutra_rbac::UserContext;

use crate::state::AppState;

/// Build the treasury routes sub-router.
pub fn treasury_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Bank accounts
        .route("/bank-accounts", post(create_bank_account).get(list_bank_accounts))
        .route("/bank-accounts/{id}", get(get_bank_account).put(update_bank_account))
        .route("/bank-accounts/{id}/deactivate", post(deactivate_bank_account))
        .route("/bank-accounts/{id}/signatories", post(add_signatory).get(list_signatories))
        .route("/bank-accounts/{id}/signatories/{userId}", delete(delete_signatory))
        .route("/bank-accounts/{id}/sync-balance", post(sync_bank_balance))
        // Reconciliations
        .route("/reconciliations", post(start_reconciliation).get(list_reconciliations))
        .route("/reconciliations/{id}", get(get_reconciliation))
        .route("/reconciliations/{id}/statement", post(upload_bank_statement))
        .route("/reconciliations/{id}/auto-match", post(auto_reconcile))
        .route("/reconciliations/{id}/manual-match", post(manual_match))
        .route("/reconciliations/{id}/unmatch", post(unmatch_line))
        .route("/reconciliations/{id}/complete", post(complete_reconciliation))
        .route("/reconciliations/{id}/verify", post(verify_reconciliation))
        .route("/reconciliations/{id}/brs", get(generate_brs))
        // Transfers
        .route("/transfers", post(create_transfer).get(list_transfers))
        .route("/transfers/{id}", get(get_transfer))
        .route("/transfers/{id}/approve", post(approve_transfer))
        .route("/transfers/{id}/process", post(process_transfer))
        .route("/transfers/{id}/complete", post(complete_transfer))
        .route("/transfers/{id}/cancel", post(cancel_transfer))
        // Petty cash
        .route("/petty-cash", post(create_petty_cash_fund).get(list_petty_cash_funds))
        .route("/petty-cash/{id}", get(get_petty_cash_fund))
        .route("/petty-cash/{id}/top-up", post(top_up_petty_cash))
        .route("/petty-cash/{id}/expense", post(record_petty_cash_expense))
        .route("/petty-cash/{id}/register", get(get_petty_cash_register))
        .route("/petty-cash/{id}/close", post(close_petty_cash_fund))
        // Gateways
        .route("/gateways", post(configure_gateway).get(list_gateways))
        .route("/gateways/settlements/reconcile", post(reconcile_gateway_settlement))
        .route("/gateways/settlements", get(list_gateway_settlements))
        // Reports
        .route("/reports/bank-book", get(get_bank_book))
        .route("/reports/cash-position", get(get_cash_position))
        .route("/reports/uncleared-cheques", get(get_uncleared_cheques))
}

// ─── Request types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateBankAccountRequest {
    #[serde(default)]
    entity_id: Option<Uuid>,
    account_number: String,
    account_name: String,
    bank_name: String,
    #[serde(default)]
    branch_name: Option<String>,
    ifsc_code: String,
    account_type: String,
    #[serde(default)]
    fund_id: Option<Uuid>,
    #[serde(default)]
    minimum_balance_paise: i64,
}

#[derive(Debug, Deserialize)]
struct UpdateBankAccountRequest {
    #[serde(default)]
    account_name: Option<String>,
    #[serde(default)]
    branch_name: Option<String>,
    #[serde(default)]
    minimum_balance_paise: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AddSignatoryRequest {
    user_id: Uuid,
    signatory_type: String,
}

#[derive(Debug, Deserialize)]
struct SyncBalanceRequest {
    balance_paise: i64,
}

#[derive(Debug, Deserialize)]
struct StartReconciliationRequest {
    bank_account_id: Uuid,
    period_id: Uuid,
    statement_date: String,
    opening_balance_paise: i64,
    closing_balance_paise: i64,
}

#[derive(Debug, Deserialize)]
struct UploadStatementRequest {
    format: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ManualMatchRequest {
    statement_line_id: Uuid,
    transaction_id: Uuid,
    transaction_type: String,
}

#[derive(Debug, Deserialize)]
struct UnmatchRequest {
    statement_line_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct CompleteReconciliationRequest {
    #[serde(default)]
    certify_open_items: bool,
}

#[derive(Debug, Deserialize)]
struct CreateTransferRequest {
    from_bank_account_id: Uuid,
    to_bank_account_id: Uuid,
    amount_paise: i64,
    transfer_date: String,
}

#[derive(Debug, Deserialize)]
struct CompleteTransferRequest {
    bank_reference: String,
}

#[derive(Debug, Deserialize)]
struct CancelTransferRequest {
    reason: String,
}

#[derive(Debug, Deserialize)]
struct CreatePettyCashFundRequest {
    entity_id: Uuid,
    fund_name: String,
    imprest_amount_paise: i64,
    custodian_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct TopUpRequest {
    amount_paise: i64,
    bank_account_id: Uuid,
    #[serde(default)]
    accounting_period_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct ExpenseRequest {
    amount_paise: i64,
    expense_account_id: Uuid,
    voucher_no: String,
    #[serde(default)]
    narration: Option<String>,
    #[serde(default)]
    accounting_period_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct ConfigureGatewayRequest {
    entity_id: Uuid,
    gateway_type: String,
    merchant_id: String,
    api_key: String,
    api_secret: String,
    #[serde(default)]
    webhook_secret: Option<String>,
    #[serde(default)]
    is_active: bool,
}

#[derive(Debug, Deserialize)]
struct ReconcileSettlementRequest {
    entity_id: Uuid,
    gateway_type: String,
    settlement_date: String,
    settled_amount_paise: i64,
    gateway_fee_amount_paise: i64,
}

#[derive(Debug, Deserialize)]
struct BankAccountsQuery {
    #[serde(default)]
    entity_id: Option<Uuid>,
    #[serde(default)]
    account_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReconciliationsQuery {
    #[serde(default)]
    bank_account_id: Option<Uuid>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DateRangeQuery {
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TransfersQuery {
    #[serde(default)]
    from_account: Option<Uuid>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PettyCashQuery {
    #[serde(default)]
    entity_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct GatewayQuery {
    #[serde(default)]
    entity_id: Option<Uuid>,
    #[serde(default)]
    status: Option<String>,
}

// ─── Helpers ───────────────────────────────────────────────────────────

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

fn parse_date(s: &str, field: &str) -> Result<chrono::NaiveDate, (StatusCode, Json<serde_json::Value>)> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| err_response(StatusCode::BAD_REQUEST, format!("Invalid {field}: {e}")))
}

fn to_uuid(id: &str, field: &str) -> Result<Uuid, (StatusCode, Json<serde_json::Value>)> {
    Uuid::parse_str(id).map_err(|e| err_response(StatusCode::BAD_REQUEST, format!("Invalid {field}: {e}")))
}

fn parse_opt_date(
    s: Option<String>,
) -> Result<Option<chrono::NaiveDate>, (StatusCode, Json<serde_json::Value>)> {
    match s {
        None => Ok(None),
        Some(v) => parse_date(&v, "date").map(Some),
    }
}

async fn err_from(e: sutra_finance_treasury::TreasuryError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match e {
        sutra_finance_treasury::TreasuryError::BankAccountNotFound(_)
        | sutra_finance_treasury::TreasuryError::ReconciliationNotFound(_)
        | sutra_finance_treasury::TreasuryError::TransferNotFound(_)
        | sutra_finance_treasury::TreasuryError::PettyCashFundNotFound(_)
        | sutra_finance_treasury::TreasuryError::StatementLineNotFound(_)
        | sutra_finance_treasury::TreasuryError::GatewayConfigNotFound(_)
        | sutra_finance_treasury::TreasuryError::GatewaySettlementNotFound(_, _, _)
        | sutra_finance_treasury::TreasuryError::GlAccountNotFound(_)
        | sutra_finance_treasury::TreasuryError::MatchTransactionNotFound(_) => StatusCode::NOT_FOUND,
        sutra_finance_treasury::TreasuryError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    };
    err_response(status, e.to_string())
}

/// Resolve the tenant from the authenticated context. Mirrors the existing
/// gl/ap handlers which currently use the nil tenant until per-tenant JWT
/// wiring lands.
fn tenant_of(user: &UserContext) -> TenantId {
    TenantId::from_uuid(user.tenant_id)
}

// ─── Bank accounts ─────────────────────────────────────────────────────

async fn create_bank_account(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBankAccountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:configure")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let cmd = CreateBankAccountCmd {
        entity_id: body.entity_id,
        account_number: body.account_number,
        account_name: body.account_name,
        bank_name: body.bank_name,
        branch_name: body.branch_name,
        ifsc_code: body.ifsc_code,
        account_type: body.account_type,
        fund_id: body.fund_id,
        minimum_balance: body.minimum_balance_paise.max(0),
    };
    match handler
        .create_bank_account(tenant_of(&user), user.user_id, cmd)
        .await
    {
        Ok(account) => Ok(Json(serde_json::json!({ "bank_account": account }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn list_bank_accounts(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<BankAccountsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:view")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    match handler
        .get_bank_accounts(
            user.tenant_id,
            query.entity_id,
            query.account_type.as_deref(),
        )
        .await
    {
        Ok(accounts) => Ok(Json(serde_json::json!({ "bank_accounts": accounts }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn get_bank_account(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:view")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    let id = to_uuid(&id, "bank_account_id")?;
    match handler.get_bank_account(user.tenant_id, id).await {
        Ok(detail) => Ok(Json(detail)),
        Err(e) => Err(err_from(e).await),
    }
}

async fn update_bank_account(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateBankAccountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:configure")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "bank_account_id")?;
    let cmd = UpdateBankAccountCmd {
        account_name: body.account_name,
        branch_name: body.branch_name,
        minimum_balance: body.minimum_balance_paise,
    };
    match handler
        .update_bank_account(tenant_of(&user), id, user.user_id, cmd)
        .await
    {
        Ok(account) => Ok(Json(serde_json::json!({ "bank_account": account }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn deactivate_bank_account(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:configure")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "bank_account_id")?;
    match handler
        .deactivate_bank_account(tenant_of(&user), id, user.user_id)
        .await
    {
        Ok(account) => Ok(Json(serde_json::json!({ "bank_account": account }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn add_signatory(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<AddSignatoryRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:configure")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "bank_account_id")?;
    let cmd = AddSignatoryCmd {
        user_id: body.user_id,
        signatory_type: body.signatory_type,
    };
    match handler
        .add_signatory(tenant_of(&user), id, user.user_id, cmd)
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({ "success": true }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn delete_signatory(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path((id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:configure")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "bank_account_id")?;
    let uid = to_uuid(&user_id, "user_id")?;
    match handler.remove_signatory(tenant_of(&user), id, uid).await {
        Ok(()) => Ok(Json(serde_json::json!({ "success": true }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn list_signatories(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:view")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    let id = to_uuid(&id, "bank_account_id")?;
    match handler.list_signatories(user.tenant_id, id).await {
        Ok(signatories) => Ok(Json(serde_json::json!({ "signatories": signatories }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn sync_bank_balance(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<SyncBalanceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:configure")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "bank_account_id")?;
    let cmd = SyncBankBalanceCmd {
        balance: body.balance_paise,
    };
    match handler
        .sync_bank_balance(tenant_of(&user), id, user.user_id, cmd)
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({ "success": true }))),
        Err(e) => Err(err_from(e).await),
    }
}

// ─── Reconciliations ───────────────────────────────────────────────────

async fn start_reconciliation(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(body): Json<StartReconciliationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:reconciliation:perform")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let statement_date = parse_date(&body.statement_date, "statement_date")?;
    let cmd = StartReconciliationCmd {
        bank_account_id: body.bank_account_id,
        period_id: body.period_id,
        statement_date,
        opening_balance: body.opening_balance_paise,
        closing_balance: body.closing_balance_paise,
    };
    match handler
        .start_reconciliation(tenant_of(&user), user.user_id, cmd)
        .await
    {
        Ok(rec) => Ok(Json(serde_json::json!({ "reconciliation": rec }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn list_reconciliations(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ReconciliationsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:reconciliation:perform")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    match handler
        .get_reconciliations(user.tenant_id, query.bank_account_id, query.status.as_deref())
        .await
    {
        Ok(recs) => Ok(Json(serde_json::json!({ "reconciliations": recs }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn get_reconciliation(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:reconciliation:perform")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    let id = to_uuid(&id, "reconciliation_id")?;
    match handler.get_reconciliation(user.tenant_id, id).await {
        Ok(detail) => Ok(Json(detail)),
        Err(e) => Err(err_from(e).await),
    }
}

async fn upload_bank_statement(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<UploadStatementRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_statement:upload")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "reconciliation_id")?;
    let cmd = UploadBankStatementCmd {
        format: body.format,
        content: body.content,
    };
    match handler
        .upload_bank_statement(tenant_of(&user), id, user.user_id, cmd)
        .await
    {
        Ok(line_count) => Ok(Json(serde_json::json!({ "line_count": line_count }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn auto_reconcile(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:reconciliation:perform")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "reconciliation_id")?;
    match handler
        .auto_reconcile(tenant_of(&user), id, user.user_id)
        .await
    {
        Ok((matched, unmatched)) => Ok(Json(serde_json::json!({
            "matched": matched,
            "unmatched": unmatched,
        }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn manual_match(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ManualMatchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:reconciliation:perform")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "reconciliation_id")?;
    let cmd = ManualMatchCmd {
        statement_line_id: body.statement_line_id,
        transaction_id: body.transaction_id,
        transaction_type: body.transaction_type,
    };
    match handler
        .manual_match(tenant_of(&user), id, user.user_id, cmd)
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({ "success": true }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn unmatch_line(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<UnmatchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:reconciliation:perform")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "reconciliation_id")?;
    match handler
        .unmatch(tenant_of(&user), id, body.statement_line_id, user.user_id)
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({ "success": true }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn complete_reconciliation(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<CompleteReconciliationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:reconciliation:perform")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "reconciliation_id")?;
    let cmd = sutra_finance_treasury::CompleteReconciliationCmd {
        certify_open_items: body.certify_open_items,
    };
    match handler
        .complete_reconciliation(tenant_of(&user), id, user.user_id, cmd)
        .await
    {
        Ok(rec) => Ok(Json(serde_json::json!({ "reconciliation": rec }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn verify_reconciliation(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:reconciliation:approve")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "reconciliation_id")?;
    match handler
        .verify_reconciliation(tenant_of(&user), id, user.user_id)
        .await
    {
        Ok(rec) => Ok(Json(serde_json::json!({ "reconciliation": rec }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn generate_brs(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:reconciliation:approve")
        .await?;
    state.permission_engine.require(&user, "reports:export").await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "reconciliation_id")?;
    match handler
        .generate_brs(tenant_of(&user), id, user.user_id)
        .await
    {
        Ok(brs) => Ok(Json(brs)),
        Err(e) => Err(err_from(e).await),
    }
}

// ─── Inter-bank transfers ──────────────────────────────────────────────

async fn create_transfer(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateTransferRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:transfer:create")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let transfer_date = parse_date(&body.transfer_date, "transfer_date")?;
    let cmd = InitiateInterBankTransferCmd {
        from_bank_account_id: body.from_bank_account_id,
        to_bank_account_id: body.to_bank_account_id,
        amount: body.amount_paise,
        transfer_date,
    };
    match handler
        .initiate_inter_bank_transfer(tenant_of(&user), user.user_id, cmd)
        .await
    {
        Ok(transfer) => Ok(Json(serde_json::json!({ "transfer": transfer }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn get_transfer(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:transfer:create")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    let id = to_uuid(&id, "transfer_id")?;
    match handler.get_transfer(user.tenant_id, id).await {
        Ok(transfer) => Ok(Json(serde_json::json!({ "transfer": transfer }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn list_transfers(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<TransfersQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:transfer:create")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    match handler
        .get_transfers(user.tenant_id, query.from_account, query.status.as_deref())
        .await
    {
        Ok(transfers) => Ok(Json(serde_json::json!({ "transfers": transfers }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn approve_transfer(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:transfer:approve")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "transfer_id")?;
    match handler
        .approve_inter_bank_transfer(tenant_of(&user), id, user.user_id)
        .await
    {
        Ok(transfer) => Ok(Json(serde_json::json!({ "transfer": transfer }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn process_transfer(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:transfer:create")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "transfer_id")?;
    match handler
        .process_inter_bank_transfer(tenant_of(&user), id, user.user_id, None)
        .await
    {
        Ok(transfer) => Ok(Json(serde_json::json!({ "transfer": transfer }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn complete_transfer(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<CompleteTransferRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:transfer:approve")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "transfer_id")?;
    match handler
        .complete_inter_bank_transfer(tenant_of(&user), id, body.bank_reference, user.user_id)
        .await
    {
        Ok(transfer) => Ok(Json(serde_json::json!({ "transfer": transfer }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn cancel_transfer(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<CancelTransferRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:transfer:create")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "transfer_id")?;
    match handler
        .cancel_inter_bank_transfer(tenant_of(&user), id, body.reason, user.user_id)
        .await
    {
        Ok(transfer) => Ok(Json(serde_json::json!({ "transfer": transfer }))),
        Err(e) => Err(err_from(e).await),
    }
}

// ─── Petty cash ────────────────────────────────────────────────────────

async fn create_petty_cash_fund(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreatePettyCashFundRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:petty_cash:create")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    match handler
        .create_petty_cash_fund(
            tenant_of(&user),
            user.user_id,
            body.entity_id,
            body.fund_name,
            body.imprest_amount_paise,
            body.custodian_id,
        )
        .await
    {
        Ok(fund) => Ok(Json(serde_json::json!({ "fund": fund }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn list_petty_cash_funds(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<PettyCashQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:view")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    match handler.get_petty_cash_funds(user.tenant_id, query.entity_id).await {
        Ok(funds) => Ok(Json(serde_json::json!({ "funds": funds }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn get_petty_cash_fund(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:view")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    let id = to_uuid(&id, "fund_id")?;
    match handler.get_petty_cash_fund(user.tenant_id, id).await {
        Ok(fund) => Ok(Json(serde_json::json!({ "fund": fund }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn top_up_petty_cash(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<TopUpRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:petty_cash:create")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "fund_id")?;
    let cmd = TopUpPettyCashCmd {
        amount: body.amount_paise,
        bank_account_id: body.bank_account_id,
    };
    match handler
        .top_up_petty_cash(tenant_of(&user), id, user.user_id, cmd, body.accounting_period_id)
        .await
    {
        Ok(fund) => Ok(Json(serde_json::json!({ "fund": fund }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn record_petty_cash_expense(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ExpenseRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:petty_cash:create")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "fund_id")?;
    let cmd = RecordPettyCashExpenseCmd {
        amount: body.amount_paise,
        expense_account_id: body.expense_account_id,
        voucher_no: body.voucher_no,
        narration: body.narration,
    };
    match handler
        .record_petty_cash_expense(tenant_of(&user), id, user.user_id, cmd, body.accounting_period_id)
        .await
    {
        Ok(fund) => Ok(Json(serde_json::json!({ "fund": fund }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn get_petty_cash_register(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<DateRangeQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:view")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    let id = to_uuid(&id, "fund_id")?;
    let from = parse_opt_date(query.from)?;
    let to = parse_opt_date(query.to)?;
    match handler
        .get_petty_cash_register(user.tenant_id, id, from, to)
        .await
    {
        Ok(register) => Ok(Json(register)),
        Err(e) => Err(err_from(e).await),
    }
}

async fn close_petty_cash_fund(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:petty_cash:create")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let id = to_uuid(&id, "fund_id")?;
    match handler
        .close_petty_cash_fund(tenant_of(&user), id, user.user_id)
        .await
    {
        Ok(fund) => Ok(Json(serde_json::json!({ "fund": fund }))),
        Err(e) => Err(err_from(e).await),
    }
}

// ─── Gateways ──────────────────────────────────────────────────────────

async fn configure_gateway(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(body): Json<ConfigureGatewayRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:gateway:configure")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let cmd = ConfigureGatewayCmd {
        entity_id: body.entity_id,
        gateway_type: body.gateway_type,
        merchant_id: body.merchant_id,
        api_key: body.api_key,
        api_secret: body.api_secret,
        webhook_secret: body.webhook_secret,
        is_active: body.is_active,
    };
    match handler
        .configure_gateway(tenant_of(&user), user.user_id, cmd)
        .await
    {
        Ok(config) => Ok(Json(serde_json::json!({ "gateway_config": config }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn list_gateways(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<GatewayQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:view")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    match handler.get_gateway_configs(user.tenant_id, query.entity_id).await {
        Ok(configs) => Ok(Json(serde_json::json!({ "gateway_configs": configs }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn reconcile_gateway_settlement(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(body): Json<ReconcileSettlementRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:gateway:configure")
        .await?;
    let handler = TreasuryCommandHandler::new(state.db.clone());
    let settlement_date = parse_date(&body.settlement_date, "settlement_date")?;
    let cmd = ReconcileGatewaySettlementCmd {
        entity_id: body.entity_id,
        gateway_type: body.gateway_type,
        settlement_date,
        settled_amount: body.settled_amount_paise,
        gateway_fee_amount: body.gateway_fee_amount_paise,
    };
    match handler
        .reconcile_gateway_settlement(tenant_of(&user), user.user_id, cmd)
        .await
    {
        Ok(settlement) => Ok(Json(serde_json::json!({ "settlement": settlement }))),
        Err(e) => Err(err_from(e).await),
    }
}

async fn list_gateway_settlements(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<GatewayQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:gateway:configure")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    match handler
        .get_gateway_settlements(user.tenant_id, query.entity_id, query.status.as_deref())
        .await
    {
        Ok(settlements) => Ok(Json(serde_json::json!({ "settlements": settlements }))),
        Err(e) => Err(err_from(e).await),
    }
}

// ─── Reports ───────────────────────────────────────────────────────────

async fn get_bank_book(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<BankBookQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "reports:financial:view")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    let from = parse_opt_date(query.from)?;
    let to = parse_opt_date(query.to)?;
    match handler
        .get_bank_book(user.tenant_id, query.bank_account_id, from, to)
        .await
    {
        Ok(book) => Ok(Json(book)),
        Err(e) => Err(err_from(e).await),
    }
}

#[derive(Debug, Deserialize)]
struct BankBookQuery {
    bank_account_id: Uuid,
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
}

async fn get_cash_position(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<BankAccountsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "reports:financial:view")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    match handler
        .get_cash_position(user.tenant_id, query.entity_id)
        .await
    {
        Ok(position) => Ok(Json(position)),
        Err(e) => Err(err_from(e).await),
    }
}

async fn get_uncleared_cheques(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<DateRangeQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .permission_engine
        .require(&user, "treasury:bank_account:view")
        .await?;
    let handler = TreasuryQueryHandler::new(state.db.clone());
    let from = parse_opt_date(query.from)?;
    let to = parse_opt_date(query.to)?;
    match handler.get_uncleared_cheques(user.tenant_id, from, to).await {
        Ok(cheques) => Ok(Json(cheques)),
        Err(e) => Err(err_from(e).await),
    }
}
