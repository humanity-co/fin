//! Authentication API routes — v1.
//!
//! | Method | Path                    | Auth | Description                    |
//! |--------|-------------------------|------|--------------------------------|
//! | POST   | /api/v1/auth/register   | No   | Create a user account          |
//! | POST   | /api/v1/auth/login      | No   | Verify password, issue JWT     |
//! | GET    | /api/v1/auth/me         | Yes  | Current user from token        |

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use sutra_auth::{
    hashing::{hash_password, verify_password},
    jwt::create_token,
    middleware::UserContext,
    models::{CreateUser, LoginRequest, LoginResponse},
};

use crate::state::AppState;

/// Create the auth sub-router.
///
/// `/register` and `/login` are public; `/me` is mounted behind the auth
/// layer by `crate::router::create_router` (see `authed` router).
pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
}

/// Standard success envelope.
fn ok_response(status: StatusCode, data: Value) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({
            "success": true,
            "error": null,
            "data": data,
        })),
    )
}

/// Standard error envelope.
fn err_response(status: StatusCode, msg: String) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({
            "success": false,
            "error": msg,
            "data": null,
        })),
    )
}

/// Row shape for `app_users` (password hash never leaves this module).
#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    tenant_id: Uuid,
    email: String,
    password_hash: String,
    full_name: String,
    is_active: bool,
    is_superadmin: bool,
    last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl UserRow {
    fn to_app_user(&self) -> sutra_auth::AppUser {
        sutra_auth::AppUser {
            id: self.id,
            tenant_id: self.tenant_id,
            email: self.email.clone(),
            password_hash: self.password_hash.clone(),
            full_name: self.full_name.clone(),
            is_active: self.is_active,
            is_superadmin: self.is_superadmin,
            last_login_at: self.last_login_at,
            created_at: self.created_at,
        }
    }
}

/// Load the role codes a user holds (from the RBAC grant tables).
///
/// Returns an empty vec if role resolution is unavailable (e.g. RBAC tables
/// not yet migrated/seeded) — authentication still succeeds, the user simply
/// has no roles until an admin assigns them.
async fn load_roles(db: &sqlx::PgPool, user_id: Uuid, tenant_id: Uuid) -> Vec<String> {
    let result = sqlx::query_scalar::<_, String>(
        "SELECT r.code
           FROM user_roles ur
           JOIN roles r ON r.id = ur.role_id AND r.is_active
          WHERE ur.user_id = $1 AND ur.tenant_id = $2
            AND ur.valid_from <= now()
            AND (ur.valid_to IS NULL OR ur.valid_to > now())",
    )
    .bind(user_id)
    .bind(tenant_id)
    .fetch_all(db)
    .await;
    match result {
        Ok(roles) => roles,
        Err(e) => {
            tracing::warn!(user_id = %user_id, error = %e, "auth: role lookup unavailable");
            Vec::new()
        }
    }
}

/// `POST /api/v1/auth/register` — create a user account (public).
pub(crate) async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateUser>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // Input validation (never trust the client).
    let email = body.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(err_response(StatusCode::BAD_REQUEST, "a valid email is required".into()));
    }
    if body.full_name.trim().is_empty() {
        return Err(err_response(StatusCode::BAD_REQUEST, "full_name is required".into()));
    }
    if body.password.len() < 8 {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            "password must be at least 8 characters".into(),
        ));
    }

    // Duplicate check within the tenant.
    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM app_users WHERE tenant_id = $1 AND email = $2",
    )
    .bind(body.tenant_id)
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if existing.is_some() {
        return Err(err_response(
            StatusCode::CONFLICT,
            "a user with this email already exists in the tenant".into(),
        ));
    }

    let password_hash = hash_password(&body.password)
        .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let row = sqlx::query_as::<_, UserRow>(
        "INSERT INTO app_users (tenant_id, email, password_hash, full_name)
         VALUES ($1, $2, $3, $4)
         RETURNING id, tenant_id, email, password_hash, full_name,
                   is_active, is_superadmin, last_login_at, created_at",
    )
    .bind(body.tenant_id)
    .bind(&email)
    .bind(password_hash)
    .bind(body.full_name.trim())
    .fetch_one(&state.db)
    .await
    .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = row.to_app_user();
    Ok(ok_response(
        StatusCode::CREATED,
        json!({ "user": user.to_user_info(Vec::new()) }),
    ))
}

/// `POST /api/v1/auth/login` — verify credentials and issue a JWT (public).
pub(crate) async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let email = body.email.trim().to_lowercase();
    if email.is_empty() || body.password.is_empty() {
        return Err(err_response(StatusCode::BAD_REQUEST, "email and password are required".into()));
    }

    // Resolve the user: tenant-scoped when provided, otherwise by email alone.
    let user = match body.tenant_id {
        Some(tenant_id) => {
            let row = sqlx::query_as::<_, UserRow>(
                "SELECT id, tenant_id, email, password_hash, full_name,
                        is_active, is_superadmin, last_login_at, created_at
                   FROM app_users
                  WHERE tenant_id = $1 AND email = $2",
            )
            .bind(tenant_id)
            .bind(&email)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            row
        }
        None => {
            let rows = sqlx::query_as::<_, UserRow>(
                "SELECT id, tenant_id, email, password_hash, full_name,
                        is_active, is_superadmin, last_login_at, created_at
                   FROM app_users
                  WHERE email = $1",
            )
            .bind(&email)
            .fetch_all(&state.db)
            .await
            .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            if rows.len() > 1 {
                return Err(err_response(
                    StatusCode::UNAUTHORIZED,
                    "email exists in multiple tenants — please provide tenant_id".into(),
                ));
            }
            rows.into_iter().next()
        }
    };

    let Some(row) = user else {
        // Do not reveal whether the account exists.
        return Err(err_response(StatusCode::UNAUTHORIZED, "invalid credentials".into()));
    };

    if !row.is_active {
        return Err(err_response(StatusCode::FORBIDDEN, "account is disabled".into()));
    }

    let password_ok = verify_password(&body.password, &row.password_hash)
        .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !password_ok {
        return Err(err_response(StatusCode::UNAUTHORIZED, "invalid credentials".into()));
    }

    let roles = load_roles(&state.db, row.id, row.tenant_id).await;

    // Record last login — best-effort, never fails the request.
    let _ = sqlx::query("UPDATE app_users SET last_login_at = now() WHERE id = $1")
        .bind(row.id)
        .execute(&state.db)
        .await;

    let token = create_token(row.id, row.tenant_id, roles.clone())
        .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response = LoginResponse {
        token,
        user: row.to_app_user().to_user_info(roles),
    };
    Ok(Json(json!({
        "success": true,
        "error": null,
        "data": response,
    })))
}

/// `GET /api/v1/auth/me` — current user from the token (authenticated).
pub(crate) async fn me(
    State(state): State<Arc<AppState>>,
    user: UserContext,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, tenant_id, email, password_hash, full_name,
                is_active, is_superadmin, last_login_at, created_at
           FROM app_users
          WHERE id = $1 AND tenant_id = $2",
    )
    .bind(user.user_id)
    .bind(user.tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let Some(row) = row else {
        return Err(err_response(StatusCode::UNAUTHORIZED, "user no longer exists".into()));
    };
    if !row.is_active {
        return Err(err_response(StatusCode::FORBIDDEN, "account is disabled".into()));
    }

    Ok(Json(json!({
        "success": true,
        "error": null,
        "data": row.to_app_user().to_user_info(user.roles),
    })))
}
