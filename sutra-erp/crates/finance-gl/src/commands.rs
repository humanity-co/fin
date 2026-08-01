//! General Ledger commands (CQRS command side).
//!
//! Each command has a corresponding handler that:
//! 1. Validates business rules
//! 2. Performs the mutation within a DB transaction
//! 3. Writes to the outbox for event publishing

use chrono::{NaiveDate, Utc, Datelike};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use sutra_core::{AuditInfo, EntityId, Money, TenantId};

use crate::errors::GlError;
use crate::events::GlEventData;
use crate::models::account::Account;
use crate::models::journal::{Journal, JournalLine, JournalStatus, JournalType};
use crate::repository::{
    AccountRepository, JournalRepository, PeriodRepository,
    PgAccountRepository, PgJournalRepository, PgPeriodRepository,
};

// ─── Command Definitions ─────────────────────────────────────────────

/// Command to create a new journal entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJournalCmd {
    pub journal_type: String,
    pub accounting_period_id: Uuid,
    pub entity_id: Uuid,
    pub fund_id: Option<Uuid>,
    pub cost_center_id: Option<Uuid>,
    pub posting_date: NaiveDate,
    pub description: String,
    pub lines: Vec<CreateJournalLineCmd>,
    pub attachment_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJournalLineCmd {
    pub line_number: i32,
    pub account_id: Uuid,
    pub debit_amount: Option<Money>,
    pub credit_amount: Option<Money>,
    pub description: Option<String>,
    pub cost_center_id: Option<Uuid>,
    pub fund_id: Option<Uuid>,
    pub reference_id: Option<String>,
    pub reference_type: Option<String>,
}

/// Command to post a journal entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostJournalCmd {
    pub journal_id: Uuid,
    pub posted_by: Uuid,
}

/// Command to reverse a posted journal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseJournalCmd {
    pub journal_id: Uuid,
    pub reason: String,
    pub reversed_by: Uuid,
}

/// Command to create a new COA account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountCmd {
    pub account_code: String,
    pub account_name: String,
    pub account_type: String,
    pub parent_account_id: Option<Uuid>,
    pub gst_classification: Option<String>,
    pub hsn_sac_code: Option<String>,
    pub itc_eligibility: Option<String>,
    pub aishe_head_code: Option<String>,
    pub naac_metric_key: Option<String>,
}

// ─── Command Handler ─────────────────────────────────────────────────

/// The GL command handler — owns the database pool and repository implementations.
pub struct GlCommandHandler {
    pool: PgPool,
    journal_repo: PgJournalRepository,
    account_repo: PgAccountRepository,
    period_repo: PgPeriodRepository,
}

impl GlCommandHandler {
    pub fn new(pool: PgPool) -> Self {
        Self {
            journal_repo: PgJournalRepository::new(pool.clone()),
            account_repo: PgAccountRepository::new(pool.clone()),
            period_repo: PgPeriodRepository::new(pool.clone()),
            pool,
        }
    }
}

impl GlCommandHandler {
    /// Create a new draft journal entry.
    ///
    /// Validates:
    /// - Period is open
    /// - At least 2 lines, at most 500 lines
    /// - All account IDs exist and are active leaf nodes
    /// - Debits == Credits (balanced)
    /// - No zero amounts
    /// - Every line has exactly one of debit_amount or credit_amount set
    pub async fn create_journal(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: CreateJournalCmd,
    ) -> Result<Journal, GlError> {
        let tid = *tenant_id.as_uuid();

        // Validate period is open
        let period_open = self.period_repo.is_open(tid, cmd.accounting_period_id).await?;
        if !period_open {
            return Err(GlError::PeriodClosed(cmd.accounting_period_id.to_string()));
        }

        // Validate line count
        if cmd.lines.len() < 2 {
            return Err(GlError::TooFewLines);
        }
        if cmd.lines.len() > 500 {
            return Err(GlError::TooManyLines);
        }

        // Validate each line
        let mut total_debit = Money::ZERO;
        let mut total_credit = Money::ZERO;
        let mut journal_lines = Vec::with_capacity(cmd.lines.len());

        for line_cmd in &cmd.lines {
            // Exactly one of debit/credit must be set
            let (debit, credit) = match (&line_cmd.debit_amount, &line_cmd.credit_amount) {
                (Some(d), None) => {
                    if d.is_zero() {
                        return Err(GlError::NegativeAmount);
                    }
                    total_debit += *d;
                    (Some(*d), None)
                }
                (None, Some(c)) => {
                    if c.is_zero() {
                        return Err(GlError::NegativeAmount);
                    }
                    total_credit += *c;
                    (None, Some(*c))
                }
                _ => return Err(GlError::NegativeAmount), // Both set or neither
            };

            // Validate account exists, is active, and is a leaf
            let account = self
                .account_repo
                .find_by_id(tid, line_cmd.account_id)
                .await?
                .ok_or_else(|| GlError::AccountNotFound(line_cmd.account_id.to_string()))?;

            if !account.is_active {
                return Err(GlError::AccountInactive(account.account_code));
            }

            let has_children = self.account_repo.has_children(tid, line_cmd.account_id).await?;
            if has_children {
                return Err(GlError::AccountNotLeaf(account.account_code));
            }

            journal_lines.push(JournalLine {
                journal_line_id: Uuid::now_v7(),
                journal_id: EntityId::from_uuid(Uuid::nil()), // Will be set below
                line_number: line_cmd.line_number,
                account_id: line_cmd.account_id,
                debit_amount: debit,
                credit_amount: credit,
                description: line_cmd.description.clone(),
                cost_center_id: line_cmd.cost_center_id,
                fund_id: line_cmd.fund_id,
                reference_id: line_cmd.reference_id.clone(),
                reference_type: line_cmd.reference_type.clone(),
                tax_rate: None,
                tax_amount: None,
                is_itc_claimed: false,
                itc_reversal_percent: None,
                version: 1,
            });
        }

        // Balance check
        if total_debit != total_credit {
            return Err(GlError::UnbalancedJournal(total_debit, total_credit));
        }

        // Determine fiscal year from posting date and generate journal number
        let fy = fiscal_year_from_date(cmd.posting_date);

        let mut tx = self.pool.begin().await?;
        let seq = self.journal_repo.next_sequence(&mut tx, tid, &fy).await?;
        let journal_number = format!("JV-{}-{:06}", fy, seq);

        let journal_type = JournalType::from_db_str(&cmd.journal_type);
        let journal_id = EntityId::new();
        let audit = AuditInfo::new(created_by);

        // Set the journal_id on each line
        for line in &mut journal_lines {
            line.journal_id = journal_id.clone();
        }

        let journal = Journal {
            journal_id: journal_id.clone(),
            tenant_id,
            journal_number,
            journal_type: journal_type.clone(),
            accounting_period_id: cmd.accounting_period_id,
            entity_id: cmd.entity_id,
            fund_id: cmd.fund_id,
            cost_center_id: cmd.cost_center_id,
            posting_date: cmd.posting_date,
            description: cmd.description,
            status: JournalStatus::Draft,
            total_debit,
            total_credit,
            lines: journal_lines,
            posted_at: None,
            posted_by: None,
            reversed_by_id: None,
            attachment_ids: cmd.attachment_ids,
            version: 1,
            audit,
        };

        // Persist within transaction
        self.journal_repo.create(&mut tx, &journal).await?;

        // Write outbox event
        write_outbox(
            &mut tx,
            tid,
            "Journal",
            &journal_id.as_uuid().to_string(),
            "JournalCreated",
            &GlEventData::JournalCreated {
                journal_id: journal_id.as_uuid().to_string(),
                journal_number: journal.journal_number.clone(),
                journal_type: journal_type.to_db_str().to_string(),
                status: "DRAFT".to_string(),
                created_by: created_by.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;

        tx.commit().await?;

        info!(
            tenant_id = %tid,
            journal_number = %journal.journal_number,
            "Journal created"
        );

        Ok(journal)
    }

    /// Post a draft journal entry.
    ///
    /// Validates:
    /// - Journal exists and is in Draft status
    /// - Period is still open
    ///
    /// Within a single DB transaction:
    /// - Updates account balances atomically
    /// - Transitions journal to Posted
    /// - Writes JournalPosted event to outbox
    pub async fn post_journal(
        &self,
        tenant_id: TenantId,
        cmd: PostJournalCmd,
    ) -> Result<Journal, GlError> {
        let tid = *tenant_id.as_uuid();

        // Load the journal
        let journal = self
            .journal_repo
            .find_by_id(tid, cmd.journal_id)
            .await?
            .ok_or_else(|| GlError::JournalNotFound(cmd.journal_id.to_string()))?;

        // Validate state
        if journal.status != JournalStatus::Draft {
            return Err(GlError::JournalNotDraft(journal.journal_number.clone()));
        }

        // Validate period is still open
        let period_open = self
            .period_repo
            .is_open(tid, journal.accounting_period_id)
            .await?;
        if !period_open {
            return Err(GlError::PeriodClosed(journal.accounting_period_id.to_string()));
        }

        // Within a single transaction: update balances + update status + outbox
        let mut tx = self.pool.begin().await?;

        for line in &journal.lines {
            let account = self
                .account_repo
                .find_by_id(tid, line.account_id)
                .await?
                .ok_or_else(|| GlError::AccountNotFound(line.account_id.to_string()))?;

            let balance_delta = compute_balance_delta(&account, line);

            self.account_repo
                .update_balance(&mut tx, line.account_id, balance_delta)
                .await?;
        }

        // Transition to Posted
        let old_version = journal.version;
        self.journal_repo
            .update_status(&mut tx, cmd.journal_id, &JournalStatus::Posted, old_version)
            .await?;

        // Publish event
        write_outbox(
            &mut tx,
            tid,
            "Journal",
            &cmd.journal_id.to_string(),
            "JournalPosted",
            &GlEventData::JournalPosted {
                journal_id: cmd.journal_id.to_string(),
                journal_number: journal.journal_number.clone(),
                total_debit: journal.total_debit.as_paise(),
                total_credit: journal.total_credit.as_paise(),
                period_id: journal.accounting_period_id.to_string(),
                posted_by: cmd.posted_by.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;

        tx.commit().await?;

        // Refresh from DB
        let posted = self
            .journal_repo
            .find_by_id(tid, cmd.journal_id)
            .await?
            .ok_or_else(|| GlError::JournalNotFound(cmd.journal_id.to_string()))?;

        info!(
            tenant_id = %tid,
            journal_number = %posted.journal_number,
            "Journal posted"
        );

        Ok(posted)
    }

    /// Reverse a posted journal entry.
    ///
    /// Creates a new journal entry with reversed debit/credit lines,
    /// linked to the original. The original becomes Reversed.
    /// The reversal is auto-posted.
    pub async fn reverse_journal(
        &self,
        tenant_id: TenantId,
        cmd: ReverseJournalCmd,
    ) -> Result<(Journal, Journal), GlError> {
        let tid = *tenant_id.as_uuid();

        // Load the original
        let original = self
            .journal_repo
            .find_by_id(tid, cmd.journal_id)
            .await?
            .ok_or_else(|| GlError::JournalNotFound(cmd.journal_id.to_string()))?;

        // Must be Posted
        if original.status != JournalStatus::Posted {
            return Err(GlError::JournalNotDraft(format!(
                "Journal {} is not in Posted status (current: {:?})",
                original.journal_number, original.status
            )));
        }

        let fy = fiscal_year_from_date(original.posting_date);
        let mut tx = self.pool.begin().await?;
        let seq = self.journal_repo.next_sequence(&mut tx, tid, &fy).await?;
        let reversal_number = format!("JV-{}-{:06}", fy, seq);

        let reversal_id = EntityId::new();
        let audit = AuditInfo::new(cmd.reversed_by);

        // Build reversal lines: swap debit/credit
        let mut reversal_lines = Vec::with_capacity(original.lines.len());
        for (i, line) in original.lines.iter().enumerate() {
            reversal_lines.push(JournalLine {
                journal_line_id: Uuid::now_v7(),
                journal_id: reversal_id.clone(),
                line_number: (i + 1) as i32,
                account_id: line.account_id,
                debit_amount: line.credit_amount,
                credit_amount: line.debit_amount,
                description: Some(format!(
                    "Reversal of {} line {}: {}",
                    original.journal_number,
                    line.line_number,
                    line.description.as_deref().unwrap_or("")
                )),
                cost_center_id: line.cost_center_id,
                fund_id: line.fund_id,
                reference_id: Some(original.journal_number.clone()),
                reference_type: Some("REVERSAL".to_string()),
                tax_rate: line.tax_rate,
                tax_amount: line.tax_amount,
                is_itc_claimed: false,
                itc_reversal_percent: None,
                version: 1,
            });
        }

        let reversal = Journal {
            journal_id: reversal_id.clone(),
            tenant_id,
            journal_number: reversal_number,
            journal_type: JournalType::Reversing,
            accounting_period_id: original.accounting_period_id,
            entity_id: original.entity_id,
            fund_id: original.fund_id,
            cost_center_id: original.cost_center_id,
            posting_date: Utc::now().date_naive(),
            description: format!(
                "Reversal of {} — {}",
                original.journal_number, cmd.reason
            ),
            status: JournalStatus::Draft,
            total_debit: original.total_credit,
            total_credit: original.total_debit,
            lines: reversal_lines,
            posted_at: None,
            posted_by: None,
            reversed_by_id: Some(original.journal_id.clone()),
            attachment_ids: vec![],
            version: 1,
            audit,
        };

        // Create reversal journal
        self.journal_repo.create(&mut tx, &reversal).await?;

        // Mark original as Reversed
        self.journal_repo
            .update_status(&mut tx, cmd.journal_id, &JournalStatus::Reversed, original.version)
            .await?;

        // Publish reversal event
        write_outbox(
            &mut tx,
            tid,
            "Journal",
            &cmd.journal_id.to_string(),
            "JournalReversed",
            &GlEventData::JournalReversed {
                original_journal_id: cmd.journal_id.to_string(),
                reversing_journal_id: reversal_id.as_uuid().to_string(),
                reason: cmd.reason.clone(),
                reversed_by: cmd.reversed_by.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;

        tx.commit().await?;

        // Auto-post the reversal
        let reversal = self
            .post_journal(
                tenant_id,
                PostJournalCmd {
                    journal_id: *reversal.journal_id.as_uuid(),
                    posted_by: cmd.reversed_by,
                },
            )
            .await?;

        info!(
            tenant_id = %tid,
            original = %original.journal_number,
            reversal = %reversal.journal_number,
            "Journal reversed"
        );

        Ok((original, reversal))
    }
}

// ─── Outbox Writer ───────────────────────────────────────────────────

async fn write_outbox(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    aggregate_type: &str,
    aggregate_id: &str,
    event_type: &str,
    event_data: &GlEventData,
) -> Result<(), GlError> {
    let payload = serde_json::to_value(event_data)
        .map_err(|e| GlError::EventPublish(e.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO event_outbox (
            outbox_id, tenant_id, aggregate_type, aggregate_id,
            event_type, event_payload, status, retry_count, max_retries, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, 'PENDING', 0, 5, now())
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(aggregate_type)
    .bind(aggregate_id)
    .bind(event_type)
    .bind(&payload)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

// ─── Business Logic: Balance Delta ───────────────────────────────────

/// Compute the effect of a journal line on an account's balance.
///
/// For Asset/Expense accounts: Debit increases, Credit decreases
/// For Liability/Equity/Income accounts: Credit increases, Debit decreases
fn compute_balance_delta(account: &Account, line: &JournalLine) -> Money {
    use crate::models::account::AccountType;

    match account.account_type {
        AccountType::Asset | AccountType::Expense => {
            line.debit_amount.unwrap_or(Money::ZERO) - line.credit_amount.unwrap_or(Money::ZERO)
        }
        AccountType::Liability | AccountType::Equity | AccountType::Income => {
            line.credit_amount.unwrap_or(Money::ZERO) - line.debit_amount.unwrap_or(Money::ZERO)
        }
    }
}

/// Determine the fiscal year string (e.g., "2026-27") from a date.
fn fiscal_year_from_date(date: NaiveDate) -> String {
    let year = date.year();
    let month = date.month();

    if month >= 4 {
        format!("{}-{:02}", year, (year + 1) % 100)
    } else {
        format!("{}-{:02}", year - 1, year % 100)
    }
}
