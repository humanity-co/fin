//! StudentFeeAccount — aggregate root tracking a student's fee assessment,
//! installments, and transactions.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

use super::fee_structure::FeeStructure;

/// The student's fee account — created when fees are assessed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentFeeAccount {
    pub student_fee_account_id: EntityId<StudentFeeAccount>,
    pub tenant_id: TenantId,
    pub student_id: Uuid,
    pub fee_structure_id: EntityId<FeeStructure>,
    pub academic_year: String,
    pub gross_fee: Money,
    pub scholarship_expected: Money,
    pub concession_amount: Money,
    pub net_payable: Money,
    pub total_paid: Money,
    pub outstanding: Money,
    pub status: FeeAccountStatus,
    pub installments: Vec<FeeInstallment>,
    pub transactions: Vec<FeeTransaction>,
    pub audit: AuditInfo,
}

impl StudentFeeAccount {
    /// Recalculate net_payable and outstanding from constituent fields.
    pub fn recalculate(&mut self) {
        self.net_payable = self.gross_fee - self.scholarship_expected - self.concession_amount;
        self.outstanding = if self.net_payable > self.total_paid {
            self.net_payable - self.total_paid
        } else {
            Money::ZERO
        };
    }
}

/// An installment within a student's fee account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeInstallment {
    pub fee_installment_id: Uuid,
    pub student_fee_account_id: EntityId<StudentFeeAccount>,
    pub installment_number: i32,
    pub due_date: NaiveDate,
    pub amount: Money,
    pub paid_amount: Money,
    pub status: InstallmentStatus,
}

/// A financial transaction recorded against the student's fee account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeTransaction {
    pub fee_transaction_id: Uuid,
    pub student_fee_account_id: EntityId<StudentFeeAccount>,
    pub transaction_type: FeeTransactionType,
    pub amount: Money,
    pub payment_mode: Option<String>,
    pub receipt_number: Option<String>,
    pub gateway_transaction_id: Option<String>,
    pub linked_journal_id: Option<Uuid>,
    pub remarks: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeeAccountStatus {
    Pending,
    PartiallyPaid,
    Paid,
    Overdue,
    Waived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstallmentStatus {
    Pending,
    PartiallyPaid,
    Paid,
    Overdue,
    Waived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeeTransactionType {
    Payment,
    Reversal,
    Adjustment,
    ScholarshipCredit,
}
