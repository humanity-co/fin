use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Permission { pub id: Uuid, pub code: String, pub module: String, pub resource: String, pub action: String, pub description: Option<String>, pub created_at: DateTime<Utc> }
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Role { pub id: Uuid, pub tenant_id: Uuid, pub code: String, pub name: String, pub is_system: bool, pub is_active: bool, pub created_at: DateTime<Utc>, pub created_by: Option<Uuid>, pub updated_at: DateTime<Utc>, pub updated_by: Option<Uuid> }
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RolePermission { pub role_id: Uuid, pub permission_id: Uuid, pub granted_at: DateTime<Utc>, pub granted_by: Option<Uuid> }
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRole { pub id: Uuid, pub tenant_id: Uuid, pub user_id: Uuid, pub role_id: Uuid, pub scope_type: String, pub scope_id: Option<Uuid>, pub valid_from: DateTime<Utc>, pub valid_to: Option<DateTime<Utc>>, pub created_at: DateTime<Utc>, pub created_by: Option<Uuid> }
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserPermission { pub user_id: Uuid, pub tenant_id: Uuid, pub permission_code: String, pub scope_type: String, pub scope_id: Option<Uuid> }
