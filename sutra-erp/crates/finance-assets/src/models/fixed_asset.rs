//! Fixed Assets models — FixedAsset aggregate root.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedAsset {
    pub fixed_asset_id: EntityId<FixedAsset>,
    pub tenant_id: TenantId,
    pub asset_code: String,
    pub asset_category: String,
    pub asset_name: String,
    pub description: Option<String>,
    pub purchase_date: NaiveDate,
    pub capitalization_date: NaiveDate,
    pub purchase_cost: Money,
    pub gst_on_purchase: Option<Money>,
    pub itc_claimed: Option<Money>,
    pub depreciation_method: String,
    pub depreciation_rate: rust_decimal::Decimal,
    pub useful_life: i32,
    pub salvage_value: Option<Money>,
    pub current_location: Option<String>,
    pub department_id: Option<Uuid>,
    pub custodian_id: Option<Uuid>,
    pub fund_id: Option<Uuid>,
    pub status: String,
    pub is_capital_goods: bool,
    pub asset_tag: Option<String>,
    pub audit: AuditInfo,
}
