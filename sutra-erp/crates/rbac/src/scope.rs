use crate::middleware::UserContext;
use uuid::Uuid;
/// Produces a parameterized fragment; callers must bind returned IDs in order.
#[derive(Debug, Clone)]
pub struct ScopeFilter { pub clause: String, pub ids: Vec<Uuid> }
impl ScopeFilter { pub fn for_context(ctx:&UserContext, entity_column:&str, cost_center_column:&str, user_column:&str)->Self { let mut clauses=Vec::new(); let mut ids=Vec::new(); for s in &ctx.scopes { match s.scope_type.as_str() { "GLOBAL"=>return Self{clause:String::new(),ids}, "CAMPUS"=>if let Some(id)=s.scope_id { ids.push(id); clauses.push(format!("AND {} = ${}",entity_column,ids.len())); }, "DEPARTMENT"=>if let Some(id)=s.scope_id { ids.push(id); clauses.push(format!("AND {} = ${}",cost_center_column,ids.len())); }, "SELF"=>{ids.push(ctx.user_id); clauses.push(format!("AND {} = ${}",user_column,ids.len()));}, _=>{} } } Self{clause: if clauses.is_empty(){"AND FALSE".into()}else{format!("AND ({})",clauses.join(" OR "))},ids} } }
