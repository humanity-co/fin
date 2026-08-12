//! Treasury models — payment gateways and settlements.
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

/// Supported Indian payment gateways.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GatewayType {
    Billdesk,
    Razorpay,
    Ccavenue,
    Phonepay,
    Paytm,
}

impl GatewayType {
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "BILLDESK" => Some(GatewayType::Billdesk),
            "RAZORPAY" => Some(GatewayType::Razorpay),
            "CCAVENUE" => Some(GatewayType::Ccavenue),
            "PHONEPE" => Some(GatewayType::Phonepay),
            "PAYTM" => Some(GatewayType::Paytm),
            _ => None,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            GatewayType::Billdesk => "BILLDESK",
            GatewayType::Razorpay => "RAZORPAY",
            GatewayType::Ccavenue => "CCAVENUE",
            GatewayType::Phonepay => "PHONEPE",
            GatewayType::Paytm => "PAYTM",
        }
    }
}

/// Payment gateway credentials per (tenant, entity, gateway).
/// Secrets (api_key / api_secret / webhook_secret) are encrypted at rest
/// at the application layer before persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentGatewayConfig {
    pub payment_gateway_config_id: EntityId<PaymentGatewayConfig>,
    pub tenant_id: TenantId,
    pub entity_id: Option<Uuid>,
    pub gateway_type: GatewayType,
    pub merchant_id: String,
    pub api_key: String,      // encrypted at rest
    pub api_secret: String,   // encrypted at rest
    pub webhook_secret: Option<String>, // encrypted at rest
    pub is_active: bool,
    pub version: i32,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GatewaySettlementStatus {
    Pending,
    Reconciled,
    Exception,
}

impl GatewaySettlementStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "RECONCILED" => GatewaySettlementStatus::Reconciled,
            "EXCEPTION" => GatewaySettlementStatus::Exception,
            _ => GatewaySettlementStatus::Pending,
        }
    }

    pub fn to_db_str(&self) -> &'static str {
        match self {
            GatewaySettlementStatus::Pending => "PENDING",
            GatewaySettlementStatus::Reconciled => "RECONCILED",
            GatewaySettlementStatus::Exception => "EXCEPTION",
        }
    }
}

/// Daily gateway settlement. Unique per (entity, gateway, date).
/// `settled_amount` must equal matched transactions minus gateway fees;
/// a variance flips the row to EXCEPTION.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySettlement {
    pub gateway_settlement_id: EntityId<GatewaySettlement>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub gateway_type: GatewayType,
    pub settlement_date: NaiveDate,
    pub settled_amount: Money,
    pub gateway_fee_amount: Money,
    pub matched_amount: Money,
    pub status: GatewaySettlementStatus,
    pub exception_reason: Option<String>,
    pub reconciled_by_id: Option<Uuid>,
    pub reconciled_at: Option<DateTime<Utc>>,
    pub version: i32,
    pub audit: AuditInfo,
}
