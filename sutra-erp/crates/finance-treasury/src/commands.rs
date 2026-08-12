//! Treasury & Banking commands (CQRS command side).
//!
//! Implements the spec's command set: bank account register, signatories,
//! reconciliation (statement upload → auto/manual match → BRS), inter-bank
//! transfers, petty cash, gateway config & settlement reconciliation.
//!
//! Cross-cutting rules enforced here (never hardcoded — see
//! `system_config` + [`TreasuryPolicy`]):
//! - money is i64 paise, never floats;
//! - every table row is tenant-scoped;
//! - every GL posting goes through the GL module's create/post commands and
//!   carries `reference_type` / `reference_id` on each line;
//! - every mutation writes an outbox event; financial records are
//!   append-only (corrections via reversal entries, never deletes).

use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use sutra_finance_gl::{
    CreateJournalCmd, CreateJournalLineCmd, GlCommandHandler, PgAccountRepository,
    PgPeriodRepository, PostJournalCmd,
};

use crate::errors::TreasuryError;
use crate::events::{write_outbox, TreasuryEventData};
use crate::models::bank_account::{BankAccount, BankAccountType, SignatoryType};
use crate::models::gateway::{GatewaySettlement, GatewaySettlementStatus, GatewayType};
use crate::models::petty_cash::{
    PettyCashFund, PettyCashFundStatus, PettyCashTransaction, PettyCashTransactionType,
};
use crate::models::reconciliation::{
    BankReconciliation, BankReconciliationStatus, BankStatementLine, MatchStatus,
};
use crate::models::transfer::{InterBankTransfer, InterBankTransferStatus};
use crate::repository::{decrypt_secret, encrypt_secret, TreasuryRepository};

// ─── Command payloads ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBankAccountCmd {
    pub entity_id: Option<Uuid>,
    pub account_number: String,
    pub account_name: String,
    pub bank_name: String,
    pub branch_name: Option<String>,
    pub ifsc_code: String,
    pub account_type: String,
    pub fund_id: Option<Uuid>,
    pub minimum_balance: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBankAccountCmd {
    pub account_name: Option<String>,
    pub branch_name: Option<String>,
    pub minimum_balance: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddSignatoryCmd {
    pub user_id: Uuid,
    pub signatory_type: String, // INDIVIDUAL | JOINT
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncBankBalanceCmd {
    pub balance: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartReconciliationCmd {
    pub bank_account_id: Uuid,
    pub period_id: Uuid,
    pub statement_date: NaiveDate,
    pub opening_balance: i64,
    pub closing_balance: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadBankStatementCmd {
    pub format: String, // CSV | OFX
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualMatchCmd {
    pub statement_line_id: Uuid,
    pub transaction_id: Uuid,
    pub transaction_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteReconciliationCmd {
    /// Set true to certify open items (CFO certification path).
    pub certify_open_items: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateInterBankTransferCmd {
    pub from_bank_account_id: Uuid,
    pub to_bank_account_id: Uuid,
    pub amount: i64,
    pub transfer_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopUpPettyCashCmd {
    pub amount: i64,
    pub bank_account_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordPettyCashExpenseCmd {
    pub amount: i64,
    pub expense_account_id: Uuid,
    pub voucher_no: String,
    pub narration: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigureGatewayCmd {
    pub entity_id: Uuid,
    pub gateway_type: String,
    pub merchant_id: String,
    pub api_key: String,
    pub api_secret: String,
    pub webhook_secret: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileGatewaySettlementCmd {
    pub entity_id: Uuid,
    pub gateway_type: String,
    pub settlement_date: NaiveDate,
    pub settled_amount: i64,
    pub gateway_fee_amount: i64,
}

// ─── Command handler ─────────────────────────────────────────────────

/// The treasury command handler — owns the pool and the repository.
pub struct TreasuryCommandHandler {
    pool: PgPool,
    repo: TreasuryRepository,
}

impl TreasuryCommandHandler {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: TreasuryRepository::new(pool.clone()),
            pool,
        }
    }

    // ── Bank accounts ───────────────────────────────────────────────

    /// Create a bank account. Validates type/IFSC, enforces the FCRA
    /// SBI-New-Delhi rule and the GRANT_SPECIFIC fund requirement, then
    /// auto-creates a dedicated GL leaf under 10.02 and links it.
    pub async fn create_bank_account(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: CreateBankAccountCmd,
    ) -> Result<BankAccount, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let policy = self.repo.load_policy(tid).await?;

        // Format validation
        let ifsc = cmd.ifsc_code.trim().to_uppercase();
        if !valid_ifsc(&ifsc) {
            return Err(TreasuryError::InvalidIfsc(ifsc));
        }
        let account_number = cmd.account_number.trim().to_string();
        if !valid_account_number(&account_number) {
            return Err(TreasuryError::InvalidInput(
                "account_number must be 9–18 alphanumeric characters".to_string(),
            ));
        }
        let account_type = BankAccountType::from_db_str(&cmd.account_type);
        if cmd.account_type.trim().is_empty() {
            return Err(TreasuryError::InvalidAccountType(cmd.account_type.clone()));
        }

        // Unique account number per tenant
        if self
            .repo
            .account_number_exists(tid, &account_number, None)
            .await?
        {
            return Err(TreasuryError::DuplicateBankAccount);
        }

        // Rule 1 (UGC): GRANT_SPECIFIC requires fund_id
        let fund_id = if account_type == BankAccountType::GrantSpecific {
            match cmd.fund_id {
                Some(f) => Some(f),
                None => return Err(TreasuryError::GrantFundRequired),
            }
        } else {
            cmd.fund_id
        };

        // Rule 2 (FCRA 2010 s.17): FCRA ⇒ SBI New Delhi Main Branch
        let is_fcra_account = account_type == BankAccountType::Fcra;
        if is_fcra_account {
            if ifsc != policy.fcra_bank_ifsc {
                return Err(TreasuryError::FcraBankMismatch {
                    expected_ifsc: policy.fcra_bank_ifsc,
                    actual_ifsc: ifsc,
                });
            }
        }

        // Auto-create GL leaf under the configured bank parent (default 10.02)
        let parent = self
            .repo
            .find_gl_parent_with_code(tid, &policy.bank_gl_parent_codes)
            .await?
            .ok_or(TreasuryError::BankGlParentMissing)?;
        let leaf_code = self.repo.next_gl_leaf_code(tid, &parent.1).await?;
        let leaf_name = format!("Bank – {} ({})", cmd.account_name, account_number);
        let gl_account_id = self
            .repo
            .insert_gl_leaf(tid, &leaf_code, &leaf_name, parent.0, created_by)
            .await?;

        let account = BankAccount {
            bank_account_id: EntityId::new(),
            tenant_id,
            entity_id: cmd.entity_id,
            account_number,
            account_name: cmd.account_name.clone(),
            bank_name: cmd.bank_name.clone(),
            branch_name: cmd.branch_name.clone(),
            ifsc_code: ifsc.clone(),
            account_type,
            fund_id,
            is_fcra_account,
            minimum_balance: Money::from_paise(cmd.minimum_balance.max(0)),
            is_active: true,
            gl_account_id: Some(gl_account_id),
            last_reconciled_at: None,
            audit: AuditInfo::new(created_by),
        };
        self.repo.insert_bank_account(&account).await?;

        self.publish_event(
            tid,
            account.bank_account_id.as_uuid().to_string(),
            TreasuryEventData::BankAccountCreated {
                bank_account_id: account.bank_account_id.as_uuid().to_string(),
                account_name: account.account_name.clone(),
                bank_name: account.bank_name.clone(),
                account_type: account.account_type.to_db_str().to_string(),
                entity_id: account.entity_id.unwrap_or(Uuid::nil()).to_string(),
                gl_account_id: gl_account_id.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;

        info!(tenant_id = %tid, account = %account.account_name, gl_code = %leaf_code, "Bank account created");
        Ok(account)
    }

    pub async fn update_bank_account(
        &self,
        tenant_id: TenantId,
        bank_account_id: Uuid,
        updated_by: Uuid,
        cmd: UpdateBankAccountCmd,
    ) -> Result<BankAccount, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let existing = self
            .repo
            .find_bank_account(tid, bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(bank_account_id.to_string()))?;

        let account_name = cmd.account_name.unwrap_or(existing.account_name.clone());
        let branch_name = cmd
            .branch_name
            .clone()
            .or_else(|| existing.branch_name.clone());
        let minimum_balance = cmd.minimum_balance.unwrap_or(existing.minimum_balance.as_paise());

        self.repo
            .update_bank_account_fields(
                tid,
                bank_account_id,
                &account_name,
                branch_name.as_deref(),
                minimum_balance.max(0),
                updated_by,
            )
            .await?;

        let updated = self
            .repo
            .find_bank_account(tid, bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(bank_account_id.to_string()))?;

        self.publish_event(
            tid,
            bank_account_id.to_string(),
            TreasuryEventData::BankAccountCreated {
                bank_account_id: bank_account_id.to_string(),
                account_name: updated.account_name.clone(),
                bank_name: updated.bank_name.clone(),
                account_type: updated.account_type.to_db_str().to_string(),
                entity_id: updated.entity_id.unwrap_or(Uuid::nil()).to_string(),
                gl_account_id: updated
                    .gl_account_id
                    .unwrap_or(Uuid::nil())
                    .to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(updated)
    }

    /// Deactivate a bank account. Refused while the account carries a
    /// non-zero unreconciled balance (spec: no deactivate with open items).
    pub async fn deactivate_bank_account(
        &self,
        tenant_id: TenantId,
        bank_account_id: Uuid,
        updated_by: Uuid,
    ) -> Result<BankAccount, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let existing = self
            .repo
            .find_bank_account(tid, bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(bank_account_id.to_string()))?;
        if !existing.is_active {
            return Err(TreasuryError::BankAccountInactive(bank_account_id.to_string()));
        }

        let unreconciled = self
            .repo
            .bank_account_unreconciled_balance(tid, bank_account_id)
            .await?;
        if unreconciled != 0 {
            return Err(TreasuryError::BankAccountHasBalance(bank_account_id.to_string()));
        }

        self.repo
            .set_bank_account_active(tid, bank_account_id, false, updated_by)
            .await?;
        let updated = self
            .repo
            .find_bank_account(tid, bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(bank_account_id.to_string()))?;

        self.publish_event(
            tid,
            bank_account_id.to_string(),
            TreasuryEventData::BankAccountDeactivated {
                bank_account_id: bank_account_id.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(updated)
    }

    // ── Signatories ─────────────────────────────────────────────────

    pub async fn add_signatory(
        &self,
        tenant_id: TenantId,
        bank_account_id: Uuid,
        created_by: Uuid,
        cmd: AddSignatoryCmd,
    ) -> Result<(), TreasuryError> {
        let tid = *tenant_id.as_uuid();
        self.require_bank_account(tid, bank_account_id).await?;
        let signatory_type = match cmd.signatory_type.to_uppercase().as_str() {
            "JOINT" => "JOINT",
            _ => "INDIVIDUAL",
        };
        self.repo
            .insert_signatory(tid, bank_account_id, cmd.user_id, signatory_type, created_by)
            .await?;
        self.publish_event(
            tid,
            bank_account_id.to_string(),
            TreasuryEventData::SignatoryAdded {
                bank_account_id: bank_account_id.to_string(),
                user_id: cmd.user_id.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn remove_signatory(
        &self,
        tenant_id: TenantId,
        bank_account_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), TreasuryError> {
        let tid = *tenant_id.as_uuid();
        self.repo
            .deactivate_signatory(tid, bank_account_id, user_id)
            .await?;
        self.publish_event(
            tid,
            bank_account_id.to_string(),
            TreasuryEventData::SignatoryRemoved {
                bank_account_id: bank_account_id.to_string(),
                user_id: user_id.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(())
    }

    /// Record a bank-provided balance. Emits `MinimumBalanceAlert` when the
    /// balance crosses the configured per-account minimum (spec rule 8).
    pub async fn sync_bank_balance(
        &self,
        tenant_id: TenantId,
        bank_account_id: Uuid,
        updated_by: Uuid,
        cmd: SyncBankBalanceCmd,
    ) -> Result<(), TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let account = self
            .repo
            .find_bank_account(tid, bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(bank_account_id.to_string()))?;
        self.repo.touch_last_sync(tid, bank_account_id, updated_by).await?;

        self.publish_event(
            tid,
            bank_account_id.to_string(),
            TreasuryEventData::BankBalanceSynced {
                bank_account_id: bank_account_id.to_string(),
                balance: cmd.balance,
                synced_at: Utc::now(),
                occurred_at: Utc::now(),
            },
        )
        .await?;

        if cmd.balance < account.minimum_balance.as_paise() {
            self.publish_event(
                tid,
                bank_account_id.to_string(),
                TreasuryEventData::MinimumBalanceAlert {
                    bank_account_id: bank_account_id.to_string(),
                    current_balance: cmd.balance,
                    minimum_balance: account.minimum_balance.as_paise(),
                    occurred_at: Utc::now(),
                },
            )
            .await?;
        }
        Ok(())
    }

    // ── Reconciliation ──────────────────────────────────────────────

    /// Start a reconciliation for (account, period). One open reconciliation
    /// per account (spec invariant).
    pub async fn start_reconciliation(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: StartReconciliationCmd,
    ) -> Result<BankReconciliation, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        self.require_bank_account(tid, cmd.bank_account_id).await?;

        if self
            .repo
            .has_open_reconciliation(tid, cmd.bank_account_id, cmd.period_id)
            .await?
        {
            return Err(TreasuryError::OpenReconciliationExists(
                cmd.bank_account_id.to_string(),
                cmd.period_id.to_string(),
            ));
        }
        if self
            .repo
            .has_any_open_reconciliation(tid, cmd.bank_account_id)
            .await?
        {
            return Err(TreasuryError::OpenReconciliationExists(
                cmd.bank_account_id.to_string(),
                "any period".to_string(),
            ));
        }

        let id = Uuid::now_v7();
        self.repo
            .insert_reconciliation(
                tid,
                id,
                cmd.bank_account_id,
                cmd.period_id,
                cmd.statement_date,
                cmd.opening_balance,
                cmd.closing_balance,
                created_by,
            )
            .await?;

        let reconciliation = self
            .repo
            .find_reconciliation(tid, id)
            .await?
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(id.to_string()))?;

        self.publish_event(
            tid,
            id.to_string(),
            TreasuryEventData::BankStatementUploaded {
                reconciliation_id: id.to_string(),
                line_count: 0,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(reconciliation)
    }

    /// Upload a bank statement (CSV or OFX). Parses lines, cross-checks the
    /// opening/closing balance against statement totals (spec invariant:
    /// closing balance must equal statement total).
    pub async fn upload_bank_statement(
        &self,
        tenant_id: TenantId,
        reconciliation_id: Uuid,
        created_by: Uuid,
        cmd: UploadBankStatementCmd,
    ) -> Result<i64, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let rec = self
            .repo
            .find_reconciliation(tid, reconciliation_id)
            .await?
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(reconciliation_id.to_string()))?;
        if rec.status != BankReconciliationStatus::InProgress {
            return Err(TreasuryError::InvalidReconciliationState(
                rec.status.to_db_str().to_string(),
                "IN_PROGRESS".to_string(),
            ));
        }

        let parsed = parse_statement(&cmd.format, &cmd.content)
            .map_err(|e| TreasuryError::StatementParse(e))?;

        // Balance cross-check: opening + Σ(debits) − Σ(credits) = closing
        let mut running = rec.opening_balance.as_paise();
        for line in &parsed {
            if let Some(d) = line.debit_amount {
                running += d.as_paise();
            }
            if let Some(c) = line.credit_amount {
                running -= c.as_paise();
            }
        }
        if running != rec.closing_balance.as_paise() {
            return Err(TreasuryError::StatementBalanceMismatch {
                opening: rec.opening_balance.as_paise(),
                closing: rec.closing_balance.as_paise(),
                computed: running,
            });
        }

        let lines: Vec<BankStatementLine> = parsed
            .into_iter()
            .map(|l| BankStatementLine {
                bank_statement_line_id: EntityId::new(),
                tenant_id,
                bank_reconciliation_id: reconciliation_id,
                transaction_date: l.transaction_date,
                transaction_ref: l.transaction_ref,
                description: l.description,
                debit_amount: l.debit_amount,
                credit_amount: l.credit_amount,
                match_status: MatchStatus::Unmatched,
                matched_transaction_id: None,
                matched_transaction_type: None,
                audit: AuditInfo::new(created_by),
            })
            .collect();
        let count = lines.len() as i64;
        self.repo
            .insert_statement_lines(tid, reconciliation_id, &lines)
            .await?;

        self.publish_event(
            tid,
            reconciliation_id.to_string(),
            TreasuryEventData::BankStatementUploaded {
                reconciliation_id: reconciliation_id.to_string(),
                line_count: count,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(count)
    }

    /// Auto-reconcile against the spec rule-4 matching set: exact UTR
    /// equality, OR amount equality + date within ±N days, OR amount
    /// equality + description similarity. Partial amount matches are marked
    /// PARTIAL_MATCH and never auto-completed.
    pub async fn auto_reconcile(
        &self,
        tenant_id: TenantId,
        reconciliation_id: Uuid,
        actor: Uuid,
    ) -> Result<(i64, i64), TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let rec = self
            .repo
            .find_reconciliation(tid, reconciliation_id)
            .await?
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(reconciliation_id.to_string()))?;
        if rec.status != BankReconciliationStatus::InProgress {
            return Err(TreasuryError::InvalidReconciliationState(
                rec.status.to_db_str().to_string(),
                "IN_PROGRESS".to_string(),
            ));
        }
        let policy = self.repo.load_policy(tid).await?;

        let lines = self
            .repo
            .list_statement_lines(tid, reconciliation_id)
            .await?;
        let candidates = self
            .repo
            .match_candidates(tid, rec.bank_account_id)
            .await?;

        let window = policy.auto_match_amount_date_window_days.max(0) as i64;
        let mut matched = 0i64;
        let mut unmatched = 0i64;

        for line in lines {
            if line.match_status != MatchStatus::Unmatched {
                continue;
            }
            let line_amount = line.amount().as_paise();
            let is_debit_line = line.debit_amount.is_some();

            // Rule 1 — exact UTR equality
            let mut best: Option<(&crate::repository::MatchCandidate, &'static str)> = None;
            if policy.auto_match_utr_exact {
                if let Some(ref line_ref) = line.transaction_ref {
                    for c in &candidates {
                        if let Some(ref c_ref) = c.transaction_ref {
                            if line_ref.trim().eq_ignore_ascii_case(c_ref.trim()) {
                                best = Some((c, "UTR"));
                                break;
                            }
                        }
                    }
                }
            }

            // Rule 2 — amount + date window
            if best.is_none() && window > 0 {
                for c in &candidates {
                    let date_diff = (c.transaction_date - line.transaction_date).num_days().abs();
                    if c.amount == line_amount && date_diff <= window {
                        best = Some((c, "AMOUNT_DATE"));
                        break;
                    }
                }
            }

            // Rule 3 — amount + description similarity
            if best.is_none() && policy.auto_match_description_similarity {
                for c in &candidates {
                    if c.amount != line_amount {
                        continue;
                    }
                    let desc_match = match (&line.description, &c.description) {
                        (Some(a), Some(b)) => description_similar(a, b),
                        _ => false,
                    };
                    if desc_match {
                        best = Some((c, "AMOUNT_DESCRIPTION"));
                        break;
                    }
                }
            }

            if let Some((candidate, _rule)) = best {
                // Direction sanity: a statement debit (money out) must match
                // an outgoing candidate (register debit / AP payment) and a
                // statement credit (money in) an incoming one (register
                // credit / AR receipt). Ref-only matches (UTR equality) are
                // authoritative regardless of register direction metadata.
                let ref_exact = policy.auto_match_utr_exact
                    && candidate.transaction_ref.is_some()
                    && line.transaction_ref.is_some()
                    && candidate
                        .transaction_ref
                        .as_deref()
                        .unwrap_or("")
                        .eq_ignore_ascii_case(line.transaction_ref.as_deref().unwrap_or(""));
                let direction_ok = candidate.is_debit == is_debit_line;
                if ref_exact || direction_ok {
                    self.repo
                        .update_line_match(
                            tid,
                            *line.bank_statement_line_id.as_uuid(),
                            &MatchStatus::Matched,
                            Some(candidate.transaction_id),
                            Some(&candidate.transaction_type),
                        )
                        .await?;
                    if candidate.transaction_type == "PaymentReceipt" {
                        self.repo
                            .mark_receipt_cleared(tid, candidate.transaction_id, line.transaction_date)
                            .await?;
                    }
                    self.publish_event(
                        tid,
                        reconciliation_id.to_string(),
                        TreasuryEventData::LineMatched {
                            reconciliation_id: reconciliation_id.to_string(),
                            statement_line_id: line.bank_statement_line_id.as_uuid().to_string(),
                            transaction_id: candidate.transaction_id.to_string(),
                            transaction_type: candidate.transaction_type.clone(),
                            occurred_at: Utc::now(),
                        },
                    )
                    .await?;
                    matched += 1;
                } else {
                    unmatched += 1;
                }
            } else {
                // Partial match detection: same UTR with a different amount
                // (e.g. gateway-fee netting) → PARTIAL_MATCH, never auto-completed.
                let same_ref = line.transaction_ref.as_ref().map(|lr| {
                    candidates.iter().any(|c| {
                        c.amount != line_amount
                            && c.transaction_ref
                                .as_deref()
                                .is_some_and(|cr| cr.trim().eq_ignore_ascii_case(lr.trim()))
                    })
                });
                if same_ref == Some(true) {
                    self.repo
                        .update_line_match(
                            tid,
                            *line.bank_statement_line_id.as_uuid(),
                            &MatchStatus::PartialMatch,
                            None,
                            None,
                        )
                        .await?;
                }
                unmatched += 1;
            }
        }

        self.publish_event(
            tid,
            reconciliation_id.to_string(),
            TreasuryEventData::AutoReconciliationCompleted {
                reconciliation_id: reconciliation_id.to_string(),
                matched_count: matched,
                unmatched_count: unmatched,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        info!(tenant_id = %tid, reconciliation = %reconciliation_id, matched, unmatched, "Auto-reconciliation done");
        let _ = actor;
        Ok((matched, unmatched))
    }

    /// Manual match — amount equality required; line must not be matched.
    pub async fn manual_match(
        &self,
        tenant_id: TenantId,
        reconciliation_id: Uuid,
        actor: Uuid,
        cmd: ManualMatchCmd,
    ) -> Result<(), TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let rec = self
            .repo
            .find_reconciliation(tid, reconciliation_id)
            .await?
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(reconciliation_id.to_string()))?;
        if rec.status != BankReconciliationStatus::InProgress {
            return Err(TreasuryError::InvalidReconciliationState(
                rec.status.to_db_str().to_string(),
                "IN_PROGRESS".to_string(),
            ));
        }
        let line = self
            .repo
            .find_statement_line(tid, reconciliation_id, cmd.statement_line_id)
            .await?
            .ok_or_else(|| {
                TreasuryError::StatementLineNotFound(cmd.statement_line_id.to_string())
            })?;
        if line.match_status == MatchStatus::Matched
            || line.match_status == MatchStatus::ManualMatch
        {
            return Err(TreasuryError::LineAlreadyMatched(
                cmd.statement_line_id.to_string(),
            ));
        }

        // Amount equality check against the referenced transaction
        let candidate_amount = self.candidate_amount(tid, cmd.transaction_id, &cmd.transaction_type).await?;
        if candidate_amount != line.amount().as_paise() {
            return Err(TreasuryError::AmountMismatch {
                statement_amount: line.amount().as_paise(),
                transaction_amount: candidate_amount,
            });
        }

        self.repo
            .update_line_match(
                tid,
                cmd.statement_line_id,
                &MatchStatus::ManualMatch,
                Some(cmd.transaction_id),
                Some(&cmd.transaction_type),
            )
            .await?;
        if cmd.transaction_type == "PaymentReceipt" {
            self.repo
                .mark_receipt_cleared(tid, cmd.transaction_id, line.transaction_date)
                .await?;
        }
        self.publish_event(
            tid,
            reconciliation_id.to_string(),
            TreasuryEventData::LineMatched {
                reconciliation_id: reconciliation_id.to_string(),
                statement_line_id: cmd.statement_line_id.to_string(),
                transaction_id: cmd.transaction_id.to_string(),
                transaction_type: cmd.transaction_type,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        let _ = actor;
        Ok(())
    }

    /// Unmatch a MATCHED / MANUAL_MATCH line.
    pub async fn unmatch(
        &self,
        tenant_id: TenantId,
        reconciliation_id: Uuid,
        statement_line_id: Uuid,
        actor: Uuid,
    ) -> Result<(), TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let line = self
            .repo
            .find_statement_line(tid, reconciliation_id, statement_line_id)
            .await?
            .ok_or_else(|| TreasuryError::StatementLineNotFound(statement_line_id.to_string()))?;
        if line.match_status != MatchStatus::Matched && line.match_status != MatchStatus::ManualMatch {
            return Err(TreasuryError::LineNotMatched(statement_line_id.to_string()));
        }
        self.repo
            .update_line_match(tid, statement_line_id, &MatchStatus::Unmatched, None, None)
            .await?;
        self.publish_event(
            tid,
            reconciliation_id.to_string(),
            TreasuryEventData::LineUnmatched {
                reconciliation_id: reconciliation_id.to_string(),
                statement_line_id: statement_line_id.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        let _ = actor;
        Ok(())
    }

    /// Complete a reconciliation: zero UNMATCHED lines, or CFO-certified
    /// open items. Period must not be closed.
    pub async fn complete_reconciliation(
        &self,
        tenant_id: TenantId,
        reconciliation_id: Uuid,
        completed_by: Uuid,
        cmd: CompleteReconciliationCmd,
    ) -> Result<BankReconciliation, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let rec = self
            .repo
            .find_reconciliation(tid, reconciliation_id)
            .await?
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(reconciliation_id.to_string()))?;
        if rec.status != BankReconciliationStatus::InProgress {
            return Err(TreasuryError::InvalidReconciliationState(
                rec.status.to_db_str().to_string(),
                "IN_PROGRESS".to_string(),
            ));
        }

        // Period not closed (spec rule 3 — BRS before period close)
        let period_open = PgPeriodRepository::new(self.pool.clone())
            .is_open(tid, rec.period_id)
            .await
            .map_err(|e| TreasuryError::Gl(e.to_string()))?;
        if !period_open {
            return Err(TreasuryError::Gl(
                "accounting period is closed — reconciliation cannot be completed".to_string(),
            ));
        }

        let unmatched = self.repo.unmatched_line_count(tid, reconciliation_id).await?;
        if unmatched > 0 && !cmd.certify_open_items {
            return Err(TreasuryError::InvalidInput(format!(
                "reconciliation has {unmatched} unmatched line(s) — certify open items to complete"
            )));
        }

        self.repo
            .update_reconciliation_status(
                tid,
                reconciliation_id,
                &BankReconciliationStatus::Completed,
                None,
                completed_by,
            )
            .await?;
        self.repo.touch_last_reconciled(tid, rec.bank_account_id).await?;

        let updated = self
            .repo
            .find_reconciliation(tid, reconciliation_id)
            .await?
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(reconciliation_id.to_string()))?;

        self.publish_event(
            tid,
            reconciliation_id.to_string(),
            TreasuryEventData::ReconciliationCompleted {
                reconciliation_id: reconciliation_id.to_string(),
                bank_account_id: rec.bank_account_id.to_string(),
                period_id: rec.period_id.to_string(),
                completed_by: completed_by.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(updated)
    }

    /// Verify a completed reconciliation (requires `treasury:reconciliation:approve`
    /// — enforced at the API layer).
    pub async fn verify_reconciliation(
        &self,
        tenant_id: TenantId,
        reconciliation_id: Uuid,
        verified_by: Uuid,
    ) -> Result<BankReconciliation, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let rec = self
            .repo
            .find_reconciliation(tid, reconciliation_id)
            .await?
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(reconciliation_id.to_string()))?;
        if rec.status != BankReconciliationStatus::Completed {
            return Err(TreasuryError::InvalidReconciliationState(
                rec.status.to_db_str().to_string(),
                "COMPLETED".to_string(),
            ));
        }
        self.repo
            .update_reconciliation_status(
                tid,
                reconciliation_id,
                &BankReconciliationStatus::Verified,
                Some(verified_by),
                verified_by,
            )
            .await?;
        let updated = self
            .repo
            .find_reconciliation(tid, reconciliation_id)
            .await?
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(reconciliation_id.to_string()))?;

        self.publish_event(
            tid,
            reconciliation_id.to_string(),
            TreasuryEventData::ReconciliationVerified {
                reconciliation_id: reconciliation_id.to_string(),
                verified_by: verified_by.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(updated)
    }

    /// Generate the BRS (bank reconciliation statement) data for a completed
    /// reconciliation. Returns the standard BRS structure; `document_url`
    /// points at the (frontend-rendered) CSV/PDF export.
    pub async fn generate_brs(
        &self,
        tenant_id: TenantId,
        reconciliation_id: Uuid,
        actor: Uuid,
    ) -> Result<serde_json::Value, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let rec = self
            .repo
            .find_reconciliation(tid, reconciliation_id)
            .await?
            .ok_or_else(|| TreasuryError::ReconciliationNotFound(reconciliation_id.to_string()))?;
        let account = self
            .repo
            .find_bank_account(tid, rec.bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(rec.bank_account_id.to_string()))?;

        let book_balance = match account.gl_account_id {
            Some(gl) => self.repo.gl_account_balance(tid, gl).await?,
            None => 0,
        };
        let lines = self.repo.list_statement_lines(tid, reconciliation_id).await?;

        let mut deposits_in_transit: Vec<serde_json::Value> = Vec::new();
        let mut outstanding_cheques: Vec<serde_json::Value> = Vec::new();
        let mut bank_side_items: Vec<serde_json::Value> = Vec::new();

        for line in &lines {
            let is_unmatched = line.match_status == MatchStatus::Unmatched
                || line.match_status == MatchStatus::PartialMatch;
            if !is_unmatched {
                continue;
            }
            let item = serde_json::json!({
                "date": line.transaction_date.to_string(),
                "reference": line.transaction_ref,
                "description": line.description,
                "debit_amount": line.debit_amount.map(|m| m.as_paise()),
                "credit_amount": line.credit_amount.map(|m| m.as_paise()),
                "match_status": line.match_status.to_db_str(),
            });
            if line.credit_amount.is_some() {
                deposits_in_transit.push(item); // book credit, bank not yet
            } else {
                bank_side_items.push(item);
            }
        }

        // Uncleared cheques/DDs (AR) — book debit, bank not yet
        let uncleared = self
            .repo
            .uncleared_cheques(tid, None, None)
            .await?;
        for (receipt_id, receipt_number, amount, date, cheque) in uncleared {
            outstanding_cheques.push(serde_json::json!({
                "receipt_id": receipt_id,
                "receipt_number": receipt_number,
                "cheque_number": cheque,
                "amount": amount,
                "date": date.to_string(),
            }));
        }

        let reconciled = lines.iter().filter(|l| l.match_status == MatchStatus::Matched || l.match_status == MatchStatus::ManualMatch).count() as i64;
        let total = lines.len() as i64;

        let document_url = format!("/api/v1/treasury/reconciliations/{}/brs", reconciliation_id);
        let brs = serde_json::json!({
            "reconciliation_id": reconciliation_id.to_string(),
            "bank_account_id": rec.bank_account_id.to_string(),
            "statement_date": rec.statement_date.to_string(),
            "status": rec.status.to_db_str(),
            "book_balance_paise": book_balance,
            "bank_balance_paise": rec.closing_balance.as_paise(),
            "difference_paise": rec.closing_balance.as_paise() - book_balance,
            "deposits_in_transit": deposits_in_transit,
            "outstanding_cheques": outstanding_cheques,
            "bank_side_items": bank_side_items,
            "matched_lines": reconciled,
            "total_lines": total,
            "unmatched_lines": total - reconciled,
            "document_url": document_url,
        });

        self.publish_event(
            tid,
            reconciliation_id.to_string(),
            TreasuryEventData::BrsGenerated {
                reconciliation_id: reconciliation_id.to_string(),
                document_url,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        let _ = actor;
        Ok(brs)
    }

    // ── Inter-bank transfers ────────────────────────────────────────

    /// Initiate a transfer. Enforces from ≠ to, same tenant, sufficient
    /// available balance, and the endowment rule; requires approval when the
    /// amount ≥ `transfer_approval_threshold`.
    pub async fn initiate_inter_bank_transfer(
        &self,
        tenant_id: TenantId,
        initiated_by: Uuid,
        cmd: InitiateInterBankTransferCmd,
    ) -> Result<InterBankTransfer, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let policy = self.repo.load_policy(tid).await?;

        if cmd.from_bank_account_id == cmd.to_bank_account_id {
            return Err(TreasuryError::InvalidTransfer(
                "from and to accounts must differ".to_string(),
            ));
        }
        if cmd.amount <= 0 {
            return Err(TreasuryError::InvalidTransfer(
                "amount must be positive".to_string(),
            ));
        }

        let from = self
            .repo
            .find_bank_account(tid, cmd.from_bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(cmd.from_bank_account_id.to_string()))?;
        let to = self
            .repo
            .find_bank_account(tid, cmd.to_bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(cmd.to_bank_account_id.to_string()))?;
        if !from.is_active || !to.is_active {
            return Err(TreasuryError::BankAccountInactive(
                if !from.is_active { from.bank_account_id.to_string() } else { to.bank_account_id.to_string() },
            ));
        }

        // Rule 7: endowment corpus protection
        if policy.endowment_transfer_blocked
            && self.repo.is_endowment_linked(tid, cmd.from_bank_account_id).await?
        {
            return Err(TreasuryError::EndowmentTransferBlocked);
        }

        // At least one active signatory on the source account
        if self.repo.active_signatory_count(tid, cmd.from_bank_account_id).await? == 0 {
            return Err(TreasuryError::NoActiveSignatory(
                cmd.from_bank_account_id.to_string(),
            ));
        }

        // Available balance check against the GL leaf balance
        if let Some(gl) = from.gl_account_id {
            let available = self.repo.gl_account_balance(tid, gl).await?;
            if available < cmd.amount {
                return Err(TreasuryError::InsufficientBalance {
                    available,
                    required: cmd.amount,
                });
            }
        }

        let requires_approval = cmd.amount >= policy.transfer_approval_threshold;
        let transfer = InterBankTransfer {
            inter_bank_transfer_id: EntityId::new(),
            tenant_id,
            from_bank_account_id: cmd.from_bank_account_id,
            to_bank_account_id: cmd.to_bank_account_id,
            amount: Money::from_paise(cmd.amount),
            transfer_date: cmd.transfer_date,
            status: InterBankTransferStatus::Initiated,
            requires_approval,
            bank_reference: None,
            journal_id: None,
            initiated_by_id: initiated_by,
            approved_by_id: None,
            processed_by_id: None,
            failure_reason: None,
            version: 1,
            audit: AuditInfo::new(initiated_by),
        };
        self.repo.insert_transfer(&transfer).await?;

        self.publish_event(
            tid,
            transfer.inter_bank_transfer_id.as_uuid().to_string(),
            TreasuryEventData::InterBankTransferInitiated {
                transfer_id: transfer.inter_bank_transfer_id.as_uuid().to_string(),
                from_account: cmd.from_bank_account_id.to_string(),
                to_account: cmd.to_bank_account_id.to_string(),
                amount: cmd.amount,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(transfer)
    }

    /// Approve a transfer. Creator ≠ approver (SoD). Only required when the
    /// transfer is flagged `requires_approval`, but always permitted before
    /// processing.
    pub async fn approve_inter_bank_transfer(
        &self,
        tenant_id: TenantId,
        transfer_id: Uuid,
        approved_by: Uuid,
    ) -> Result<InterBankTransfer, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let transfer = self
            .repo
            .find_transfer(tid, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))?;
        if transfer.status != InterBankTransferStatus::Initiated {
            return Err(TreasuryError::InvalidTransferState(
                transfer.status.to_db_str().to_string(),
                "INITIATED".to_string(),
                "APPROVED".to_string(),
            ));
        }
        if transfer.initiated_by_id == approved_by {
            return Err(TreasuryError::CreatorApproverConflict);
        }

        self.repo
            .update_transfer_status(
                tid,
                transfer_id,
                &InterBankTransferStatus::Approved,
                None,
                None,
                Some(approved_by),
                None,
                None,
                approved_by,
            )
            .await?;
        let updated = self
            .repo
            .find_transfer(tid, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))?;

        self.publish_event(
            tid,
            transfer_id.to_string(),
            TreasuryEventData::InterBankTransferApproved {
                transfer_id: transfer_id.to_string(),
                approved_by: approved_by.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(updated)
    }

    /// Process the transfer: posts the GL contra journal
    /// `DR Bank-B / CR Bank-A` (net zero) via the GL module and marks the
    /// transfer PROCESSED. If approval is required it must have happened.
    pub async fn process_inter_bank_transfer(
        &self,
        tenant_id: TenantId,
        transfer_id: Uuid,
        processed_by: Uuid,
        accounting_period_id: Option<Uuid>,
    ) -> Result<InterBankTransfer, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let transfer = self
            .repo
            .find_transfer(tid, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))?;
        let required_state = if transfer.requires_approval {
            InterBankTransferStatus::Approved
        } else {
            InterBankTransferStatus::Initiated
        };
        if transfer.status != required_state {
            return Err(TreasuryError::InvalidTransferState(
                transfer.status.to_db_str().to_string(),
                required_state.to_db_str().to_string(),
                "PROCESSED".to_string(),
            ));
        }

        let from = self
            .repo
            .find_bank_account(tid, transfer.from_bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(transfer.from_bank_account_id.to_string()))?;
        let to = self
            .repo
            .find_bank_account(tid, transfer.to_bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(transfer.to_bank_account_id.to_string()))?;

        let from_gl = from
            .gl_account_id
            .ok_or_else(|| TreasuryError::GlAccountNotFound(from.bank_account_id.to_string()))?;
        let to_gl = to
            .gl_account_id
            .ok_or_else(|| TreasuryError::GlAccountNotFound(to.bank_account_id.to_string()))?;

        // Available balance re-check at process time
        let available = self.repo.gl_account_balance(tid, from_gl).await?;
        if available < transfer.amount.as_paise() {
            return Err(TreasuryError::InsufficientBalance {
                available,
                required: transfer.amount.as_paise(),
            });
        }

        let ref_id = transfer_id.to_string();
        let lines = vec![
            CreateJournalLineCmd {
                line_number: 1,
                account_id: to_gl,
                debit_amount: Some(transfer.amount),
                credit_amount: None,
                description: Some(format!(
                    "Inter-bank transfer {} → {}",
                    from.account_name, to.account_name
                )),
                cost_center_id: None,
                fund_id: to.fund_id,
                reference_id: Some(ref_id.clone()),
                reference_type: Some("INTER_BANK_TRANSFER".to_string()),
            },
            CreateJournalLineCmd {
                line_number: 2,
                account_id: from_gl,
                debit_amount: None,
                credit_amount: Some(transfer.amount),
                description: Some(format!(
                    "Inter-bank transfer {} → {}",
                    from.account_name, to.account_name
                )),
                cost_center_id: None,
                fund_id: from.fund_id,
                reference_id: Some(ref_id),
                reference_type: Some("INTER_BANK_TRANSFER".to_string()),
            },
        ];

        let journal_id = self
            .post_to_gl(
                tenant_id,
                processed_by,
                "STANDARD",
                from.entity_id.unwrap_or(Uuid::nil()),
                transfer.transfer_date,
                format!(
                    "Inter-bank transfer {} → {} ({})",
                    from.account_name,
                    to.account_name,
                    transfer.amount
                ),
                lines,
                accounting_period_id,
            )
            .await?;

        self.repo
            .update_transfer_status(
                tid,
                transfer_id,
                &InterBankTransferStatus::Processed,
                None,
                Some(journal_id),
                None,
                Some(processed_by),
                None,
                processed_by,
            )
            .await?;

        let updated = self
            .repo
            .find_transfer(tid, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))?;

        self.publish_event(
            tid,
            transfer_id.to_string(),
            TreasuryEventData::InterBankTransferProcessed {
                transfer_id: transfer_id.to_string(),
                journal_id: journal_id.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(updated)
    }

    /// Complete the transfer: record the bank reference, mark COMPLETED.
    pub async fn complete_inter_bank_transfer(
        &self,
        tenant_id: TenantId,
        transfer_id: Uuid,
        bank_reference: String,
        completed_by: Uuid,
    ) -> Result<InterBankTransfer, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let transfer = self
            .repo
            .find_transfer(tid, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))?;
        if transfer.status != InterBankTransferStatus::Processed {
            return Err(TreasuryError::InvalidTransferState(
                transfer.status.to_db_str().to_string(),
                "PROCESSED".to_string(),
                "COMPLETED".to_string(),
            ));
        }
        if bank_reference.trim().is_empty() {
            return Err(TreasuryError::InvalidInput(
                "bank_reference (UTR) is required to complete a transfer".to_string(),
            ));
        }

        self.repo
            .update_transfer_status(
                tid,
                transfer_id,
                &InterBankTransferStatus::Completed,
                Some(&bank_reference),
                None,
                None,
                None,
                None,
                completed_by,
            )
            .await?;
        let updated = self
            .repo
            .find_transfer(tid, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))?;

        self.publish_event(
            tid,
            transfer_id.to_string(),
            TreasuryEventData::InterBankTransferCompleted {
                transfer_id: transfer_id.to_string(),
                from_account: transfer.from_bank_account_id.to_string(),
                to_account: transfer.to_bank_account_id.to_string(),
                amount: transfer.amount.as_paise(),
                bank_reference,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(updated)
    }

    /// Cancel an INITIATED / APPROVED transfer.
    pub async fn cancel_inter_bank_transfer(
        &self,
        tenant_id: TenantId,
        transfer_id: Uuid,
        reason: String,
        cancelled_by: Uuid,
    ) -> Result<InterBankTransfer, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let transfer = self
            .repo
            .find_transfer(tid, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))?;
        if transfer.status != InterBankTransferStatus::Initiated
            && transfer.status != InterBankTransferStatus::Approved
        {
            return Err(TreasuryError::InvalidTransferState(
                transfer.status.to_db_str().to_string(),
                "INITIATED/APPROVED".to_string(),
                "CANCELLED".to_string(),
            ));
        }
        self.repo
            .update_transfer_status(
                tid,
                transfer_id,
                &InterBankTransferStatus::Cancelled,
                None,
                None,
                None,
                None,
                Some(&reason),
                cancelled_by,
            )
            .await?;
        let updated = self
            .repo
            .find_transfer(tid, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))?;
        self.publish_event(
            tid,
            transfer_id.to_string(),
            TreasuryEventData::InterBankTransferCancelled {
                transfer_id: transfer_id.to_string(),
                reason,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(updated)
    }

    /// Mark a processed transfer FAILED (bank rejection). The GL contra
    /// journal must be reversed by the caller (or a compensating journal
    /// posted); treasury records the failure reason.
    pub async fn fail_inter_bank_transfer(
        &self,
        tenant_id: TenantId,
        transfer_id: Uuid,
        reason: String,
        failed_by: Uuid,
    ) -> Result<InterBankTransfer, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let transfer = self
            .repo
            .find_transfer(tid, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))?;
        if transfer.status != InterBankTransferStatus::Processed {
            return Err(TreasuryError::InvalidTransferState(
                transfer.status.to_db_str().to_string(),
                "PROCESSED".to_string(),
                "FAILED".to_string(),
            ));
        }
        self.repo
            .update_transfer_status(
                tid,
                transfer_id,
                &InterBankTransferStatus::Failed,
                None,
                None,
                None,
                None,
                Some(&reason),
                failed_by,
            )
            .await?;
        let updated = self
            .repo
            .find_transfer(tid, transfer_id)
            .await?
            .ok_or_else(|| TreasuryError::TransferNotFound(transfer_id.to_string()))?;
        Ok(updated)
    }

    // ── Petty cash ──────────────────────────────────────────────────

    /// Create a petty-cash fund. Single ACTIVE fund per entity (assumption),
    /// imprest capped by `petty_cash_imprest_limit` (default ₹25,000).
    pub async fn create_petty_cash_fund(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        entity_id: Uuid,
        fund_name: String,
        imprest_amount: i64,
        custodian_id: Uuid,
    ) -> Result<PettyCashFund, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let policy = self.repo.load_policy(tid).await?;

        if imprest_amount <= 0 {
            return Err(TreasuryError::InvalidInput(
                "imprest amount must be positive".to_string(),
            ));
        }
        if imprest_amount > policy.petty_cash_imprest_limit {
            return Err(TreasuryError::ImprestLimitExceeded(
                imprest_amount,
                policy.petty_cash_imprest_limit,
            ));
        }
        if self.repo.has_active_fund_for_entity(tid, entity_id).await? {
            return Err(TreasuryError::ActivePettyCashFundExists(
                entity_id.to_string(),
            ));
        }

        let cash_gl = self
            .repo
            .find_account_by_code(tid, &policy.cash_gl_code)
            .await?
            .ok_or_else(|| {
                TreasuryError::Config(format!(
                    "cash GL account {} not found in chart of accounts",
                    policy.cash_gl_code
                ))
            })?;

        let fund = PettyCashFund {
            petty_cash_fund_id: EntityId::new(),
            tenant_id,
            entity_id,
            fund_name,
            cash_gl_account_id: cash_gl,
            imprest_amount: Money::from_paise(imprest_amount),
            balance: Money::ZERO,
            custodian_id,
            status: PettyCashFundStatus::Active,
            version: 1,
            audit: AuditInfo::new(created_by),
        };
        self.repo.insert_petty_cash_fund(&fund).await?;
        Ok(fund)
    }

    /// Top up a petty-cash fund: DR Cash-in-hand / CR Bank (spec GL rule).
    pub async fn top_up_petty_cash(
        &self,
        tenant_id: TenantId,
        fund_id: Uuid,
        actor: Uuid,
        cmd: TopUpPettyCashCmd,
        accounting_period_id: Option<Uuid>,
    ) -> Result<PettyCashFund, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let fund = self
            .repo
            .find_petty_cash_fund(tid, fund_id)
            .await?
            .ok_or_else(|| TreasuryError::PettyCashFundNotFound(fund_id.to_string()))?;
        if fund.status != PettyCashFundStatus::Active {
            return Err(TreasuryError::PettyCashFundInactive(fund_id.to_string()));
        }
        if cmd.amount <= 0 {
            return Err(TreasuryError::InvalidInput(
                "top-up amount must be positive".to_string(),
            ));
        }

        let bank = self
            .repo
            .find_bank_account(tid, cmd.bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(cmd.bank_account_id.to_string()))?;
        let bank_gl = bank
            .gl_account_id
            .ok_or_else(|| TreasuryError::GlAccountNotFound(bank.bank_account_id.to_string()))?;

        let txn_id = Uuid::now_v7();
        let lines = vec![
            CreateJournalLineCmd {
                line_number: 1,
                account_id: fund.cash_gl_account_id,
                debit_amount: Some(Money::from_paise(cmd.amount)),
                credit_amount: None,
                description: Some(format!("Petty cash top-up — {}", fund.fund_name)),
                cost_center_id: None,
                fund_id: None,
                reference_id: Some(txn_id.to_string()),
                reference_type: Some("PETTY_CASH_TOP_UP".to_string()),
            },
            CreateJournalLineCmd {
                line_number: 2,
                account_id: bank_gl,
                debit_amount: None,
                credit_amount: Some(Money::from_paise(cmd.amount)),
                description: Some(format!("Petty cash top-up — {}", fund.fund_name)),
                cost_center_id: None,
                fund_id: None,
                reference_id: Some(txn_id.to_string()),
                reference_type: Some("PETTY_CASH_TOP_UP".to_string()),
            },
        ];
        let journal_id = self
            .post_to_gl(
                tenant_id,
                actor,
                "STANDARD",
                fund.entity_id,
                Utc::now().date_naive(),
                format!("Petty cash top-up — {} — ₹{}", fund.fund_name, cmd.amount),
                lines,
                accounting_period_id,
            )
            .await?;

        let txn = PettyCashTransaction {
            petty_cash_txn_id: EntityId::from_uuid(txn_id),
            tenant_id,
            petty_cash_fund_id: fund_id,
            transaction_type: PettyCashTransactionType::TopUp,
            amount: Money::from_paise(cmd.amount),
            account_id: None,
            voucher_no: None,
            narration: Some(format!("Top-up from {}", bank.account_name)),
            journal_id: Some(journal_id),
            transaction_date: Utc::now().date_naive(),
            recorded_by_id: actor,
            version: 1,
            audit: AuditInfo::new(actor),
        };
        self.repo.insert_petty_cash_txn(&txn).await?;

        let new_balance = fund.balance.as_paise() + cmd.amount;
        self.repo
            .update_petty_cash_balance(tid, fund_id, new_balance, actor)
            .await?;

        self.publish_event(
            tid,
            fund_id.to_string(),
            TreasuryEventData::PettyCashTopUp {
                fund_id: fund_id.to_string(),
                amount: cmd.amount,
                occurred_at: Utc::now(),
            },
        )
        .await?;

        let updated = self
            .repo
            .find_petty_cash_fund(tid, fund_id)
            .await?
            .ok_or_else(|| TreasuryError::PettyCashFundNotFound(fund_id.to_string()))?;
        Ok(updated)
    }

    /// Record a petty-cash expense: expense ≤ current balance; every expense
    /// carries an expense account + voucher (internal control, spec rule 9);
    /// posts DR Expense / CR Cash.
    pub async fn record_petty_cash_expense(
        &self,
        tenant_id: TenantId,
        fund_id: Uuid,
        actor: Uuid,
        cmd: RecordPettyCashExpenseCmd,
        accounting_period_id: Option<Uuid>,
    ) -> Result<PettyCashFund, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let fund = self
            .repo
            .find_petty_cash_fund(tid, fund_id)
            .await?
            .ok_or_else(|| TreasuryError::PettyCashFundNotFound(fund_id.to_string()))?;
        if fund.status != PettyCashFundStatus::Active {
            return Err(TreasuryError::PettyCashFundInactive(fund_id.to_string()));
        }
        if cmd.voucher_no.trim().is_empty() {
            return Err(TreasuryError::ExpenseVoucherRequired);
        }
        if cmd.amount <= 0 {
            return Err(TreasuryError::InvalidInput(
                "expense amount must be positive".to_string(),
            ));
        }
        if cmd.amount > fund.balance.as_paise() {
            return Err(TreasuryError::InsufficientPettyCashBalance(
                cmd.amount,
                fund.balance.as_paise(),
            ));
        }

        // Expense account must exist and be an EXPENSE-type leaf (GL create
        // will enforce the leaf part).
        let expense_account = self
            .repo
            .find_gl_account(tid, cmd.expense_account_id)
            .await?
            .ok_or_else(|| TreasuryError::GlAccountNotFound(cmd.expense_account_id.to_string()))?;
        if expense_account.account_type != sutra_finance_gl::AccountType::Expense {
            return Err(TreasuryError::InvalidInput(format!(
                "account {} is not an expense account",
                expense_account.account_code
            )));
        }

        let txn_id = Uuid::now_v7();
        let lines = vec![
            CreateJournalLineCmd {
                line_number: 1,
                account_id: cmd.expense_account_id,
                debit_amount: Some(Money::from_paise(cmd.amount)),
                credit_amount: None,
                description: Some(format!("Petty cash expense — voucher {}", cmd.voucher_no)),
                cost_center_id: None,
                fund_id: None,
                reference_id: Some(txn_id.to_string()),
                reference_type: Some("PETTY_CASH_EXPENSE".to_string()),
            },
            CreateJournalLineCmd {
                line_number: 2,
                account_id: fund.cash_gl_account_id,
                debit_amount: None,
                credit_amount: Some(Money::from_paise(cmd.amount)),
                description: Some(format!("Petty cash expense — voucher {}", cmd.voucher_no)),
                cost_center_id: None,
                fund_id: None,
                reference_id: Some(txn_id.to_string()),
                reference_type: Some("PETTY_CASH_EXPENSE".to_string()),
            },
        ];
        let journal_id = self
            .post_to_gl(
                tenant_id,
                actor,
                "STANDARD",
                fund.entity_id,
                Utc::now().date_naive(),
                format!("Petty cash expense — {} — ₹{}", cmd.voucher_no, cmd.amount),
                lines,
                accounting_period_id,
            )
            .await?;

        let txn = PettyCashTransaction {
            petty_cash_txn_id: EntityId::from_uuid(txn_id),
            tenant_id,
            petty_cash_fund_id: fund_id,
            transaction_type: PettyCashTransactionType::Expense,
            amount: Money::from_paise(cmd.amount),
            account_id: Some(cmd.expense_account_id),
            voucher_no: Some(cmd.voucher_no.clone()),
            narration: cmd.narration.clone(),
            journal_id: Some(journal_id),
            transaction_date: Utc::now().date_naive(),
            recorded_by_id: actor,
            version: 1,
            audit: AuditInfo::new(actor),
        };
        self.repo.insert_petty_cash_txn(&txn).await?;

        let new_balance = fund.balance.as_paise() - cmd.amount;
        self.repo
            .update_petty_cash_balance(tid, fund_id, new_balance, actor)
            .await?;

        self.publish_event(
            tid,
            fund_id.to_string(),
            TreasuryEventData::PettyCashExpenseRecorded {
                fund_id: fund_id.to_string(),
                voucher_no: cmd.voucher_no.clone(),
                amount: cmd.amount,
                account_id: cmd.expense_account_id.to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;

        let updated = self
            .repo
            .find_petty_cash_fund(tid, fund_id)
            .await?
            .ok_or_else(|| TreasuryError::PettyCashFundNotFound(fund_id.to_string()))?;
        Ok(updated)
    }

    /// Close a petty-cash fund (custodian change / zero balance).
    pub async fn close_petty_cash_fund(
        &self,
        tenant_id: TenantId,
        fund_id: Uuid,
        actor: Uuid,
    ) -> Result<PettyCashFund, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let fund = self
            .repo
            .find_petty_cash_fund(tid, fund_id)
            .await?
            .ok_or_else(|| TreasuryError::PettyCashFundNotFound(fund_id.to_string()))?;
        if fund.status != PettyCashFundStatus::Active {
            return Err(TreasuryError::PettyCashFundInactive(fund_id.to_string()));
        }
        self.repo.close_petty_cash_fund(tid, fund_id, actor).await?;
        let updated = self
            .repo
            .find_petty_cash_fund(tid, fund_id)
            .await?
            .ok_or_else(|| TreasuryError::PettyCashFundNotFound(fund_id.to_string()))?;
        Ok(updated)
    }

    // ── Gateways ────────────────────────────────────────────────────

    /// Configure a payment gateway. Secrets are encrypted at rest; the
    /// config is upserted per (entity, gateway).
    pub async fn configure_gateway(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: ConfigureGatewayCmd,
    ) -> Result<PaymentGatewayConfig, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let gateway_type = GatewayType::from_db_str(&cmd.gateway_type).ok_or_else(|| {
            TreasuryError::InvalidInput(format!("unknown gateway type {}", cmd.gateway_type))
        })?;

        // Credential validation (sandbox round-trip requires per-gateway
        // SDKs; structural validation is enforced here).
        if cmd.merchant_id.trim().is_empty()
            || cmd.api_key.trim().is_empty()
            || cmd.api_secret.trim().is_empty()
        {
            return Err(TreasuryError::InvalidInput(
                "merchant_id, api_key and api_secret are required".to_string(),
            ));
        }
        if cmd.api_key.len() < 8 || cmd.api_secret.len() < 8 {
            return Err(TreasuryError::InvalidInput(
                "api_key and api_secret look too short to be valid credentials".to_string(),
            ));
        }

        let config = PaymentGatewayConfig {
            payment_gateway_config_id: EntityId::new(),
            tenant_id,
            entity_id: Some(cmd.entity_id),
            gateway_type,
            merchant_id: cmd.merchant_id.trim().to_string(),
            api_key: encrypt_secret(cmd.api_key.trim()),
            api_secret: encrypt_secret(cmd.api_secret.trim()),
            webhook_secret: cmd.webhook_secret.as_deref().map(|s| encrypt_secret(s.trim())),
            is_active: cmd.is_active,
            version: 1,
            audit: AuditInfo::new(created_by),
        };
        self.repo.upsert_gateway_config(&config).await?;

        let stored = self
            .repo
            .find_gateway_config(tid, cmd.entity_id, gateway_type.to_db_str())
            .await?
            .ok_or_else(|| TreasuryError::GatewayConfigNotFound(cmd.entity_id.to_string()))?;

        self.publish_event(
            tid,
            cmd.entity_id.to_string(),
            TreasuryEventData::GatewayConfigured {
                entity_id: cmd.entity_id.to_string(),
                gateway_type: gateway_type.to_db_str().to_string(),
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(stored)
    }

    /// Reconcile a gateway settlement against AR `payment_gateway_transactions`
    /// for the date. `settled_amount` must equal matched − fees; variance
    /// flips the settlement to EXCEPTION.
    pub async fn reconcile_gateway_settlement(
        &self,
        tenant_id: TenantId,
        actor: Uuid,
        cmd: ReconcileGatewaySettlementCmd,
    ) -> Result<GatewaySettlement, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let gateway_type = GatewayType::from_db_str(&cmd.gateway_type).ok_or_else(|| {
            TreasuryError::InvalidInput(format!("unknown gateway type {}", cmd.gateway_type))
        })?;

        let (matched_amount, fee_total) = self
            .repo
            .gateway_txn_totals_for_date(
                tid,
                cmd.entity_id,
                gateway_type.to_db_str(),
                cmd.settlement_date,
            )
            .await?;

        let (status, reason) = if matched_amount != cmd.settled_amount {
            (
                GatewaySettlementStatus::Exception,
                Some(format!(
                    "settled {} ≠ matched {} (net of fees {})",
                    cmd.settled_amount, matched_amount, fee_total
                )),
            )
        } else {
            (GatewaySettlementStatus::Reconciled, None)
        };

        let settlement = GatewaySettlement {
            gateway_settlement_id: EntityId::new(),
            tenant_id,
            entity_id: cmd.entity_id,
            gateway_type,
            settlement_date: cmd.settlement_date,
            settled_amount: Money::from_paise(cmd.settled_amount),
            gateway_fee_amount: Money::from_paise(cmd.gateway_fee_amount),
            matched_amount: Money::from_paise(matched_amount),
            status,
            exception_reason: reason,
            reconciled_by_id: Some(actor),
            reconciled_at: Some(Utc::now()),
            version: 1,
            audit: AuditInfo::new(actor),
        };
        self.repo.upsert_gateway_settlement(&settlement).await?;

        let stored = self
            .repo
            .find_gateway_settlement(
                tid,
                cmd.entity_id,
                gateway_type.to_db_str(),
                cmd.settlement_date,
            )
            .await?
            .ok_or_else(|| {
                TreasuryError::GatewaySettlementNotFound(
                    cmd.entity_id.to_string(),
                    gateway_type.to_db_str().to_string(),
                    cmd.settlement_date.to_string(),
                )
            })?;

        self.publish_event(
            tid,
            cmd.entity_id.to_string(),
            TreasuryEventData::GatewaySettlementReconciled {
                entity_id: cmd.entity_id.to_string(),
                gateway_type: gateway_type.to_db_str().to_string(),
                settlement_date: cmd.settlement_date.to_string(),
                settled_amount: cmd.settled_amount,
                fee_amount: cmd.gateway_fee_amount,
                occurred_at: Utc::now(),
            },
        )
        .await?;
        Ok(stored)
    }

    // ── Internal helpers ────────────────────────────────────────────

    async fn require_bank_account(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<(), TreasuryError> {
        let acc = self
            .repo
            .find_bank_account(tenant_id, bank_account_id)
            .await?
            .ok_or_else(|| TreasuryError::BankAccountNotFound(bank_account_id.to_string()))?;
        if !acc.is_active {
            return Err(TreasuryError::BankAccountInactive(
                bank_account_id.to_string(),
            ));
        }
        Ok(())
    }

    /// Post a journal through the GL module (create + post) with the given
    /// lines. Returns the posted journal id. All lines already carry
    /// reference_type / reference_id for traceability.
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
    ) -> Result<Uuid, TreasuryError> {
        let tid = *tenant_id.as_uuid();
        let period_id = match accounting_period_id {
            Some(pid) => {
                let open = PgPeriodRepository::new(self.pool.clone())
                    .is_open(tid, pid)
                    .await
                    .map_err(|e| TreasuryError::Gl(e.to_string()))?;
                if !open {
                    return Err(TreasuryError::Gl(
                        "accounting period is closed — cannot post".to_string(),
                    ));
                }
                pid
            }
            None => {
                let period = PgPeriodRepository::new(self.pool.clone())
                    .get_current(tid)
                    .await
                    .map_err(|e| TreasuryError::Gl(e.to_string()))?
                    .ok_or(TreasuryError::NoOpenPeriod)?;
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
            .map_err(|e| TreasuryError::Gl(e.to_string()))?;
        let journal_id = *journal.journal_id.as_uuid();
        gl.post_journal(
            tenant_id,
            PostJournalCmd {
                journal_id,
                posted_by: created_by,
            },
        )
        .await
        .map_err(|e| TreasuryError::Gl(e.to_string()))?;
        Ok(journal_id)
    }

    async fn candidate_amount(
        &self,
        tenant_id: Uuid,
        transaction_id: Uuid,
        transaction_type: &str,
    ) -> Result<i64, TreasuryError> {
        match transaction_type {
            "PaymentReceipt" => {
                let row: Option<(i64,)> = sqlx::query_as(
                    "SELECT amount FROM ar_payment_receipts WHERE tenant_id = $1 AND payment_receipt_id = $2",
                )
                .bind(tenant_id)
                .bind(transaction_id)
                .fetch_optional(&self.pool)
                .await?;
                row.map(|r| r.0)
                    .ok_or_else(|| TreasuryError::MatchTransactionNotFound(transaction_id.to_string()))
            }
            "VendorPayment" => {
                let row: Option<(Option<i64>, i64)> = sqlx::query_as(
                    "SELECT net_amount, amount FROM vendor_payments WHERE tenant_id = $1 AND payment_id = $2",
                )
                .bind(tenant_id)
                .bind(transaction_id)
                .fetch_optional(&self.pool)
                .await?;
                row.map(|(net, amount)| net.unwrap_or(amount))
                    .ok_or_else(|| TreasuryError::MatchTransactionNotFound(transaction_id.to_string()))
            }
            _ => {
                let row: Option<(Option<i64>, Option<i64>)> = sqlx::query_as(
                    "SELECT debit_amount, credit_amount FROM bank_transactions WHERE tenant_id = $1 AND bank_transaction_id = $2",
                )
                .bind(tenant_id)
                .bind(transaction_id)
                .fetch_optional(&self.pool)
                .await?;
                row.map(|(d, c)| d.or(c).unwrap_or(0))
                    .ok_or_else(|| TreasuryError::MatchTransactionNotFound(transaction_id.to_string()))
            }
        }
    }

    async fn publish_event(
        &self,
        tenant_id: Uuid,
        aggregate_id: String,
        event: TreasuryEventData,
    ) -> Result<(), TreasuryError> {
        let mut tx = self.pool.begin().await?;
        write_outbox(&mut tx, tenant_id, &aggregate_id, &event).await?;
        tx.commit().await?;
        Ok(())
    }
}

// ─── Parsers ──────────────────────────────────────────────────────────

/// A parsed statement line before it becomes a domain row.
#[derive(Debug, Clone)]
pub struct ParsedStatementLine {
    pub transaction_date: NaiveDate,
    pub transaction_ref: Option<String>,
    pub description: Option<String>,
    pub debit_amount: Option<Money>,
    pub credit_amount: Option<Money>,
}

/// Parse a statement in CSV or OFX format. CSV format:
/// `date,ref,description,debit,credit` (ISO date, paise integers, empty side).
/// OFX: minimal `<STMTTRN>` scan for DTPOSTED/TRNTYPE/FITID/TRNAMT/NAME/MEMO.
pub fn parse_statement(format: &str, content: &str) -> Result<Vec<ParsedStatementLine>, String> {
    match format.to_uppercase().as_str() {
        "CSV" => parse_csv(content),
        "OFX" => parse_ofx(content),
        other => Err(format!("unsupported statement format: {other}")),
    }
}

fn parse_csv(content: &str) -> Result<Vec<ParsedStatementLine>, String> {
    let mut out = Vec::new();
    let mut reader = csv_reader(content);
    // Skip a header row when the first line looks like a header.
    if let Some(first) = reader.first().cloned() {
        let looks_header = first
            .iter()
            .any(|f| f.trim().to_ascii_lowercase().contains("date"));
        if !looks_header {
            push_csv_line(&mut out, &first)?;
        }
    }
    for row in reader.skip(1) {
        push_csv_line(&mut out, &row)?;
    }
    if out.is_empty() {
        return Err("CSV contains no data rows".to_string());
    }
    Ok(out)
}

fn push_csv_line(out: &mut Vec<ParsedStatementLine>, fields: &[String]) -> Result<(), String> {
    if fields.len() < 5 {
        return Err(format!(
            "CSV row must have 5 columns (date,ref,description,debit,credit); got {}",
            fields.len()
        ));
    }
    let date = NaiveDate::parse_from_str(fields[0].trim(), "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(fields[0].trim(), "%d/%m/%Y"))
        .map_err(|e| format!("invalid date '{}': {e}", fields[0]))?;
    let debit = parse_amount(&fields[3])?;
    let credit = parse_amount(&fields[4])?;
    if debit.is_some() && credit.is_some() {
        return Err("debit and credit cannot both be set on one line".to_string());
    }
    if debit.is_none() && credit.is_none() {
        return Err("line has neither debit nor credit amount".to_string());
    }
    let ref_ = fields[1].trim();
    let desc = fields[2].trim();
    out.push(ParsedStatementLine {
        transaction_date: date,
        transaction_ref: if ref_.is_empty() { None } else { Some(ref_.to_string()) },
        description: if desc.is_empty() { None } else { Some(desc.to_string()) },
        debit_amount: debit,
        credit_amount: credit,
    });
    Ok(())
}

fn parse_amount(s: &str) -> Result<Option<Money>, String> {
    let t = s.trim();
    if t.is_empty() || t == "0" || t == "0.00" || t == "0.0" {
        return Ok(None);
    }
    let paise = if let Ok(v) = t.parse::<i64>() {
        v // already paise
    } else if let Ok(v) = t.parse::<f64>() {
        (v * 100.0).round() as i64
    } else {
        return Err(format!("invalid amount '{}'", s));
    };
    if paise < 0 {
        return Err(format!("amount must be non-negative, got '{}'", s));
    }
    Ok(Some(Money::from_paise(paise)))
}

/// Minimal OFX statement parser. Extracts `<STMTTRN>` records and treats
/// TRNTYPE DEBIT as money out. Handles both OFX XML and QFX.
fn parse_ofx(content: &str) -> Result<Vec<ParsedStatementLine>, String> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let mut found = 0usize;
    while let Some(start) = content[pos..].find("<STMTTRN>") {
        found += 1;
        let start = pos + start;
        let end = content[start..]
            .find("</STMTTRN>")
            .map(|e| start + e)
            .ok_or_else(|| "unterminated <STMTTRN>".to_string())?;
        let block = &content[start + "<STMTTRN>".len()..end];
        pos = end + "</STMTTRN>".len();

        let tag = |name: &str| -> Option<String> {
            let open = format!("<{name}>");
            let close = format!("</{name}>");
            let s = block.find(&open)? + open.len();
            let e = block[s..].find(&close)? + s;
            Some(block[s..e].trim().to_string())
        };

        let dtposted = tag("DTPOSTED").unwrap_or_default();
        // OFX timestamps: YYYYMMDDHHMMSS[.XXX][gmt-offset]
        let date_str = dtposted.get(0..8).unwrap_or("");
        let date = NaiveDate::parse_from_str(date_str, "%Y%m%d")
            .map_err(|e| format!("invalid DTPOSTED '{}': {e}", dtposted))?;
        let trnamt = tag("TRNAMT").unwrap_or_default();
        let amount = trnamt
            .parse::<f64>()
            .map_err(|e| format!("invalid TRNAMT '{}': {e}", trnamt))?;
        let amount_paise = (amount.abs() * 100.0).round() as i64;
        let is_debit = amount < 0.0;
        let fitid = tag("FITID");
        let name = tag("NAME");
        let memo = tag("MEMO");

        let mut description = name.clone();
        if let Some(m) = memo {
            if !m.is_empty() {
                description = Some(match description {
                    Some(n) if !n.is_empty() => format!("{n} — {m}"),
                    _ => m,
                });
            }
        }

        out.push(ParsedStatementLine {
            transaction_date: date,
            transaction_ref: fitid.filter(|s| !s.is_empty()),
            description: description.filter(|s| !s.is_empty()),
            debit_amount: if is_debit {
                Some(Money::from_paise(amount_paise))
            } else {
                None
            },
            credit_amount: if is_debit {
                None
            } else {
                Some(Money::from_paise(amount_paise))
            },
        });
    }
    if found == 0 {
        return Err("OFX contains no <STMTTRN> records".to_string());
    }
    Ok(out)
}

/// Minimal RFC-4180 CSV row splitter (no external dep).
fn csv_reader(content: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut field = String::new();
    let mut row: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                row.push(std::mem::take(&mut field));
            }
            '\n' if !in_quotes => {
                row.push(std::mem::take(&mut field));
                if !(row.len() == 1 && row[0].trim().is_empty()) {
                    rows.push(std::mem::take(&mut row));
                }
            }
            '\r' if !in_quotes => {}
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        if !(row.len() == 1 && row[0].trim().is_empty()) {
            rows.push(row);
        }
    }
    rows
}

// ─── Validators ───────────────────────────────────────────────────────

/// Indian IFSC: 4 alpha + 0 + 6 alphanumeric (11 chars).
pub fn valid_ifsc(ifsc: &str) -> bool {
    let b = ifsc.as_bytes();
    if b.len() != 11 {
        return false;
    }
    b[0].is_ascii_alphabetic()
        && b[1].is_ascii_alphabetic()
        && b[2].is_ascii_alphabetic()
        && b[3].is_ascii_alphabetic()
        && b[4] == b'0'
        && b[5..].iter().all(|c| c.is_ascii_alphanumeric())
}

pub fn valid_account_number(account: &str) -> bool {
    let len = account.len();
    (9..=18).contains(&len) && account.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Normalized-token description similarity for auto-match rule 3.
fn description_similar(a: &str, b: &str) -> bool {
    let norm = |s: &str| -> Vec<String> {
        s.to_ascii_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect()
    };
    let ta = norm(a);
    let tb = norm(b);
    if ta.is_empty() || tb.is_empty() {
        return false;
    }
    let shorter = ta.len().min(tb.len());
    if shorter == 0 {
        return false;
    }
    let overlap = ta
        .iter()
        .filter(|t| tb.contains(t))
        .count();
    // At least half the tokens of the shorter side overlap.
    overlap * 2 >= shorter
}

// keep Datelike import meaningful (used in fiscal-year helpers elsewhere)
#[allow(dead_code)]
fn _fiscal_year(date: NaiveDate) -> String {
    if date.month() >= 4 {
        format!("{}-{:02}", date.year(), (date.year() + 1) % 100)
    } else {
        format!("{}-{:02}", date.year() - 1, date.year() % 100)
    }
}

// Re-exported for tests and external validators.
#[allow(unused_imports)]
use crate::repository::TreasuryRepository as _Repo;
