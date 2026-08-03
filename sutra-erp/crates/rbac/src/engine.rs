use crate::models::UserPermission;
use deadpool_redis::Pool as RedisPool;
use redis::AsyncCommands;
use sqlx::PgPool;
use uuid::Uuid;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PermissionError { #[error("database error: {0}")] Database(#[from] sqlx::Error), #[error("redis error: {0}")] Redis(String) }
#[derive(Clone)]
pub struct PermissionEngine { pub db: PgPool, pub redis: Option<RedisPool> }
impl std::fmt::Debug for PermissionEngine { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("PermissionEngine").finish_non_exhaustive() } }
impl PermissionEngine {
 pub fn new(db: PgPool, redis: Option<RedisPool>) -> Self { Self { db, redis } }
 pub async fn permissions(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Vec<UserPermission>, PermissionError> {
  let key = format!("rbac:permissions:{}:{}", tenant_id, user_id);
  if let Some(pool) = &self.redis { if let Ok(mut c) = pool.get().await { let cached: Option<String> = c.get(&key).await.map_err(|e| PermissionError::Redis(e.to_string()))?; if let Some(v)=cached { if let Ok(p)=serde_json::from_str(&v) { return Ok(p); } } } }
  let p = sqlx::query_as::<_, UserPermission>("SELECT user_id, tenant_id, permission_code, scope_type, scope_id FROM v_user_permissions WHERE user_id=$1 AND tenant_id=$2").bind(user_id).bind(tenant_id).fetch_all(&self.db).await?;
  if let Some(pool)=&self.redis { if let Ok(mut c)=pool.get().await { let _: Result<(), _> = c.set_ex(&key, serde_json::to_string(&p).unwrap_or_default(), 300).await; } }
  Ok(p)
 }
 pub async fn has_permission(&self, user_id: Uuid, tenant_id: Uuid, code: &str, scope_type: &str, scope_id: Option<Uuid>) -> Result<bool, PermissionError> {
  // IT administrators are deliberately barred from finance data permissions.
  let finance = ["gl:","ar:","ap:","treasury:","tax:","budget:"] .iter().any(|p| code.starts_with(p));
  let rows=self.permissions(user_id,tenant_id).await?;
  if rows.iter().any(|r| r.permission_code=="admin:role:manage") && finance { return Ok(false); }
  Ok(rows.iter().any(|r| r.permission_code==code && (r.scope_type=="GLOBAL" || r.scope_type==scope_type && (r.scope_id.is_none() || r.scope_id==scope_id))))
 }
}
