//! Tax Engine queries (CQRS read side).
//!
//! All queries are tenant-scoped and return read projections for the API
//! layer: TDS register & pending deposits, ITC register & FY summary, RCM
//! payable, GSTR-1/3B preview, GST liability summary, trust exemption,
//! income application & accumulated income, s.11(5) compliance and FCRA
//! status/compliance.

use chrono::NaiveDate;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::TaxError;
use crate::models::income::{FcraRegistration, IncomeApplication, IncomeApplicationLine, TrustExemption};
use crate::models::tds::TdsSection;
use crate::repository::{TdsDeductionRow, TdsRegisterBalanceRow, TaxRepository};

/// TDS register row — GL balance of one TDS Payable account (24.03.x),
/// with the section derived from the account-code suffix.
#[derive(Debug, Clone, Serialize)]
pub struct TdsRegisterRow {
    pub account_id: Uuid,
    pub account_code: String,
    pub account_name: String,
    /// Section inferred from the account code suffix (e.g. "24.03.194C").
    pub section: Option<String>,
    /// Credit − debit on the TDS Payable account within the period.
    pub balance_paise: i64,
}

/// ITC summary row — one period's totals, for the FY summary query.
#[derive(Debug, Clone, Serialize)]
pub struct ItcSummaryRow {
    pub period: String,
    pub status: String,
    pub total_itc: i64,
    pub itc_on_inputs: i64,
    pub itc_on_capital_goods: i64,
    pub itc_reversal_rule_42: i64,
    pub itc_reversal_rule_43: i64,
    pub net_itc_eligible: i64,
}

/// GST liability summary row — per return of a fiscal year.
#[derive(Debug, Clone, Serialize)]
pub struct GstLiabilitySummaryRow {
    pub gst_return_id: Uuid,
    pub return_type: String,
    pub period: String,
    pub status: String,
    pub tax_liability: i64,
    pub itc_claimed: i64,
    pub net_tax_payable: i64,
}

/// Income application read projection with its lines.
#[derive(Debug, Clone, Serialize)]
pub struct IncomeApplicationView {
    #[serde(flatten)]
    pub application: IncomeApplication,
    pub lines: Vec<IncomeApplicationLine>,
}

/// The tax query handler.
pub struct TaxQueryHandler {
    repo: TaxRepository,
}

impl TaxQueryHandler {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: TaxRepository::new(pool),
        }
    }

    pub fn repository(&self) -> &TaxRepository {
        &self.repo
    }

    // ── TDS ─────────────────────────────────────────────────────────

    /// TDS register — GL balance of the TDS Payable accounts (24.03) by
    /// section, within a period (spec `GetTdsRegister`).
    pub async fn get_tds_register(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<TdsRegisterRow>, TaxError> {
        let rows = self
            .repo
            .tds_register_balances(tenant_id, entity_id, from, to)
            .await?;
        Ok(rows.into_iter().map(register_row_to_row).collect())
    }

    /// Pending TDS deposits — deductions whose deposit leg is PENDING.
    pub async fn get_pending_tds_deposits(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
    ) -> Result<Vec<TdsDeductionRow>, TaxError> {
        self.repo.pending_tds_deposits(tenant_id, entity_id).await
    }

    /// TDS section master (effective as of a date).
    pub async fn get_tds_sections(
        &self,
        tenant_id: Uuid,
        as_of: NaiveDate,
    ) -> Result<Vec<TdsSection>, TaxError> {
        self.repo.list_tds_sections(tenant_id, as_of).await
    }

    // ── ITC ─────────────────────────────────────────────────────────

    /// ITC register for a registration/period, with its invoice lines.
    pub async fn get_itc_register(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
        period: &str,
    ) -> Result<Option<serde_json::Value>, TaxError> {
        let register = self
            .repo
            .find_itc_register(tenant_id, registration_id, period)
            .await?;
        match register {
            Some(r) => {
                let lines = self
                    .repo
                    .list_itc_register_lines(tenant_id, *r.itc_register_id.as_uuid())
                    .await?;
                Ok(Some(serde_json::json!({
                    "register": r,
                    "lines": lines,
                })))
            }
            None => Ok(None),
        }
    }

    /// ITC summary for a fiscal year — per-period totals (spec
    /// `GetItcSummary(fy)`). `fiscal_year` is "2026-27"; periods are
    /// matched by their FY start year.
    pub async fn get_itc_summary(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
        fiscal_year: &str,
    ) -> Result<Vec<ItcSummaryRow>, TaxError> {
        let registers = self.repo.list_itc_registers(tenant_id, registration_id).await?;
        let fy_start = crate::commands::fy_start_year(fiscal_year);
        Ok(registers
            .into_iter()
            .filter(|r| {
                let p = crate::commands::period_to_fy(&r.period);
                crate::commands::fy_start_year(&p) == fy_start
            })
            .map(|r| ItcSummaryRow {
                period: r.period.clone(),
                status: r.status.to_db_str().to_string(),
                total_itc: r.total_itc,
                itc_on_inputs: r.itc_on_inputs,
                itc_on_capital_goods: r.itc_on_capital_goods,
                itc_reversal_rule_42: r.itc_reversal_rule_42,
                itc_reversal_rule_43: r.itc_reversal_rule_43,
                net_itc_eligible: r.net_itc_eligible,
            })
            .collect())
    }

    // ── RCM ─────────────────────────────────────────────────────────

    /// RCM payable — credit balance of the RCM Payable accounts (24.02)
    /// within a period (spec `GetRcmPayable(period)`).
    pub async fn get_rcm_payable(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<TdsRegisterRow>, TaxError> {
        let rows = self
            .repo
            .tds_register_balances_for_prefix(tenant_id, entity_id, from, to, "24.02")
            .await?;
        Ok(rows.into_iter().map(register_row_to_row).collect())
    }

    // ── GST returns ─────────────────────────────────────────────────

    /// GSTR-1 / GSTR-3B preview — the stored return + its section lines.
    pub async fn get_gstr_preview(
        &self,
        tenant_id: Uuid,
        return_id: Uuid,
    ) -> Result<Option<serde_json::Value>, TaxError> {
        let ret = self.repo.find_gst_return(tenant_id, return_id).await?;
        match ret {
            Some(r) => {
                let lines = self.repo.list_gst_return_lines(tenant_id, return_id).await?;
                Ok(Some(serde_json::json!({
                    "return": r,
                    "lines": lines,
                })))
            }
            None => Ok(None),
        }
    }

    /// GST liability summary by fiscal year (spec
    /// `GetGstLiabilitySummary(fy)`).
    pub async fn get_gst_liability_summary(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
        fiscal_year: &str,
    ) -> Result<Vec<GstLiabilitySummaryRow>, TaxError> {
        let returns = self
            .repo
            .list_gst_returns(tenant_id, registration_id, Some(fiscal_year))
            .await?;
        Ok(returns
            .into_iter()
            .map(|r| GstLiabilitySummaryRow {
                gst_return_id: *r.gst_return_id.as_uuid(),
                return_type: r.return_type.to_db_str().to_string(),
                period: r.period.clone(),
                status: r.status.to_db_str().to_string(),
                tax_liability: r.tax_liability,
                itc_claimed: r.itc_claimed,
                net_tax_payable: r.net_tax_payable,
            })
            .collect())
    }

    // ── Trust exemption & income-tax compliance ─────────────────────

    pub async fn get_trust_exemption(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<TrustExemption>, TaxError> {
        self.repo.list_trust_exemptions(tenant_id, entity_id).await
    }

    /// Income application for a fiscal year, with its category lines.
    pub async fn get_income_application(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        fiscal_year_id: Uuid,
    ) -> Result<Option<IncomeApplicationView>, TaxError> {
        let app = self
            .repo
            .find_income_application(tenant_id, fiscal_year_id, entity_id)
            .await?;
        match app {
            Some(a) => {
                let lines = self
                    .repo
                    .list_income_application_lines(tenant_id, *a.income_application_id.as_uuid())
                    .await?;
                Ok(Some(IncomeApplicationView {
                    application: a,
                    lines,
                }))
            }
            None => Ok(None),
        }
    }

    /// Accumulated (unapplied) income across FYs — the s.11(2) pool.
    pub async fn get_accumulated_income(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
    ) -> Result<i64, TaxError> {
        self.repo.accumulated_income(tenant_id, entity_id).await
    }

    /// s.11(5) investment compliance — latest breach event (breaches are
    /// evented, not stored in a fact table) + the FY's income application.
    pub async fn get_section115_compliance(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        fiscal_year_id: Uuid,
    ) -> Result<serde_json::Value, TaxError> {
        let breach = self.repo.latest_section115_breach(tenant_id).await?;
        let application = self
            .repo
            .find_income_application(tenant_id, fiscal_year_id, entity_id)
            .await?;
        Ok(serde_json::json!({
            "is_compliant": breach.is_none(),
            "last_breach_event": breach,
            "income_application": application,
        }))
    }

    // ── FCRA ────────────────────────────────────────────────────────

    pub async fn get_fcra_status(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<FcraRegistration>, TaxError> {
        self.repo.list_fcra_registrations(tenant_id, entity_id).await
    }

    /// FCRA compliance for a registration — receipts, admin expenses and
    /// the ratio (must be ≤ `tax.fcra_admin_expense_ratio_limit`).
    pub async fn get_fcra_compliance(
        &self,
        tenant_id: Uuid,
        registration_id: Uuid,
    ) -> Result<Option<serde_json::Value>, TaxError> {
        let reg = self.repo.find_fcra_registration(tenant_id, registration_id).await?;
        match reg {
            Some(r) => Ok(Some(serde_json::json!({
                "registration": r,
                "admin_expense_ratio": r.admin_expense_ratio,
            }))),
            None => Ok(None),
        }
    }
}

/// Convert a raw register-balance row into the query projection with the
/// section inferred from the account code suffix.
fn register_row_to_row(row: TdsRegisterBalanceRow) -> TdsRegisterRow {
    let section = row
        .account_code
        .split('.')
        .last()
        .filter(|s| !s.is_empty() && s.len() <= 6)
        .map(|s| s.to_string());
    TdsRegisterRow {
        account_id: row.account_id,
        account_code: row.account_code,
        account_name: row.account_name,
        section,
        balance_paise: row.balance_paise,
    }
}
