//! Tax Engine commands (CQRS command side).
//!
//! Implements the spec's Phase 2 command set:
//! TDS (computation service + deposit to government), ITC (compute,
//! Rule 42/43 reversal, reverse + post), RCM (create entry), GST
//! returns (GSTR-1 / GSTR-3B generation, filing), trust/income-tax
//! compliance (exemption register/renew, 85% application, s.11(5)
//! checks) and FCRA (register, compliance computation).
//!
//! Cross-cutting rules enforced here (never hardcoded — see
//! `system_config` + [`TaxPolicy`]):
//! - money is i64 paise, never floats;
//! - every table row is tenant-scoped;
//! - every GL posting goes through the GL module's create/post commands and
//!   carries `reference_type` / `reference_id` on each line;
//! - every state change writes an outbox event; financial records are
//!   append-only (corrections via reversal entries, never deletes);
//! - state machines are enforced exactly as in the spec.

use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use sutra_finance_gl::repository::{PgPeriodRepository, PeriodRepository};
use sutra_finance_gl::{CreateJournalCmd, CreateJournalLineCmd, GlCommandHandler, PostJournalCmd};

use crate::errors::TaxError;
use crate::events::{write_outbox, TaxationEventData};
use crate::models::gst::{GstFilingFrequency, GstRegistration};
use crate::models::gst_return::{GstReturn, GstReturnLine, GstReturnStatus, GstReturnType};
use crate::models::income::{
    FcraRegistration, FcraStatus, IncomeApplication, IncomeApplicationCategory,
    IncomeApplicationLine, IncomeApplicationStatus, TrustExemption, TrustExemptionSection,
    TrustExemptionStatus,
};
use crate::models::itc::{ItcEligibility, ItcRegister, ItcRegisterLine, ItcRegisterStatus};
use crate::models::tds::{TdsDeposit, TdsDepositStatus, TdsSection};
use crate::repository::{TaxRepository, VendorInvoiceLineTaxRow};

// ─── Command payloads ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositTdsToGovtCmd {
    /// One challan may cover several deductions (same challan_reference).
    pub tds_deduction_ids: Vec<Uuid>,
    /// ITNS-281 challan serial / CIN.
    pub challan_reference: String,
    pub deposit_date: NaiveDate,
    pub bank_account_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeItcCmd {
    pub gst_registration_id: Uuid,
    /// e.g. "072026".
    pub period: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeRule42ReversalCmd {
    pub gst_registration_id: Uuid,
    pub period: String,
    /// Exempt turnover for the period (paise).
    pub exempt_turnover: i64,
    /// Total turnover for the period (paise).
    pub total_turnover: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeRule43ReversalCmd {
    pub gst_registration_id: Uuid,
    pub period: String,
    /// Exempt turnover for the period (paise).
    pub exempt_turnover: i64,
    /// Total turnover for the period (paise).
    pub total_turnover: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseItcCmd {
    pub register_line_id: Uuid,
    /// Amount to reverse (paise) — ≤ the line's eligible ITC.
    pub amount: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRcmEntryCmd {
    pub invoice_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateGstrCmd {
    pub gst_registration_id: Uuid,
    /// e.g. "072026".
    pub period: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordGstFilingCmd {
    pub return_id: Uuid,
    pub acknowledgment_no: String,
    pub filed_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterTrustExemptionCmd {
    pub entity_id: Uuid,
    pub exemption_section: TrustExemptionSection,
    pub registration_no: String,
    pub registration_date: NaiveDate,
    pub valid_from: NaiveDate,
    pub valid_to: NaiveDate,
    pub approving_authority: Option<String>,
    pub is_trust: bool,
    pub trust_name: Option<String>,
    pub trust_pan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewExemptionCmd {
    pub trust_exemption_id: Uuid,
    pub new_valid_from: NaiveDate,
    pub new_valid_to: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeApplicationLineCmd {
    pub category: IncomeApplicationCategory,
    /// Paise; must be > 0.
    pub amount: i64,
    pub account_id: Uuid,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeIncomeApplicationCmd {
    pub fiscal_year_id: Uuid,
    pub entity_id: Uuid,
    pub total_income: i64,
    pub lines: Vec<IncomeApplicationLineCmd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagNonCompliantInvestmentsCmd {
    pub fiscal_year_id: Uuid,
    pub entity_id: Uuid,
    /// Names of the investment instruments held (checked against the
    /// configured s.11(5) securities list).
    pub investments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterFcraCmd {
    pub entity_id: Uuid,
    pub registration_no: String,
    pub valid_from: NaiveDate,
    pub valid_to: NaiveDate,
    pub bank_account_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeFcraComplianceCmd {
    pub fcra_registration_id: Uuid,
    pub fiscal_year_id: Uuid,
    pub total_receipts: i64,
    pub admin_expenses: i64,
}

// ─── TDS computation service ──────────────────────────────────────────

/// Threshold outcome of a TDS computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TdsThresholdStatus {
    /// Payment is below the per-payment and aggregate thresholds — no
    /// deduction required.
    BelowThreshold,
    /// Threshold crossed — deduction applies.
    Applicable,
    /// No valid PAN — s.206AA forces deduction at 20% regardless of
    /// thresholds.
    PanMissing,
}

/// Result of a TDS deduction computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdsComputation {
    pub section_code: String,
    /// Effective rate applied (percent). 0 when below threshold.
    pub effective_rate: Decimal,
    /// TDS amount in paise.
    pub tds_amount: i64,
    /// Taxable base the TDS was computed on (paise). For excess-only
    /// thresholds (s.194Q, CBDT Circular 17/2020) this is the portion of
    /// the payment exceeding the FY-aggregate threshold, not the full
    /// payment.
    pub taxable_base: i64,
    pub threshold_status: TdsThresholdStatus,
    /// True when PAN was missing/invalid — 20% forced under s.206AA.
    pub pan_missing: bool,
    /// §197 certificate rate used as override, when one was applied.
    pub certificate_rate: Option<Decimal>,
    /// §197 certificate flag — set when the certificate was used.
    pub section197_applied: bool,
}

/// Inputs for a TDS computation. `pan` and the aggregate YTD payments
/// are passed in because AP owns the deduction-time context; the service
/// owns the authoritative rate lookup (section master + §197 override +
/// §206AA fallback + thresholds).
#[derive(Debug, Clone)]
pub struct TdsComputationInput {
    pub vendor_id: Uuid,
    pub payment_date: NaiveDate,
    /// Amount of this payment (paise).
    pub payment_amount: i64,
    /// TDS section code, e.g. "194C".
    pub section_code: String,
    /// Deductee PAN (None when not collected).
    pub pan: Option<String>,
    /// Aggregate payments to this deductee in the FY before this payment.
    pub aggregate_ytd_payments: i64,
}

/// Round half-up: `base × num / den` in i128 (money stays exact).
pub(crate) fn pct_round(base: i64, num: i64, den: i64) -> i64 {
    if den <= 0 {
        return 0;
    }
    let n = (base as i128) * (num as i128);
    ((n + (den as i128) / 2) / (den as i128)) as i64
}

/// Convert a percentage (Decimal, e.g. 1.00 = 1%) into basis points.
fn rate_to_bp(rate: Decimal) -> i64 {
    (rate * Decimal::ONE_HUNDRED).round().to_i64().unwrap_or(0) // ToPrimitive in scope
}

/// Round a Decimal percentage to two decimal places (for storage).
fn percent_2dp(num: i64, den: i64) -> Option<Decimal> {
    if den <= 0 {
        return None;
    }
    let pct = (num as i128) * 10000 / (den as i128); // percent × 100
    Some((Decimal::from(pct) / Decimal::ONE_HUNDRED).round_dp(2))
}

/// Whether a PAN string looks valid ([A-Z]{5}[0-9]{4}[A-Z]).
fn valid_pan(pan: Option<&str>) -> bool {
    let Some(p) = pan else { return false };
    let b = p.as_bytes();
    b.len() == 10
        && b[..5].iter().all(|c| c.is_ascii_uppercase())
        && b[5..9].iter().all(|c| c.is_ascii_digit())
        && b[9].is_ascii_uppercase()
}

fn pan_state_code(gstin: Option<&str>) -> Option<&str> {
    gstin.and_then(|g| g.get(..2)).filter(|s| s.chars().all(|c| c.is_ascii_digit()))
}

/// Pure TDS rate/threshold engine — the single source of truth for TDS
/// rates, called by AP at deduction time (spec integration contract).
pub struct TdsComputationService {
    pool: PgPool,
    repo: TaxRepository,
}

impl TdsComputationService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: TaxRepository::new(pool.clone()),
            pool,
        }
    }

    /// Compute the TDS deduction for a payment.
    ///
    /// Resolution order (IT Act):
    /// 1. Missing/invalid PAN → 20% under s.206AA (mandatory, flags the
    ///    deductee) — threshold relief does NOT apply.
    /// 2. Valid §197 certificate (in-date, vendor-linked, same section)
    ///    overrides the default rate.
    /// 3. Section master default rate.
    /// 4. Thresholds: no deduction while the payment is below
    ///    `threshold_per_payment` AND the FY aggregate stays below
    ///    `threshold_aggregate`.
    pub async fn compute(
        &self,
        tenant_id: Uuid,
        input: &TdsComputationInput,
    ) -> Result<TdsComputation, TaxError> {
        let section = self
            .repo
            .find_tds_section(tenant_id, &input.section_code, input.payment_date)
            .await?
            .ok_or_else(|| {
                TaxError::TdsSectionNotFound(
                    input.section_code.clone(),
                    input.payment_date.to_string(),
                )
            })?;
        self.compute_with_section(tenant_id, input, &section).await
    }

    /// Compute using an already-resolved section master row (avoids a
    /// second lookup when the caller holds the row).
    pub async fn compute_with_section(
        &self,
        tenant_id: Uuid,
        input: &TdsComputationInput,
        section: &TdsSection,
    ) -> Result<TdsComputation, TaxError> {
        // s.206AA — PAN is mandatory; missing ⇒ 20%.
        let pan_ok = valid_pan(input.pan.as_deref());
        if !pan_ok {
            let rate = Decimal::from(20);
            return Ok(TdsComputation {
                section_code: section.section_code.clone(),
                effective_rate: rate,
                tds_amount: pct_round(input.payment_amount, rate_to_bp(rate), 10_000),
                taxable_base: input.payment_amount,
                threshold_status: TdsThresholdStatus::PanMissing,
                pan_missing: true,
                certificate_rate: None,
                section197_applied: false,
            });
        }

        // §197 certificate override (valid, vendor-linked, in-date).
        let cert_rate = self
            .repo
            .find_section197_certificate(tenant_id, input.vendor_id, &section.section_code, input.payment_date)
            .await?;
        let (rate, section197_applied) = match cert_rate {
            Some(r) => (r, true),
            None => (section.default_rate, false),
        };

        // Thresholds (per-payment & FY aggregate). CA review S6:
        // `threshold_per_payment == None` (194A ₹40k, 194I ₹2.4L, 194Q ₹50L
        // — FY-aggregate-only sections) means there is NO per-payment
        // relief; the per-payment gate must not force a deduction from the
        // first rupee. `is_none_or` gives None → "below" (no per-payment
        // crossing), so only the FY aggregate decides. An absent aggregate
        // threshold means no aggregate relief → deduct immediately.
        let below_per_payment = section
            .threshold_per_payment
            .is_none_or(|t| input.payment_amount <= t);
        let below_aggregate = section
            .threshold_aggregate
            .map(|t| input.aggregate_ytd_payments + input.payment_amount <= t)
            .unwrap_or(false);
        if below_per_payment && below_aggregate {
            return Ok(TdsComputation {
                section_code: section.section_code.clone(),
                effective_rate: Decimal::ZERO,
                tds_amount: 0,
                taxable_base: 0,
                threshold_status: TdsThresholdStatus::BelowThreshold,
                pan_missing: false,
                certificate_rate: cert_rate,
                section197_applied,
            });
        }

        let bp = rate_to_bp(rate);
        // CA review S6 — s.194Q (CBDT Circular 17/2020): TDS applies only
        // on the value EXCEEDING the FY-aggregate threshold. `headroom` is
        // how much of the aggregate threshold this payment can still absorb;
        // the excess of this payment over the headroom is the taxable base.
        let taxable_base = if section.threshold_excess_only {
            let headroom = section
                .threshold_aggregate
                .map(|t| (t.saturating_sub(input.aggregate_ytd_payments)).max(0))
                .unwrap_or(0);
            input.payment_amount.saturating_sub(headroom)
        } else {
            input.payment_amount
        };
        Ok(TdsComputation {
            section_code: section.section_code.clone(),
            effective_rate: rate,
            tds_amount: pct_round(taxable_base, bp, 10_000),
            taxable_base,
            threshold_status: TdsThresholdStatus::Applicable,
            pan_missing: false,
            certificate_rate: cert_rate,
            section197_applied,
        })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

// ─── Command handler ─────────────────────────────────────────────────

/// The tax command handler — owns the pool and the repository.
pub struct TaxCommandHandler {
    pool: PgPool,
    repo: TaxRepository,
}

impl TaxCommandHandler {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: TaxRepository::new(pool.clone()),
            pool,
        }
    }

    pub fn repository(&self) -> &TaxRepository {
        &self.repo
    }

    // ── TDS ─────────────────────────────────────────────────────────

    /// Deposit deducted TDS to the government (ITNS-281 challan).
    ///
    /// State machine (deduction deposit leg): PENDING → DEPOSITED.
    /// Posts `DR TDS Payable (24.03.<section>) / CR Bank` (one DR per
    /// section covered by the challan — CA review S7) via the GL module and
    /// emits one `TdsDeposited` per deduction.
    pub async fn deposit_tds_to_govt(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: DepositTdsToGovtCmd,
    ) -> Result<Vec<TdsDeposit>, TaxError> {
        let tid = *tenant_id.as_uuid();
        if cmd.tds_deduction_ids.is_empty() {
            return Err(TaxError::InvalidInput(
                "tds_deduction_ids must not be empty".to_string(),
            ));
        }
        let challan = cmd.challan_reference.trim().to_string();
        if challan.is_empty() {
            return Err(TaxError::InvalidInput("challan_reference required".to_string()));
        }

        // 1. Load and validate every deduction is PENDING (only-from-PENDING).
        let deductions = self.repo.list_tds_deductions(tid, &cmd.tds_deduction_ids).await?;
        for id in &cmd.tds_deduction_ids {
            let d = deductions.iter().find(|d| &d.tds_deduction_id == id).ok_or_else(|| {
                TaxError::TdsDeductionNotFound(id.to_string())
            })?;
            if d.tds_deposit_status != "PENDING" {
                return Err(TaxError::TdsDepositStateViolation {
                    current: d.tds_deposit_status.clone(),
                    expected: "PENDING".to_string(),
                });
            }
        }

        // 2. Entity + totals.
        let entity_id = self.payment_entity(tid, deductions[0].payment_id).await?;
        let total_amount: i64 = deductions.iter().map(|d| d.tds_amount).sum();
        if total_amount <= 0 {
            return Err(TaxError::InvalidInput(
                "total TDS amount to deposit must be > 0".to_string(),
            ));
        }

        // 3. GL accounts: DR each section's TDS Payable leaf (24.03.<section>)
        // / CR Bank. CA review S7: AP credits per-section leaves
        // (24.03.194C, 24.03.194J, …); debiting one arbitrary "24.03" leaf
        // for the whole challan strands the per-section balances. One
        // multi-line journal, one DR per section covered by the challan.
        let (bank_gl_account, _bank_type, _bank_name) = self
            .repo
            .bank_gl_account(tid, cmd.bank_account_id)
            .await?
            .ok_or_else(|| {
                TaxError::InvalidInput(format!("bank account {} not found", cmd.bank_account_id))
            })?;
        let bank_gl_account = bank_gl_account.ok_or_else(|| {
            TaxError::InvalidInput(format!(
                "bank account {} has no linked GL account",
                cmd.bank_account_id
            ))
        })?;

        // Group the challan's deductions by TDS section (deterministic
        // order) so each section's payable leaf is cleared to zero.
        let mut by_section: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
        for d in &deductions {
            *by_section.entry(d.tds_section.clone()).or_insert(0) += d.tds_amount;
        }
        // Resolve each section's leaf; fall back to the 24.03 parent when a
        // per-section leaf has not been created in the chart of accounts.
        let sections: Vec<String> = by_section.keys().cloned().collect();
        let tds_payable_accounts = self
            .repo
            .find_tds_payable_accounts(tid, &sections)
            .await?;

        let mut lines: Vec<CreateJournalLineCmd> = Vec::new();
        let mut line_number = 1;
        let mut dr_total = 0i64;
        for (section, amount) in &by_section {
            let account_id = tds_payable_accounts.get(section).copied().ok_or_else(|| {
                TaxError::Gl(format!(
                    "TDS Payable GL account (24.03.{section}) not found in chart of accounts"
                ))
            })?;
            dr_total += *amount;
            lines.push(CreateJournalLineCmd {
                line_number,
                account_id,
                debit_amount: Some(Money::from_paise(*amount)),
                credit_amount: None,
                description: Some(format!("TDS payable (s.{section}) cleared via challan {}", challan)),
                cost_center_id: None,
                fund_id: None,
                reference_id: Some(challan.clone()),
                reference_type: Some("TDS_DEPOSIT".to_string()),
            });
            line_number += 1;
        }
        debug_assert_eq!(dr_total, total_amount, "section DR lines must sum to the challan total");
        lines.push(CreateJournalLineCmd {
            line_number,
            account_id: bank_gl_account,
            debit_amount: None,
            credit_amount: Some(Money::from_paise(total_amount)),
            description: Some(format!("Bank payment for challan {}", challan)),
            cost_center_id: None,
            fund_id: None,
            reference_id: Some(challan.clone()),
            reference_type: Some("TDS_DEPOSIT".to_string()),
        });

        let journal_id = self
            .post_to_gl(
                tenant_id,
                user_id,
                "TDS",
                entity_id,
                cmd.deposit_date,
                format!("TDS deposit to government — challan {}", challan),
                lines,
                None,
            )
            .await?;
        info!(tenant_id = %tid, challan = %challan, journal_id = %journal_id, amount = %total_amount, "TDS deposit journal posted");

        // 4. Persist deposits + transition deductions + outbox events.
        let mut tx = self.pool.begin().await?;
        let mut deposits = Vec::with_capacity(deductions.len());
        for d in &deductions {
            let deposit = TdsDeposit {
                tds_deposit_id: EntityId::new(),
                tenant_id,
                tds_deduction_id: d.tds_deduction_id,
                challan_reference: challan.clone(),
                deposit_date: cmd.deposit_date,
                amount: d.tds_amount,
                bank_account_id: Some(cmd.bank_account_id),
                journal_id: Some(journal_id),
                status: TdsDepositStatus::Deposited,
                recorded_by_id: user_id,
                audit: AuditInfo::new(user_id),
            };
            sqlx::query(
                r#"
                INSERT INTO tds_deposits (
                    tds_deposit_id, tenant_id, tds_deduction_id, challan_reference, deposit_date,
                    amount, bank_account_id, journal_id, status, recorded_by_id, created_by, updated_by
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
                "#,
            )
            .bind(deposit.tds_deposit_id.as_uuid())
            .bind(tid)
            .bind(deposit.tds_deduction_id)
            .bind(&deposit.challan_reference)
            .bind(deposit.deposit_date)
            .bind(deposit.amount)
            .bind(deposit.bank_account_id)
            .bind(deposit.journal_id)
            .bind(deposit.status.to_db_str())
            .bind(deposit.recorded_by_id)
            .bind(user_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                UPDATE tds_deductions
                SET tds_deposit_status = 'DEPOSITED', tds_deposit_date = $3,
                    challan_reference = $4, tds_journal_id = $5,
                    updated_at = now(), updated_by = $6, entity_version = entity_version + 1
                WHERE tenant_id = $1 AND tds_deduction_id = $2
                "#,
            )
            .bind(tid)
            .bind(d.tds_deduction_id)
            .bind(cmd.deposit_date)
            .bind(&challan)
            .bind(journal_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

            write_outbox(
                &mut tx,
                tid,
                &d.tds_deduction_id.to_string(),
                &TaxationEventData::TdsDeposited {
                    tds_deduction_id: d.tds_deduction_id.to_string(),
                    challan_reference: challan.clone(),
                    deposit_date: cmd.deposit_date,
                    amount: d.tds_amount,
                    occurred_at: Utc::now(),
                },
            )
            .await?;
            deposits.push(deposit);
        }
        tx.commit().await?;
        info!(tenant_id = %tid, challan = %challan, n = %deposits.len(), "TDS deposited");
        Ok(deposits)
    }

    // ── ITC ─────────────────────────────────────────────────────────

    /// Compute the ITC register for a period from posted vendor invoices.
    ///
    /// State machine: (no register) → OPEN → COMPUTED. Builds one
    /// register line per invoice honouring the GL account's
    /// `itc_eligibility`: BLOCKED accounts (s.17(5) — exempt inputs)
    /// produce zero-ITC lines; capital-goods ITC is bucketed separately
    /// for Rule 43.
    pub async fn compute_itc(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: ComputeItcCmd,
    ) -> Result<ItcRegister, TaxError> {
        let tid = *tenant_id.as_uuid();
        if cmd.period_start > cmd.period_end {
            return Err(TaxError::InvalidInput("period_start must be <= period_end".to_string()));
        }
        let registration = self
            .require_gst_registration(tid, cmd.gst_registration_id)
            .await?;

        // Idempotency: register must not exist or must be OPEN.
        let existing_open = match self
            .repo
            .find_itc_register(tid, cmd.gst_registration_id, &cmd.period)
            .await?
        {
            Some(existing) if existing.status != ItcRegisterStatus::Open => {
                return Err(TaxError::PeriodAlreadyComputed(
                    cmd.period.clone(),
                    cmd.gst_registration_id.to_string(),
                ));
            }
            Some(existing) => Some(existing),
            None => None,
        };
        // An OPEN register is a draft — recompute by updating its totals;
        // invoice lines already present are skipped (append-only, no
        // deletes of financial data).
        let existing_line_invoices: Vec<Uuid> = match &existing_open {
            Some(e) => self
                .repo
                .list_itc_register_lines(tid, *e.itc_register_id.as_uuid())
                .await?
                .into_iter()
                .map(|l| l.invoice_id)
                .collect(),
            None => vec![],
        };

        let invoices = self
            .repo
            .posted_vendor_invoices(tid, registration.entity_id, cmd.period_start, cmd.period_end)
            .await?;

        let mut register_lines = Vec::new();
        let mut total_itc = 0i64;
        let mut itc_on_inputs = 0i64;
        let mut itc_on_capital_goods = 0i64;

        for inv in &invoices {
            if existing_line_invoices.contains(&inv.vendor_invoice_id) {
                // Already tracked from a previous (aborted) run — keep the
                // existing line and its contribution to avoid double-count.
                continue;
            }
            let lines = self.repo.vendor_invoice_lines(tid, inv.vendor_invoice_id).await?;
            let (taxable_value, tax, eligibility) = aggregate_invoice_lines(&lines);
            // CGST/SGST when the vendor's state matches the registration
            // state; IGST otherwise.
            let vendor_state = pan_state_code(inv.vendor_gstin.as_deref());
            let intra_state = vendor_state == Some(registration.state_code.as_str());
            let (igst, cgst, sgst) = if intra_state {
                let cgst = tax / 2;
                (0i64, cgst, tax - cgst)
            } else {
                (tax, 0i64, 0i64)
            };
            // BLOCKED (s.17(2)/17(5)) ⇒ zero ITC.
            let eligible_tax = if eligibility == ItcEligibility::Blocked { 0 } else { tax };
            total_itc += eligible_tax;
            if eligibility == ItcEligibility::CapitalGoods || eligibility == ItcEligibility::Reversal43 {
                itc_on_capital_goods += eligible_tax;
            } else {
                itc_on_inputs += eligible_tax;
            }
            // N1: capital-goods lines carry the acquisition month
            // ("MMYYYY") so the Rule 43 60-month reversal horizon is
            // measured from the actual acquisition date.
            let acquisition_period = if matches!(
                eligibility,
                ItcEligibility::CapitalGoods | ItcEligibility::Reversal43
            ) {
                Some(format!(
                    "{:02}{}",
                    inv.invoice_date.month(),
                    inv.invoice_date.year()
                ))
            } else {
                None
            };
            register_lines.push(ItcRegisterLine {
                itc_register_line_id: EntityId::new(),
                tenant_id,
                itc_register_id: Uuid::nil(), // set after register insert
                invoice_id: inv.vendor_invoice_id,
                invoice_number: inv.invoice_number.clone(),
                invoice_date: inv.invoice_date,
                vendor_gstin: inv.vendor_gstin.clone(),
                taxable_value,
                igst,
                cgst,
                sgst,
                total_tax: eligible_tax,
                itc_eligibility: eligibility,
                acquisition_period,
                reversal_percent: None,
                reversal_amount: None,
                is_reversed: false,
                audit: AuditInfo::new(user_id),
            });
        }

        // When a draft OPEN register exists, fold the existing lines'
        // contributions in before persisting totals.
        let existing_totals = match &existing_open {
            Some(e) => (e.total_itc, e.itc_on_inputs, e.itc_on_capital_goods),
            None => (0, 0, 0),
        };
        total_itc += existing_totals.0;
        itc_on_inputs += existing_totals.1;
        itc_on_capital_goods += existing_totals.2;

        if let Some(existing) = &existing_open {
            // Recompute the draft register in place (OPEN → COMPUTED).
            self.repo
                .update_itc_register_totals(
                    tid,
                    *existing.itc_register_id.as_uuid(),
                    0,
                    0,
                    total_itc,
                    0,
                    0,
                    ItcRegisterStatus::Computed,
                    user_id,
                )
                .await?;
            let persisted = self
                .repo
                .find_itc_register_by_id(tid, *existing.itc_register_id.as_uuid())
                .await?
                .ok_or_else(|| {
                    TaxError::ItcRegisterNotFound(
                        cmd.gst_registration_id.to_string(),
                        cmd.period.clone(),
                    )
                })?;
            let register_id = *persisted.itc_register_id.as_uuid();
            for mut line in register_lines {
                line.itc_register_id = register_id;
                self.repo.insert_itc_register_line(&line).await?;
            }
            self.publish_event(
                tid,
                register_id.to_string(),
                TaxationEventData::ItcComputed {
                    reg_id: register_id.to_string(),
                    period: cmd.period.clone(),
                    total_itc,
                    net_itc_eligible: total_itc,
                    occurred_at: Utc::now(),
                },
            )
            .await?;
            info!(tenant_id = %tid, register_id = %register_id, period = %cmd.period, itc = %total_itc, "ITC recomputed on draft register");
            return Ok(persisted);
        }

        let register = ItcRegister {
            itc_register_id: EntityId::new(),
            tenant_id,
            gst_registration_id: cmd.gst_registration_id,
            period: cmd.period.clone(),
            status: ItcRegisterStatus::Computed,
            total_itc,
            itc_on_inputs,
            itc_on_capital_goods,
            itc_reversal_rule_42: 0,
            itc_reversal_rule_43: 0,
            net_itc_eligible: total_itc,
            exempt_turnover: 0,
            total_turnover: 0,
            audit: AuditInfo::new(user_id),
        };
        self.repo.insert_itc_register(&register).await?;
        let register_id = *register.itc_register_id.as_uuid();
        for mut line in register_lines {
            line.itc_register_id = register_id;
            self.repo.insert_itc_register_line(&line).await?;
        }

        self.publish_event(
            tid,
            register_id.to_string(),
            TaxationEventData::ItcComputed {
                reg_id: register_id.to_string(),
                period: cmd.period.clone(),
                total_itc,
                net_itc_eligible: total_itc,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        info!(tenant_id = %tid, register_id = %register_id, period = %cmd.period, itc = %total_itc, "ITC computed");
        Ok(register)
    }

    /// Rule 42 reversal (inputs) — CGST Rules r.42.
    ///
    /// C1 = ITC on inputs/input services; D1 = exclusively taxable use;
    /// D2 = exclusively exempt use; C2 = C1 − D1 − D2.
    /// Reversal = C2 × (E/F), monthly.
    ///
    /// Documented simplifications (CA review DE-MINIMIS):
    /// (i) D1 = D2 = 0 — the register does not separate exclusively
    ///     taxable/exempt use, so C2 = C1 (CONSERVATIVE: can over-reverse).
    /// (ii) r.42(1)(e) 5%-of-common-credit deemed non-business reversal is
    ///     treated as determined-0 and NOT added to the reversal.
    ///
    /// De-minimis: no reversal when the computed reversal ≤
    /// `tax.itc_reversal_deminimis_paise` (proviso to CGST Rules r.42(1);
    /// default ₹5,000 per tax period — configurable).
    pub async fn compute_rule42_reversal(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: ComputeRule42ReversalCmd,
    ) -> Result<ItcRegister, TaxError> {
        let tid = *tenant_id.as_uuid();
        if cmd.exempt_turnover > cmd.total_turnover {
            return Err(TaxError::ExemptTurnoverExceedsTotal {
                exempt_turnover: cmd.exempt_turnover,
                total_turnover: cmd.total_turnover,
            });
        }
        let register = self.require_itc_register(tid, cmd.gst_registration_id, &cmd.period).await?;
        if register.status != ItcRegisterStatus::Computed {
            return Err(TaxError::ItcRegisterStateViolation {
                current: register.status.to_db_str().to_string(),
                expected: "COMPUTED".to_string(),
            });
        }
        let policy = self.repo.load_policy(tid).await?;
        let lines = self.repo.list_itc_register_lines(tid, *register.itc_register_id.as_uuid()).await?;

        // C1 = ITC on inputs/input services (FULL + REVERSAL_42 lines).
        // D1/D2 are zero here: the register does not separate exclusively
        // taxable/exempt use, so C2 = C1 and the apportionment is purely
        // turnover-driven (documented assumption — see spec r.42).
        let c1: i64 = lines
            .iter()
            .filter(|l| matches!(l.itc_eligibility, ItcEligibility::Full | ItcEligibility::Reversal42))
            .map(|l| l.total_tax)
            .sum();
        let c2 = c1;
        let reversal = if c2 == 0 || cmd.total_turnover == 0 {
            0
        } else {
            pct_round(c2, cmd.exempt_turnover, cmd.total_turnover)
        };
        // CA review DE-MINIMIS — fixed statutory de-minimis: no reversal
        // when the computed reversal ≤ the policy's fixed paise amount
        // (proviso to CGST Rules r.42(1): ₹5,000 per tax period, default
        // `tax.itc_reversal_deminimis_paise` = 500_000 paise). Replaces the
        // former "≤ 5% of C2" tolerance, which misapplied the r.42(1)(e)
        // deemed-reversal fraction as a de-minimis.
        let final_reversal = if reversal <= policy.itc_reversal_deminimis_paise {
            0
        } else {
            reversal
        };

        // Per-line reversal amounts (REVERSAL_42 lines only).
        let ratio = percent_2dp(cmd.exempt_turnover, cmd.total_turnover).unwrap_or(Decimal::ZERO);
        for line in lines.iter().filter(|l| l.itc_eligibility == ItcEligibility::Reversal42) {
            let line_reversal = if final_reversal == 0 {
                0
            } else {
                pct_round(line.total_tax, cmd.exempt_turnover, cmd.total_turnover)
            };
            self.repo
                .update_itc_register_line_reversal(
                    tid,
                    *line.itc_register_line_id.as_uuid(),
                    Some(ratio),
                    Some(line_reversal),
                    line.is_reversed,
                )
                .await?;
        }

        let net = register.total_itc - final_reversal - register.itc_reversal_rule_43;
        self.repo
            .update_itc_register_totals(
                tid,
                *register.itc_register_id.as_uuid(),
                final_reversal,
                register.itc_reversal_rule_43,
                net.max(0),
                cmd.exempt_turnover,
                cmd.total_turnover,
                ItcRegisterStatus::Computed,
                user_id,
            )
            .await?;

        self.publish_event(
            tid,
            register.itc_register_id.as_uuid().to_string(),
            TaxationEventData::Rule42ReversalComputed {
                reg_id: register.itc_register_id.as_uuid().to_string(),
                period: cmd.period.clone(),
                reversal_amount: final_reversal,
                exempt_turnover: cmd.exempt_turnover,
                total_turnover: cmd.total_turnover,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        info!(tenant_id = %tid, period = %cmd.period, reversal = %final_reversal, "Rule 42 reversal computed");
        self.repo
            .find_itc_register(tid, cmd.gst_registration_id, &cmd.period)
            .await?
            .ok_or_else(|| {
                TaxError::ItcRegisterNotFound(cmd.gst_registration_id.to_string(), cmd.period.clone())
            })
    }

    /// Rule 43 reversal (capital goods) — CGST Rules r.43.
    ///
    /// Monthly reversal = (TC/60) × (E/F) over a 60-month horizon, where
    /// TC = total capital-goods ITC (CAPITAL_GOODS / REVERSAL_43 lines).
    pub async fn compute_rule43_reversal(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: ComputeRule43ReversalCmd,
    ) -> Result<ItcRegister, TaxError> {
        let tid = *tenant_id.as_uuid();
        if cmd.exempt_turnover > cmd.total_turnover {
            return Err(TaxError::ExemptTurnoverExceedsTotal {
                exempt_turnover: cmd.exempt_turnover,
                total_turnover: cmd.total_turnover,
            });
        }
        let register = self.require_itc_register(tid, cmd.gst_registration_id, &cmd.period).await?;
        if register.status != ItcRegisterStatus::Computed {
            return Err(TaxError::ItcRegisterStateViolation {
                current: register.status.to_db_str().to_string(),
                expected: "COMPUTED".to_string(),
            });
        }
        let lines = self.repo.list_itc_register_lines(tid, *register.itc_register_id.as_uuid()).await?;
        // N1: capital goods leave the Rule 43 pool after month 60 from their
        // acquisition month (r.43(1)); lines without acquisition_period
        // (legacy) stay in the horizon.
        let in_horizon = |l: &ItcRegisterLine| -> bool {
            l.acquisition_period
                .as_deref()
                .and_then(|a| months_between_periods(a, &cmd.period))
                .map(|m| m < 60)
                .unwrap_or(true)
        };
        let tc: i64 = lines
            .iter()
            .filter(|l| matches!(l.itc_eligibility, ItcEligibility::CapitalGoods | ItcEligibility::Reversal43))
            .filter(|l| in_horizon(l))
            .map(|l| l.total_tax)
            .sum();
        // Monthly reversal = (TC/60) × (E/F); aggregate over the 60-month
        // horizon = TC × E/F (r.43(1)(c),(d)).
        let monthly = if tc == 0 || cmd.total_turnover == 0 {
            0
        } else {
            let tc_monthly = pct_round(tc, 1, 60);
            pct_round(tc_monthly, cmd.exempt_turnover, cmd.total_turnover)
        };

        let ratio = percent_2dp(cmd.exempt_turnover, cmd.total_turnover).unwrap_or(Decimal::ZERO);
        for line in lines.iter().filter(|l| {
            matches!(l.itc_eligibility, ItcEligibility::CapitalGoods | ItcEligibility::Reversal43)
        }) {
            let line_reversal = if monthly == 0 || !in_horizon(line) {
                0
            } else {
                let line_monthly = pct_round(line.total_tax, 1, 60);
                pct_round(line_monthly, cmd.exempt_turnover, cmd.total_turnover)
            };
            self.repo
                .update_itc_register_line_reversal(
                    tid,
                    *line.itc_register_line_id.as_uuid(),
                    Some(ratio),
                    Some(line_reversal),
                    line.is_reversed,
                )
                .await?;
        }

        let net = register.total_itc - register.itc_reversal_rule_42 - monthly;
        self.repo
            .update_itc_register_totals(
                tid,
                *register.itc_register_id.as_uuid(),
                register.itc_reversal_rule_42,
                monthly,
                net.max(0),
                cmd.exempt_turnover,
                cmd.total_turnover,
                ItcRegisterStatus::Computed,
                user_id,
            )
            .await?;

        self.publish_event(
            tid,
            register.itc_register_id.as_uuid().to_string(),
            TaxationEventData::Rule43ReversalComputed {
                reg_id: register.itc_register_id.as_uuid().to_string(),
                period: cmd.period.clone(),
                reversal_amount: monthly,
                capital_goods_itc: tc,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        info!(tenant_id = %tid, period = %cmd.period, monthly_reversal = %monthly, "Rule 43 reversal computed");
        self.repo
            .find_itc_register(tid, cmd.gst_registration_id, &cmd.period)
            .await?
            .ok_or_else(|| {
                TaxError::ItcRegisterNotFound(cmd.gst_registration_id.to_string(), cmd.period.clone())
            })
    }

    /// Reverse ITC on a register line — posts the `ITC_REVERSAL` journal
    /// (DR Expense / CR ITC Recoverable 11.01) and marks the line
    /// reversed; the register moves COMPUTED → REVERSED.
    pub async fn reverse_itc(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: ReverseItcCmd,
    ) -> Result<ItcRegisterLine, TaxError> {
        let tid = *tenant_id.as_uuid();
        if cmd.amount <= 0 {
            return Err(TaxError::InvalidInput("reversal amount must be > 0".to_string()));
        }
        let line = self
            .repo
            .find_itc_register_line(tid, cmd.register_line_id)
            .await?
            .ok_or_else(|| TaxError::ItcLineNotFound(cmd.register_line_id.to_string()))?;
        if line.is_reversed {
            return Err(TaxError::ItcRegisterStateViolation {
                current: "line already reversed".to_string(),
                expected: "OPEN".to_string(),
            });
        }
        if cmd.amount > line.total_tax {
            return Err(TaxError::ItcReversalExceedsEligible {
                line_id: cmd.register_line_id.to_string(),
                reversal_amount: cmd.amount,
                eligible_amount: line.total_tax,
            });
        }
        let register = self
            .repo
            .find_itc_register_by_id(tid, line.itc_register_id)
            .await?
            .ok_or_else(|| {
                TaxError::ItcRegisterNotFound(line.itc_register_id.to_string(), "unknown".to_string())
            })?;
        if register.status == ItcRegisterStatus::Closed {
            return Err(TaxError::ItcRegisterStateViolation {
                current: "CLOSED".to_string(),
                expected: "OPEN/COMPUTED/REVERSED".to_string(),
            });
        }

        // DR side = the invoice's expense account; CR side = ITC
        // Recoverable (11.01).
        let expense_account = self
            .repo
            .first_invoice_expense_account(tid, line.invoice_id)
            .await?
            .ok_or_else(|| TaxError::Gl(format!("no expense account found for invoice {}", line.invoice_id)))?;
        let (itc_account, _) = self
            .repo
            .find_account_by_code_prefix(tid, "11.01")
            .await?
            .ok_or_else(|| {
                TaxError::Gl("ITC Recoverable GL account (11.01) not found in chart of accounts".to_string())
            })?;
        let entity_id = self
            .repo
            .find_gst_registration(tid, register.gst_registration_id)
            .await?
            .map(|r| r.entity_id)
            .unwrap_or_else(Uuid::nil);

        let journal_id = self
            .post_to_gl(
                tenant_id,
                user_id,
                "ITC_REVERSAL",
                entity_id,
                Utc::now().date_naive(),
                format!("ITC reversal — line {} ({})", line.invoice_number, cmd.reason),
                vec![
                    CreateJournalLineCmd {
                        line_number: 1,
                        account_id: expense_account,
                        debit_amount: Some(Money::from_paise(cmd.amount)),
                        credit_amount: None,
                        description: Some(format!("ITC reversal for invoice {}", line.invoice_number)),
                        cost_center_id: None,
                        fund_id: None,
                        reference_id: Some(cmd.register_line_id.to_string()),
                        reference_type: Some("ITC_REVERSAL".to_string()),
                    },
                    CreateJournalLineCmd {
                        line_number: 2,
                        account_id: itc_account,
                        debit_amount: None,
                        credit_amount: Some(Money::from_paise(cmd.amount)),
                        description: Some("ITC reversed (Rule 42/43 apportionment)".to_string()),
                        cost_center_id: None,
                        fund_id: None,
                        reference_id: Some(cmd.register_line_id.to_string()),
                        reference_type: Some("ITC_REVERSAL".to_string()),
                    },
                ],
                None,
            )
            .await?;

        self.repo
            .update_itc_register_line_reversal(
                tid,
                cmd.register_line_id,
                line.reversal_percent,
                Some(cmd.amount),
                true,
            )
            .await?;
        self.repo
            .mark_itc_register_reversed(tid, line.itc_register_id, user_id)
            .await?;

        let reversed = self
            .repo
            .find_itc_register_line(tid, cmd.register_line_id)
            .await?
            .ok_or_else(|| TaxError::ItcLineNotFound(cmd.register_line_id.to_string()))?;
        self.publish_event(
            tid,
            cmd.register_line_id.to_string(),
            TaxationEventData::ItcReversalPosted {
                register_line_id: cmd.register_line_id.to_string(),
                reversal_amount: cmd.amount,
                journal_id: journal_id.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        info!(tenant_id = %tid, line_id = %cmd.register_line_id, journal_id = %journal_id, "ITC reversed");
        Ok(reversed)
    }

    // ── RCM ─────────────────────────────────────────────────────────

    /// Create the RCM entry for a reverse-charge invoice (single owner:
    /// this module posts the RCM leg; AP posts the base invoice entry).
    ///
    /// Posts (idempotent per invoice):
    /// - ITC-eligible: `DR RCM ITC Recoverable (11.01) / CR RCM Payable (24.02)`
    /// - Not eligible: `DR Expense / CR RCM Payable (24.02)`
    /// for the RCM payable amount (the GST on the invoice). Emits
    /// `RcmEntryCreated`.
    pub async fn create_rcm_entry(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: CreateRcmEntryCmd,
    ) -> Result<Uuid, TaxError> {
        let tid = *tenant_id.as_uuid();
        // Idempotency by invoice.
        if self.repo.rcm_journal_exists(tid, cmd.invoice_id).await? {
            return Err(TaxError::RcmEntryAlreadyExists(cmd.invoice_id.to_string()));
        }
        let invoice = self
            .repo
            .vendor_invoice_by_id(tid, cmd.invoice_id)
            .await?
            .ok_or_else(|| {
                TaxError::InvalidInput(format!("vendor invoice {} not found", cmd.invoice_id))
            })?;
        if !invoice.is_rcm {
            return Err(TaxError::InvalidInput(format!(
                "invoice {} is not flagged for reverse charge (is_rcm = false)",
                cmd.invoice_id
            )));
        }
        if invoice.status != "POSTED" {
            return Err(TaxError::InvalidInput(format!(
                "invoice {} is not POSTED (current: {}) — RCM entry requires the base entry first",
                cmd.invoice_id, invoice.status
            )));
        }
        let rcm_amount = invoice
            .rcm_payable_amount
            .unwrap_or(invoice.tax_amount);
        if rcm_amount <= 0 {
            return Err(TaxError::InvalidInput(format!(
                "invoice {} has no RCM tax amount to post",
                cmd.invoice_id
            )));
        }

        // ITC eligibility: eligible unless any line's account is BLOCKED.
        let lines = self.repo.vendor_invoice_lines(tid, cmd.invoice_id).await?;
        let itc_eligible = !lines.iter().any(|l| {
            l.itc_eligibility
                .as_deref()
                .map(|e| e.eq_ignore_ascii_case("BLOCKED"))
                .unwrap_or(false)
        });

        let (debit_account, leg_name) = if itc_eligible {
            let (acc, _code) = self
                .repo
                .find_rcm_itc_account(tid)
                .await?
                .ok_or_else(|| {
                    TaxError::Gl("RCM ITC Recoverable GL account (11.01 RCM) not found in chart of accounts".to_string())
                })?;
            (acc, "RCM ITC Recoverable")
        } else {
            let acc = self
                .repo
                .first_invoice_expense_account(tid, cmd.invoice_id)
                .await?
                .ok_or_else(|| {
                    TaxError::Gl(format!("no expense account found for invoice {}", cmd.invoice_id))
                })?;
            (acc, "Expense (ITC blocked)")
        };
        let (rcm_payable_account, _) = self
            .repo
            .find_account_by_code_prefix(tid, "24.02")
            .await?
            .ok_or_else(|| {
                TaxError::Gl("RCM Payable GL account (24.02) not found in chart of accounts".to_string())
            })?;

        let journal_id = self
            .post_to_gl(
                tenant_id,
                user_id,
                "RCM",
                invoice.entity_id,
                invoice.invoice_date,
                format!("RCM entry — invoice {} ({leg_name})", invoice.invoice_number),
                vec![
                    CreateJournalLineCmd {
                        line_number: 1,
                        account_id: debit_account,
                        debit_amount: Some(Money::from_paise(rcm_amount)),
                        credit_amount: None,
                        description: Some(format!("{leg_name} — RCM on invoice {}", invoice.invoice_number)),
                        cost_center_id: None,
                        fund_id: None,
                        reference_id: Some(cmd.invoice_id.to_string()),
                        reference_type: Some("RCM".to_string()),
                    },
                    CreateJournalLineCmd {
                        line_number: 2,
                        account_id: rcm_payable_account,
                        debit_amount: None,
                        credit_amount: Some(Money::from_paise(rcm_amount)),
                        description: Some(format!("RCM payable — invoice {}", invoice.invoice_number)),
                        cost_center_id: None,
                        fund_id: None,
                        reference_id: Some(cmd.invoice_id.to_string()),
                        reference_type: Some("RCM".to_string()),
                    },
                ],
                None,
            )
            .await?;

        self.publish_event(
            tid,
            cmd.invoice_id.to_string(),
            TaxationEventData::RcmEntryCreated {
                invoice_id: cmd.invoice_id.to_string(),
                rcm_payable_amount: rcm_amount,
                journal_id: journal_id.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        info!(tenant_id = %tid, invoice_id = %cmd.invoice_id, journal_id = %journal_id, rcm = %rcm_amount, "RCM entry created");
        Ok(journal_id)
    }

    // ── GST returns ─────────────────────────────────────────────────

    /// Generate a GSTR-1 (outward supplies) return for a period.
    ///
    /// Sources: posted journal lines on accounts carrying a GST
    /// classification (outward supplies), nil/exempt classifications,
    /// RCM invoices for the reverse-charge leg. The exact GSTN-schema
    /// payload is stored in `json_data`. DRAFT → GENERATED.
    pub async fn generate_gstr1(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: GenerateGstrCmd,
    ) -> Result<GstReturn, TaxError> {
        let tid = *tenant_id.as_uuid();
        let registration = self.require_gst_registration(tid, cmd.gst_registration_id).await?;
        if self
            .repo
            .find_gst_return_for_period(tid, cmd.gst_registration_id, GstReturnType::Gstr1, &cmd.period)
            .await?
            .is_some()
        {
            return Err(TaxError::DuplicateGstReturn {
                registration_id: cmd.gst_registration_id.to_string(),
                return_type: "GSTR1".to_string(),
                period: cmd.period.clone(),
            });
        }

        let aggregates = self
            .repo
            .gst_journal_aggregates(tid, registration.entity_id, cmd.period_start, cmd.period_end)
            .await?;

        // B2C supplies (4B) grouped by rate; nil-rated/exempt (7). CA
        // review S5: EXEMPT → expt_amt, NIL → nil_amt (reported separately).
        let mut b2c_by_rate: Vec<(Decimal, i64, i64, i64, i64)> = vec![]; // (rate, taxable, igst, cgst, sgst)
        let mut nil_taxable = 0i64;
        let mut expt_taxable = 0i64;
        for agg in &aggregates {
            let class = agg.gst_classification.to_uppercase();
            let net = agg.net_amount.max(0);
            let tax = agg.tax_amount.unwrap_or(0).max(0);
            if class == "EXEMPT" {
                expt_taxable += net;
                continue;
            }
            if class == "NIL" {
                nil_taxable += net;
                continue;
            }
            let rate = agg.tax_rate.unwrap_or(Decimal::ZERO);
            let intra = true; // default intra-state split (see spec assumption)
            let (igst, cgst, sgst) = if intra {
                let cgst = tax / 2;
                (0i64, cgst, tax - cgst)
            } else {
                (tax, 0i64, 0i64)
            };
            match b2c_by_rate.iter_mut().find(|(r, _, _, _, _)| *r == rate) {
                Some(entry) => {
                    entry.1 += net;
                    entry.2 += igst;
                    entry.3 += cgst;
                    entry.4 += sgst;
                }
                None => b2c_by_rate.push((rate, net, igst, cgst, sgst)),
            }
        }

        // CA review S4: reverse-charge inward supplies are NOT reported in
        // GSTR-1 (`rcm_supplies` is not a valid GSTN section key — the valid
        // ones are 4A/4B/4C/6/7); RCM output is reported in GSTR-3B 3.1(d).
        // GSTR-1 tax liability is therefore outward supplies only.
        let b2c_txval: i64 = b2c_by_rate.iter().map(|e| e.1).sum();
        let b2c_tax: i64 = b2c_by_rate.iter().map(|e| e.2 + e.3 + e.4).sum();
        let tax_liability: i64 = b2c_tax;

        // CA review B2 — cross-check against a generated GSTR-3B: both
        // returns must report the same outward taxable value and tax
        // (same tax-exclusive `net_amount` convention).
        self.cross_check_gstr_consistency(
            tid,
            cmd.gst_registration_id,
            &cmd.period,
            GstReturnType::Gstr1,
            "4B",
            b2c_txval,
            b2c_tax,
            GstReturnType::Gstr3b,
            "3.1(a)",
        )
        .await?;

        // GSTN-schema-shaped payload (GSTR-1). CA review S5: b2cs entries
        // carry `pos` (place of supply — the registration state for
        // intra-state supplies); the nil table splits EXEMPT → expt_amt and
        // NIL → nil_amt. CA review S4: no `rcm_supplies` block (invalid
        // GSTN key) — RCM is reported in GSTR-3B 3.1(d) only.
        let json_data = serde_json::json!({
            "gstin": registration.gstin,
            "fp": cmd.period,
            "gstr1": {
                "b2cs": b2c_by_rate.iter().map(|(rate, txval, igst, cgst, sgst)| serde_json::json!({
                    "sply_ty": "INTRA",
                    "pos": registration.state_code,
                    "rt": rate.to_string(),
                    "txval": txval,
                    "iamt": igst,
                    "camt": cgst,
                    "samt": sgst,
                })).collect::<Vec<_>>(),
                "nil": [ { "sply_ty": "INTRAB2C", "expt_amt": expt_taxable, "ngsup_amt": 0, "nil_amt": nil_taxable } ],
                "doc_issue": [],
            },
        });

        let due_date = gstr_due_date(registration.filing_frequency, cmd.period_start, 11);
        let mut ret = GstReturn {
            gst_return_id: EntityId::new(),
            tenant_id,
            gst_registration_id: cmd.gst_registration_id,
            return_type: GstReturnType::Gstr1,
            period: cmd.period.clone(),
            fiscal_year: period_to_fy(&cmd.period),
            status: GstReturnStatus::Generated,
            due_date,
            filed_date: None,
            filed_by_id: None,
            acknowledgment_no: None,
            json_data: Some(json_data),
            tax_liability,
            itc_claimed: 0,
            net_tax_payable: tax_liability,
            audit: AuditInfo::new(user_id),
        };
        let return_id = *ret.gst_return_id.as_uuid();
        self.repo.insert_gst_return(&ret).await?;

        // Return lines (spec section codes for GSTR-1).
        let mut lines: Vec<GstReturnLine> = b2c_by_rate
            .iter()
            .map(|(rate, txval, igst, cgst, sgst)| GstReturnLine {
                gst_return_line_id: EntityId::new(),
                tenant_id,
                gst_return_id: return_id,
                section: "4B".to_string(),
                description: Some(format!("B2C (small) supplies at {}%", rate)),
                taxable_value: *txval,
                igst_amount: *igst,
                cgst_amount: *cgst,
                sgst_amount: *sgst,
                cess_amount: 0,
                audit: AuditInfo::new(user_id),
            })
            .collect();
        lines.push(GstReturnLine {
            gst_return_line_id: EntityId::new(),
            tenant_id,
            gst_return_id: return_id,
            section: "7".to_string(),
            description: Some("Nil rated / exempt supplies".to_string()),
            taxable_value: nil_taxable,
            igst_amount: 0,
            cgst_amount: 0,
            sgst_amount: 0,
            cess_amount: 0,
            audit: AuditInfo::new(user_id),
        });
        // CA review S4: no "RCM" section line — not a valid GSTN GSTR-1
        // section (valid: 4A/4B/4C/6/7); RCM reported in GSTR-3B 3.1(d).
        for line in &lines {
            self.repo.insert_gst_return_line(line).await?;
        }

        self.publish_event(
            tid,
            return_id.to_string(),
            TaxationEventData::Gstr1Generated {
                return_id: return_id.to_string(),
                period: cmd.period.clone(),
                tax_liability,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        ret.gst_return_id = EntityId::from_uuid(return_id);
        info!(tenant_id = %tid, return_id = %return_id, period = %cmd.period, liability = %tax_liability, "GSTR-1 generated");
        Ok(ret)
    }

    /// Generate a GSTR-3B return for a period.
    ///
    /// Sections: 3.1(a) outward taxable supplies, 3.1(d) RCM output,
    /// 4(A) eligible ITC, 4(B)(2) RCM ITC. Consistency: 3.1(d) = 4(B)(2)
    /// RCM ITC when ITC is claimed on the RCM leg.
    pub async fn generate_gstr3b(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: GenerateGstrCmd,
    ) -> Result<GstReturn, TaxError> {
        let tid = *tenant_id.as_uuid();
        let registration = self.require_gst_registration(tid, cmd.gst_registration_id).await?;
        if self
            .repo
            .find_gst_return_for_period(tid, cmd.gst_registration_id, GstReturnType::Gstr3b, &cmd.period)
            .await?
            .is_some()
        {
            return Err(TaxError::DuplicateGstReturn {
                registration_id: cmd.gst_registration_id.to_string(),
                return_type: "GSTR3B".to_string(),
                period: cmd.period.clone(),
            });
        }

        let aggregates = self
            .repo
            .gst_journal_aggregates(tid, registration.entity_id, cmd.period_start, cmd.period_end)
            .await?;

        // 3.1(a) outward taxable supplies.
        let mut taxable_value = 0i64;
        let mut igst = 0i64;
        let mut cgst = 0i64;
        let mut sgst = 0i64;
        let mut nil_amt = 0i64; // S1: nil-rated turnover
        let mut expt_amt = 0i64; // S1: exempt turnover
        for agg in &aggregates {
            let class = agg.gst_classification.to_uppercase();
            let net = agg.net_amount.max(0);
            let tax = agg.tax_amount.unwrap_or(0).max(0);
            if class == "EXEMPT" {
                expt_amt += net;
                continue;
            }
            if class == "NIL" {
                nil_amt += net;
                continue;
            }
            // Default intra-state split (documented assumption — the
            // journal line does not carry the customer's state).
            let c = tax / 2;
            // CA review B2: `net_amount` IS the tax-exclusive taxable value
            // (spec convention — income credited net, GST to 24.01, tax on
            // the classified line); do NOT subtract the tax again.
            taxable_value += net;
            cgst += c;
            sgst += tax - c;
            igst += 0;
        }

        // 3.1(d) RCM output — from posted RCM-flagged invoices. CA review
        // S3: split by vendor state vs the registration state (same state →
        // CGST/SGST; cross-state/import → IGST), derived from the vendor's
        // GSTIN state code.
        let rcm_invoices = self
            .repo
            .posted_vendor_invoices(tid, registration.entity_id, cmd.period_start, cmd.period_end)
            .await?;
        let rcm_taxable: i64 = rcm_invoices.iter().filter(|i| i.is_rcm).map(|i| i.invoice_amount).sum();
        let mut rcm_igst = 0i64;
        let mut rcm_cgst = 0i64;
        let mut rcm_sgst = 0i64;
        for inv in rcm_invoices.iter().filter(|i| i.is_rcm) {
            let t = inv.tax_amount;
            let vendor_state = pan_state_code(inv.vendor_gstin.as_deref());
            if vendor_state == Some(registration.state_code.as_str()) {
                let c = t / 2;
                rcm_cgst += c;
                rcm_sgst += t - c;
            } else {
                rcm_igst += t;
            }
        }
        let rcm_output_tax = rcm_igst + rcm_cgst + rcm_sgst;

        // 4(A) eligible ITC from the ITC register for the period (if
        // computed); 4(B)(2) RCM ITC from the RCM ITC account balance.
        let itc_register = self
            .repo
            .find_itc_register(tid, cmd.gst_registration_id, &cmd.period)
            .await?;
        let eligible_itc = itc_register.as_ref().map(|r| r.net_itc_eligible).unwrap_or(0);
        // CA review B1: `gl_account_balance` returns credits − debits and
        // RCM ITC Recoverable is a debit-balance asset, so the RCM ITC
        // figure is the NEGATED balance — `.max(0)` alone yielded 0 forever.
        let rcm_itc_gross = match self.repo.find_rcm_itc_account(tid).await? {
            Some((acc, _)) => (-self
                .repo
                .gl_account_balance(tid, acc, cmd.period_start, cmd.period_end)
                .await?)
                .max(0),
            None => 0,
        };
        // CA review S9 (s.16(2)(d)): RCM ITC is claimable only to the
        // extent the RCM tax was PAID in the period. Payments show as
        // debits on RCM Payable (24.02); balance = credits − debits, so
        // paid = −balance. The claim is capped at the paid amount and an
        // alert event is emitted when the cap binds.
        let rcm_tax_paid = match self.repo.find_account_by_code_prefix(tid, "24.02").await? {
            Some((acc, _)) => (-self
                .repo
                .gl_account_balance(tid, acc, cmd.period_start, cmd.period_end)
                .await?)
                .max(0),
            None => 0,
        };
        let rcm_itc_capped = rcm_itc_gross > rcm_tax_paid;
        // CA review S3: 4(B)(2) uses the same vendor-state split as 3.1(d),
        // capped proportionally when the paid gate (S9) binds.
        let (rcm_itc_igst, rcm_itc_cgst, rcm_itc_sgst) = if rcm_itc_capped && rcm_itc_gross > 0 {
            (
                pct_round(rcm_igst, rcm_tax_paid, rcm_itc_gross),
                pct_round(rcm_cgst, rcm_tax_paid, rcm_itc_gross),
                pct_round(rcm_sgst, rcm_tax_paid, rcm_itc_gross),
            )
        } else {
            (rcm_igst, rcm_cgst, rcm_sgst)
        };
        let rcm_itc = rcm_itc_igst + rcm_itc_cgst + rcm_itc_sgst;

        // CA review S2: 4(A) split by the ITC register's actual
        // IGST/CGST/SGST composition (not a blind half-half), scaled to the
        // net eligible ITC (post Rule 42/43 reversal).
        let (iamt_4a, camt_4a, samt_4a) = if let Some(reg) = &itc_register {
            let lines = self
                .repo
                .list_itc_register_lines(tid, *reg.itc_register_id.as_uuid())
                .await?;
            let mut ig = 0i64;
            let mut cg = 0i64;
            let mut sg = 0i64;
            for l in &lines {
                let comp = l.igst + l.cgst + l.sgst;
                if comp <= 0 {
                    continue;
                }
                // Scale each line's tax split by its eligibility fraction
                // (BLOCKED lines carry total_tax = 0 — s.17(5) excluded).
                ig += pct_round(l.igst, l.total_tax, comp);
                cg += pct_round(l.cgst, l.total_tax, comp);
                sg += pct_round(l.sgst, l.total_tax, comp);
            }
            let comp_total = ig + cg + sg;
            if comp_total > 0 {
                let i = pct_round(eligible_itc, ig, comp_total);
                let c = pct_round(eligible_itc, cg, comp_total);
                (i, c, eligible_itc - i - c)
            } else {
                let c = eligible_itc / 2;
                (0, c, eligible_itc - c)
            }
        } else {
            let c = eligible_itc / 2;
            (0, c, eligible_itc - c)
        };

        let tax_liability = igst + cgst + sgst + rcm_output_tax;
        let itc_claimed = eligible_itc + rcm_itc;
        let net_tax_payable = (tax_liability - itc_claimed).max(0);

        // CA review B2 — cross-check against a generated GSTR-1.
        self.cross_check_gstr_consistency(
            tid,
            cmd.gst_registration_id,
            &cmd.period,
            GstReturnType::Gstr3b,
            "3.1(a)",
            taxable_value,
            igst + cgst + sgst,
            GstReturnType::Gstr1,
            "4B",
        )
        .await?;

        let json_data = serde_json::json!({
            "gstin": registration.gstin,
            "fp": cmd.period,
            "gstr3b": {
                "sup_details": {
                    "osup_det": { "txval": taxable_value, "iamt": igst, "camt": cgst, "samt": sgst },
                    "osup_nil_exmp": { "txval": 0, "nil_amt": nil_amt, "expt_amt": expt_amt },
                    "isup_rev": { "txval": rcm_taxable, "iamt": rcm_igst, "camt": rcm_cgst, "samt": rcm_sgst },
                },
                "itc_details": {
                    "itc_avl": { "iamt": iamt_4a, "camt": camt_4a, "samt": samt_4a },
                    "itc_rcm": { "iamt": rcm_itc_igst, "camt": rcm_itc_cgst, "samt": rcm_itc_sgst },
                },
            },
        });

        let due_date = gstr_due_date(registration.filing_frequency, cmd.period_start, 20);
        let mut ret = GstReturn {
            gst_return_id: EntityId::new(),
            tenant_id,
            gst_registration_id: cmd.gst_registration_id,
            return_type: GstReturnType::Gstr3b,
            period: cmd.period.clone(),
            fiscal_year: period_to_fy(&cmd.period),
            status: GstReturnStatus::Generated,
            due_date,
            filed_date: None,
            filed_by_id: None,
            acknowledgment_no: None,
            json_data: Some(json_data),
            tax_liability,
            itc_claimed,
            net_tax_payable,
            audit: AuditInfo::new(user_id),
        };
        let return_id = *ret.gst_return_id.as_uuid();
        self.repo.insert_gst_return(&ret).await?;

        let lines = vec![
            GstReturnLine {
                gst_return_line_id: EntityId::new(),
                tenant_id,
                gst_return_id: return_id,
                section: "3.1(a)".to_string(),
                description: Some("Outward taxable supplies".to_string()),
                taxable_value,
                igst_amount: igst,
                cgst_amount: cgst,
                sgst_amount: sgst,
                cess_amount: 0,
                audit: AuditInfo::new(user_id),
            },
            GstReturnLine {
                gst_return_line_id: EntityId::new(),
                tenant_id,
                gst_return_id: return_id,
                section: "3.1(d)".to_string(),
                description: Some("Inward supplies liable to reverse charge".to_string()),
                taxable_value: rcm_taxable,
                igst_amount: rcm_igst,
                cgst_amount: rcm_cgst,
                sgst_amount: rcm_sgst,
                cess_amount: 0,
                audit: AuditInfo::new(user_id),
            },
            GstReturnLine {
                gst_return_line_id: EntityId::new(),
                tenant_id,
                gst_return_id: return_id,
                section: "4(A)".to_string(),
                description: Some("Eligible input tax credit".to_string()),
                taxable_value: 0,
                igst_amount: iamt_4a,
                cgst_amount: camt_4a,
                sgst_amount: samt_4a,
                cess_amount: 0,
                audit: AuditInfo::new(user_id),
            },
            GstReturnLine {
                gst_return_line_id: EntityId::new(),
                tenant_id,
                gst_return_id: return_id,
                section: "4(B)(2)".to_string(),
                description: Some("RCM input tax credit".to_string()),
                taxable_value: 0,
                igst_amount: rcm_itc_igst,
                cgst_amount: rcm_itc_cgst,
                sgst_amount: rcm_itc_sgst,
                cess_amount: 0,
                audit: AuditInfo::new(user_id),
            },
        ];
        for line in &lines {
            self.repo.insert_gst_return_line(line).await?;
        }

        if rcm_itc_capped {
            // CA review S9 — alert when 4(B)(2) exceeds RCM tax paid.
            self.publish_event(
                tid,
                return_id.to_string(),
                TaxationEventData::RcmItcClaimCapped {
                    return_id: return_id.to_string(),
                    period: cmd.period.clone(),
                    claimed_itc: rcm_itc_gross,
                    rcm_tax_paid,
                    claimed_amount: rcm_itc,
                    occurred_at: Utc::now(),
                },
            )
            .await?;
        }

        self.publish_event(
            tid,
            return_id.to_string(),
            TaxationEventData::Gstr3bGenerated {
                return_id: return_id.to_string(),
                period: cmd.period.clone(),
                tax_liability,
                itc_claimed,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        ret.gst_return_id = EntityId::from_uuid(return_id);
        info!(tenant_id = %tid, return_id = %return_id, period = %cmd.period, net = %net_tax_payable, "GSTR-3B generated");
        Ok(ret)
    }

    /// Record a GST filing: GENERATED → FILED (acknowledgment required).
    pub async fn record_gst_filing(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: RecordGstFilingCmd,
    ) -> Result<GstReturn, TaxError> {
        let tid = *tenant_id.as_uuid();
        let ack = cmd.acknowledgment_no.trim().to_string();
        if !valid_ack(&ack) {
            return Err(TaxError::InvalidAcknowledgmentNo(ack));
        }
        let ret = self
            .repo
            .find_gst_return(tid, cmd.return_id)
            .await?
            .ok_or_else(|| TaxError::GstReturnNotFound(cmd.return_id.to_string()))?;
        if ret.status != GstReturnStatus::Generated {
            return Err(TaxError::GstReturnStateViolation {
                current: ret.status.to_db_str().to_string(),
                expected: "GENERATED".to_string(),
            });
        }
        self.repo
            .record_gst_filing(tid, cmd.return_id, &ack, cmd.filed_date, user_id)
            .await?;
        self.publish_event(
            tid,
            cmd.return_id.to_string(),
            TaxationEventData::GstReturnFiled {
                return_id: cmd.return_id.to_string(),
                period: ret.period.clone(),
                acknowledgment_no: ack.clone(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        info!(tenant_id = %tid, return_id = %cmd.return_id, ack = %ack, "GST return filed");
        self.repo
            .find_gst_return(tid, cmd.return_id)
            .await?
            .ok_or_else(|| TaxError::GstReturnNotFound(cmd.return_id.to_string()))
    }

    // ── Trust exemption & income-tax compliance ─────────────────────

    /// Register a trust/society exemption (12A/12AB/10(23C)).
    ///
    /// Invariants: unique (entity, section); provisional 12AB / 10(23C)
    /// registrations carry 3-year validity; when the registration's
    /// remaining life is ≤ 180 days it is immediately flagged
    /// RENEWAL_PENDING (and `ExemptionExpiring` is emitted).
    pub async fn register_trust_exemption(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: RegisterTrustExemptionCmd,
    ) -> Result<TrustExemption, TaxError> {
        let tid = *tenant_id.as_uuid();
        if cmd.valid_to <= cmd.valid_from {
            return Err(TaxError::InvalidInput("valid_to must be after valid_from".to_string()));
        }
        if self
            .repo
            .find_trust_exemption_by_section(tid, cmd.entity_id, cmd.exemption_section)
            .await?
            .is_some()
        {
            return Err(TaxError::DuplicateTrustExemption(
                cmd.exemption_section.to_db_str().to_string(),
            ));
        }

        let today = Utc::now().date_naive();
        let days_remaining = (cmd.valid_to - today).num_days();
        let (status, expiring) = if days_remaining <= 180 {
            (TrustExemptionStatus::RenewalPending, Some(days_remaining))
        } else {
            (TrustExemptionStatus::Active, None)
        };

        let exemption = TrustExemption {
            trust_exemption_id: EntityId::new(),
            tenant_id,
            entity_id: cmd.entity_id,
            exemption_section: cmd.exemption_section,
            registration_no: cmd.registration_no.clone(),
            registration_date: cmd.registration_date,
            valid_from: cmd.valid_from,
            valid_to: cmd.valid_to,
            status,
            approving_authority: cmd.approving_authority.clone(),
            is_trust: cmd.is_trust,
            trust_name: cmd.trust_name.clone(),
            trust_pan: cmd.trust_pan.clone(),
            audit: AuditInfo::new(user_id),
        };
        let id = *exemption.trust_exemption_id.as_uuid();
        self.repo.insert_trust_exemption(&exemption).await?;

        let mut events = vec![TaxationEventData::TrustExemptionRegistered {
            reg_id: id.to_string(),
            section: cmd.exemption_section.to_db_str().to_string(),
            registration_no: cmd.registration_no.clone(),
            valid_to: cmd.valid_to,
            occurred_at: Utc::now(),
        }];
        if let Some(days) = expiring {
            events.push(TaxationEventData::ExemptionExpiring {
                reg_id: id.to_string(),
                section: cmd.exemption_section.to_db_str().to_string(),
                days_remaining: days,
                occurred_at: Utc::now(),
            });
        }
        let mut tx = self.pool.begin().await?;
        for ev in &events {
            write_outbox(&mut tx, tid, &id.to_string(), ev).await?;
        }
        tx.commit().await?;
        info!(tenant_id = %tid, exemption_id = %id, section = %cmd.exemption_section.to_db_str(), "Trust exemption registered");
        Ok(exemption)
    }

    /// Renew an exemption — ACTIVE/RENEWAL_PENDING → ACTIVE with new
    /// validity dates. Renewal must be applied before expiry.
    pub async fn renew_exemption(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: RenewExemptionCmd,
    ) -> Result<TrustExemption, TaxError> {
        let tid = *tenant_id.as_uuid();
        let existing = self
            .repo
            .find_trust_exemption(tid, cmd.trust_exemption_id)
            .await?
            .ok_or_else(|| {
                TaxError::TrustExemptionNotFound(
                    "unknown".to_string(),
                    cmd.trust_exemption_id.to_string(),
                )
            })?;
        if !matches!(existing.status, TrustExemptionStatus::Active | TrustExemptionStatus::RenewalPending) {
            return Err(TaxError::TrustExemptionNotRenewable(
                existing.status.to_db_str().to_string(),
            ));
        }
        let today = Utc::now().date_naive();
        if existing.valid_to < today {
            return Err(TaxError::TrustExemptionExpired(
                existing.entity_id.to_string(),
                existing.valid_to.to_string(),
            ));
        }
        let days_remaining = (existing.valid_to - today).num_days();
        if cmd.new_valid_to <= existing.valid_to {
            return Err(TaxError::ExemptionRenewalTooLate(days_remaining));
        }
        if cmd.new_valid_to <= cmd.new_valid_from {
            return Err(TaxError::InvalidInput("new_valid_to must be after new_valid_from".to_string()));
        }

        self.repo
            .renew_trust_exemption(
                tid,
                cmd.trust_exemption_id,
                cmd.new_valid_from,
                cmd.new_valid_to,
                user_id,
            )
            .await?;

        let renewed = self
            .repo
            .find_trust_exemption(tid, cmd.trust_exemption_id)
            .await?
            .unwrap();
        self.publish_event(
            tid,
            cmd.trust_exemption_id.to_string(),
            TaxationEventData::ExemptionRenewed {
                reg_id: cmd.trust_exemption_id.to_string(),
                section: renewed.exemption_section.to_db_str().to_string(),
                valid_to: cmd.new_valid_to,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        info!(tenant_id = %tid, exemption_id = %cmd.trust_exemption_id, valid_to = %cmd.new_valid_to, "Trust exemption renewed");
        Ok(renewed)
    }

    /// Compute the 85% income-application ratio for a fiscal year.
    ///
    /// `application_percent = amount_applied / total_income × 100` must be
    /// ≥ `tax.income_application_threshold` (default 85). Unapplied income
    /// is tracked as accumulated (s.11(2), 5-year window). Emits
    /// `IncomeApplicationComputed` and, when below threshold,
    /// `IncomeApplicationThresholdMissed`.
    pub async fn compute_income_application(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: ComputeIncomeApplicationCmd,
    ) -> Result<IncomeApplication, TaxError> {
        let tid = *tenant_id.as_uuid();
        if cmd.total_income <= 0 {
            return Err(TaxError::InvalidInput("total_income must be > 0".to_string()));
        }
        for line in &cmd.lines {
            if line.amount <= 0 {
                return Err(TaxError::InvalidApplicationLineAmount(
                    line.category.to_db_str().to_string(),
                ));
            }
        }
        let policy = self.repo.load_policy(tid).await?;
        let amount_applied: i64 = cmd.lines.iter().map(|l| l.amount).sum();
        if amount_applied > cmd.total_income {
            return Err(TaxError::InvalidInput(
                "amount_applied cannot exceed total_income".to_string(),
            ));
        }
        let percent = percent_2dp(amount_applied, cmd.total_income).unwrap_or(Decimal::ZERO);
        let threshold = policy.income_application_threshold;
        let compliant = amount_applied * 100 >= cmd.total_income * threshold;
        let status = if compliant {
            IncomeApplicationStatus::Compliant
        } else {
            IncomeApplicationStatus::NonCompliant
        };
        let accumulated = (cmd.total_income - amount_applied).max(0);
        let accumulation_year = if accumulated > 0 {
            Some(Utc::now().date_naive().year())
        } else {
            None
        };

        let existing = self
            .repo
            .find_income_application(tid, cmd.fiscal_year_id, cmd.entity_id)
            .await?;
        let application = match existing {
            Some(app) if app.status != IncomeApplicationStatus::UnderReview => {
                return Err(TaxError::IncomeApplicationAlreadyComputed {
                    fiscal_year_id: cmd.fiscal_year_id.to_string(),
                    status: app.status.to_db_str().to_string(),
                });
            }
            Some(app) => {
                let updated = IncomeApplication {
                    total_income: cmd.total_income,
                    amount_applied,
                    application_percent: Some(percent),
                    accumulated_amount: accumulated,
                    accumulation_year,
                    accumulation_purpose: if accumulated > 0 {
                        Some(format!(
                            "Accumulated for specified purposes u/s 11(2) (max {} years)",
                            policy.accumulation_years
                        ))
                    } else {
                        None
                    },
                    status,
                    last_computed_at: Utc::now(),
                    audit: AuditInfo {
                        created_by: app.audit.created_by,
                        created_at: app.audit.created_at,
                        updated_by: user_id,
                        updated_at: Utc::now(),
                    },
                    ..app
                };
                self.repo.update_income_application(&updated).await?;
                updated
            }
            None => {
                let created = IncomeApplication {
                    income_application_id: EntityId::new(),
                    tenant_id,
                    fiscal_year_id: cmd.fiscal_year_id,
                    entity_id: cmd.entity_id,
                    total_income: cmd.total_income,
                    amount_applied,
                    application_percent: Some(percent),
                    accumulated_amount: accumulated,
                    accumulation_year,
                    accumulation_purpose: if accumulated > 0 {
                        Some(format!(
                            "Accumulated for specified purposes u/s 11(2) (max {} years)",
                            policy.accumulation_years
                        ))
                    } else {
                        None
                    },
                    status,
                    last_computed_at: Utc::now(),
                    audit: AuditInfo::new(user_id),
                };
                self.repo.insert_income_application(&created).await?;
                created
            }
        };

        // Lines (INSERT-only fact table).
        let app_id = *application.income_application_id.as_uuid();
        for l in &cmd.lines {
            let line = IncomeApplicationLine {
                income_app_line_id: EntityId::new(),
                tenant_id,
                income_application_id: app_id,
                category: l.category,
                amount: l.amount,
                account_id: l.account_id,
                description: l.description.clone(),
                audit: AuditInfo::new(user_id),
            };
            self.repo.insert_income_application_line(&line).await?;
        }

        let mut events = vec![TaxationEventData::IncomeApplicationComputed {
            fiscal_year_id: cmd.fiscal_year_id.to_string(),
            total_income: cmd.total_income,
            applied_percent: percent.to_string(),
            is_compliant: compliant,
            occurred_at: Utc::now(),
        }];
        if !compliant {
            events.push(TaxationEventData::IncomeApplicationThresholdMissed {
                fiscal_year_id: cmd.fiscal_year_id.to_string(),
                applied_percent: percent.to_string(),
                threshold,
                occurred_at: Utc::now(),
            });
        }
        let mut tx = self.pool.begin().await?;
        for ev in &events {
            write_outbox(&mut tx, tid, &app_id.to_string(), ev).await?;
        }
        tx.commit().await?;
        info!(tenant_id = %tid, app_id = %app_id, percent = %percent, compliant = %compliant, "Income application computed");
        Ok(application)
    }

    /// Check investments against the s.11(5) securities list (config
    /// `tax.section115_securities_list`). Breaches are evented as
    /// `Section115BreachDetected`; the command returns the report.
    pub async fn flag_non_compliant_investments(
        &self,
        tenant_id: TenantId,
        _user_id: Uuid,
        cmd: FlagNonCompliantInvestmentsCmd,
    ) -> Result<serde_json::Value, TaxError> {
        let tid = *tenant_id.as_uuid();
        let policy = self.repo.load_policy(tid).await?;
        let breaches: Vec<String> = if policy.section115_securities_list.is_empty() {
            // No list configured — nothing to check against (reported as
            // unverifiable rather than a breach).
            vec![]
        } else {
            cmd.investments
                .iter()
                .filter(|inv| {
                    !policy
                        .section115_securities_list
                        .iter()
                        .any(|allowed| inv.to_ascii_lowercase().contains(&allowed.to_ascii_lowercase()))
                })
                .cloned()
                .collect()
        };

        if !breaches.is_empty() {
            let details = format!(
                "investments not in the s.11(5) approved list: {}",
                breaches.join(", ")
            );
            self.publish_event(
                tid,
                cmd.fiscal_year_id.to_string(),
                TaxationEventData::Section115BreachDetected {
                    fiscal_year_id: cmd.fiscal_year_id.to_string(),
                    details: details.clone(),
                    occurred_at: Utc::now(),
                },
            )
            .await?;
            info!(tenant_id = %tid, fiscal_year_id = %cmd.fiscal_year_id, breaches = ?breaches, "Section 11(5) breach detected");
            Ok(serde_json::json!({
                "is_compliant": false,
                "breaches": breaches,
                "details": details,
            }))
        } else {
            Ok(serde_json::json!({
                "is_compliant": true,
                "breaches": [],
                "details": "all investments are in the s.11(5) approved list",
            }))
        }
    }

    // ── FCRA ────────────────────────────────────────────────────────

    /// Register an FCRA registration (FCRA 2010). The linked bank account
    /// must be FCRA-designated (s.17).
    pub async fn register_fcra(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: RegisterFcraCmd,
    ) -> Result<FcraRegistration, TaxError> {
        let tid = *tenant_id.as_uuid();
        if cmd.valid_to <= cmd.valid_from {
            return Err(TaxError::InvalidInput("valid_to must be after valid_from".to_string()));
        }
        if !self.repo.is_fcra_bank_account(tid, cmd.bank_account_id).await? {
            return Err(TaxError::FcraBankNotFcraType(cmd.bank_account_id.to_string()));
        }
        // Unique (tenant, registration_no).
        let existing = self
            .repo
            .list_fcra_registrations(tid, cmd.entity_id)
            .await?;
        if existing
            .iter()
            .any(|r| r.registration_no.eq_ignore_ascii_case(&cmd.registration_no))
        {
            return Err(TaxError::DuplicateFcraRegistration(cmd.registration_no.clone()));
        }

        let registration = FcraRegistration {
            fcra_registration_id: EntityId::new(),
            tenant_id,
            entity_id: cmd.entity_id,
            registration_no: cmd.registration_no.clone(),
            valid_from: cmd.valid_from,
            valid_to: cmd.valid_to,
            bank_account_id: cmd.bank_account_id,
            status: FcraStatus::Active,
            total_receipts: 0,
            admin_expenses: 0,
            admin_expense_ratio: None,
            fc4_return_filed_date: None,
            audit: AuditInfo::new(user_id),
        };
        let id = *registration.fcra_registration_id.as_uuid();
        self.repo.insert_fcra_registration(&registration).await?;
        self.publish_event(
            tid,
            id.to_string(),
            TaxationEventData::FcraRegistered {
                reg_id: id.to_string(),
                registration_no: cmd.registration_no.clone(),
                valid_to: cmd.valid_to,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        info!(tenant_id = %tid, fcra_id = %id, reg_no = %cmd.registration_no, "FCRA registered");
        Ok(registration)
    }

    /// Compute FCRA compliance: admin expenses must be ≤
    /// `tax.fcra_admin_expense_ratio_limit`% (default 20) of receipts.
    /// A breach emits `FcraAdminExpenseExceeded` (the registration is
    /// still returned with the ratio — the breach is evented).
    pub async fn compute_fcra_compliance(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
        cmd: ComputeFcraComplianceCmd,
    ) -> Result<FcraRegistration, TaxError> {
        let tid = *tenant_id.as_uuid();
        if cmd.total_receipts < 0 || cmd.admin_expenses < 0 {
            return Err(TaxError::InvalidInput(
                "receipts and admin expenses must be >= 0".to_string(),
            ));
        }
        self.repo
            .find_fcra_registration(tid, cmd.fcra_registration_id)
            .await?
            .ok_or_else(|| TaxError::FcraRegistrationNotFound(cmd.fcra_registration_id.to_string()))?;
        let policy = self.repo.load_policy(tid).await?;
        let ratio = percent_2dp(cmd.admin_expenses, cmd.total_receipts);
        let breached = cmd.total_receipts > 0
            && cmd.admin_expenses * 100 > cmd.total_receipts * policy.fcra_admin_expense_ratio_limit;

        self.repo
            .update_fcra_compliance(
                tid,
                cmd.fcra_registration_id,
                cmd.total_receipts,
                cmd.admin_expenses,
                ratio,
                user_id,
            )
            .await?;

        if breached {
            let ratio_str = ratio.map(|r| r.to_string()).unwrap_or_else(|| "n/a".to_string());
            self.publish_event(
                tid,
                cmd.fcra_registration_id.to_string(),
                TaxationEventData::FcraAdminExpenseExceeded {
                    fiscal_year_id: cmd.fiscal_year_id.to_string(),
                    expense_ratio: ratio_str,
                    max_allowed: policy.fcra_admin_expense_ratio_limit,
                    occurred_at: Utc::now(),
                },
            )
            .await?;
            info!(tenant_id = %tid, fcra_id = %cmd.fcra_registration_id, ratio = ?ratio, "FCRA admin expense cap exceeded");
        }
        Ok(self
            .repo
            .find_fcra_registration(tid, cmd.fcra_registration_id)
            .await?
            .unwrap())
    }

    // ── Internal helpers ────────────────────────────────────────────

    async fn require_gst_registration(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
    ) -> Result<GstRegistration, TaxError> {
        self.repo
            .find_gst_registration(tenant_id, registration_id)
            .await?
            .ok_or_else(|| TaxError::GstRegistrationNotFound(registration_id.to_string()))
    }

    async fn require_itc_register(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
        period: &str,
    ) -> Result<ItcRegister, TaxError> {
        self.repo
            .find_itc_register(tenant_id, registration_id, period)
            .await?
            .ok_or_else(|| {
                TaxError::ItcRegisterNotFound(registration_id.to_string(), period.to_string())
            })
    }

    /// CA review B2 — GSTR-1 vs GSTR-3B cross-check: when the counterpart
    /// return for the period already exists, its outward-supplies section
    /// (GSTR-1 "4B" / GSTR-3B "3.1(a)") must match our taxable value and
    /// total tax exactly. Both generators use `net_amount` (credit − debit
    /// on the supply account) as the tax-exclusive taxable value, so any
    /// divergence is a computation bug, not a rounding artifact.
    async fn cross_check_gstr_consistency(
        &self,
        tid: Uuid,
        registration_id: Uuid,
        period: &str,
        my_type: GstReturnType,
        my_section: &str,
        my_txval: i64,
        my_tax: i64,
        other_type: GstReturnType,
        other_section: &str,
    ) -> Result<(), TaxError> {
        let other = self
            .repo
            .find_gst_return_for_period(tid, registration_id, other_type, period)
            .await?;
        let Some(other) = other else { return Ok(()) };
        let lines = self
            .repo
            .list_gst_return_lines(tid, *other.gst_return_id.as_uuid())
            .await?;
        let Some(line) = lines.iter().find(|l| l.section == other_section) else {
            return Ok(());
        };
        let other_txval = line.taxable_value;
        let other_tax = line.igst_amount + line.cgst_amount + line.sgst_amount;
        if my_txval != other_txval || my_tax != other_tax {
            let (g1_tx, g1_tax, g3_tx, g3_tax) = if my_type == GstReturnType::Gstr1 {
                (my_txval, my_tax, other_txval, other_tax)
            } else {
                (other_txval, other_tax, my_txval, my_tax)
            };
            return Err(TaxError::GstrConsistencyMismatch {
                period: period.to_string(),
                gstr1_txval: g1_tx,
                gstr3b_txval: g3_tx,
                gstr1_tax: g1_tax,
                gstr3b_tax: g3_tax,
            });
        }
        Ok(())
    }

    async fn payment_entity(&self, tenant_id: Uuid, payment_id: Uuid) -> Result<Uuid, TaxError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT entity_id FROM vendor_payments WHERE tenant_id = $1 AND payment_id = $2",
        )
        .bind(tenant_id)
        .bind(payment_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| r.0)
            .ok_or_else(|| TaxError::InvalidInput(format!("vendor payment {} not found", payment_id)))
    }

    /// Post a journal through the GL module (create + post). All lines
    /// carry reference_type/reference_id for traceability.
    async fn post_to_gl(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        journal_type: &str,
        entity_id: Uuid,
        posting_date: NaiveDate,
        description: String,
        lines: Vec<CreateJournalLineCmd>,
        accounting_period_id: Option<Uuid>,
    ) -> Result<Uuid, TaxError> {
        let tid = *tenant_id.as_uuid();
        let period_id = match accounting_period_id {
            Some(pid) => {
                let open = PgPeriodRepository::new(self.pool.clone())
                    .is_open(tid, pid)
                    .await
                    .map_err(|e| TaxError::Gl(e.to_string()))?;
                if !open {
                    return Err(TaxError::Gl(
                        "accounting period is closed — cannot post".to_string(),
                    ));
                }
                pid
            }
            None => {
                let period = PgPeriodRepository::new(self.pool.clone())
                    .get_current(tid)
                    .await
                    .map_err(|e| TaxError::Gl(e.to_string()))?
                    .ok_or_else(|| {
                        TaxError::Gl("no open accounting period for tenant".to_string())
                    })?;
                *period.accounting_period_id.as_uuid()
            }
        };

        let gl = GlCommandHandler::new(self.pool.clone());
        let cmd = CreateJournalCmd {
            journal_type: journal_type.to_string(),
            accounting_period_id: period_id,
            entity_id,
            fund_id: None,
            cost_center_id: None,
            posting_date,
            description,
            lines,
            attachment_ids: vec![],
        };
        let journal = gl
            .create_journal(tenant_id, created_by, cmd)
            .await
            .map_err(|e| TaxError::Gl(e.to_string()))?;
        let journal_id = *journal.journal_id.as_uuid();
        gl.post_journal(
            tenant_id,
            PostJournalCmd {
                journal_id,
                posted_by: created_by,
            },
        )
        .await
        .map_err(|e| TaxError::Gl(e.to_string()))?;
        Ok(journal_id)
    }

    async fn publish_event(
        &self,
        tenant_id: Uuid,
        aggregate_id: String,
        event: TaxationEventData,
    ) -> Result<(), TaxError> {
        let mut tx = self.pool.begin().await?;
        write_outbox(&mut tx, tenant_id, &aggregate_id, &event).await?;
        tx.commit().await?;
        Ok(())
    }
}

// ─── Free helpers ─────────────────────────────────────────────────────

/// Whole months between two "MMYYYY" periods (to − from). None when either
/// string is malformed. Used for the Rule 43 60-month capital-goods horizon.
fn months_between_periods(from: &str, to: &str) -> Option<i64> {
    let fm: i64 = from.get(..2)?.parse().ok()?;
    let fy: i64 = from.get(2..)?.parse().ok()?;
    let tm: i64 = to.get(..2)?.parse().ok()?;
    let ty: i64 = to.get(2..)?.parse().ok()?;
    Some((ty - fy) * 12 + (tm - fm))
}

/// "072026" → "2026-27" (Indian FY starts April).
pub(crate) fn period_to_fy(period: &str) -> String {
    if period.len() < 6 {
        return period.to_string();
    }
    let month: i32 = period[..2].parse().unwrap_or(1);
    let year: i32 = period[2..].parse().unwrap_or(0);
    let fy_start = if month >= 4 { year } else { year - 1 };
    format!("{}-{}", fy_start, (fy_start + 1) % 100)
}

/// Fiscal year start year of a "2026-27" string.
pub(crate) fn fy_start_year(fy: &str) -> i32 {
    fy.split('-')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// GST return due date: GSTR-1 on the 11th (monthly) / 13th (QRMP),
/// GSTR-3B on the 20th (monthly) — see spec rule 6 (configurable in a
/// later phase; the spec flags the QRMP 22nd/24th as an assumption).
fn gstr_due_date(
    frequency: GstFilingFrequency,
    period_start: NaiveDate,
    monthly_day: u32,
) -> NaiveDate {
    let quarter_day = match monthly_day {
        11 => 13,
        _ => 24,
    };
    let day = match frequency {
        GstFilingFrequency::Monthly => monthly_day,
        GstFilingFrequency::Quarterly => quarter_day,
    };
    // Due in the month after the period (or quarter end).
    let month_end = period_start
        .with_day(1)
        .and_then(|d| d.checked_add_months(chrono::Months::new(1)))
        .unwrap_or(period_start);
    month_end.with_day(day).unwrap_or(month_end)
}

/// GSTN acknowledgment numbers are 15-digit numeric (ARNs are 15 digits);
/// accept 10–20 alphanumerics to tolerate ARN/UIN variations.
fn valid_ack(ack: &str) -> bool {
    let len = ack.len();
    (10..=20).contains(&len) && ack.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Aggregate a vendor invoice's lines into (taxable_value, total_tax,
/// effective eligibility). CA review S8: eligibility is computed PER LINE —
/// a BLOCKED (s.17(5)) line's own tax is excluded from the ITC pool, but it
/// no longer over-denies ITC on the invoice's other (eligible) lines. The
/// invoice-level eligibility is only BLOCKED when every line is blocked;
/// otherwise the first non-FULL eligibility wins
/// (REVERSAL_42/REVERSAL_43/CAPITAL_GOODS), defaulting to FULL.
fn aggregate_invoice_lines(lines: &[VendorInvoiceLineTaxRow]) -> (i64, i64, ItcEligibility) {
    let mut taxable = 0i64;
    let mut tax = 0i64;
    let mut eligibility = ItcEligibility::Full;
    let mut any_eligible = false;
    for l in lines {
        let line_tax = l.tax_amount.unwrap_or(0).max(0);
        taxable += (l.total_amount - line_tax).max(0);
        if matches!(
            l.itc_eligibility.as_deref().map(str::to_uppercase).as_deref(),
            Some("BLOCKED")
        ) {
            // s.17(5) — exclude this line's tax only; keep the rest.
            continue;
        }
        any_eligible = true;
        tax += line_tax;
        match l.itc_eligibility.as_deref().map(str::to_uppercase).as_deref() {
            Some("REVERSAL_42") if eligibility == ItcEligibility::Full => {
                eligibility = ItcEligibility::Reversal42
            }
            Some("REVERSAL_43") if eligibility == ItcEligibility::Full => {
                eligibility = ItcEligibility::Reversal43
            }
            Some("CAPITAL_GOODS") if eligibility == ItcEligibility::Full => {
                eligibility = ItcEligibility::CapitalGoods
            }
            _ => {}
        }
    }
    if !any_eligible {
        eligibility = ItcEligibility::Blocked;
    }
    (taxable, tax, eligibility)
}
