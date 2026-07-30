//! Concession — fee waiver granted to a student.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

use super::fee_head::FeeHead;

/// The type of concession calculation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConcessionType {
    Percentage,
    FixedAmount,
    FullWaiver,
}

impl ConcessionType {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            ConcessionType::Percentage => "PERCENTAGE",
            ConcessionType::FixedAmount => "FIXED_AMOUNT",
            ConcessionType::FullWaiver => "FULL_WAIVER",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "PERCENTAGE" => ConcessionType::Percentage,
            "FIXED_AMOUNT" => ConcessionType::FixedAmount,
            "FULL_WAIVER" => ConcessionType::FullWaiver,
            _ => ConcessionType::Percentage,
        }
    }
}

/// Status of the concession.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConcessionStatus {
    Applied,
    Approved,
    Rejected,
    Expired,
}

impl ConcessionStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            ConcessionStatus::Applied => "APPLIED",
            ConcessionStatus::Approved => "APPROVED",
            ConcessionStatus::Rejected => "REJECTED",
            ConcessionStatus::Expired => "EXPIRED",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "APPLIED" => ConcessionStatus::Applied,
            "APPROVED" => ConcessionStatus::Approved,
            "REJECTED" => ConcessionStatus::Rejected,
            "EXPIRED" => ConcessionStatus::Expired,
            _ => ConcessionStatus::Applied,
        }
    }
}

/// A fee concession granted to a student.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concession {
    pub concession_id: EntityId<Concession>,
    pub tenant_id: TenantId,
    pub student_id: Uuid,
    pub student_fee_account_id: Option<EntityId<super::student_fee::StudentFeeAccount>>,
    pub fee_head_id: Option<EntityId<FeeHead>>,
    pub concession_type: ConcessionType,
    pub value: Decimal,
    pub calculated_amount: Money,
    pub reason: String,
    pub approved_by: Option<Uuid>,
    pub status: ConcessionStatus,
    pub audit: AuditInfo,
}
