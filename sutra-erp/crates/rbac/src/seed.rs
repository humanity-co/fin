use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;
const PERMISSIONS: &[&str] = &["gl:coa:view","gl:coa:configure","gl:journal:view","gl:journal:create","gl:journal:approve","gl:journal:post","gl:period_close:execute","ar:fee_structure:view","ar:fee_structure:configure","ar:fee_collection:collect","ar:fee_collection:view","ar:receipt:cancel","ar:concession:create","ar:scholarship:create","ar:scholarship:approve","ar:refund:create","ar:refund:approve","ap:vendor:create","ap:po:create","ap:po:approve","ap:grn:create","ap:invoice:create","ap:invoice:approve","ap:payment:create","ap:payment:approve","ap:reimbursement:create","ap:reimbursement:approve","treasury:bank_account:view","treasury:bank_account:configure","treasury:reconciliation:perform","treasury:reconciliation:approve","treasury:transfer:create","treasury:transfer:approve","treasury:bank_statement:upload","treasury:bank_statement:view","treasury:gateway:configure","treasury:petty_cash:create","tax:return:view","tax:gst_return:prepare","tax:gst_return:file","tax:tds:deduct","tax:config:configure","tax:exemption:register","tax:form16:generate","tax:income:compute","tax:itc:compute","tax:itc:reverse","tax:tds:deposit","tax:tds_return:prepare","tax:tds_return:file","budget:budget:view","budget:budget:create","budget:budget:approve","budget:revision:create","budget:encumbrance:view","reports:financial:view","reports:statutory:view","reports:dashboard:view","reports:export","workflow:approval_queue:view","workflow:approval:action","workflow:rule:configure","workflow:exception:handle","admin:user:manage","admin:role:manage","admin:config:manage","admin:audit_log:view","admin:entity:manage"];
/// Tax-module role grants per rbac-extension.md role matrix (scope is set on
/// `user_roles.scope_type` at assignment, so grants here are scope-agnostic).
///   CFO               — all 7 (GLOBAL)
///   Finance Controller— itc:compute/reverse (G), tds:deposit,
///                       tds_return:prepare, form16:generate (C), income:compute (G);
///                       NOT exemption:register
///   Accountant        — itc:compute (G), tds:deposit, tds_return:prepare,
///                       form16:generate (C)
/// Trustee/Mgmt, Cashier, Principal, Registrar, HOD, Faculty, Student,
/// Auditor, Procurement, Compliance, IT Admin — none of the 7.
const TAX_ROLE_GRANTS: &[(&str, &[&str])] = &[
    (
        "cfo",
        &[
            "tax:itc:compute",
            "tax:itc:reverse",
            "tax:tds:deposit",
            "tax:tds_return:prepare",
            "tax:tds_return:file",
            "tax:form16:generate",
            "tax:income:compute",
            "tax:exemption:register",
        ],
    ),
    (
        "finance_controller",
        &[
            "tax:itc:compute",
            "tax:itc:reverse",
            "tax:tds:deposit",
            "tax:tds_return:prepare",
            "tax:form16:generate",
            "tax:income:compute",
        ],
    ),
    (
        "accountant",
        &[
            "tax:itc:compute",
            "tax:tds:deposit",
            "tax:tds_return:prepare",
            "tax:form16:generate",
        ],
    ),
    // Compliance Officer — ratified TDS-return filing holder (rbac-extension.md §2).
    (
        "compliance",
        &["tax:tds_return:file"],
    ),
];
#[derive(Debug, Error)] pub enum SeedError { #[error(transparent)] Db(#[from] sqlx::Error) }
pub async fn seed(pool: &PgPool, tenant_id: Uuid) -> Result<(), SeedError> {
 // Permissions are global and inserts are idempotent — run on every seed so
 // existing tenants pick up newly-added permission codes (e.g. the 7 tax
 // permissions) without a destructive re-seed.
 for code in PERMISSIONS {
  let mut p=code.split(':');
  sqlx::query("INSERT INTO permissions(code,module,resource,action) VALUES($1,$2,$3,$4) ON CONFLICT(code) DO NOTHING")
   .bind(code).bind(p.next().unwrap_or_default()).bind(p.next().unwrap_or_default()).bind(p.next().unwrap_or_default())
   .execute(pool).await?;
 }
 let n: i64 = sqlx::query_scalar("SELECT count(*) FROM roles WHERE tenant_id=$1").bind(tenant_id).fetch_one(pool).await?;
 if n == 0 {
  for (code,name) in [("trustee","Trustee/Mgmt"),("cfo","CFO"),("finance_controller","Finance Controller"),("accountant","Accountant"),("cashier","Cashier"),("principal","Principal"),("registrar","Registrar"),("hod","HOD"),("faculty","Faculty"),("student","Student/Parent"),("auditor","Auditor"),("procurement","Procurement"),("compliance","Compliance"),("it_admin","IT Admin")] {
   sqlx::query("INSERT INTO roles(tenant_id,code,name,is_system) VALUES($1,$2,$3,TRUE)").bind(tenant_id).bind(code).bind(name).execute(pool).await?;
  }
 }
 // Role grants for the tax module's 7 NEW permissions (idempotent).
 for (role_code, perms) in TAX_ROLE_GRANTS {
  for perm in *perms {
   sqlx::query(
    "INSERT INTO role_permissions (role_id, permission_id)
     SELECT r.id, p.id FROM roles r
     JOIN permissions p ON p.code = $3
     WHERE r.tenant_id = $1 AND r.code = $2
     ON CONFLICT (role_id, permission_id) DO NOTHING",
   )
   .bind(tenant_id).bind(role_code).bind(perm)
   .execute(pool).await?;
  }
 }
 Ok(())
}
