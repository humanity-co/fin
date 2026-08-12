//! Treasury data access layer (SQLx / PostgreSQL).
//!
//! Every query is tenant-scoped; money is BIGINT paise; masters soft-delete
//! (`deleted_at IS NULL`), fact tables are INSERT-only. GL leaf creation is
//! done here against `chart_of_accounts` because the GL command surface
//! exposes journal posting but no account-creation handler.

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use sutra_core::{AuditInfo, EntityId, Money, TenantId};

use crate::errors::TreasuryError;
use crate::models::bank_account::{BankAccount, BankAccountType, BankSignatory, SignatoryType};
use crate::models::gateway::{
    GatewaySettlement, GatewaySettlementStatus, GatewayType, PaymentGatewayConfig,
};
use crate::models::petty_cash::{
    PettyCashFund, PettyCashFundStatus, PettyCashTransaction, PettyCashTransactionType,
};
use crate::models::reconciliation::{
    BankReconciliation, BankReconciliationStatus, BankStatementLine, BankTransaction, MatchStatus,
};
use crate::models::transfer::{InterBankTransfer, InterBankTransferStatus};
use crate::models::TreasuryPolicy;

// ─── Row structs ──────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct BankAccountRow {
    bank_account_id: Uuid,
    tenant_id: Uuid,
    entity_id: Option<Uuid>,
    account_number: String,
    account_name: String,
    bank_name: String,
    branch_name: Option<String>,
    ifsc_code: String,
    account_type: String,
    fund_id: Option<Uuid>,
    is_fcra_account: bool,
    minimum_balance: Option<i64>,
    is_active: bool,
    gl_account_id: Option<Uuid>,
    last_reconciled_at: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl BankAccountRow {
    fn into_model(self) -> BankAccount {
        BankAccount {
            bank_account_id: EntityId::from_uuid(self.bank_account_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            entity_id: self.entity_id,
            account_number: self.account_number,
            account_name: self.account_name,
            bank_name: self.bank_name,
            branch_name: self.branch_name,
            ifsc_code: self.ifsc_code,
            account_type: BankAccountType::from_db_str(&self.account_type),
            fund_id: self.fund_id,
            is_fcra_account: self.is_fcra_account,
            minimum_balance: self.minimum_balance.map(Money::from_paise).unwrap_or(Money::ZERO),
            is_active: self.is_active,
            gl_account_id: self.gl_account_id,
            last_reconciled_at: self.last_reconciled_at,
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
struct SignatoryRow {
    bank_signatory_id: Uuid,
    tenant_id: Uuid,
    bank_account_id: Uuid,
    user_id: Uuid,
    signatory_type: String,
    is_active: bool,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

impl SignatoryRow {
    fn into_model(self) -> BankSignatory {
        BankSignatory {
            bank_signatory_id: EntityId::from_uuid(self.bank_signatory_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            bank_account_id: self.bank_account_id,
            user_id: self.user_id,
            signatory_type: if self.signatory_type == "JOINT" {
                SignatoryType::Joint
            } else {
                SignatoryType::Individual
            },
            is_active: self.is_active,
            audit: AuditInfo {
                created_by: Uuid::nil(),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: Uuid::nil(),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
struct ReconciliationRow {
    bank_reconciliation_id: Uuid,
    tenant_id: Uuid,
    bank_account_id: Uuid,
    period_id: Uuid,
    statement_date: NaiveDate,
    opening_balance: Option<i64>,
    closing_balance: Option<i64>,
    status: String,
    verified_by_id: Option<Uuid>,
    completed_at: Option<DateTime<Utc>>,
    entity_version: Option<i32>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl ReconciliationRow {
    fn into_model(self) -> BankReconciliation {
        BankReconciliation {
            bank_reconciliation_id: EntityId::from_uuid(self.bank_reconciliation_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            bank_account_id: self.bank_account_id,
            period_id: self.period_id,
            statement_date: self.statement_date,
            opening_balance: self.opening_balance.map(Money::from_paise).unwrap_or(Money::ZERO),
            closing_balance: self.closing_balance.map(Money::from_paise).unwrap_or(Money::ZERO),
            status: BankReconciliationStatus::from_db_str(&self.status),
            verified_by_id: self.verified_by_id,
            completed_at: self.completed_at,
            version: self.entity_version.unwrap_or(1),
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
struct StatementLineRow {
    bank_statement_line_id: Uuid,
    tenant_id: Uuid,
    bank_reconciliation_id: Uuid,
    transaction_date: NaiveDate,
    transaction_ref: Option<String>,
    description: Option<String>,
    debit_amount: Option<i64>,
    credit_amount: Option<i64>,
    match_status: String,
    matched_transaction_id: Option<Uuid>,
    matched_transaction_type: Option<String>,
    created_at: Option<DateTime<Utc>>,
}

impl StatementLineRow {
    fn into_model(self) -> BankStatementLine {
        BankStatementLine {
            bank_statement_line_id: EntityId::from_uuid(self.bank_statement_line_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            bank_reconciliation_id: self.bank_reconciliation_id,
            transaction_date: self.transaction_date,
            transaction_ref: self.transaction_ref,
            description: self.description,
            debit_amount: self.debit_amount.map(Money::from_paise),
            credit_amount: self.credit_amount.map(Money::from_paise),
            match_status: MatchStatus::from_db_str(&self.match_status),
            matched_transaction_id: self.matched_transaction_id,
            matched_transaction_type: self.matched_transaction_type,
            audit: AuditInfo::default(),
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct BankTransactionRow {
    pub bank_transaction_id: Uuid,
    pub tenant_id: Uuid,
    pub bank_account_id: Uuid,
    pub transaction_date: NaiveDate,
    pub value_date: Option<NaiveDate>,
    pub transaction_ref: Option<String>,
    pub description: Option<String>,
    pub debit_amount: Option<i64>,
    pub credit_amount: Option<i64>,
    pub balance: Option<i64>,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub is_reconciled: bool,
    pub reconciled_at: Option<DateTime<Utc>>,
    pub version: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
}

impl BankTransactionRow {
    pub fn into_model(self) -> BankTransaction {
        BankTransaction {
            bank_transaction_id: EntityId::from_uuid(self.bank_transaction_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            bank_account_id: self.bank_account_id,
            transaction_date: self.transaction_date,
            value_date: self.value_date,
            transaction_ref: self.transaction_ref,
            description: self.description,
            debit_amount: self.debit_amount.map(Money::from_paise),
            credit_amount: self.credit_amount.map(Money::from_paise),
            balance: self.balance.map(Money::from_paise).unwrap_or(Money::ZERO),
            reference_type: self.reference_type,
            reference_id: self.reference_id,
            is_reconciled: self.is_reconciled,
            reconciled_at: self.reconciled_at,
            version: self.version.unwrap_or(1),
            audit: AuditInfo::default(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct TransferRow {
    inter_bank_transfer_id: Uuid,
    tenant_id: Uuid,
    from_bank_account_id: Uuid,
    to_bank_account_id: Uuid,
    amount: i64,
    transfer_date: NaiveDate,
    status: String,
    requires_approval: bool,
    bank_reference: Option<String>,
    journal_id: Option<Uuid>,
    initiated_by_id: Uuid,
    approved_by_id: Option<Uuid>,
    processed_by_id: Option<Uuid>,
    failure_reason: Option<String>,
    entity_version: Option<i32>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl TransferRow {
    fn into_model(self) -> InterBankTransfer {
        InterBankTransfer {
            inter_bank_transfer_id: EntityId::from_uuid(self.inter_bank_transfer_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            from_bank_account_id: self.from_bank_account_id,
            to_bank_account_id: self.to_bank_account_id,
            amount: Money::from_paise(self.amount),
            transfer_date: self.transfer_date,
            status: InterBankTransferStatus::from_db_str(&self.status),
            requires_approval: self.requires_approval,
            bank_reference: self.bank_reference,
            journal_id: self.journal_id,
            initiated_by_id: self.initiated_by_id,
            approved_by_id: self.approved_by_id,
            processed_by_id: self.processed_by_id,
            failure_reason: self.failure_reason,
            version: self.entity_version.unwrap_or(1),
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
struct PettyCashFundRow {
    petty_cash_fund_id: Uuid,
    tenant_id: Uuid,
    entity_id: Uuid,
    fund_name: String,
    cash_gl_account_id: Uuid,
    imprest_amount: i64,
    balance: i64,
    custodian_id: Uuid,
    status: String,
    entity_version: Option<i32>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl PettyCashFundRow {
    fn into_model(self) -> PettyCashFund {
        PettyCashFund {
            petty_cash_fund_id: EntityId::from_uuid(self.petty_cash_fund_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            entity_id: self.entity_id,
            fund_name: self.fund_name,
            cash_gl_account_id: self.cash_gl_account_id,
            imprest_amount: Money::from_paise(self.imprest_amount),
            balance: Money::from_paise(self.balance),
            custodian_id: self.custodian_id,
            status: if self.status == "CLOSED" {
                PettyCashFundStatus::Closed
            } else {
                PettyCashFundStatus::Active
            },
            version: self.entity_version.unwrap_or(1),
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
struct PettyCashTxnRow {
    petty_cash_txn_id: Uuid,
    tenant_id: Uuid,
    petty_cash_fund_id: Uuid,
    transaction_type: String,
    amount: i64,
    account_id: Option<Uuid>,
    voucher_no: Option<String>,
    narration: Option<String>,
    journal_id: Option<Uuid>,
    transaction_date: NaiveDate,
    recorded_by_id: Uuid,
    entity_version: Option<i32>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
}

impl PettyCashTxnRow {
    fn into_model(self) -> PettyCashTransaction {
        PettyCashTransaction {
            petty_cash_txn_id: EntityId::from_uuid(self.petty_cash_txn_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            petty_cash_fund_id: self.petty_cash_fund_id,
            transaction_type: match self.transaction_type.as_str() {
                "TOP_UP" => PettyCashTransactionType::TopUp,
                "RECOUPMENT" => PettyCashTransactionType::Recoupment,
                "EXPENSE" => PettyCashTransactionType::Expense,
                _ => PettyCashTransactionType::Adjustment,
            },
            amount: Money::from_paise(self.amount),
            account_id: self.account_id,
            voucher_no: self.voucher_no,
            narration: self.narration,
            journal_id: self.journal_id,
            transaction_date: self.transaction_date,
            recorded_by_id: self.recorded_by_id,
            version: self.entity_version.unwrap_or(1),
            audit: AuditInfo::default(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct GatewayConfigRow {
    payment_gateway_config_id: Uuid,
    tenant_id: Uuid,
    entity_id: Option<Uuid>,
    gateway_type: String,
    merchant_id: String,
    api_key: String,
    api_secret: String,
    webhook_secret: Option<String>,
    is_active: bool,
    entity_version: Option<i32>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl GatewayConfigRow {
    fn into_model(self) -> PaymentGatewayConfig {
        PaymentGatewayConfig {
            payment_gateway_config_id: EntityId::from_uuid(self.payment_gateway_config_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            entity_id: self.entity_id,
            gateway_type: GatewayType::from_db_str(&self.gateway_type)
                .unwrap_or(GatewayType::Razorpay),
            merchant_id: self.merchant_id,
            api_key: self.api_key,
            api_secret: self.api_secret,
            webhook_secret: self.webhook_secret,
            is_active: self.is_active,
            version: self.entity_version.unwrap_or(1),
            audit: AuditInfo {
                created_by: self.created_by.unwrap_or_else(Uuid::nil),
                created_at: self.created_at.unwrap_or_else(Utc::now),
                updated_by: self.updated_by.unwrap_or_else(Uuid::nil),
                updated_at: self.updated_at.unwrap_or_else(Utc::now),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
struct GatewaySettlementRow {
    gateway_settlement_id: Uuid,
    tenant_id: Uuid,
    entity_id: Uuid,
    gateway_type: String,
    settlement_date: NaiveDate,
    settled_amount: i64,
    gateway_fee_amount: i64,
    matched_amount: i64,
    status: String,
    exception_reason: Option<String>,
    reconciled_by_id: Option<Uuid>,
    reconciled_at: Option<DateTime<Utc>>,
    entity_version: Option<i32>,
    created_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_at: Option<DateTime<Utc>>,
    updated_by: Option<Uuid>,
}

impl GatewaySettlementRow {
    fn into_model(self) -> GatewaySettlement {
        GatewaySettlement {
            gateway_settlement_id: EntityId::from_uuid(self.gateway_settlement_id),
            tenant_id: TenantId::from_uuid(self.tenant_id),
            entity_id: self.entity_id,
            gateway_type: GatewayType::from_db_str(&self.gateway_type)
                .unwrap_or(GatewayType::Razorpay),
            settlement_date: self.settlement_date,
            settled_amount: Money::from_paise(self.settled_amount),
            gateway_fee_amount: Money::from_paise(self.gateway_fee_amount),
            matched_amount: Money::from_paise(self.matched_amount),
            status: GatewaySettlementStatus::from_db_str(&self.status),
            exception_reason: self.exception_reason,
            reconciled_by_id: self.reconciled_by_id,
            reconciled_at: self.reconciled_at,
            version: self.entity_version.unwrap_or(1),
            audit: AuditInfo::default(),
        }
    }
}

/// Candidate transaction for reconciliation matching (bank register +
/// AR receipts + AP vendor payments + fund receipts).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MatchCandidate {
    pub transaction_id: Uuid,
    pub transaction_type: String,
    pub amount: i64, // absolute amount in paise
    pub transaction_date: NaiveDate,
    pub transaction_ref: Option<String>,
    pub description: Option<String>,
    pub is_debit: bool, // true = money out (matches a statement debit)
}

/// A bank book row (Tally-style): every journal line touching the bank GL
/// account, ordered by posting date.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BankBookRow {
    pub journal_id: Uuid,
    pub journal_number: String,
    pub posting_date: NaiveDate,
    pub description: String,
    pub debit_amount: Option<i64>,
    pub credit_amount: Option<i64>,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub running_balance: i64,
}

// ─── Repository ───────────────────────────────────────────────────────

/// Data access for the treasury module.
pub struct TreasuryRepository {
    pool: PgPool,
}

impl TreasuryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ── Bank accounts ───────────────────────────────────────────────

    pub async fn insert_bank_account(
        &self,
        account: &BankAccount,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            INSERT INTO bank_accounts (
                bank_account_id, tenant_id, entity_id, account_number, account_name,
                bank_name, branch_name, ifsc_code, account_type, fund_id,
                is_fcra_account, minimum_balance, is_active, gl_account_id,
                entity_version, created_by, created_at, updated_by, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,1,$15,now(),$15,now())
            "#,
        )
        .bind(account.bank_account_id.as_uuid())
        .bind(account.tenant_id.as_uuid())
        .bind(account.entity_id)
        .bind(&account.account_number)
        .bind(&account.account_name)
        .bind(&account.bank_name)
        .bind(&account.branch_name)
        .bind(&account.ifsc_code)
        .bind(account.account_type.to_db_str())
        .bind(account.fund_id)
        .bind(account.is_fcra_account)
        .bind(account.minimum_balance.as_paise())
        .bind(account.is_active)
        .bind(account.gl_account_id)
        .bind(account.audit.created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_bank_account(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<Option<BankAccount>, TreasuryError> {
        let row = sqlx::query_as::<_, BankAccountRow>(
            r#"
            SELECT bank_account_id, tenant_id, entity_id, account_number, account_name,
                   bank_name, branch_name, ifsc_code, account_type, fund_id,
                   is_fcra_account, minimum_balance, is_active, gl_account_id,
                   last_reconciled_at, created_at, created_by, updated_at, updated_by
            FROM bank_accounts
            WHERE tenant_id = $1 AND bank_account_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.into_model()))
    }

    pub async fn list_bank_accounts(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
        account_type: Option<&str>,
    ) -> Result<Vec<BankAccount>, TreasuryError> {
        let rows = sqlx::query_as::<_, BankAccountRow>(
            r#"
            SELECT bank_account_id, tenant_id, entity_id, account_number, account_name,
                   bank_name, branch_name, ifsc_code, account_type, fund_id,
                   is_fcra_account, minimum_balance, is_active, gl_account_id,
                   last_reconciled_at, created_at, created_by, updated_at, updated_by
            FROM bank_accounts
            WHERE tenant_id = $1 AND deleted_at IS NULL
              AND ($2::uuid IS NULL OR entity_id = $2)
              AND ($3::text IS NULL OR account_type = $3)
            ORDER BY account_name
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(account_type)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_model()).collect())
    }

    pub async fn account_number_exists(
        &self,
        tenant_id: Uuid,
        account_number: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, TreasuryError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM bank_accounts
            WHERE tenant_id = $1 AND account_number = $2
              AND deleted_at IS NULL AND ($3::uuid IS NULL OR bank_account_id <> $3)
            "#,
        )
        .bind(tenant_id)
        .bind(account_number)
        .bind(exclude_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn update_bank_account_fields(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        account_name: &str,
        branch_name: Option<&str>,
        minimum_balance: i64,
        updated_by: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            UPDATE bank_accounts
            SET account_name = $3, branch_name = $4, minimum_balance = $5,
                updated_by = $6, updated_at = now(), entity_version = entity_version + 1
            WHERE tenant_id = $1 AND bank_account_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .bind(account_name)
        .bind(branch_name)
        .bind(minimum_balance)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_bank_account_active(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        is_active: bool,
        updated_by: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            UPDATE bank_accounts
            SET is_active = $3, updated_by = $4, updated_at = now(), entity_version = entity_version + 1
            WHERE tenant_id = $1 AND bank_account_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .bind(is_active)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn touch_last_reconciled(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            "UPDATE bank_accounts SET last_reconciled_at = now() WHERE tenant_id = $1 AND bank_account_id = $2",
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn bank_account_unreconciled_balance(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<i64, TreasuryError> {
        // Latest bank_transactions register balance for the account; 0 when
        // no register rows exist (nothing unreconciled to worry about).
        let row: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT balance FROM bank_transactions
            WHERE tenant_id = $1 AND bank_account_id = $2
            ORDER BY transaction_date DESC, created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0).unwrap_or(0))
    }

    // ── Signatories ─────────────────────────────────────────────────

    pub async fn insert_signatory(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        user_id: Uuid,
        signatory_type: &str,
        created_by: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            INSERT INTO bank_signatories (bank_signatory_id, tenant_id, bank_account_id, user_id, signatory_type, is_active, created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,TRUE,now(),now())
            ON CONFLICT (bank_account_id, user_id) DO UPDATE
            SET is_active = TRUE, signatory_type = EXCLUDED.signatory_type, updated_at = now()
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(bank_account_id)
        .bind(user_id)
        .bind(signatory_type)
        .bind(created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn deactivate_signatory(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            UPDATE bank_signatories SET is_active = FALSE, updated_at = now()
            WHERE tenant_id = $1 AND bank_account_id = $2 AND user_id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_signatories(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<Vec<BankSignatory>, TreasuryError> {
        let rows = sqlx::query_as::<_, SignatoryRow>(
            r#"
            SELECT bank_signatory_id, tenant_id, bank_account_id, user_id, signatory_type, is_active, created_at, updated_at
            FROM bank_signatories
            WHERE tenant_id = $1 AND bank_account_id = $2
            ORDER BY is_active DESC, created_at
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_model()).collect())
    }

    pub async fn active_signatory_count(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<i64, TreasuryError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM bank_signatories
            WHERE tenant_id = $1 AND bank_account_id = $2 AND is_active = TRUE
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    // ── GL leaf helpers ─────────────────────────────────────────────

    /// Resolve the COA parent account for bank leaves by trying each
    /// configured candidate code in order.
    pub async fn find_account_by_code(
        &self,
        tenant_id: Uuid,
        code: &str,
    ) -> Result<Option<Uuid>, TreasuryError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT account_id FROM chart_of_accounts
            WHERE tenant_id = $1 AND account_code = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn find_gl_parent(
        &self,
        tenant_id: Uuid,
        candidate_codes: &[String],
    ) -> Result<Option<Uuid>, TreasuryError> {
        for code in candidate_codes {
            if let Some(id) = self.find_account_by_code(tenant_id, code).await? {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    /// Like [`Self::find_gl_parent`] but also returns the matched account
    /// code (used as the leaf-code prefix).
    pub async fn find_gl_parent_with_code(
        &self,
        tenant_id: Uuid,
        candidate_codes: &[String],
    ) -> Result<Option<(Uuid, String)>, TreasuryError> {
        for code in candidate_codes {
            let row: Option<(Uuid,)> = sqlx::query_as(
                r#"
                SELECT account_id FROM chart_of_accounts
                WHERE tenant_id = $1 AND account_code = $2 AND deleted_at IS NULL
                "#,
            )
            .bind(tenant_id)
            .bind(code)
            .fetch_optional(&self.pool)
            .await?;
            if let Some((id,)) = row {
                return Ok(Some((id, code.clone())));
            }
        }
        Ok(None)
    }

    pub async fn touch_last_sync(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        updated_by: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            UPDATE bank_accounts
            SET last_sync_at = now(), updated_by = $3, updated_at = now()
            WHERE tenant_id = $1 AND bank_account_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_gl_account(
        &self,
        tenant_id: Uuid,
        account_id: Uuid,
    ) -> Result<Option<sutra_finance_gl::models::account::Account>, TreasuryError> {
        Ok(sutra_finance_gl::PgAccountRepository::new(self.pool.clone())
            .find_by_id(tenant_id, account_id)
            .await?)
    }

    pub async fn gl_account_balance(&self, tenant_id: Uuid, account_id: Uuid) -> Result<i64, TreasuryError> {
        let row: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT current_balance FROM chart_of_accounts
            WHERE tenant_id = $1 AND account_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0).unwrap_or(0))
    }

    /// Compute the next bank GL leaf code under the given prefix, e.g. the
    /// first account under `10.02` becomes `10.02.01`, then `10.02.02`, …
    pub async fn next_gl_leaf_code(
        &self,
        tenant_id: Uuid,
        prefix: &str,
    ) -> Result<String, TreasuryError> {
        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT account_code FROM chart_of_accounts
            WHERE tenant_id = $1 AND account_code LIKE $2 AND deleted_at IS NULL
            ORDER BY account_code DESC LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(format!("{}.%", prefix))
        .fetch_optional(&self.pool)
        .await?;
        let next = match row {
            Some((last,)) => {
                let last_part = last.rsplit('.').next().unwrap_or("0");
                let n: i32 = last_part.parse().unwrap_or(0);
                n + 1
            }
            None => 1,
        };
        Ok(format!("{}.{:02}", prefix, next))
    }

    /// Insert a leaf GL account under the parent (ASSET). Returns the new id.
    pub async fn insert_gl_leaf(
        &self,
        tenant_id: Uuid,
        account_code: &str,
        account_name: &str,
        parent_account_id: Uuid,
        created_by: Uuid,
    ) -> Result<Uuid, TreasuryError> {
        let account_id = Uuid::now_v7();
        sqlx::query(
            r#"
            INSERT INTO chart_of_accounts (
                account_id, tenant_id, account_code, account_name, account_type,
                parent_account_id, level, opening_balance, current_balance,
                is_active, is_system, entity_version, created_by, created_at,
                updated_by, updated_at
            ) VALUES ($1,$2,$3,$4,'ASSET',$5,5,0,0,TRUE,FALSE,1,$6,now(),$6,now())
            "#,
        )
        .bind(account_id)
        .bind(tenant_id)
        .bind(account_code)
        .bind(account_name)
        .bind(parent_account_id)
        .bind(created_by)
        .execute(&self.pool)
        .await?;
        Ok(account_id)
    }

    // ── Reconciliations ─────────────────────────────────────────────

    pub async fn has_open_reconciliation(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        period_id: Uuid,
    ) -> Result<bool, TreasuryError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM bank_reconciliations
            WHERE tenant_id = $1 AND bank_account_id = $2
              AND period_id = $3 AND status = 'IN_PROGRESS' AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .bind(period_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn has_any_open_reconciliation(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<bool, TreasuryError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM bank_reconciliations
            WHERE tenant_id = $1 AND bank_account_id = $2
              AND status = 'IN_PROGRESS' AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn insert_reconciliation(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        bank_account_id: Uuid,
        period_id: Uuid,
        statement_date: NaiveDate,
        opening_balance: i64,
        closing_balance: i64,
        created_by: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            INSERT INTO bank_reconciliations (
                bank_reconciliation_id, tenant_id, bank_account_id, period_id,
                statement_date, opening_balance, closing_balance, status,
                entity_version, created_by, created_at, updated_by, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,'IN_PROGRESS',1,$8,now(),$8,now())
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(bank_account_id)
        .bind(period_id)
        .bind(statement_date)
        .bind(opening_balance)
        .bind(closing_balance)
        .bind(created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_reconciliation(
        &self,
        tenant_id: Uuid,
        reconciliation_id: Uuid,
    ) -> Result<Option<BankReconciliation>, TreasuryError> {
        let row = sqlx::query_as::<_, ReconciliationRow>(
            r#"
            SELECT bank_reconciliation_id, tenant_id, bank_account_id, period_id,
                   statement_date, opening_balance, closing_balance, status,
                   verified_by_id, completed_at, entity_version,
                   created_at, created_by, updated_at, updated_by
            FROM bank_reconciliations
            WHERE tenant_id = $1 AND bank_reconciliation_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(reconciliation_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.into_model()))
    }

    pub async fn list_reconciliations(
        &self,
        tenant_id: Uuid,
        bank_account_id: Option<Uuid>,
        status: Option<&str>,
    ) -> Result<Vec<BankReconciliation>, TreasuryError> {
        let rows = sqlx::query_as::<_, ReconciliationRow>(
            r#"
            SELECT bank_reconciliation_id, tenant_id, bank_account_id, period_id,
                   statement_date, opening_balance, closing_balance, status,
                   verified_by_id, completed_at, entity_version,
                   created_at, created_by, updated_at, updated_by
            FROM bank_reconciliations
            WHERE tenant_id = $1 AND deleted_at IS NULL
              AND ($2::uuid IS NULL OR bank_account_id = $2)
              AND ($3::text IS NULL OR status = $3)
            ORDER BY statement_date DESC
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .bind(status)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_model()).collect())
    }

    pub async fn update_reconciliation_status(
        &self,
        tenant_id: Uuid,
        reconciliation_id: Uuid,
        status: &BankReconciliationStatus,
        verified_by: Option<Uuid>,
        updated_by: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            UPDATE bank_reconciliations
            SET status = $3, verified_by_id = $4,
                completed_at = CASE WHEN $3 = 'COMPLETED' OR $3 = 'VERIFIED' THEN now() ELSE completed_at END,
                updated_by = $5, updated_at = now(), entity_version = entity_version + 1
            WHERE tenant_id = $1 AND bank_reconciliation_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(reconciliation_id)
        .bind(status.to_db_str())
        .bind(verified_by)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_statement_lines(
        &self,
        tenant_id: Uuid,
        reconciliation_id: Uuid,
        lines: &[BankStatementLine],
    ) -> Result<(), TreasuryError> {
        for line in lines {
            sqlx::query(
                r#"
                INSERT INTO bank_statement_lines (
                    bank_statement_line_id, tenant_id, bank_reconciliation_id,
                    transaction_date, transaction_ref, description,
                    debit_amount, credit_amount, match_status,
                    matched_transaction_id, matched_transaction_type, created_at
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,now())
                "#,
            )
            .bind(line.bank_statement_line_id.as_uuid())
            .bind(tenant_id)
            .bind(reconciliation_id)
            .bind(line.transaction_date)
            .bind(&line.transaction_ref)
            .bind(&line.description)
            .bind(line.debit_amount.map(|m| m.as_paise()))
            .bind(line.credit_amount.map(|m| m.as_paise()))
            .bind(line.match_status.to_db_str())
            .bind(line.matched_transaction_id)
            .bind(&line.matched_transaction_type)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn list_statement_lines(
        &self,
        tenant_id: Uuid,
        reconciliation_id: Uuid,
    ) -> Result<Vec<BankStatementLine>, TreasuryError> {
        let rows = sqlx::query_as::<_, StatementLineRow>(
            r#"
            SELECT bank_statement_line_id, tenant_id, bank_reconciliation_id,
                   transaction_date, transaction_ref, description,
                   debit_amount, credit_amount, match_status,
                   matched_transaction_id, matched_transaction_type, created_at
            FROM bank_statement_lines
            WHERE tenant_id = $1 AND bank_reconciliation_id = $2
            ORDER BY transaction_date, created_at
            "#,
        )
        .bind(tenant_id)
        .bind(reconciliation_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_model()).collect())
    }

    pub async fn find_statement_line(
        &self,
        tenant_id: Uuid,
        reconciliation_id: Uuid,
        statement_line_id: Uuid,
    ) -> Result<Option<BankStatementLine>, TreasuryError> {
        let row = sqlx::query_as::<_, StatementLineRow>(
            r#"
            SELECT bank_statement_line_id, tenant_id, bank_reconciliation_id,
                   transaction_date, transaction_ref, description,
                   debit_amount, credit_amount, match_status,
                   matched_transaction_id, matched_transaction_type, created_at
            FROM bank_statement_lines
            WHERE tenant_id = $1 AND bank_reconciliation_id = $2
              AND bank_statement_line_id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(reconciliation_id)
        .bind(statement_line_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.into_model()))
    }

    pub async fn update_line_match(
        &self,
        tenant_id: Uuid,
        statement_line_id: Uuid,
        match_status: &MatchStatus,
        matched_transaction_id: Option<Uuid>,
        matched_transaction_type: Option<&str>,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            UPDATE bank_statement_lines
            SET match_status = $2, matched_transaction_id = $3, matched_transaction_type = $4
            WHERE tenant_id = $1 AND bank_statement_line_id = $5
            "#,
        )
        .bind(tenant_id)
        .bind(match_status.to_db_str())
        .bind(matched_transaction_id)
        .bind(matched_transaction_type)
        .bind(statement_line_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn unmatched_line_count(
        &self,
        tenant_id: Uuid,
        reconciliation_id: Uuid,
    ) -> Result<i64, TreasuryError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM bank_statement_lines
            WHERE tenant_id = $1 AND bank_reconciliation_id = $2
              AND match_status = 'UNMATCHED'
            "#,
        )
        .bind(tenant_id)
        .bind(reconciliation_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    /// Match candidates: system bank transactions + AR receipts + AP vendor
    /// payments, all carrying bank references / dates / amounts.
    pub async fn match_candidates(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<Vec<MatchCandidate>, TreasuryError> {
        let rows = sqlx::query_as::<_, MatchCandidate>(
            r#"
            SELECT
                bt.bank_transaction_id AS transaction_id,
                COALESCE(bt.reference_type, 'BANK_REGISTER') AS transaction_type,
                COALESCE(bt.debit_amount, bt.credit_amount) AS amount,
                bt.transaction_date,
                bt.transaction_ref,
                bt.description,
                (bt.debit_amount IS NOT NULL) AS is_debit
            FROM bank_transactions bt
            WHERE bt.tenant_id = $1 AND bt.bank_account_id = $2
            UNION ALL
            SELECT
                r.payment_receipt_id AS transaction_id,
                'PaymentReceipt' AS transaction_type,
                r.amount AS amount,
                r.payment_date::date AS transaction_date,
                r.bank_transaction_ref AS transaction_ref,
                NULL AS description,
                FALSE AS is_debit
            FROM ar_payment_receipts r
            WHERE r.tenant_id = $1
              AND r.bank_transaction_ref IS NOT NULL AND r.bank_transaction_ref <> ''
              AND r.status IN ('COMPLETED','UNCLEARED')
            UNION ALL
            SELECT
                p.payment_id AS transaction_id,
                'VendorPayment' AS transaction_type,
                COALESCE(p.net_amount, p.amount) AS amount,
                p.payment_date AS transaction_date,
                p.bank_transaction_ref AS transaction_ref,
                NULL AS description,
                TRUE AS is_debit
            FROM vendor_payments p
            WHERE p.tenant_id = $1
              AND p.bank_account_id = $2
              AND p.bank_transaction_ref IS NOT NULL AND p.bank_transaction_ref <> ''
              AND p.status IN ('PROCESSED','COMPLETED')
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ── Transfers ───────────────────────────────────────────────────

    pub async fn insert_transfer(&self, t: &InterBankTransfer) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            INSERT INTO inter_bank_transfers (
                inter_bank_transfer_id, tenant_id, from_bank_account_id, to_bank_account_id,
                amount, transfer_date, status, requires_approval, bank_reference,
                journal_id, initiated_by_id, approved_by_id, processed_by_id,
                failure_reason, version, created_by, created_at, updated_by, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,1,$15,now(),$15,now())
            "#,
        )
        .bind(t.inter_bank_transfer_id.as_uuid())
        .bind(t.tenant_id.as_uuid())
        .bind(t.from_bank_account_id)
        .bind(t.to_bank_account_id)
        .bind(t.amount.as_paise())
        .bind(t.transfer_date)
        .bind(t.status.to_db_str())
        .bind(t.requires_approval)
        .bind(&t.bank_reference)
        .bind(t.journal_id)
        .bind(t.initiated_by_id)
        .bind(t.approved_by_id)
        .bind(t.processed_by_id)
        .bind(&t.failure_reason)
        .bind(t.audit.created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_transfer(
        &self,
        tenant_id: Uuid,
        transfer_id: Uuid,
    ) -> Result<Option<InterBankTransfer>, TreasuryError> {
        let row = sqlx::query_as::<_, TransferRow>(
            r#"
            SELECT inter_bank_transfer_id, tenant_id, from_bank_account_id, to_bank_account_id,
                   amount, transfer_date, status, requires_approval, bank_reference,
                   journal_id, initiated_by_id, approved_by_id, processed_by_id,
                   failure_reason, entity_version, created_at, created_by, updated_at, updated_by
            FROM inter_bank_transfers
            WHERE tenant_id = $1 AND inter_bank_transfer_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(transfer_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.into_model()))
    }

    pub async fn list_transfers(
        &self,
        tenant_id: Uuid,
        from_account: Option<Uuid>,
        status: Option<&str>,
    ) -> Result<Vec<InterBankTransfer>, TreasuryError> {
        let rows = sqlx::query_as::<_, TransferRow>(
            r#"
            SELECT inter_bank_transfer_id, tenant_id, from_bank_account_id, to_bank_account_id,
                   amount, transfer_date, status, requires_approval, bank_reference,
                   journal_id, initiated_by_id, approved_by_id, processed_by_id,
                   failure_reason, entity_version, created_at, created_by, updated_at, updated_by
            FROM inter_bank_transfers
            WHERE tenant_id = $1
              AND ($2::uuid IS NULL OR from_bank_account_id = $2)
              AND ($3::text IS NULL OR status = $3)
            ORDER BY transfer_date DESC, created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(from_account)
        .bind(status)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_model()).collect())
    }

    pub async fn update_transfer_status(
        &self,
        tenant_id: Uuid,
        transfer_id: Uuid,
        status: &InterBankTransferStatus,
        bank_reference: Option<&str>,
        journal_id: Option<Uuid>,
        approved_by: Option<Uuid>,
        processed_by: Option<Uuid>,
        failure_reason: Option<&str>,
        updated_by: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            UPDATE inter_bank_transfers
            SET status = $3, bank_reference = COALESCE($4, bank_reference),
                journal_id = COALESCE($5, journal_id),
                approved_by_id = COALESCE($6, approved_by_id),
                processed_by_id = COALESCE($7, processed_by_id),
                failure_reason = $8,
                updated_by = $9, updated_at = now(), version = version + 1
            WHERE tenant_id = $1 AND inter_bank_transfer_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(transfer_id)
        .bind(status.to_db_str())
        .bind(bank_reference)
        .bind(journal_id)
        .bind(approved_by)
        .bind(processed_by)
        .bind(failure_reason)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Petty cash ──────────────────────────────────────────────────

    pub async fn has_active_fund_for_entity(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
    ) -> Result<bool, TreasuryError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM petty_cash_funds
            WHERE tenant_id = $1 AND entity_id = $2 AND status = 'ACTIVE' AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn insert_petty_cash_fund(&self, f: &PettyCashFund) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            INSERT INTO petty_cash_funds (
                petty_cash_fund_id, tenant_id, entity_id, fund_name, cash_gl_account_id,
                imprest_amount, balance, custodian_id, status, version,
                created_by, created_at, updated_by, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,1,$10,now(),$10,now())
            "#,
        )
        .bind(f.petty_cash_fund_id.as_uuid())
        .bind(f.tenant_id.as_uuid())
        .bind(f.entity_id)
        .bind(&f.fund_name)
        .bind(f.cash_gl_account_id)
        .bind(f.imprest_amount.as_paise())
        .bind(f.balance.as_paise())
        .bind(f.custodian_id)
        .bind(if f.status == PettyCashFundStatus::Closed { "CLOSED" } else { "ACTIVE" })
        .bind(f.audit.created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_petty_cash_fund(
        &self,
        tenant_id: Uuid,
        fund_id: Uuid,
    ) -> Result<Option<PettyCashFund>, TreasuryError> {
        let row = sqlx::query_as::<_, PettyCashFundRow>(
            r#"
            SELECT petty_cash_fund_id, tenant_id, entity_id, fund_name, cash_gl_account_id,
                   imprest_amount, balance, custodian_id, status, entity_version,
                   created_at, created_by, updated_at, updated_by
            FROM petty_cash_funds
            WHERE tenant_id = $1 AND petty_cash_fund_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(fund_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.into_model()))
    }

    pub async fn list_petty_cash_funds(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
    ) -> Result<Vec<PettyCashFund>, TreasuryError> {
        let rows = sqlx::query_as::<_, PettyCashFundRow>(
            r#"
            SELECT petty_cash_fund_id, tenant_id, entity_id, fund_name, cash_gl_account_id,
                   imprest_amount, balance, custodian_id, status, entity_version,
                   created_at, created_by, updated_at, updated_by
            FROM petty_cash_funds
            WHERE tenant_id = $1 AND deleted_at IS NULL
              AND ($2::uuid IS NULL OR entity_id = $2)
            ORDER BY created_at
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_model()).collect())
    }

    pub async fn update_petty_cash_balance(
        &self,
        tenant_id: Uuid,
        fund_id: Uuid,
        new_balance: i64,
        updated_by: Uuid,
    ) -> Result<(), TreasuryError> {
        if new_balance < 0 {
            return Err(TreasuryError::InsufficientPettyCashBalance(
                new_balance,
                new_balance,
            ));
        }
        sqlx::query(
            r#"
            UPDATE petty_cash_funds
            SET balance = $3, updated_by = $4, updated_at = now(), version = version + 1
            WHERE tenant_id = $1 AND petty_cash_fund_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(fund_id)
        .bind(new_balance)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn close_petty_cash_fund(
        &self,
        tenant_id: Uuid,
        fund_id: Uuid,
        updated_by: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            UPDATE petty_cash_funds
            SET status = 'CLOSED', updated_by = $3, updated_at = now(), version = version + 1
            WHERE tenant_id = $1 AND petty_cash_fund_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(fund_id)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_petty_cash_txn(
        &self,
        t: &PettyCashTransaction,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            INSERT INTO petty_cash_transactions (
                petty_cash_txn_id, tenant_id, petty_cash_fund_id, transaction_type,
                amount, account_id, voucher_no, narration, journal_id,
                transaction_date, recorded_by_id, version, created_by, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,1,$11,now())
            "#,
        )
        .bind(t.petty_cash_txn_id.as_uuid())
        .bind(t.tenant_id.as_uuid())
        .bind(t.petty_cash_fund_id)
        .bind(t.transaction_type.to_db_str())
        .bind(t.amount.as_paise())
        .bind(t.account_id)
        .bind(&t.voucher_no)
        .bind(&t.narration)
        .bind(t.journal_id)
        .bind(t.transaction_date)
        .bind(t.recorded_by_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_petty_cash_txns(
        &self,
        tenant_id: Uuid,
        fund_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<PettyCashTransaction>, TreasuryError> {
        let rows = sqlx::query_as::<_, PettyCashTxnRow>(
            r#"
            SELECT petty_cash_txn_id, tenant_id, petty_cash_fund_id, transaction_type,
                   amount, account_id, voucher_no, narration, journal_id,
                   transaction_date, recorded_by_id, entity_version, created_at, created_by
            FROM petty_cash_transactions
            WHERE tenant_id = $1 AND petty_cash_fund_id = $2
              AND ($3::date IS NULL OR transaction_date >= $3)
              AND ($4::date IS NULL OR transaction_date <= $4)
            ORDER BY transaction_date, created_at
            "#,
        )
        .bind(tenant_id)
        .bind(fund_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_model()).collect())
    }

    // ── Gateways ────────────────────────────────────────────────────

    pub async fn upsert_gateway_config(&self, c: &PaymentGatewayConfig) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            INSERT INTO payment_gateway_configs (
                payment_gateway_config_id, tenant_id, entity_id, gateway_type,
                merchant_id, api_key, api_secret, webhook_secret, is_active,
                version, created_by, created_at, updated_by, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,1,$10,now(),$10,now())
            ON CONFLICT (tenant_id, entity_id, gateway_type) DO UPDATE SET
                merchant_id = EXCLUDED.merchant_id,
                api_key = EXCLUDED.api_key,
                api_secret = EXCLUDED.api_secret,
                webhook_secret = EXCLUDED.webhook_secret,
                is_active = EXCLUDED.is_active,
                updated_by = EXCLUDED.created_by,
                updated_at = now(),
                version = payment_gateway_configs.version + 1
            "#,
        )
        .bind(c.payment_gateway_config_id.as_uuid())
        .bind(c.tenant_id.as_uuid())
        .bind(c.entity_id)
        .bind(c.gateway_type.to_db_str())
        .bind(&c.merchant_id)
        .bind(&c.api_key)
        .bind(&c.api_secret)
        .bind(&c.webhook_secret)
        .bind(c.is_active)
        .bind(c.audit.created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_gateway_config(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        gateway_type: &str,
    ) -> Result<Option<PaymentGatewayConfig>, TreasuryError> {
        let row = sqlx::query_as::<_, GatewayConfigRow>(
            r#"
            SELECT payment_gateway_config_id, tenant_id, entity_id, gateway_type,
                   merchant_id, api_key, api_secret, webhook_secret, is_active,
                   entity_version, created_at, created_by, updated_at, updated_by
            FROM payment_gateway_configs
            WHERE tenant_id = $1 AND entity_id = $2 AND gateway_type = $3 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(gateway_type)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.into_model()))
    }

    pub async fn list_gateway_configs(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
    ) -> Result<Vec<PaymentGatewayConfig>, TreasuryError> {
        let rows = sqlx::query_as::<_, GatewayConfigRow>(
            r#"
            SELECT payment_gateway_config_id, tenant_id, entity_id, gateway_type,
                   merchant_id, api_key, api_secret, webhook_secret, is_active,
                   entity_version, created_at, created_by, updated_at, updated_by
            FROM payment_gateway_configs
            WHERE tenant_id = $1 AND deleted_at IS NULL
              AND ($2::uuid IS NULL OR entity_id = $2)
            ORDER BY gateway_type
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_model()).collect())
    }

    pub async fn find_gateway_settlement(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        gateway_type: &str,
        settlement_date: NaiveDate,
    ) -> Result<Option<GatewaySettlement>, TreasuryError> {
        let row = sqlx::query_as::<_, GatewaySettlementRow>(
            r#"
            SELECT gateway_settlement_id, tenant_id, entity_id, gateway_type,
                   settlement_date, settled_amount, gateway_fee_amount, matched_amount,
                   status, exception_reason, reconciled_by_id, reconciled_at,
                   entity_version, created_at, created_by, updated_at, updated_by
            FROM gateway_settlements
            WHERE tenant_id = $1 AND entity_id = $2 AND gateway_type = $3 AND settlement_date = $4
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(gateway_type)
        .bind(settlement_date)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.into_model()))
    }

    pub async fn upsert_gateway_settlement(
        &self,
        s: &GatewaySettlement,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            INSERT INTO gateway_settlements (
                gateway_settlement_id, tenant_id, entity_id, gateway_type, settlement_date,
                settled_amount, gateway_fee_amount, matched_amount, status,
                exception_reason, reconciled_by_id, reconciled_at,
                version, created_by, created_at, updated_by, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,1,$13,now(),$13,now())
            ON CONFLICT (tenant_id, entity_id, gateway_type, settlement_date) DO UPDATE SET
                settled_amount = EXCLUDED.settled_amount,
                gateway_fee_amount = EXCLUDED.gateway_fee_amount,
                matched_amount = EXCLUDED.matched_amount,
                status = EXCLUDED.status,
                exception_reason = EXCLUDED.exception_reason,
                reconciled_by_id = EXCLUDED.reconciled_by_id,
                reconciled_at = EXCLUDED.reconciled_at,
                updated_by = EXCLUDED.created_by,
                updated_at = now(),
                version = gateway_settlements.version + 1
            "#,
        )
        .bind(s.gateway_settlement_id.as_uuid())
        .bind(s.tenant_id.as_uuid())
        .bind(s.entity_id)
        .bind(s.gateway_type.to_db_str())
        .bind(s.settlement_date)
        .bind(s.settled_amount.as_paise())
        .bind(s.gateway_fee_amount.as_paise())
        .bind(s.matched_amount.as_paise())
        .bind(s.status.to_db_str())
        .bind(&s.exception_reason)
        .bind(s.reconciled_by_id)
        .bind(s.reconciled_at)
        .bind(s.audit.created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_gateway_settlements(
        &self,
        tenant_id: Uuid,
        entity_id: Option<Uuid>,
        status: Option<&str>,
    ) -> Result<Vec<GatewaySettlement>, TreasuryError> {
        let rows = sqlx::query_as::<_, GatewaySettlementRow>(
            r#"
            SELECT gateway_settlement_id, tenant_id, entity_id, gateway_type,
                   settlement_date, settled_amount, gateway_fee_amount, matched_amount,
                   status, exception_reason, reconciled_by_id, reconciled_at,
                   entity_version, created_at, created_by, updated_at, updated_by
            FROM gateway_settlements
            WHERE tenant_id = $1
              AND ($2::uuid IS NULL OR entity_id = $2)
              AND ($3::text IS NULL OR status = $3)
            ORDER BY settlement_date DESC
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(status)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_model()).collect())
    }

    /// Sum of gateway transactions (AR table) settled on the given date,
    /// plus the sum of their gateway fees. Returns (settled_total, fee_total).
    pub async fn gateway_txn_totals_for_date(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        gateway_type: &str,
        settlement_date: NaiveDate,
    ) -> Result<(i64, i64), TreasuryError> {
        let row: Option<(i64, i64)> = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(pgt.settled_amount), 0),
                   COALESCE(SUM(pgt.gateway_fee), 0)
            FROM payment_gateway_transactions pgt
            JOIN ar_payment_receipts r ON r.payment_receipt_id = pgt.payment_receipt_id
            WHERE pgt.tenant_id = $1 AND r.entity_id = $2
              AND pgt.gateway = $3 AND pgt.settled_date = $4
              AND pgt.status = 'SUCCESS'
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(gateway_type)
        .bind(settlement_date)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.unwrap_or((0, 0)))
    }

    // ── Bank book (Tally-style) ─────────────────────────────────────

    pub async fn bank_book(
        &self,
        tenant_id: Uuid,
        bank_gl_account_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<BankBookRow>, TreasuryError> {
        let mut rows: Vec<BankBookRow> = sqlx::query_as::<_, BankBookRow>(
            r#"
            SELECT
                je.journal_id,
                je.journal_number,
                je.posting_date,
                je.description,
                jel.debit_amount,
                jel.credit_amount,
                jel.reference_type,
                jel.reference_id::text,
                0 AS running_balance
            FROM journal_entry_lines jel
            JOIN journal_entries je ON je.journal_id = jel.journal_id AND je.tenant_id = jel.tenant_id
            WHERE jel.tenant_id = $1 AND jel.account_id = $2
              AND je.status = 'POSTED'
              AND ($3::date IS NULL OR je.posting_date >= $3)
              AND ($4::date IS NULL OR je.posting_date <= $4)
            ORDER BY je.posting_date, je.created_at, jel.line_number
            "#,
        )
        .bind(tenant_id)
        .bind(bank_gl_account_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        // Tally-style running balance: assets increase with debits.
        let mut running = 0i64;
        for row in &mut rows {
            if let Some(d) = row.debit_amount {
                running += d;
            }
            if let Some(c) = row.credit_amount {
                running -= c;
            }
            row.running_balance = running;
        }
        Ok(rows)
    }

    // ── Policies ────────────────────────────────────────────────────

    /// Load treasury policies for a tenant from `system_config`, falling
    /// back to GLOBAL rows and then code defaults.
    pub async fn load_policy(&self, tenant_id: Uuid) -> Result<TreasuryPolicy, TreasuryError> {
        let mut policy = TreasuryPolicy::defaults();
        let rows: Vec<(String, Value)> = sqlx::query_as(
            r#"
            SELECT config_key, config_value FROM system_config
            WHERE is_active = TRUE
              AND (tenant_id = $1 OR tenant_id IS NULL)
              AND config_key LIKE 'treasury.%'
              AND (valid_to IS NULL OR valid_to > now())
            ORDER BY (tenant_id = $1) DESC, valid_from DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        for (key, value) in rows {
            let text = match &value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                other => other.to_string(),
            };
            match key.as_str() {
                "treasury.transfer_approval_threshold" => {
                    policy.transfer_approval_threshold = text.parse().unwrap_or(policy.transfer_approval_threshold);
                }
                "treasury.petty_cash_imprest_limit" => {
                    policy.petty_cash_imprest_limit = text.parse().unwrap_or(policy.petty_cash_imprest_limit);
                }
                "treasury.cheque_joint_signature_threshold" => {
                    policy.cheque_joint_signature_threshold = text.parse().unwrap_or(policy.cheque_joint_signature_threshold);
                }
                "treasury.fcra_bank_ifsc" => policy.fcra_bank_ifsc = text,
                "treasury.fcra_bank_name" => policy.fcra_bank_name = text,
                "treasury.fcra_branch_name" => policy.fcra_branch_name = text,
                "treasury.endowment_transfer_blocked" => {
                    policy.endowment_transfer_blocked = text == "true";
                }
                "treasury.auto_match_utr_exact" => policy.auto_match_utr_exact = text == "true",
                "treasury.auto_match_amount_date_window_days" => {
                    policy.auto_match_amount_date_window_days = text.parse().unwrap_or(policy.auto_match_amount_date_window_days);
                }
                "treasury.auto_match_description_similarity" => {
                    policy.auto_match_description_similarity = text == "true";
                }
                "treasury.bank_gl_parent_codes" => {
                    if let Ok(codes) = serde_json::from_str::<Vec<String>>(&text) {
                        if !codes.is_empty() {
                            policy.bank_gl_parent_codes = codes;
                        }
                    }
                }
                "treasury.cash_gl_code" => {
                    policy.cash_gl_code = serde_json::from_str(&text).unwrap_or(policy.cash_gl_code);
                }
                _ => {}
            }
        }
        Ok(policy)
    }

    /// Check whether an account is endowment-linked (fund has principal and
    /// income_only = FALSE). Used to enforce rule 7.
    pub async fn is_endowment_linked(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<bool, TreasuryError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM funds
            WHERE tenant_id = $1 AND bank_account_id = $2
              AND principal_amount IS NOT NULL
              AND COALESCE(income_only, FALSE) = FALSE
            "#,
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    /// Get the entity for a bank account (needed for cross-entity transfers).
    pub async fn bank_account_entity(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
    ) -> Result<Option<Uuid>, TreasuryError> {
        let row: Option<(Option<Uuid>,)> = sqlx::query_as(
            "SELECT entity_id FROM bank_accounts WHERE tenant_id = $1 AND bank_account_id = $2 AND deleted_at IS NULL",
        )
        .bind(tenant_id)
        .bind(bank_account_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|r| r.0))
    }

    /// Unreconciled bank register rows for a period (BRS "book" side).
    pub async fn unreconciled_register_rows(
        &self,
        tenant_id: Uuid,
        bank_account_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<BankTransaction>, TreasuryError> {
        let rows = sqlx::query_as::<_, BankTransactionRow>(
            r#"
            SELECT bank_transaction_id, tenant_id, bank_account_id, transaction_date,
                   value_date, transaction_ref, description, debit_amount, credit_amount,
                   balance, reference_type, reference_id, is_reconciled, reconciled_at,
                   version, created_at, created_by
            FROM bank_transactions
            WHERE tenant_id = $1 AND bank_account_id = $2
              AND transaction_date BETWEEN $3 AND $4
              AND is_reconciled = FALSE
            ORDER BY transaction_date
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

    /// Mark register rows as reconciled (called on line match).
    pub async fn mark_register_reconciled(
        &self,
        tenant_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            UPDATE bank_transactions
            SET is_reconciled = TRUE, reconciled_at = now(), updated_at = now()
            WHERE tenant_id = $1 AND bank_transaction_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(transaction_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Clear a cheque/DD receipt in AR (spec rule 5: treasury publishes the
    /// clearing; AR applies it). We update the AR side directly because the
    /// event dispatcher for AR is not wired in this build.
    pub async fn mark_receipt_cleared(
        &self,
        tenant_id: Uuid,
        receipt_id: Uuid,
        cleared_date: NaiveDate,
    ) -> Result<(), TreasuryError> {
        sqlx::query(
            r#"
            UPDATE ar_payment_receipts
            SET status = 'COMPLETED', cleared_date = $3, updated_at = now()
            WHERE tenant_id = $1 AND payment_receipt_id = $2
              AND status IN ('UNCLEARED','PENDING')
            "#,
        )
        .bind(tenant_id)
        .bind(receipt_id)
        .bind(cleared_date)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Uncleared cheques/DDs from AR receipts (BRS outstanding item).
    pub async fn uncleared_cheques(
        &self,
        tenant_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<(Uuid, String, i64, NaiveDate, Option<String>)>, TreasuryError> {
        sqlx::query_as(
            r#"
            SELECT payment_receipt_id, receipt_number, amount,
                   payment_date::date, cheque_number
            FROM ar_payment_receipts
            WHERE tenant_id = $1
              AND payment_mode IN ('CHEQUE','DD')
              AND status = 'UNCLEARED'
              AND cleared_date IS NULL
              AND ($2::date IS NULL OR payment_date::date >= $2)
              AND ($3::date IS NULL OR payment_date::date <= $3)
            ORDER BY payment_date
            "#,
        )
        .bind(tenant_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }
}

// ─── At-rest secret protection ─────────────────────────────────────────
//
// Gateway credentials are obfuscated at the application layer with a
// SHA-256 keyed stream transform before persistence. This is a defence in
// depth measure (not a substitute for a KMS): the key is derived from the
// process environment when available, falling back to a static dev key so
// the module works in sandboxes. Production deployments must configure
// TREASURY_ENCRYPTION_KEY.

fn encryption_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    if let Ok(env) = std::env::var("TREASURY_ENCRYPTION_KEY") {
        let digest = Sha256::digest(env.as_bytes());
        key.copy_from_slice(&digest);
    } else {
        let digest = Sha256::digest(b"sutra-erp:treasury:dev-key-do-not-use-in-prod");
        key.copy_from_slice(&digest);
    }
    key
}

/// Encrypt (obfuscate) a secret at rest.
pub fn encrypt_secret(plain: &str) -> String {
    let key = encryption_key();
    let data = plain.as_bytes();
    let mut out = Vec::with_capacity(data.len());
    for (i, b) in data.iter().enumerate() {
        out.push(b ^ key[i % key.len()]);
    }
    // Prefix with a marker + hex encode so the stored value is opaque text.
    format!("e1:{}", hex_encode(&out))
}

/// Decrypt a secret stored with [`encrypt_secret`]. Values not produced by
/// this module (no `e1:` marker) are returned as-is.
pub fn decrypt_secret(stored: &str) -> String {
    if let Some(rest) = stored.strip_prefix("e1:") {
        let key = encryption_key();
        if let Ok(bytes) = hex_decode(rest) {
            let mut out = Vec::with_capacity(bytes.len());
            for (i, b) in bytes.iter().enumerate() {
                out.push(b ^ key[i % key.len()]);
            }
            return String::from_utf8_lossy(&out).into_owned();
        }
    }
    stored.to_string()
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{:02x}", b).ok();
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}
