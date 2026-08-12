//! Treasury domain events (outbox payloads).
//!
//! These mirror the shared `sutra_events::TreasuryEvent` enum. The treasury
//! module writes its own `TreasuryEventData` into the transactional outbox
//! (`event_outbox.event_payload`) with `aggregate_type = 'Treasury'`; the
//! outbox dispatcher serializes the shared enum for subscribers.
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum TreasuryEventData {
    BankAccountCreated {
        bank_account_id: String,
        account_name: String,
        bank_name: String,
        account_type: String,
        entity_id: String,
        gl_account_id: String,
        occurred_at: DateTime<Utc>,
    },
    BankAccountDeactivated {
        bank_account_id: String,
        occurred_at: DateTime<Utc>,
    },
    SignatoryAdded {
        bank_account_id: String,
        user_id: String,
        occurred_at: DateTime<Utc>,
    },
    SignatoryRemoved {
        bank_account_id: String,
        user_id: String,
        occurred_at: DateTime<Utc>,
    },
    BankBalanceSynced {
        bank_account_id: String,
        balance: i64,
        synced_at: DateTime<Utc>,
        occurred_at: DateTime<Utc>,
    },
    MinimumBalanceAlert {
        bank_account_id: String,
        current_balance: i64,
        minimum_balance: i64,
        occurred_at: DateTime<Utc>,
    },
    BankStatementUploaded {
        reconciliation_id: String,
        line_count: i64,
        occurred_at: DateTime<Utc>,
    },
    AutoReconciliationCompleted {
        reconciliation_id: String,
        matched_count: i64,
        unmatched_count: i64,
        occurred_at: DateTime<Utc>,
    },
    LineMatched {
        reconciliation_id: String,
        statement_line_id: String,
        transaction_id: String,
        transaction_type: String,
        occurred_at: DateTime<Utc>,
    },
    LineUnmatched {
        reconciliation_id: String,
        statement_line_id: String,
        occurred_at: DateTime<Utc>,
    },
    ReconciliationCompleted {
        reconciliation_id: String,
        bank_account_id: String,
        period_id: String,
        completed_by: String,
        occurred_at: DateTime<Utc>,
    },
    ReconciliationVerified {
        reconciliation_id: String,
        verified_by: String,
        occurred_at: DateTime<Utc>,
    },
    BrsGenerated {
        reconciliation_id: String,
        document_url: String,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferInitiated {
        transfer_id: String,
        from_account: String,
        to_account: String,
        amount: i64,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferApproved {
        transfer_id: String,
        approved_by: String,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferProcessed {
        transfer_id: String,
        journal_id: String,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferCompleted {
        transfer_id: String,
        from_account: String,
        to_account: String,
        amount: i64,
        bank_reference: String,
        occurred_at: DateTime<Utc>,
    },
    InterBankTransferCancelled {
        transfer_id: String,
        reason: String,
        occurred_at: DateTime<Utc>,
    },
    PettyCashTopUp {
        fund_id: String,
        amount: i64,
        occurred_at: DateTime<Utc>,
    },
    PettyCashExpenseRecorded {
        fund_id: String,
        voucher_no: String,
        amount: i64,
        account_id: String,
        occurred_at: DateTime<Utc>,
    },
    GatewayConfigured {
        entity_id: String,
        gateway_type: String,
        occurred_at: DateTime<Utc>,
    },
    GatewaySettlementReconciled {
        entity_id: String,
        gateway_type: String,
        settlement_date: String,
        settled_amount: i64,
        fee_amount: i64,
        occurred_at: DateTime<Utc>,
    },
}

/// Write a treasury event into the transactional outbox within `tx`.
pub async fn write_outbox(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: uuid::Uuid,
    aggregate_id: &str,
    event: &TreasuryEventData,
) -> Result<(), crate::errors::TreasuryError> {
    let payload = serde_json::to_value(event)
        .map_err(|e| crate::errors::TreasuryError::EventPublish(e.to_string()))?;
    let event_type = event_type_name(event);
    sqlx::query(
        r#"
        INSERT INTO event_outbox (
            outbox_id, tenant_id, aggregate_type, aggregate_id,
            event_type, event_payload, status, retry_count, max_retries, created_at
        ) VALUES ($1, $2, 'Treasury', $3, $4, $5, 'PENDING', 0, 5, now())
        "#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(tenant_id)
    .bind(aggregate_id)
    .bind(event_type)
    .bind(&payload)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn event_type_name(event: &TreasuryEventData) -> &'static str {
    match event {
        TreasuryEventData::BankAccountCreated { .. } => "BankAccountCreated",
        TreasuryEventData::BankAccountDeactivated { .. } => "BankAccountDeactivated",
        TreasuryEventData::SignatoryAdded { .. } => "SignatoryAdded",
        TreasuryEventData::SignatoryRemoved { .. } => "SignatoryRemoved",
        TreasuryEventData::BankBalanceSynced { .. } => "BankBalanceSynced",
        TreasuryEventData::MinimumBalanceAlert { .. } => "MinimumBalanceAlert",
        TreasuryEventData::BankStatementUploaded { .. } => "BankStatementUploaded",
        TreasuryEventData::AutoReconciliationCompleted { .. } => "AutoReconciliationCompleted",
        TreasuryEventData::LineMatched { .. } => "LineMatched",
        TreasuryEventData::LineUnmatched { .. } => "LineUnmatched",
        TreasuryEventData::ReconciliationCompleted { .. } => "ReconciliationCompleted",
        TreasuryEventData::ReconciliationVerified { .. } => "ReconciliationVerified",
        TreasuryEventData::BrsGenerated { .. } => "BrsGenerated",
        TreasuryEventData::InterBankTransferInitiated { .. } => "InterBankTransferInitiated",
        TreasuryEventData::InterBankTransferApproved { .. } => "InterBankTransferApproved",
        TreasuryEventData::InterBankTransferProcessed { .. } => "InterBankTransferProcessed",
        TreasuryEventData::InterBankTransferCompleted { .. } => "InterBankTransferCompleted",
        TreasuryEventData::InterBankTransferCancelled { .. } => "InterBankTransferCancelled",
        TreasuryEventData::PettyCashTopUp { .. } => "PettyCashTopUp",
        TreasuryEventData::PettyCashExpenseRecorded { .. } => "PettyCashExpenseRecorded",
        TreasuryEventData::GatewayConfigured { .. } => "GatewayConfigured",
        TreasuryEventData::GatewaySettlementReconciled { .. } => "GatewaySettlementReconciled",
    }
}
