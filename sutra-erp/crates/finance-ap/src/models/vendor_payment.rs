//! VendorPayment aggregate root with invoice allocations.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

/// Vendor Payment aggregate root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorPayment {
    pub payment_id: EntityId<VendorPayment>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub payment_number: String,
    pub vendor_id: Uuid,
    pub payment_type: PaymentType,
    pub payment_mode: PaymentMode,
    pub payment_date: NaiveDate,
    pub amount: Money,
    pub tds_amount: Money,
    pub net_amount: Money,
    pub status: VpStatus,
    pub bank_account_id: Option<Uuid>,
    pub bank_transaction_ref: Option<String>,
    pub cheque_number: Option<String>,
    pub cheque_date: Option<NaiveDate>,
    pub approved_by_id: Option<Uuid>,
    pub processed_by_id: Option<Uuid>,
    pub payment_journal_id: Option<Uuid>,
    pub remarks: Option<String>,
    pub allocations: Vec<PaymentAllocation>,
    pub audit: AuditInfo,
}

/// Allocation of a payment to a specific vendor invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAllocation {
    pub vendor_payment_alloc_id: Uuid,
    pub payment_id: EntityId<VendorPayment>,
    pub invoice_id: Uuid,
    pub allocated_amount: Money,
    pub tds_amount: Money,
}

/// TDS deduction record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdsDeduction {
    pub tds_deduction_id: EntityId<TdsDeduction>,
    pub tenant_id: TenantId,
    pub payment_id: EntityId<VendorPayment>,
    pub section: String,
    pub rate: rust_decimal::Decimal,
    pub tds_amount: Money,
    pub pan_of_deductee: Option<String>,
    pub section_197_certificate_id: Option<Uuid>,
    pub tds_deposit_status: TdsDepositStatus,
    pub tds_deposit_date: Option<NaiveDate>,
    pub tds_return_filed_date: Option<NaiveDate>,
    pub tds_journal_id: Option<Uuid>,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentType {
    VendorPayment,
    RcmPayment,
    TdsDeposit,
    Advance,
    Refund,
    Other,
}

impl PaymentType {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            PaymentType::VendorPayment => "VENDOR_PAYMENT",
            PaymentType::RcmPayment => "RCM_PAYMENT",
            PaymentType::TdsDeposit => "TDS_DEPOSIT",
            PaymentType::Advance => "ADVANCE",
            PaymentType::Refund => "REFUND",
            PaymentType::Other => "OTHER",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "VENDOR_PAYMENT" => PaymentType::VendorPayment,
            "RCM_PAYMENT" => PaymentType::RcmPayment,
            "TDS_DEPOSIT" => PaymentType::TdsDeposit,
            "ADVANCE" => PaymentType::Advance,
            "REFUND" => PaymentType::Refund,
            _ => PaymentType::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMode {
    Neft,
    Rtgs,
    Imps,
    Cheque,
    #[serde(rename = "DD")]
    Dd,
    Cash,
}

impl PaymentMode {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            PaymentMode::Neft => "NEFT",
            PaymentMode::Rtgs => "RTGS",
            PaymentMode::Imps => "IMPS",
            PaymentMode::Cheque => "CHEQUE",
            PaymentMode::Dd => "DD",
            PaymentMode::Cash => "CASH",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "NEFT" => PaymentMode::Neft,
            "RTGS" => PaymentMode::Rtgs,
            "IMPS" => PaymentMode::Imps,
            "CHEQUE" => PaymentMode::Cheque,
            "DD" => PaymentMode::Dd,
            _ => PaymentMode::Cash,
        }
    }
}

/// Payment lifecycle states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VpStatus {
    Initiated,
    Approved,
    Scheduled,
    Processed,
    Completed,
    Failed,
    Cancelled,
}

impl VpStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            VpStatus::Initiated => "INITIATED",
            VpStatus::Approved => "APPROVED",
            VpStatus::Scheduled => "SCHEDULED",
            VpStatus::Processed => "PROCESSED",
            VpStatus::Completed => "COMPLETED",
            VpStatus::Failed => "FAILED",
            VpStatus::Cancelled => "CANCELLED",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "APPROVED" => VpStatus::Approved,
            "SCHEDULED" => VpStatus::Scheduled,
            "PROCESSED" => VpStatus::Processed,
            "COMPLETED" => VpStatus::Completed,
            "FAILED" => VpStatus::Failed,
            "CANCELLED" => VpStatus::Cancelled,
            _ => VpStatus::Initiated,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TdsDepositStatus {
    Pending,
    Deposited,
    Filed,
}

impl TdsDepositStatus {
    pub fn to_db_str(&self) -> &'static str {
        match self {
            TdsDepositStatus::Pending => "PENDING",
            TdsDepositStatus::Deposited => "DEPOSITED",
            TdsDepositStatus::Filed => "FILED",
        }
    }
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "DEPOSITED" => TdsDepositStatus::Deposited,
            "FILED" => TdsDepositStatus::Filed,
            _ => TdsDepositStatus::Pending,
        }
    }
}
