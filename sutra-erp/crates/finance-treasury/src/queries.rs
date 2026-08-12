//! Treasury & Banking queries (CQRS read side).
//!
//! All queries are tenant-scoped and return read projections for the
//! API layer: bank account register, cash position, reconciliation
//! state, BRS, Tally-style bank book, petty-cash register, gateway
//! settlements and uncleared cheques.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::TreasuryError;
use crate::models::bank_account::{BankAccount, BankSignatory};
use crate::models::gateway::{GatewaySettlement, PaymentGatewayConfig};
use crate::models::petty_cash::{PettyCashFund, PettyCashTransaction};
use crate::models::reconciliation::{
    BankReconciliation, BankStatementLine, BankTransaction, MatchStatus,
};
use crate::models::transfer::InterBankTransfer;
use crate::repository::TreasuryRepository;

/// Cash position row — one per bank account with its GL balance.
#[derive(Debug, Clone, Serialize)]
pub struct CashPositionRow {
    pub bank_account_id: Uuid,
    pub account_name: String,
    pub bank_name: String,
    pub account_type: String,
    pub entity_id: Option<Uuid>,
    pub gl_account_id: Option<Uuid>,
    pub gl_balance_paise: i64,
    pub minimum_balance_paise: i64,
    pub is_active: bool,
    pub is_fcra_account: bool,
}

/// Reconciliation summary for a fiscal year (per account).
#[derive(Debug, Clone, Serialize)]
pub struct ReconciliationSummaryRow {
    pub period_id: Uuid,
    pub statement_date: NaiveDate,
    pub status: String,
    pub opening_balance_paise: i64,
    pub closing_balance_paise: i64,
    pub matched_lines: i64,
    pub total_lines: i64,
    pub verified_by_id: Option<Uuid>,
}

/// The treasury query handler.
pub struct TreasuryQueryHandler {
    pool: PgPool,
    repo: TreasuryRepository,
}

impl TreasuryQueryHandler {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: TreasuryRepository::new(pool.clone()),
            pool,
        }
    }

    // ── Bank accounts ───────────────────────────────────────────────

    pub async fn get_bank_account(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<serde_json::Value, TreasuryError> {
        let account = self
            .repo
            .find_bank_account(tenant_id, bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(bank_account_id.to_string()))?;
        let gl_balance = match account.gl_account_id {
            Some(gl) => self.repo.gl_account_balance(tenant_id, gl).await?,
            None => 0,
        };
        let signatories = self.repo.list_signatories(tenant_id, bank_account_id).await?;
        Ok(serde_json::json!({
            "bank_account": account,
            "gl_balance_paise": gl_balance,
            "signatories": signatories,
        }))
    }

    pub async fn get_bank_accounts(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
        account_type: Option<&str>,
    ) -> Result<Vec<BankAccount>, TreasuryError> {
        self.repo
            .list_bank_accounts(tenant_id, entity_id, account_type)
            .await
    }

    /// The FCRA account (rule 2 — should be the SBI New Delhi Main Branch one).
    pub async fn get_fcra_account(&self, tenant_id: Uuid) -> Result<Option<BankAccount>, TreasuryError> {
        let accounts = self.repo.list_bank_accounts(tenant_id, None, Some("FCRA")).await?;
        Ok(accounts.into_iter().find(|a| a.is_fcra_account))
    }

    /// GRANT_SPECIFIC accounts (UGC separate-account rule).
    pub async fn get_grant_bank_accounts(
        &self,
        tenant_id: Uuid,
        fund_id: Option<Uuid>,
    ) -> Result<Vec<BankAccount>, TreasuryError> {
        let accounts = self
            .repo
            .list_bank_accounts(tenant_id, None, Some("GRANT_SPECIFIC"))
            .await?;
        Ok(match fund_id {
            Some(f) => accounts.into_iter().filter(|a| a.fund_id == Some(f)).collect(),
            None => accounts,
        })
    }

    /// Cash position dashboard (spec `GetBankBalanceSummary`).
    pub async fn get_bank_balance_summary(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
    ) -> Result<serde_json::Value, TreasuryError> {
        let accounts = self.repo.list_bank_accounts(tenant_id, entity_id, None).await?;
        let mut rows = Vec::with_capacity(accounts.len());
        let mut total = 0i64;
        for a in accounts {
            let gl_balance = match a.gl_account_id {
                Some(gl) => self.repo.gl_account_balance(tenant_id, gl).await?,
                None => 0,
            };
            total += gl_balance;
            rows.push(CashPositionRow {
                bank_account_id: *a.bank_account_id.as_uuid(),
                account_name: a.account_name,
                bank_name: a.bank_name,
                account_type: a.account_type.to_db_str().to_string(),
                entity_id: a.entity_id,
                gl_account_id: a.gl_account_id,
                gl_balance_paise: gl_balance,
                minimum_balance_paise: a.minimum_balance.as_paise(),
                is_active: a.is_active,
                is_fcra_account: a.is_fcra_account,
            });
        }
        Ok(serde_json::json!({
            "rows": rows,
            "total_balance_paise": total,
            "as_of": chrono::Utc::now().to_rfc3339(),
        }))
    }

    pub async fn list_signatories(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<Vec<BankSignatory>, TreasuryError> {
        self.repo.list_signatories(tenant_id, bank_account_id).await
    }

    // ── Reconciliations ─────────────────────────────────────────────

    pub async fn get_reconciliation(
        &self,
        tenant_id: Uuid,
        reconciliation_id: Uuid,
    ) -> Result<serde_json::Value, TreasuryError> {
        let rec = self
            .repo
            .find_reconciliation(tenant_id, reconciliation_id)
            .await?
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(reconciliation_id.to_string()))?;
        let lines = self.repo.list_statement_lines(tenant_id, reconciliation_id).await?;
        Ok(serde_json::json!({
            "reconciliation": rec,
            "statement_lines": lines,
            "line_count": lines.len(),
        }))
    }

    pub async fn get_reconciliations(
        &self,
        tenant_id: Uuid,
        bank_account_id: Option<Uuid>,
        status: Option<&str>,
    ) -> Result<Vec<BankReconciliation>, TreasuryError> {
        self.repo
            .list_reconciliations(tenant_id, bank_account_id, status)
            .await
    }

    /// Unmatched statement lines for an account + period (spec query).
    pub async fn get_unmatched_items(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        period_id: Uuid,
    ) -> Result<Vec<BankStatementLine>, TreasuryError> {
        let recs = self
            .repo
            .list_reconciliations(tenant_id, Some(bank_account_id), None)
            .await?;
        let mut out = Vec::new();
        for rec in recs {
            if rec.period_id != period_id {
                continue;
            }
            let lines = self.repo.list_statement_lines(tenant_id, *rec.bank_reconciliation_id.as_uuid()).await?;
            out.extend(
                lines
                    .into_iter()
                    .filter(|l| l.match_status == MatchStatus::Unmatched || l.match_status == MatchStatus::PartialMatch),
            );
        }
        Ok(out)
    }

    /// BRS view for an account + period (reads the completed reconciliation
    /// and re-derives the statement; the generation command returns the full
    /// BRS payload, this returns the persisted view).
    pub async fn get_brs(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        period_id: Uuid,
    ) -> Result<serde_json::Value, TreasuryError> {
        let rec = self
            .repo
            .list_reconciliations(tenant_id, Some(bank_account_id), None)
            .await?
            .into_iter()
            .find(|r| r.period_id == period_id)
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(period_id.to_string()))?;
        let rid = *rec.bank_reconciliation_id.as_uuid();
        let lines = self.repo.list_statement_lines(tenant_id, rid).await?;
        let matched = lines
            .iter()
            .filter(|l| {
                l.match_status == MatchStatus::Matched || l.match_status == MatchStatus::ManualMatch
            })
            .count() as i64;
        Ok(serde_json::json!({
            "reconciliation_id": rid,
            "bank_account_id": bank_account_id,
            "period_id": period_id,
            "statement_date": rec.statement_date.to_string(),
            "status": rec.status.to_db_str(),
            "opening_balance_paise": rec.opening_balance.as_paise(),
            "closing_balance_paise": rec.closing_balance.as_paise(),
            "matched_lines": matched,
            "total_lines": lines.len() as i64,
            "unmatched_lines": lines.len() as i64 - matched,
            "lines": lines,
        }))
    }

    /// Per-period reconciliation summary for a fiscal year (spec query).
    pub async fn get_reconciliation_summary(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ReconciliationSummaryRow>, TreasuryError> {
        let recs = self
            .repo
            .list_reconciliations(tenant_id, Some(bank_account_id), None)
            .await?;
        let mut out = Vec::new();
        for rec in recs {
            if rec.statement_date < from || rec.statement_date > to {
                continue;
            }
            let lines = self.repo.list_statement_lines(tenant_id, *rec.bank_reconciliation_id.as_uuid()).await?;
            let matched = lines
                .iter()
                .filter(|l| {
                    l.match_status == MatchStatus::Matched
                        || l.match_status == MatchStatus::ManualMatch
                })
                .count() as i64;
            out.push(ReconciliationSummaryRow {
                period_id: rec.period_id,
                statement_date: rec.statement_date,
                status: rec.status.to_db_str().to_string(),
                opening_balance_paise: rec.opening_balance.as_paise(),
                closing_balance_paise: rec.closing_balance.as_paise(),
                matched_lines: matched,
                total_lines: lines.len() as i64,
                verified_by_id: rec.verified_by_id,
            });
        }
        Ok(out)
    }

    // ── Bank book & transactions ────────────────────────────────────

    /// Tally-style Bank Book: all posted journal lines on the bank's GL
    /// leaf account with a running balance.
    pub async fn get_bank_book(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<serde_json::Value, TreasuryError> {
        let account = self
            .repo
            .find_bank_account(tenant_id, bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(bank_account_id.to_string()))?;
        let gl = account
            .gl_account_id
            .ok_or_else(|| TreasuryError::GlAccountNotFound(bank_account_id.to_string()))?;
        let rows = self.repo.bank_book(tenant_id, gl, from, to).await?;
        Ok(serde_json::json!({
            "bank_account_id": bank_account_id,
            "gl_account_id": gl,
            "account_name": account.account_name,
            "rows": rows,
            "closing_balance_paise": rows.last().map(|r| r.running_balance).unwrap_or(0),
        }))
    }

    pub async fn get_bank_transactions(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<BankTransaction>, TreasuryError> {
        let from = from.unwrap_or(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
        let to = to.unwrap_or(NaiveDate::from_ymd_opt(2100, 1, 1).unwrap());
        let rows = sqlx::query_as::<_, crate::repository::BankTransactionRow>(
            r#"
            SELECT bank_transaction_id, tenant_id, bank_account_id, transaction_date,
                   value_date, transaction_ref, description, debit_amount, credit_amount,
                   balance, reference_type, reference_id, is_reconciled, reconciled_at,
                   version, created_at, created_by
            FROM bank_transactions
            WHERE tenant_id = $1 AND bank_account_id = $2
              AND transaction_date BETWEEN $3 AND $4
            ORDER BY transaction_date, created_at
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_model()).collect())
    }

    // ── Transfers ───────────────────────────────────────────────────

    pub async fn get_transfer(
        &self,
        tenant_id: Uuid,
        transfer_id: Uuid,
    ) -> Result<InterBankTransfer, TreasuryError> {
        self.repo
            .find_transfer(tenant_id, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))
    }

    pub async fn get_transfers(
        &self,
        tenant_id: Uuid,
        from_account: Option<Uuid>,
        status: Option<&str>,
    ) -> Result<Vec<InterBankTransfer>, TreasuryError> {
        self.repo
            .list_transfers(tenant_id, from_account, status)
            .await
    }

    // ── Petty cash ──────────────────────────────────────────────────

    pub async fn get_petty_cash_fund(
        &self,
        tenant_id: Uuid,
        fund_id: Uuid,
    ) -> Result<PettyCashFund, TreasuryError> {
        self.repo
            .find_petty_cash_fund(tenant_id, fund_id)
            .await?
            .ok_or_else(|| TreasuryError::PettyCashFundNotFound(fund_id.to_string()))
    }

    pub async fn get_petty_cash_funds(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
    ) -> Result<Vec<PettyCashFund>, TreasuryError> {
        self.repo.list_petty_cash_funds(tenant_id, entity_id).await
    }

    /// Petty-cash register for a fund over a date range.
    pub async fn get_petty_cash_register(
        &self,
        tenant_id: Uuid,
        fund_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<serde_json::Value, TreasuryError> {
        let fund = self.get_petty_cash_fund(tenant_id, fund_id).await?;
        let txns = self
            .repo
            .list_petty_cash_txns(tenant_id, fund_id, from, to)
            .await?;
        let mut debit_total = 0i64; // money in (top-ups)
        let mut credit_total = 0i64; // money out (expenses)
        for t in &txns {
            match t.transaction_type {
                crate::models::petty_cash::PettyCashTransactionType::TopUp
                | crate::models::petty_cash::PettyCashTransactionType::Recoupment => {
                    debit_total += t.amount.as_paise();
                }
                _ => credit_total += t.amount.as_paise(),
            }
        }
        Ok(serde_json::json!({
            "fund": fund,
            "transactions": txns,
            "total_in_paise": debit_total,
            "total_out_paise": credit_total,
        }))
    }

    // ── Gateways ────────────────────────────────────────────────────

    pub async fn get_gateway_configs(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
    ) -> Result<Vec<serde_json::Value>, TreasuryError> {
        let configs = self.repo.list_gateway_configs(tenant_id, entity_id).await?;
        // Never leak secrets — mask them.
        Ok(configs.into_iter().map(mask_gateway_config).collect())
    }

    /// Pending (unreconciled) gateway settlements for an entity.
    pub async fn get_pending_gateway_settlements(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
    ) -> Result<Vec<GatewaySettlement>, TreasuryError> {
        self.repo
            .list_gateway_settlements(tenant_id, entity_id, Some("PENDING"))
            .await
    }

    pub async fn get_gateway_settlements(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
        status: Option<&str>,
    ) -> Result<Vec<GatewaySettlement>, TreasuryError> {
        self.repo
            .list_gateway_settlements(tenant_id, entity_id, status)
            .await
    }

    // ── Reports ─────────────────────────────────────────────────────

    /// Uncleared cheques/DDs from AR receipts (spec query).
    pub async fn get_uncleared_cheques(
        &self,
        tenant_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<serde_json::Value, TreasuryError> {
        let rows = self.repo.uncleared_cheques(tenant_id, from, to).await?;
        let items: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|(receipt_id, receipt_number, amount, date, cheque)| {
                serde_json::json!({
                    "receipt_id": receipt_id,
                    "receipt_number": receipt_number,
                    "cheque_number": cheque,
                    "amount_paise": amount,
                    "payment_date": date.to_string(),
                })
            })
            .collect();
        Ok(serde_json::json!({ "uncleared_cheques": items }))
    }

    /// Cash position dashboard endpoint (reports).
    pub async fn get_cash_position(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
    ) -> Result<serde_json::Value, TreasuryError> {
        let summary = self.get_bank_balance_summary(tenant_id, entity_id).await?;
        let funds = self.repo.list_petty_cash_funds(tenant_id, entity_id).await?;
        let petty_cash_total: i64 = funds.iter().map(|f| f.balance.as_paise()).sum();
        let mut value = summary;
        if let Some(obj) = value.as_object_mut() {
            obj.insert("petty_cash_total_paise".to_string(), serde_json::json!(petty_cash_total));
        }
        Ok(value)
    }
}

fn mask_gateway_config(c: PaymentGatewayConfig) -> serde_json::Value {
    let mask = |s: &str| -> String {
        if s.len() <= 8 {
            "****".to_string()
        } else {
            format!("{}…{}", &s[..4], &s[s.len() - 4..])
        }
    };
    serde_json::json!({
        "payment_gateway_config_id": c.payment_gateway_config_id,
        "tenant_id": c.tenant_id,
        "entity_id": c.entity_id,
        "gateway_type": c.gateway_type.to_db_str(),
        "merchant_id": c.merchant_id,
        "api_key_masked": mask(&c.api_key),
        "api_secret_masked": mask(&c.api_secret),
        "webhook_secret_configured": c.webhook_secret.is_some(),
        "is_active": c.is_active,
        "audit": c.audit,
    })
}

// Re-export the repository row type used by queries.
#[allow(unused_imports)]
use crate::repository::BankTransactionRow as _BankTransactionRow;
