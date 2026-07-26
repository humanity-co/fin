//! General Ledger queries (CQRS read side).
//!
//! Query handlers that read from the database using direct SQL.
//! Read models are separate from write models — these return
//! projection types optimized for the UI/reporting layer.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use sutra_core::{EntityId, Money, TenantId};

use crate::errors::GlError;
use crate::models::account::Account;
use crate::models::journal::{Journal, JournalStatus, JournalType};
use crate::repository::{
    AccountRepository, JournalRepository, PeriodRepository,
    PgAccountRepository, PgJournalRepository, PgPeriodRepository,
};

// ─── Query Parameter Types ───────────────────────────────────────────

/// Filter parameters for listing journal entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalFilter {
    pub entity_id: Option<Uuid>,
    pub accounting_period_id: Option<Uuid>,
    pub status: Option<String>,
    pub journal_type: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

/// Parameters for trial balance query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialBalanceQuery {
    pub period_id: Uuid,
    pub entity_id: Option<Uuid>,
    pub cost_center_id: Option<Uuid>,
    pub fund_id: Option<Uuid>,
}

// ─── Query Result Types (Read Models) ────────────────────────────────

/// A single trial balance row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialBalanceRow {
    pub account_id: String,
    pub account_code: String,
    pub account_name: String,
    pub account_type: String,
    pub opening_balance: i64,
    pub total_debits: i64,
    pub total_credits: i64,
    pub closing_balance: i64,
}

/// A paginated list of journals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalListResponse {
    pub journals: Vec<Journal>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

/// A single entry in the account ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub date: NaiveDate,
    pub journal_number: String,
    pub journal_type: String,
    pub description: String,
    pub debit_amount: Option<i64>,
    pub credit_amount: Option<i64>,
    pub running_balance: i64,
}

/// Hierarchical COA tree node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoaTreeNode {
    pub account_id: String,
    pub account_code: String,
    pub account_name: String,
    pub account_type: String,
    pub level: i32,
    pub current_balance: i64,
    pub children: Vec<CoaTreeNode>,
}

// ─── Query Handler ───────────────────────────────────────────────────

/// The GL query handler.
pub struct GlQueryHandler {
    journal_repo: PgJournalRepository,
    account_repo: PgAccountRepository,
    period_repo: PgPeriodRepository,
    pool: PgPool,
}

impl GlQueryHandler {
    pub fn new(pool: PgPool) -> Self {
        Self {
            journal_repo: PgJournalRepository::new(pool.clone()),
            account_repo: PgAccountRepository::new(pool.clone()),
            period_repo: PgPeriodRepository::new(pool.clone()),
            pool,
        }
    }

    /// Get a single journal entry by ID with all lines.
    pub async fn get_journal_by_id(
        &self,
        tenant_id: TenantId,
        journal_id: Uuid,
    ) -> Result<Option<Journal>, GlError> {
        self.journal_repo
            .find_by_id(*tenant_id.as_uuid(), journal_id)
            .await
    }

    /// List journals with pagination and filtering.
    pub async fn list_journals(
        &self,
        tenant_id: TenantId,
        filter: JournalFilter,
    ) -> Result<JournalListResponse, GlError> {
        let tid = *tenant_id.as_uuid();
        let page = filter.page.unwrap_or(1).max(1);
        let per_page = filter.per_page.unwrap_or(20).min(100);

        // Parse date filters
        let from_date = filter
            .from_date
            .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
        let to_date = filter
            .to_date
            .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

        let (journals, total) = self
            .journal_repo
            .list(
                tid,
                filter.entity_id,
                filter.accounting_period_id,
                filter.status.as_deref(),
                filter.journal_type.as_deref(),
                from_date,
                to_date,
                page,
                per_page,
            )
            .await?;

        Ok(JournalListResponse {
            journals,
            total,
            page,
            per_page,
        })
    }

    /// Get trial balance for a period.
    ///
    /// Returns: account_id, opening_balance, total_debits, total_credits, closing_balance
    /// for all accounts that had activity in the period.
    pub async fn get_trial_balance(
        &self,
        tenant_id: TenantId,
        query: TrialBalanceQuery,
    ) -> Result<Vec<TrialBalanceRow>, GlError> {
        let tid = *tenant_id.as_uuid();

        // Get the period to determine date range
        let period = self
            .period_repo
            .find_by_id(tid, query.period_id)
            .await?
            .ok_or_else(|| GlError::PeriodNotFound(query.period_id.to_string()))?;

        // Compute trial balance: sum of all journal lines for accounts in this period
        let rows = sqlx::query_as::<_, TrialBalanceDbRow>(
            r#"
            SELECT
                coa.account_id,
                coa.account_code,
                coa.account_name,
                coa.account_type,
                COALESCE(coa.opening_balance, 0) AS opening_balance,
                COALESCE(SUM(jel.debit_amount), 0)  AS total_debits,
                COALESCE(SUM(jel.credit_amount), 0) AS total_credits
            FROM chart_of_accounts coa
            LEFT JOIN journal_entry_lines jel
                ON jel.account_id = coa.account_id
                AND jel.tenant_id = coa.tenant_id
            LEFT JOIN journal_entries je
                ON je.journal_id = jel.journal_id
                AND je.tenant_id = coa.tenant_id
                AND je.status = 'POSTED'
                AND je.posting_date >= $2
                AND je.posting_date <= $3
            WHERE coa.tenant_id = $1
              AND coa.deleted_at IS NULL
              AND coa.is_active = TRUE
            GROUP BY coa.account_id, coa.account_code, coa.account_name, coa.account_type, coa.opening_balance
            HAVING COALESCE(SUM(jel.debit_amount), 0) > 0
                OR COALESCE(SUM(jel.credit_amount), 0) > 0
                OR coa.opening_balance != 0
            ORDER BY coa.account_code
            "#,
        )
        .bind(tid)
        .bind(period.start_date)
        .bind(period.end_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let opening = r.opening_balance;
                let debits = r.total_debits;
                let credits = r.total_credits;
                let closing = compute_closing_balance(&r.account_type, opening, debits, credits);

                TrialBalanceRow {
                    account_id: r.account_id.to_string(),
                    account_code: r.account_code,
                    account_name: r.account_name,
                    account_type: r.account_type,
                    opening_balance: opening,
                    total_debits: debits,
                    total_credits: credits,
                    closing_balance: closing,
                }
            })
            .collect())
    }

    /// Get the ledger (all entries) for a specific account within a date range.
    pub async fn get_account_ledger(
        &self,
        tenant_id: TenantId,
        account_id: Uuid,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<Vec<LedgerEntry>, GlError> {
        let tid = *tenant_id.as_uuid();

        // Default date range
        let from = from_date.unwrap_or_else(|| NaiveDate::from_ymd_opt(2020, 4, 1).unwrap());
        let to = to_date.unwrap_or_else(|| Utc::now().date_naive());

        let rows = sqlx::query_as::<_, LedgerDbRow>(
            r#"
            SELECT
                je.posting_date,
                je.journal_number,
                je.journal_type,
                COALESCE(jel.description, je.description) AS description,
                jel.debit_amount,
                jel.credit_amount
            FROM journal_entry_lines jel
            JOIN journal_entries je
                ON je.journal_id = jel.journal_id
                AND je.tenant_id = $1
                AND je.status = 'POSTED'
            WHERE jel.tenant_id = $1
              AND jel.account_id = $2
              AND je.posting_date >= $3
              AND je.posting_date <= $4
            ORDER BY je.posting_date ASC, je.created_at ASC, jel.line_number ASC
            "#,
        )
        .bind(tid)
        .bind(account_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        // Compute running balance
        let mut running = 0i64;

        // We need to get the account type for proper balance calculation
        let account = self.account_repo.find_by_id(tid, account_id).await?;
        let account_type = account
            .as_ref()
            .map(|a| format!("{:?}", a.account_type))
            .unwrap_or_default();

        // Add opening balance (all debits - credits before from_date)
        let opening: Option<(i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN jel.debit_amount IS NOT NULL THEN jel.debit_amount ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN jel.credit_amount IS NOT NULL THEN jel.credit_amount ELSE 0 END), 0)
            FROM journal_entry_lines jel
            JOIN journal_entries je
                ON je.journal_id = jel.journal_id
                AND je.tenant_id = $1
                AND je.status = 'POSTED'
                AND je.posting_date < $3
            WHERE jel.tenant_id = $1
              AND jel.account_id = $2
            "#,
        )
        .bind(tid)
        .bind(account_id)
        .bind(from)
        .fetch_optional(&self.pool)
        .await?
        .map(|(d, c): (i64, i64)| (d, c));

        if let Some((open_debit, open_credit)) = opening {
            running = open_debit - open_credit;
        }

        let entries: Vec<LedgerEntry> = rows
            .into_iter()
            .map(|r| {
                let debit = r.debit_amount;
                let credit = r.credit_amount;

                if let Some(d) = debit {
                    running += d;
                }
                if let Some(c) = credit {
                    running -= c;
                }

                LedgerEntry {
                    date: r.posting_date,
                    journal_number: r.journal_number,
                    journal_type: r.journal_type,
                    description: r.description,
                    debit_amount: debit,
                    credit_amount: credit,
                    running_balance: running,
                }
            })
            .collect();

        Ok(entries)
    }

    /// Get the full Chart of Accounts as a hierarchical tree.
    pub async fn get_chart_of_accounts(
        &self,
        tenant_id: TenantId,
    ) -> Result<Vec<CoaTreeNode>, GlError> {
        let tid = *tenant_id.as_uuid();
        let all_accounts = self.account_repo.list_all(tid).await?;

        // Build tree: accounts with no parent are roots
        let roots: Vec<CoaTreeNode> = all_accounts
            .iter()
            .filter(|a| a.parent_account_id.is_none())
            .map(|a| build_coa_node(&all_accounts, a))
            .collect();

        Ok(roots)
    }
}

// ─── Database Row Types ──────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct TrialBalanceDbRow {
    account_id: Uuid,
    account_code: String,
    account_name: String,
    account_type: String,
    opening_balance: i64,
    total_debits: i64,
    total_credits: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct LedgerDbRow {
    posting_date: NaiveDate,
    journal_number: String,
    journal_type: String,
    description: String,
    debit_amount: Option<i64>,
    credit_amount: Option<i64>,
}

// ─── Helpers ─────────────────────────────────────────────────────────

use chrono::Utc;

fn compute_closing_balance(account_type: &str, opening: i64, debits: i64, credits: i64) -> i64 {
    match account_type.to_uppercase().as_str() {
        "ASSET" | "EXPENSE" => opening + debits - credits,
        "LIABILITY" | "EQUITY" | "INCOME" => opening + credits - debits,
        _ => opening + debits - credits, // Default to asset behavior
    }
}

fn build_coa_node(all: &[Account], account: &Account) -> CoaTreeNode {
    let children: Vec<CoaTreeNode> = all
        .iter()
        .filter(|a| a.parent_account_id == Some(account.account_id))
        .map(|a| build_coa_node(all, a))
        .collect();

    CoaTreeNode {
        account_id: account.account_id.as_uuid().to_string(),
        account_code: account.account_code.clone(),
        account_name: account.account_name.clone(),
        account_type: format!("{:?}", account.account_type),
        level: account.level,
        current_balance: account.current_balance.as_paise(),
        children,
    }
}
