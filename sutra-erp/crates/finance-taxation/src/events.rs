//! Tax Engine domain events (outbox payloads).
//!
//! These mirror the shared `sutra_events::TaxationEvent` enum. The tax
//! module writes its own `TaxationEventData` into the transactional outbox
//! (`event_outbox.event_payload`) with `aggregate_type = 'Taxation'`; the
//! outbox dispatcher serializes the shared enum for subscribers
//! (GL, AP, compliance calendar, budget/forecast).
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

/// Events published by the Tax Engine (GST, ITC, RCM, TDS, income tax).
///
/// `TdsDeducted` is published by AP on deduction using the shared enum —
/// the tax module subscribes to it; it is listed here for completeness of
/// the event_type mapping.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum TaxationEventData {
    // ── GST ─────────────────────────────────────────────────────────
    GstinRegistered {
        reg_id: String,
        entity_id: String,
        gstin: String,
        occurred_at: DateTime<Utc>,
    },
    GstRateUpdated {
        hsn_sac_code: String,
        rate: i64,
        effective_from: String,
        occurred_at: DateTime<Utc>,
    },
    Gstr1Generated {
        return_id: String,
        period: String,
        tax_liability: i64,
        occurred_at: DateTime<Utc>,
    },
    Gstr3bGenerated {
        return_id: String,
        period: String,
        tax_liability: i64,
        itc_claimed: i64,
        occurred_at: DateTime<Utc>,
    },
    /// Annual return (GSTR-9) — aggregated outward + inward supplies of
    /// the FY, generated from the FY's GSTR-1 / GSTR-3B data.
    Gstr9Generated {
        return_id: String,
        fiscal_year: String,
        tax_liability: i64,
        occurred_at: DateTime<Utc>,
    },
    /// Annual reconciliation statement (GSTR-9C) — turnover as per
    /// audited books vs as per returns, with the difference.
    Gstr9cGenerated {
        return_id: String,
        fiscal_year: String,
        turnover_as_per_books: i64,
        turnover_as_per_returns: i64,
        occurred_at: DateTime<Utc>,
    },
    GstReturnFiled {
        return_id: String,
        period: String,
        acknowledgment_no: String,
        occurred_at: DateTime<Utc>,
    },
    RcmEntryCreated {
        invoice_id: String,
        rcm_payable_amount: i64,
        journal_id: String,
        occurred_at: DateTime<Utc>,
    },
    /// s.16(2)(d) alert — RCM ITC claimed in GSTR-3B 4(B)(2) exceeds the
    /// RCM tax actually paid to the government in the period; the claim is
    /// capped at the paid amount until the credit is settled (CA review S9).
    RcmItcClaimCapped {
        return_id: String,
        period: String,
        claimed_itc: i64,
        rcm_tax_paid: i64,
        claimed_amount: i64,
        occurred_at: DateTime<Utc>,
    },
    // ── ITC register ────────────────────────────────────────────────
    ItcComputed {
        reg_id: String,
        period: String,
        total_itc: i64,
        net_itc_eligible: i64,
        occurred_at: DateTime<Utc>,
    },
    Rule42ReversalComputed {
        reg_id: String,
        period: String,
        reversal_amount: i64,
        exempt_turnover: i64,
        total_turnover: i64,
        occurred_at: DateTime<Utc>,
    },
    Rule43ReversalComputed {
        reg_id: String,
        period: String,
        reversal_amount: i64,
        capital_goods_itc: i64,
        occurred_at: DateTime<Utc>,
    },
    ItcReversalPosted {
        register_line_id: String,
        reversal_amount: i64,
        journal_id: String,
        occurred_at: DateTime<Utc>,
    },
    // ── TDS ─────────────────────────────────────────────────────────
    TdsSectionUpdated {
        section_code: String,
        rate: String,
        threshold: Option<i64>,
        occurred_at: DateTime<Utc>,
    },
    TdsDeposited {
        tds_deduction_id: String,
        challan_reference: String,
        deposit_date: NaiveDate,
        amount: i64,
        occurred_at: DateTime<Utc>,
    },
    TdsReturnGenerated {
        return_id: String,
        return_type: String,
        quarter: String,
        fiscal_year: String,
        total_deductions: i64,
        occurred_at: DateTime<Utc>,
    },
    TdsReturnFiled {
        return_id: String,
        acknowledgment_no: String,
        occurred_at: DateTime<Utc>,
    },
    Form16Generated {
        employee_id: String,
        fiscal_year: String,
        document_url: String,
        occurred_at: DateTime<Utc>,
    },
    Form16AGenerated {
        vendor_id: String,
        fiscal_year: String,
        document_url: String,
        occurred_at: DateTime<Utc>,
    },
    // ── Income tax compliance (trust/society) ───────────────────────
    TrustExemptionRegistered {
        reg_id: String,
        section: String,
        registration_no: String,
        valid_to: NaiveDate,
        occurred_at: DateTime<Utc>,
    },
    ExemptionRenewed {
        reg_id: String,
        section: String,
        valid_to: NaiveDate,
        occurred_at: DateTime<Utc>,
    },
    ExemptionExpiring {
        reg_id: String,
        section: String,
        days_remaining: i64,
        occurred_at: DateTime<Utc>,
    },
    IncomeApplicationComputed {
        fiscal_year_id: String,
        total_income: i64,
        applied_percent: String,
        is_compliant: bool,
        occurred_at: DateTime<Utc>,
    },
    IncomeApplicationThresholdMissed {
        fiscal_year_id: String,
        applied_percent: String,
        threshold: i64,
        occurred_at: DateTime<Utc>,
    },
    Section115BreachDetected {
        fiscal_year_id: String,
        details: String,
        occurred_at: DateTime<Utc>,
    },
    FcraRegistered {
        reg_id: String,
        registration_no: String,
        valid_to: NaiveDate,
        occurred_at: DateTime<Utc>,
    },
    FcraAdminExpenseExceeded {
        fiscal_year_id: String,
        expense_ratio: String,
        max_allowed: i64,
        occurred_at: DateTime<Utc>,
    },
    // ── Shared with AP (published by AP, subscribed by tax) ─────────
    TdsDeducted {
        tds_deduction_id: String,
        payment_id: String,
        tds_section: String,
        tds_amount: i64,
        occurred_at: DateTime<Utc>,
    },
}

/// Write a tax event into the transactional outbox within `tx`.
pub async fn write_outbox(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: uuid::Uuid,
    aggregate_id: &str,
    event: &TaxationEventData,
) -> Result<(), crate::errors::TaxError> {
    let payload = serde_json::to_value(event)
        .map_err(|e| crate::errors::TaxError::EventPublish(e.to_string()))?;
    let event_type = event_type_name(event);
    sqlx::query(
        r#"
        INSERT INTO event_outbox (
            outbox_id, tenant_id, aggregate_type, aggregate_id,
            event_type, event_payload, status, retry_count, max_retries, created_at
        ) VALUES ($1, $2, 'Taxation', $3, $4, $5, 'PENDING', 0, 5, now())
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

fn event_type_name(event: &TaxationEventData) -> &'static str {
    match event {
        TaxationEventData::GstinRegistered { .. } => "GstinRegistered",
        TaxationEventData::GstRateUpdated { .. } => "GstRateUpdated",
        TaxationEventData::Gstr1Generated { .. } => "Gstr1Generated",
        TaxationEventData::Gstr3bGenerated { .. } => "Gstr3bGenerated",
        TaxationEventData::Gstr9Generated { .. } => "Gstr9Generated",
        TaxationEventData::Gstr9cGenerated { .. } => "Gstr9cGenerated",
        TaxationEventData::GstReturnFiled { .. } => "GstReturnFiled",
        TaxationEventData::RcmEntryCreated { .. } => "RcmEntryCreated",
        TaxationEventData::RcmItcClaimCapped { .. } => "RcmItcClaimCapped",
        TaxationEventData::ItcComputed { .. } => "ItcComputed",
        TaxationEventData::Rule42ReversalComputed { .. } => "Rule42ReversalComputed",
        TaxationEventData::Rule43ReversalComputed { .. } => "Rule43ReversalComputed",
        TaxationEventData::ItcReversalPosted { .. } => "ItcReversalPosted",
        TaxationEventData::TdsSectionUpdated { .. } => "TdsSectionUpdated",
        TaxationEventData::TdsDeposited { .. } => "TdsDeposited",
        TaxationEventData::TdsReturnGenerated { .. } => "TdsReturnGenerated",
        TaxationEventData::TdsReturnFiled { .. } => "TdsReturnFiled",
        TaxationEventData::Form16Generated { .. } => "Form16Generated",
        TaxationEventData::Form16AGenerated { .. } => "Form16AGenerated",
        TaxationEventData::TrustExemptionRegistered { .. } => "TrustExemptionRegistered",
        TaxationEventData::ExemptionRenewed { .. } => "ExemptionRenewed",
        TaxationEventData::ExemptionExpiring { .. } => "ExemptionExpiring",
        TaxationEventData::IncomeApplicationComputed { .. } => "IncomeApplicationComputed",
        TaxationEventData::IncomeApplicationThresholdMissed { .. } => {
            "IncomeApplicationThresholdMissed"
        }
        TaxationEventData::Section115BreachDetected { .. } => "Section115BreachDetected",
        TaxationEventData::FcraRegistered { .. } => "FcraRegistered",
        TaxationEventData::FcraAdminExpenseExceeded { .. } => "FcraAdminExpenseExceeded",
        TaxationEventData::TdsDeducted { .. } => "TdsDeducted",
    }
}
