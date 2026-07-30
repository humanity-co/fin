//! InstallmentPlan — defines how a fee structure is split into installments.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, TenantId};

/// A single installment within a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallmentSlot {
    pub number: i32,
    pub percentage: Decimal,
    pub due_date: NaiveDate,
}

/// An installment plan defines the number and timing of fee installments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallmentPlan {
    pub installment_plan_id: EntityId<InstallmentPlan>,
    pub tenant_id: TenantId,
    pub name: String,
    pub fee_structure_id: Option<EntityId<super::fee_structure::FeeStructure>>,
    pub slots: Vec<InstallmentSlot>,
    pub audit: AuditInfo,
}

impl InstallmentPlan {
    /// Validate that all slot percentages sum to 100.
    pub fn is_valid(&self) -> bool {
        let total: Decimal = self.slots.iter().map(|s| s.percentage).sum();
        total == Decimal::new(100, 0)
    }
}
