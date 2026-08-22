//! Tax Engine API routes — v1.
//!
//! All endpoints live under `/api/v1/tax`. They wrap the existing
//! `sutra-finance-taxation` command/query handlers (Phase 2/3a) — no
//! business logic is duplicated here, only request shaping and RBAC
//! enforcement.
//!
//! Permission mapping follows the lead-ratified RBAC extension
//! (`rbac-extension.md`):
//! - `tax:config:configure`       — GSTIN / rate / TDS-section master setup
//! - `tax:itc:compute`            — ITC register + Rule 42/43 computation
//! - `tax:itc:reverse`            — ITC reversal on a register line
//! - `tax:gst_return:prepare`     — GSTR-1/3B/9/9C generation, RCM entry
//! - `tax:gst_return:file`        — GSTR filing
//! - `tax:tds:deposit`            — TDS deposit to government
//! - `tax:tds_return:prepare`     — TDS return (24Q/26Q/27Q) generation
//! - `tax:tds_return:file`        — TDS return filing (ratified per rbac-extension.md §2)
//! - `tax:form16:generate`        — Form 16 / 16A generation
//! - `tax:income:compute`         — 85% application + s.11(5) + FCRA compute
//! - `tax:exemption:register`     — trust exemption (12A/12AB/10(23C)) + FCRA register/renew
//! - `tax:return:view`            — all read-only tax views (GLOBAL statutory visibility)
//!
//! Money is transported as integer paise everywhere — never floats.
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
use sutra_core::TenantId;
use sutra_finance_taxation::{
    ComputeFcraComplianceCmd, ComputeIncomeApplicationCmd, ComputeItcCmd,
    ComputeRule42ReversalCmd, ComputeRule43ReversalCmd, ConfigureTdsSectionCmd,
    CreateRcmEntryCmd, DepositTdsToGovtCmd, FileTdsReturnCmd, FlagNonCompliantInvestmentsCmd,
    GenerateForm16ACmd, GenerateForm16Cmd, GenerateGstr9cCmd, GenerateGstr9Cmd,
    GenerateGstrCmd, GenerateTdsReturnCmd, RecordGstFilingCmd, RegisterFcraCmd,
    RegisterGstinCmd, RegisterTrustExemptionCmd, RenewExemptionCmd, ReverseItcCmd,
    TaxCommandHandler, TaxError, TaxQueryHandler, UpsertGstRateCmd,
    GstSupplyType,
};
use sutra_rbac::UserContext;
use crate::state::AppState;

/// Build the Tax routes sub-router (nested at `/api/v1/tax`).
pub fn tax_routes() -> Router<Arc<AppState>> {
    Router::new()
        // ── GSTIN / configuration ─────────────────────────────────────
        .route("/gstin", get(get_gst_registrations))
        .route("/gstin/register", post(register_gstin))
        .route("/rates", get(get_gst_rates).put(upsert_gst_rate))
        // ── TDS ───────────────────────────────────────────────────────
        .route("/tds/register", get(get_tds_register))
        .route("/tds/deposits/pending", get(get_pending_tds_deposits))
        .route("/tds/deposit", post(deposit_tds_to_govt))
        .route("/tds/sections", get(get_tds_sections))
        .route("/tds/sections/configure", put(configure_tds_section))
        .route("/tds/returns", get(get_tds_returns))
        .route("/tds/returns/{id}", get(get_tds_return))
        .route("/tds/returns/generate", post(generate_tds_return))
        .route("/tds/returns/file", post(file_tds_return))
        .route("/tds/form16", get(get_form16_certificate).post(generate_form16))
        .route("/tds/form16a", get(get_form16a_certificate).post(generate_form16a))
        // ── ITC / RCM ─────────────────────────────────────────────────
        .route("/itc/register", get(get_itc_register))
        .route("/itc/summary", get(get_itc_summary))
        .route("/itc/compute", post(compute_itc))
        .route("/itc/reversal/rule42", post(compute_rule42_reversal))
        .route("/itc/reversal/rule43", post(compute_rule43_reversal))
        .route("/itc/reverse", post(reverse_itc))
        .route("/rcm/payable", get(get_rcm_payable))
        .route("/rcm", post(create_rcm_entry))
        // ── GSTR ──────────────────────────────────────────────────────
        .route("/gst/preview", get(get_gstr_preview))
        .route("/gst/liability", get(get_gst_liability_summary))
        .route("/gst/generate/1", post(generate_gstr1))
        .route("/gst/generate/3b", post(generate_gstr3b))
        .route("/gst/generate/9", post(generate_gstr9))
        .route("/gst/generate/9c", post(generate_gstr9c))
        .route("/gst/file", post(record_gst_filing))
        // ── Income / trust / exemption ────────────────────────────────
        .route("/income/exemptions", get(get_trust_exemption))
        .route("/income/exemption", post(register_trust_exemption))
        .route("/income/exemption/renew", post(renew_exemption))
        .route("/income/application", get(get_income_application))
        .route("/income/compute", post(compute_income_application))
        .route("/income/accumulated", get(get_accumulated_income))
        .route("/income/section115", get(get_section115_compliance))
        .route("/income/investments/flag", post(flag_non_compliant_investments))
        .route("/income/itr7", get(get_itr7_data))
        .route("/income/audit-requirements", get(get_audit_requirements))
        // ── FCRA ──────────────────────────────────────────────────────
        .route("/fcra", get(get_fcra_status))
        .route("/fcra/compliance", get(get_fcra_compliance))
        .route("/fcra/register", post(register_fcra))
        .route("/fcra/compute", post(compute_fcra_compliance))
}

// ─── Query parameter types ─────────────────────────────────────────────
#[derive(Debug, Deserialize)]
struct TdsRegisterQuery {
    entity_id: Option<Uuid>,
    from: Option<String>,
    to: Option<String>,
}
#[derive(Debug, Deserialize)]
struct EntityOptionalQuery {
    entity_id: Option<Uuid>,
}
#[derive(Debug, Deserialize)]
struct AsOfQuery {
    #[serde(default)]
    as_of: Option<String>,
}
#[derive(Debug, Deserialize)]
struct RegistrationPeriodQuery {
    registration_id: Uuid,
    period: String,
}
#[derive(Debug, Deserialize)]
struct RegistrationFyQuery {
    registration_id: Uuid,
    fiscal_year: String,
}
#[derive(Debug, Deserialize)]
struct ReturnIdQuery {
    return_id: Uuid,
}
#[derive(Debug, Deserialize)]
struct GstRatesQuery {
    #[serde(default)]
    hsn_sac_code: Option<String>,
    #[serde(default)]
    supply_type: Option<GstSupplyType>,
    #[serde(default)]
    as_of: Option<String>,
}
#[derive(Debug, Deserialize)]
struct TdsReturnsQuery {
    entity_id: Option<Uuid>,
    fiscal_year: Option<String>,
}
#[derive(Debug, Deserialize)]
struct Form16Query {
    employee_id: Uuid,
    fiscal_year: String,
}
#[derive(Debug, Deserialize)]
struct Form16AQuery {
    vendor_id: Uuid,
    fiscal_year: String,
}
#[derive(Debug, Deserialize)]
struct EntityFyQuery {
    entity_id: Uuid,
    fiscal_year: String,
}
#[derive(Debug, Deserialize)]
struct EntityFyIdQuery {
    entity_id: Uuid,
    fiscal_year_id: Uuid,
}
#[derive(Debug, Deserialize)]
struct Itr7Query {
    entity_id: Uuid,
    fiscal_year_id: Uuid,
    fiscal_year: String,
}
#[derive(Debug, Deserialize)]
struct EntityQuery {
    entity_id: Uuid,
}
#[derive(Debug, Deserialize)]
struct RegistrationIdQuery {
    registration_id: Uuid,
}

// ─── Helpers ───────────────────────────────────────────────────────────
fn err_response(status: StatusCode, msg: String) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

fn err_from(e: TaxError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match &e {
        TaxError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        TaxError::GstRegistrationNotFound(_)
        | TaxError::GstReturnNotFound(_)
        | TaxError::ItcRegisterNotFound(_, _)
        | TaxError::ItcLineNotFound(_)
        | TaxError::TdsSectionNotFound(_, _)
        | TaxError::TdsDeductionNotFound(_)
        | TaxError::TdsDepositNotFound(_)
        | TaxError::TdsReturnNotFound(_)
        | TaxError::Form16CertificateNotFound(_)
        | TaxError::TrustExemptionNotFound(_, _)
        | TaxError::IncomeApplicationNotFound(_)
        | TaxError::FcraRegistrationNotFound(_) => StatusCode::NOT_FOUND,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    };
    err_response(status, e.to_string())
}

/// Resolve the tenant from the authenticated context. Mirrors the existing
/// gl/ap/treasury handlers, using the caller's tenant until per-tenant
/// provisioning wiring is finalized.
fn tenant_of(user: &UserContext) -> TenantId {
    TenantId::from_uuid(user.tenant_id)
}

fn parse_date(s: &str, field: &str) -> Result<chrono::NaiveDate, (StatusCode, Json<serde_json::Value>)> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| err_response(StatusCode::BAD_REQUEST, format!("invalid {field}, expected YYYY-MM-DD")))
}
fn opt_date(
    s: Option<String>,
    field: &str,
) -> Result<Option<chrono::NaiveDate>, (StatusCode, Json<serde_json::Value>)> {
    match s {
        None => Ok(None),
        Some(v) => parse_date(&v, field).map(Some),
    }
}

// ─── GSTIN / configuration ─────────────────────────────────────────────
async fn get_gst_registrations(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<EntityOptionalQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_gst_registrations(user.tenant_id, query.entity_id).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn register_gstin(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<RegisterGstinCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:config:configure").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.register_gstin(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_gst_rates(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<GstRatesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    let code = query.hsn_sac_code.unwrap_or_default();
    let as_of = match query.as_of {
        Some(v) => parse_date(&v, "as_of")?,
        None => chrono::Utc::now().date_naive(),
    };
    match handler.get_gst_rates(user.tenant_id, &code, query.supply_type, as_of).await {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(err_from(e)),
    }
}

async fn upsert_gst_rate(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<UpsertGstRateCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:config:configure").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.upsert_gst_rate(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

// ─── TDS ───────────────────────────────────────────────────────────────
async fn get_tds_register(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<TdsRegisterQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    let from = parse_date(&query.from.clone().unwrap_or_default(), "from")?;
    let to = parse_date(&query.to.clone().unwrap_or_default(), "to")?;
    match handler.get_tds_register(user.tenant_id, query.entity_id, from, to).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_pending_tds_deposits(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<EntityOptionalQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_pending_tds_deposits(user.tenant_id, query.entity_id).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn deposit_tds_to_govt(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<DepositTdsToGovtCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:tds:deposit").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.deposit_tds_to_govt(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_tds_sections(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<AsOfQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    let as_of = match query.as_of {
        Some(v) => parse_date(&v, "as_of")?,
        None => chrono::Utc::now().date_naive(),
    };
    match handler.get_tds_sections(user.tenant_id, as_of).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn configure_tds_section(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<ConfigureTdsSectionCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:config:configure").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.configure_tds_section(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_tds_returns(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<TdsReturnsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_tds_returns(user.tenant_id, query.entity_id, query.fiscal_year.as_deref()).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_tds_return(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_tds_return(user.tenant_id, id).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(err_response(StatusCode::NOT_FOUND, "tds return not found".into())),
        Err(e) => Err(err_from(e)),
    }
}

async fn generate_tds_return(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<GenerateTdsReturnCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:tds_return:prepare").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.generate_tds_return(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

// Uses the ratified `tax:tds_return:file` permission (rbac-extension.md §2).
async fn file_tds_return(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<FileTdsReturnCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:tds_return:file").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.file_tds_return(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_form16_certificate(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<Form16Query>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_form16_certificate(user.tenant_id, query.employee_id, &query.fiscal_year).await {
        Ok(Some(v)) => Ok(Json(serde_json::to_value(v).unwrap())),
        Ok(None) => Err(err_response(StatusCode::NOT_FOUND, "form16 not found".into())),
        Err(e) => Err(err_from(e)),
    }
}

async fn generate_form16(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<GenerateForm16Cmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:form16:generate").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.generate_form16(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_form16a_certificate(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<Form16AQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_form16a_certificate(user.tenant_id, query.vendor_id, &query.fiscal_year).await {
        Ok(Some(v)) => Ok(Json(serde_json::to_value(v).unwrap())),
        Ok(None) => Err(err_response(StatusCode::NOT_FOUND, "form16a not found".into())),
        Err(e) => Err(err_from(e)),
    }
}

async fn generate_form16a(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<GenerateForm16ACmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:form16:generate").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.generate_form16a(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

// ─── ITC / RCM ─────────────────────────────────────────────────────────
async fn get_itc_register(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<RegistrationPeriodQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_itc_register(user.tenant_id, query.registration_id, &query.period).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(err_response(StatusCode::NOT_FOUND, "itc register not found".into())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_itc_summary(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<RegistrationFyQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_itc_summary(user.tenant_id, query.registration_id, &query.fiscal_year).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn compute_itc(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<ComputeItcCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:itc:compute").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.compute_itc(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn compute_rule42_reversal(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<ComputeRule42ReversalCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:itc:compute").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.compute_rule42_reversal(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn compute_rule43_reversal(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<ComputeRule43ReversalCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:itc:compute").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.compute_rule43_reversal(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn reverse_itc(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<ReverseItcCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:itc:reverse").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.reverse_itc(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_rcm_payable(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<TdsRegisterQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    let from = parse_date(&query.from.clone().unwrap_or_default(), "from")?;
    let to = parse_date(&query.to.clone().unwrap_or_default(), "to")?;
    match handler.get_rcm_payable(user.tenant_id, query.entity_id, from, to).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn create_rcm_entry(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<CreateRcmEntryCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:gst_return:prepare").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.create_rcm_entry(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

// ─── GSTR ──────────────────────────────────────────────────────────────
async fn get_gstr_preview(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ReturnIdQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_gstr_preview(user.tenant_id, query.return_id).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(err_response(StatusCode::NOT_FOUND, "gst return not found".into())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_gst_liability_summary(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<RegistrationFyQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_gst_liability_summary(user.tenant_id, query.registration_id, &query.fiscal_year).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn generate_gstr1(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<GenerateGstrCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:gst_return:prepare").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.generate_gstr1(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn generate_gstr3b(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<GenerateGstrCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:gst_return:prepare").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.generate_gstr3b(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn generate_gstr9(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<GenerateGstr9Cmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:gst_return:prepare").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.generate_gstr9(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn generate_gstr9c(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<GenerateGstr9cCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:gst_return:prepare").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.generate_gstr9c(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn record_gst_filing(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<RecordGstFilingCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:gst_return:file").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.record_gst_filing(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

// ─── Income / trust / exemption ────────────────────────────────────────
async fn get_trust_exemption(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_trust_exemption(user.tenant_id, query.entity_id).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

// Uses the ratified `tax:exemption:register` permission (rbac-extension.md §3).
async fn register_trust_exemption(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<RegisterTrustExemptionCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:exemption:register").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.register_trust_exemption(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

// Uses the ratified `tax:exemption:register` permission (rbac-extension.md §3).
async fn renew_exemption(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<RenewExemptionCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:exemption:register").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.renew_exemption(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_income_application(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<EntityFyIdQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_income_application(user.tenant_id, query.entity_id, query.fiscal_year_id).await {
        Ok(Some(v)) => Ok(Json(serde_json::to_value(v).unwrap())),
        Ok(None) => Err(err_response(StatusCode::NOT_FOUND, "income application not found".into())),
        Err(e) => Err(err_from(e)),
    }
}

async fn compute_income_application(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<ComputeIncomeApplicationCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:income:compute").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.compute_income_application(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_accumulated_income(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_accumulated_income(user.tenant_id, query.entity_id).await {
        Ok(v) => Ok(Json(serde_json::json!({ "accumulated_income": v }))),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_section115_compliance(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<EntityFyIdQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_section115_compliance(user.tenant_id, query.entity_id, query.fiscal_year_id).await {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(err_from(e)),
    }
}

async fn flag_non_compliant_investments(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<FlagNonCompliantInvestmentsCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:income:compute").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.flag_non_compliant_investments(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_itr7_data(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<Itr7Query>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_itr7_data(user.tenant_id, query.entity_id, query.fiscal_year_id, &query.fiscal_year).await {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_audit_requirements(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<EntityFyQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_audit_requirements(user.tenant_id, query.entity_id, &query.fiscal_year).await {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(err_from(e)),
    }
}

// ─── FCRA ──────────────────────────────────────────────────────────────
async fn get_fcra_status(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_fcra_status(user.tenant_id, query.entity_id).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn get_fcra_compliance(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<RegistrationIdQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:return:view").await?;
    let handler = TaxQueryHandler::new(state.db.clone());
    match handler.get_fcra_compliance(user.tenant_id, query.registration_id).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(err_response(StatusCode::NOT_FOUND, "fcra registration not found".into())),
        Err(e) => Err(err_from(e)),
    }
}

async fn register_fcra(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<RegisterFcraCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:exemption:register").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.register_fcra(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}

async fn compute_fcra_compliance(
    user: UserContext,
    State(state): State<Arc<AppState>>,
    Json(cmd): Json<ComputeFcraComplianceCmd>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.permission_engine.require(&user, "tax:income:compute").await?;
    let handler = TaxCommandHandler::new(state.db.clone());
    match handler.compute_fcra_compliance(tenant_of(&user), user.user_id, cmd).await {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err(err_from(e)),
    }
}
