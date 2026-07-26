//! PaymentReceipt — aggregate root for student fee payments.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReceipt {
    pub payment_receipt_id: EntityId<PaymentReceipt>,
    pub tenant_id: TenantId,
    pub entity_id: Uuid,
    pub receipt_number: String,
    pub student_id: Uuid,
    pub student_fee_account_id: Option<Uuid>,
    pub payment_mode: PaymentMode,
    pub payment_date: DateTime<Utc>,
    pub amount: Money,
    pub status: ReceiptStatus,
    pub gateway_payment_id: Option<String>,
    pub gateway_reference: Option<String>,
    pub bank_transaction_ref: Option<String>,
    pub cheque_number: Option<String>,
    pub cheque_date: Option<chrono::NaiveDate>,
    pub cleared_date: Option<chrono::NaiveDate>,
    pub remarks: Option<String>,
    pub received_by_id: Uuid,
    pub payment_journal_id: Option<Uuid>,
    pub version: i32,
    pub audit: AuditInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMode {
    Cash,
    Cheque,
    Dd,
    Neft,
    Rtgs,
    Imps,
    Upi,
    CreditCard,
    DebitCard,
    Pos,
    PaymentGateway,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReceiptStatus {
    Pending,
    Completed,
    Failed,
    Refunded,
    Cancelled,
    Uncleared,
    Bounced,
}
