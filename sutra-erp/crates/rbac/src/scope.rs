use crate::middleware::ScopeGrant;
use uuid::Uuid;

/// Produces a parameterized SQL fragment restricting rows to the caller's
/// scope grants; callers must bind the returned IDs in order.
///
/// Scope grants are resolved by the RBAC engine from the DB (`user_roles`);
/// the auth [`UserContext`](sutra_auth::UserContext) carries only identity
/// (user, tenant, roles) — not per-grant scopes.
#[derive(Debug, Clone)]
pub struct ScopeFilter { pub clause: String, pub ids: Vec<Uuid> }

impl ScopeFilter {
    /// Build a WHERE fragment for the given grants.
    ///
    /// - a GLOBAL grant short-circuits to no filter (full access),
    /// - otherwise CAMPUS/DEPARTMENT/SELF grants are OR-ed together,
    /// - no grants at all yields `AND FALSE` (deny by default).
    pub fn for_context(
        user_id: Uuid,
        scopes: &[ScopeGrant],
        entity_column: &str,
        cost_center_column: &str,
        user_column: &str,
    ) -> Self {
        let mut clauses = Vec::new();
        let mut ids = Vec::new();
        for s in scopes {
            match s.scope_type.as_str() {
                "GLOBAL" => return Self { clause: String::new(), ids },
                "CAMPUS" => if let Some(id) = s.scope_id {
                    ids.push(id);
                    clauses.push(format!("AND {} = ${}", entity_column, ids.len()));
                },
                "DEPARTMENT" => if let Some(id) = s.scope_id {
                    ids.push(id);
                    clauses.push(format!("AND {} = ${}", cost_center_column, ids.len()));
                },
                "SELF" => {
                    ids.push(user_id);
                    clauses.push(format!("AND {} = ${}", user_column, ids.len()));
                }
                _ => {}
            }
        }
        Self {
            clause: if clauses.is_empty() { "AND FALSE".into() } else { format!("AND ({})", clauses.join(" OR ")) },
            ids,
        }
    }
}
