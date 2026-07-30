//! AR domain event handling.

use serde::Serialize;
use chrono::{DateTime, Utc};

/// AR-specific event payload for outbox serialization.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ArEventData {
    StudentFeeAssessed {
        student_fee_account_id: String,
        student_id: String,
        fee_structure_id: String,
        gross_fee: i64,
        net_payable: i64,
        occurred_at: DateTime<Utc>,
    },
    PaymentReceiptCreated {
        receipt_id: String,
        receipt_number: String,
        student_id: String,
        amount: i64,
        payment_mode: String,
        linked_journal_id: String,
        occurred_at: DateTime<Utc>,
    },
    ConcessionApproved {
        concession_id: String,
        student_id: String,
        amount: i64,
        approved_by: String,
        occurred_at: DateTime<Utc>,
    },
    ScholarshipApplied {
        scholarship_id: String,
        student_id: String,
        scheme_id: String,
        expected_amount: i64,
        occurred_at: DateTime<Utc>,
    },
    ScholarshipVerified {
        scholarship_id: String,
        verified_by: String,
        occurred_at: DateTime<Utc>,
    },
    ScholarshipDisbursed {
        scholarship_id: String,
        dbt_amount: i64,
        dbt_date: DateTime<Utc>,
        transaction_ref: String,
        occurred_at: DateTime<Utc>,
    },
    RefundInitiated {
        refund_id: String,
        amount: i64,
        refund_reason: String,
        occurred_at: DateTime<Utc>,
    },
    RefundProcessed {
        refund_id: String,
        reversal_journal_id: String,
        amount: i64,
        occurred_at: DateTime<Utc>,
    },
}

pub async fn handle_ar_event(_event: &sutra_events::ArEvent) {
    // Event handling logic — triggered by outbox processor
}
