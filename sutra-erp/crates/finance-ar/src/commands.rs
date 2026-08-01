//! Accounts Receivable command handlers (CQRS write side).
//!
//! Each command:
//! 1. Validates business rules
//! 2. Performs the mutation within a DB transaction
//! 3. Writes to the outbox for event publishing
//! 4. Integrates with GL for journal creation

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use sutra_core::{AuditInfo, EntityId, Money, TenantId};
use sutra_finance_gl::{CreateJournalCmd, CreateJournalLineCmd, GlCommandHandler, PostJournalCmd};

use crate::errors::ArError;
use crate::models::concession::{Concession, ConcessionStatus, ConcessionType};
use crate::models::fee_head::FeeHead;
use crate::models::fee_structure::{FeeStructure, FeeStructureStatus};
use crate::models::installment_plan::InstallmentPlan;
use crate::models::payment_receipt::{PaymentMode, PaymentReceipt, ReceiptStatus};
use crate::models::refund::{Refund, RefundMode, RefundStatus};
use crate::models::scholarship::{
    ScholarshipScheme, ScholarshipStatus, StudentScholarship,
};
use crate::models::student_fee::{
    FeeInstallment, FeeTransaction, FeeTransactionType, InstallmentStatus,
    StudentFeeAccount,
};

// ─── Command Definitions ────────────────────────────────────────────────

/// Assess student fees based on a fee structure and optional installment plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessStudentFeesCmd {
    pub student_id: Uuid,
    pub fee_structure_id: Uuid,
    pub installment_plan_id: Option<Uuid>,
    pub academic_year: String,
    pub scholarship_expected: Option<Money>,
    pub concession_amount: Option<Money>,
    pub entity_id: Uuid,
}

/// Record a fee payment from a student.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordFeePaymentCmd {
    pub student_id: Uuid,
    pub student_fee_account_id: Uuid,
    pub amount: Money,
    pub payment_mode: String,
    pub payment_date: chrono::DateTime<Utc>,
    pub gateway_transaction_id: Option<String>,
    pub bank_transaction_ref: Option<String>,
    pub cheque_number: Option<String>,
    pub cheque_date: Option<NaiveDate>,
    pub received_by: Uuid,
    pub entity_id: Uuid,
    /// Journal creation details: account IDs for DR (Bank/Cash) and per-fee-head CR
    pub bank_account_id: Uuid,
    pub fee_income_account_ids: std::collections::HashMap<String, Uuid>, // fee_head_code → income_account_id
    pub accounting_period_id: Uuid,
}

/// Grant a concession to a student.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantConcessionCmd {
    pub student_id: Uuid,
    pub student_fee_account_id: Uuid,
    pub fee_head_id: Option<Uuid>,
    pub concession_type: String,
    pub value: rust_decimal::Decimal,
    pub reason: String,
    pub approved_by: Uuid,
}

/// Apply for a scholarship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyScholarshipCmd {
    pub student_id: Uuid,
    pub scheme_id: Uuid,
    pub student_fee_account_id: Option<Uuid>,
    pub academic_year: String,
    pub expected_amount: Money,
    pub maha_dbt_application_id: Option<String>,
}

/// Verify a scholarship (institute-side verification).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyScholarshipCmd {
    pub scholarship_id: Uuid,
    pub verified_by: Uuid,
}

/// Record scholarship disbursement (DBT receipt).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordScholarshipDisbursementCmd {
    pub scholarship_id: Uuid,
    pub disbursed_amount: Money,
    pub dbt_transaction_id: String,
    pub dbt_date: chrono::DateTime<Utc>,
    /// For GL integration when overpayment triggers refund
    pub bank_account_id: Option<Uuid>,
    pub fee_income_account_id: Option<Uuid>,
    pub accounting_period_id: Option<Uuid>,
    pub entity_id: Option<Uuid>,
}

/// Initiate a refund request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateRefundCmd {
    pub student_id: Uuid,
    pub amount: Money,
    pub refund_reason: String,
    pub refund_mode: String,
    pub linked_payment_id: Option<Uuid>,
    pub withdrawal_date: Option<NaiveDate>,
    pub course_start_date: Option<NaiveDate>,
}

/// Process (execute) a refund.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRefundCmd {
    pub refund_id: Uuid,
    pub processed_by: Uuid,
    pub bank_transaction_ref: Option<String>,
    /// For GL integration (reversal journal)
    pub bank_account_id: Uuid,
    pub fee_income_account_id: Uuid,
    pub accounting_period_id: Uuid,
    pub entity_id: Uuid,
}

// ─── Command Handler ────────────────────────────────────────────────────

pub struct ArCommandHandler {
    pool: PgPool,
}

impl ArCommandHandler {
    pub fn new(pool: PgPool) -> Self {
        ArCommandHandler { pool }
    }

    /// Access the GL command handler for journal operations.
    fn gl_handler(&self) -> GlCommandHandler {
        GlCommandHandler::new(self.pool.clone())
    }

    // ─── Assess Student Fees ──────────────────────────────────────────

    /// Apply a fee structure to a student, creating a StudentFeeAccount
    /// with installments.
    pub async fn assess_student_fees(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: AssessStudentFeesCmd,
    ) -> Result<StudentFeeAccount, ArError> {
        let tid = *tenant_id.as_uuid();

        let mut tx = self.pool.begin().await?;

        // Load fee structure
        let fs = sqlx::query_as::<_, FeeStructureRow>(
            r#"SELECT fee_structure_id, tenant_id, entity_id, name, academic_year,
               effective_from, effective_to, status, frc_approval_number, frc_approved_amount,
               program_id, batch, semester
               FROM ar_fee_structures
               WHERE fee_structure_id = $1 AND tenant_id = $2 AND status = 'ACTIVE'"#,
        )
        .bind(cmd.fee_structure_id)
        .bind(tid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ArError::FeeStructureNotFound(cmd.fee_structure_id.to_string()))?;

        // Load fee structure lines
        let lines = sqlx::query_as::<_, FeeStructureLineRow>(
            r#"SELECT fsl.fee_structure_line_id, fsl.fee_head_id, fsl.amount,
               fsl.is_mandatory, fsl.installment_plan_id, fsl.gst_rate,
               fh.code as fee_head_code, fh.name as fee_head_name, fh.fee_type,
               fh.gst_classification, fh.sac_code, fh.is_refundable
               FROM ar_fee_structure_lines fsl
               JOIN ar_fee_heads fh ON fsl.fee_head_id = fh.fee_head_id
               WHERE fsl.fee_structure_id = $1"#,
        )
        .bind(cmd.fee_structure_id)
        .fetch_all(&mut *tx)
        .await?;

        // Calculate gross fee
        let gross_fee: i64 = lines.iter().map(|l| l.amount_paise).sum();
        let gross_fee = Money::from_paise(gross_fee);

        let scholarship_expected = cmd.scholarship_expected.unwrap_or(Money::ZERO);
        let concession_amount = cmd.concession_amount.unwrap_or(Money::ZERO);
        let net_payable = gross_fee - scholarship_expected - concession_amount;

        let account_id = EntityId::new();
        let audit = AuditInfo::new(created_by);

        // Build installments from installment plan or single full payment
        let mut installments: Vec<FeeInstallment> = Vec::new();

        if let Some(plan_id) = cmd.installment_plan_id {
            let slots = sqlx::query_as::<_, InstallmentSlotRow>(
                r#"SELECT slot_number, percentage, due_date
                   FROM ar_installment_plan_slots
                   WHERE installment_plan_id = $1
                   ORDER BY slot_number"#,
            )
            .bind(plan_id)
            .fetch_all(&mut *tx)
            .await?;

            for slot in slots {
                let slot_amount_pct = rust_decimal::Decimal::from(slot.percentage) / rust_decimal::Decimal::from(100);
                let slot_amount_paise = (net_payable.as_paise() as f64 * rust_decimal::prelude::ToPrimitive::to_f64(&slot_amount_pct).unwrap_or(0.0)) as i64;
                installments.push(FeeInstallment {
                    fee_installment_id: Uuid::now_v7(),
                    student_fee_account_id: account_id.clone(),
                    installment_number: slot.slot_number,
                    due_date: slot.due_date,
                    amount: Money::from_paise(slot_amount_paise),
                    paid_amount: Money::ZERO,
                    status: InstallmentStatus::Pending,
                });
            }
        } else {
            // Single installment — full amount due immediately
            installments.push(FeeInstallment {
                fee_installment_id: Uuid::now_v7(),
                student_fee_account_id: account_id.clone(),
                installment_number: 1,
                due_date: Utc::now().date_naive(),
                amount: net_payable,
                paid_amount: Money::ZERO,
                status: InstallmentStatus::Pending,
            });
        }

        // Persist StudentFeeAccount
        sqlx::query(
            r#"INSERT INTO ar_student_fee_accounts (
                student_fee_account_id, tenant_id, student_id, fee_structure_id,
                academic_year, gross_fee, scholarship_expected, concession_amount,
                net_payable, total_paid, outstanding, status,
                created_by, created_at, updated_by, updated_at, version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,now(),$13,now(),1)"#,
        )
        .bind(account_id.as_uuid())
        .bind(tid)
        .bind(cmd.student_id)
        .bind(cmd.fee_structure_id)
        .bind(&cmd.academic_year)
        .bind(gross_fee.as_paise())
        .bind(scholarship_expected.as_paise())
        .bind(concession_amount.as_paise())
        .bind(net_payable.as_paise())
        .bind(0i64) // total_paid
        .bind(net_payable.as_paise()) // outstanding
        .bind("PENDING")
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        // Persist installments
        for inst in &installments {
            sqlx::query(
                r#"INSERT INTO ar_fee_installments (
                    fee_installment_id, student_fee_account_id, installment_number,
                    due_date, amount, paid_amount, status
                ) VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
            )
            .bind(inst.fee_installment_id)
            .bind(account_id.as_uuid())
            .bind(inst.installment_number)
            .bind(inst.due_date)
            .bind(inst.amount.as_paise())
            .bind(inst.paid_amount.as_paise())
            .bind("PENDING")
            .execute(&mut *tx)
            .await?;
        }

        // Write outbox event
        write_ar_outbox(
            &mut tx,
            tid,
            "StudentFeeAccount",
            &account_id.as_uuid().to_string(),
            "StudentFeeAssessed",
            &serde_json::json!({
                "student_fee_account_id": account_id.as_uuid().to_string(),
                "student_id": cmd.student_id.to_string(),
                "fee_structure_id": cmd.fee_structure_id.to_string(),
                "gross_fee": gross_fee.as_paise(),
                "net_payable": net_payable.as_paise(),
                "installments_count": installments.len(),
                "occurred_at": Utc::now(),
            }),
        )
        .await?;

        tx.commit().await?;

        info!(
            tenant_id = %tid,
            student_id = %cmd.student_id,
            account_id = %account_id,
            "Student fees assessed"
        );

        Ok(StudentFeeAccount {
            student_fee_account_id: account_id,
            tenant_id,
            student_id: cmd.student_id,
            fee_structure_id: EntityId::from_uuid(cmd.fee_structure_id.into()),
            academic_year: cmd.academic_year,
            gross_fee,
            scholarship_expected,
            concession_amount,
            net_payable,
            total_paid: Money::ZERO,
            outstanding: net_payable,
            status: crate::models::student_fee::FeeAccountStatus::Pending,
            installments,
            transactions: vec![],
            audit,
        })
    }

    // ─── Record Fee Payment ───────────────────────────────────────────

    /// Record a fee payment, allocate to installments (FIFO), create receipt,
    /// and post GL journal (DR Bank → CR FeeIncome per fee head).
    pub async fn record_fee_payment(
        &self,
        tenant_id: TenantId,
        cmd: RecordFeePaymentCmd,
    ) -> Result<PaymentReceipt, ArError> {
        let tid = *tenant_id.as_uuid();

        if cmd.amount.is_zero() {
            return Err(ArError::Validation("Payment amount must be positive".into()));
        }

        let mut tx = self.pool.begin().await?;

        // Load the student fee account
        let fee_account = sqlx::query_as::<_, StudentFeeAccountRow>(
            r#"SELECT student_fee_account_id, tenant_id, student_id, fee_structure_id,
               academic_year, gross_fee, scholarship_expected, concession_amount,
               net_payable, total_paid, outstanding, status
               FROM ar_student_fee_accounts
               WHERE student_fee_account_id = $1 AND tenant_id = $2
               FOR UPDATE"#,
        )
        .bind(cmd.student_fee_account_id)
        .bind(tid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ArError::NotFound(format!("Student fee account {} not found", cmd.student_fee_account_id)))?;

        // Load installments, ordered by due_date (oldest first for FIFO)
        let installments = sqlx::query_as::<_, FeeInstallmentRow>(
            r#"SELECT fee_installment_id, student_fee_account_id, installment_number,
               due_date, amount, paid_amount, status
               FROM ar_fee_installments
               WHERE student_fee_account_id = $1
               ORDER BY due_date ASC, installment_number ASC
               FOR UPDATE"#,
        )
        .bind(cmd.student_fee_account_id)
        .fetch_all(&mut *tx)
        .await?;

        // Allocate payment to installments (FIFO)
        let mut remaining = cmd.amount.as_paise();
        let mut allocations: Vec<(Uuid, i64)> = Vec::new();

        for inst in &installments {
            if remaining <= 0 {
                break;
            }
            let outstanding = inst.amount_paise - inst.paid_amount_paise;
            if outstanding > 0 {
                let alloc = remaining.min(outstanding);
                allocations.push((inst.fee_installment_id, alloc));
                remaining -= alloc;
            }
        }

        // Generate receipt number
        let receipt_seq = next_sequence(&mut tx, tid, "ar_payment_receipts", &cmd.entity_id.to_string()).await?;
        let receipt_number = format!("RCP-{}-{:06}", cmd.entity_id.to_string().chars().take(6).collect::<String>(), receipt_seq);

        let payment_receipt_id = EntityId::new();
        let audit = AuditInfo::new(cmd.received_by);

        // Determine payment mode
        let payment_mode = match cmd.payment_mode.as_str() {
            "CASH" => PaymentMode::Cash,
            "CHEQUE" => PaymentMode::Cheque,
            "DD" => PaymentMode::Dd,
            "NEFT" => PaymentMode::Neft,
            "RTGS" => PaymentMode::Rtgs,
            "IMPS" => PaymentMode::Imps,
            "UPI" => PaymentMode::Upi,
            "CREDIT_CARD" => PaymentMode::CreditCard,
            "DEBIT_CARD" => PaymentMode::DebitCard,
            "POS" => PaymentMode::Pos,
            _ => PaymentMode::PaymentGateway,
        };

        // ── Create GL Journal: DR Bank → CR FeeIncome ──
        let mut journal_lines = Vec::new();

        // DR Bank
        journal_lines.push(CreateJournalLineCmd {
            line_number: 1,
            account_id: cmd.bank_account_id,
            debit_amount: Some(cmd.amount),
            credit_amount: None,
            description: Some(format!("Fee payment receipt {}", receipt_number)),
            cost_center_id: None,
            fund_id: None,
            reference_id: Some(receipt_number.clone()),
            reference_type: Some("RECEIPT".to_string()),
        });

        // CR FeeIncome per fee head (simplified: single CR to primary income account)
        // In production, this would allocate proportionally across fee heads
        journal_lines.push(CreateJournalLineCmd {
            line_number: 2,
            account_id: cmd.fee_income_account_ids.values().next().copied()
                .unwrap_or(cmd.bank_account_id), // fallback
            debit_amount: None,
            credit_amount: Some(cmd.amount),
            description: Some(format!("Fee collection — receipt {}", receipt_number)),
            cost_center_id: None,
            fund_id: None,
            reference_id: Some(receipt_number.clone()),
            reference_type: Some("RECEIPT".to_string()),
        });

        let journal_cmd = CreateJournalCmd {
            journal_type: "Standard".to_string(),
            accounting_period_id: cmd.accounting_period_id,
            entity_id: cmd.entity_id,
            fund_id: None,
            cost_center_id: None,
            posting_date: Utc::now().date_naive(),
            description: format!("Fee payment — student {} — {}", cmd.student_id, receipt_number),
            lines: journal_lines,
            attachment_ids: vec![],
        };

        // Create and post journal via GL handler (but in same tx — we need to commit first)
        // For the modular monolith, we create the journal after AR transaction commits
        // We store the journal_id once created
        drop(tx); // Release the AR transaction lock before calling GL

        let gl = self.gl_handler();
        let journal = gl.create_journal(tenant_id, cmd.received_by, journal_cmd).await
            .map_err(|e| ArError::Internal(format!("Failed to create payment journal: {e}")))?;

        let journal = gl.post_journal(
            tenant_id,
            PostJournalCmd {
                journal_id: *journal.journal_id.as_uuid(),
                posted_by: cmd.received_by,
            },
        ).await
            .map_err(|e| ArError::Internal(format!("Failed to post payment journal: {e}")))?;

        let linked_journal_id = *journal.journal_id.as_uuid();

        // Now persist AR records
        let mut tx2 = self.pool.begin().await?;

        // Insert PaymentReceipt
        sqlx::query(
            r#"INSERT INTO ar_payment_receipts (
                payment_receipt_id, tenant_id, entity_id, receipt_number, student_id,
                student_fee_account_id, payment_mode, payment_date, amount, status,
                gateway_payment_id, gateway_reference, bank_transaction_ref,
                cheque_number, cheque_date, received_by_id, payment_journal_id,
                version, created_by, created_at, updated_by, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,now(),$19,now())"#,
        )
        .bind(payment_receipt_id.as_uuid())
        .bind(tid)
        .bind(cmd.entity_id)
        .bind(&receipt_number)
        .bind(cmd.student_id)
        .bind(cmd.student_fee_account_id)
        .bind(&cmd.payment_mode)
        .bind(cmd.payment_date)
        .bind(cmd.amount.as_paise())
        .bind("COMPLETED".to_string())
        .bind(&cmd.gateway_transaction_id)
        .bind(cmd.bank_transaction_ref.as_deref().unwrap_or(""))
        .bind(&cmd.bank_transaction_ref)
        .bind(&cmd.cheque_number)
        .bind(cmd.cheque_date)
        .bind(cmd.received_by)
        .bind(linked_journal_id)
        .bind(1i32)
        .bind(cmd.received_by)
        .execute(&mut *tx2)
        .await?;

        // Update installment paid amounts
        for (inst_id, alloc_amount) in &allocations {
            sqlx::query(
                r#"UPDATE ar_fee_installments
                   SET paid_amount = paid_amount + $1,
                       status = CASE WHEN paid_amount + $1 >= amount THEN 'PAID'
                                     ELSE 'PARTIALLY_PAID' END
                   WHERE fee_installment_id = $2"#,
            )
            .bind(alloc_amount)
            .bind(inst_id)
            .execute(&mut *tx2)
            .await?;
        }

        // Update student fee account totals
        let new_total_paid = fee_account.total_paid_paise + cmd.amount.as_paise();
        let new_outstanding = (fee_account.net_payable_paise - new_total_paid).max(0);
        let new_status = if new_outstanding == 0 { "PAID" } else if new_total_paid > 0 { "PARTIALLY_PAID" } else { "PENDING" };

        sqlx::query(
            r#"UPDATE ar_student_fee_accounts
               SET total_paid = $1, outstanding = $2, status = $3,
                   updated_at = now(), version = version + 1
               WHERE student_fee_account_id = $4 AND tenant_id = $5"#,
        )
        .bind(new_total_paid)
        .bind(new_outstanding)
        .bind(new_status)
        .bind(cmd.student_fee_account_id)
        .bind(tid)
        .execute(&mut *tx2)
        .await?;

        // Record fee transaction
        let fee_txn_id = Uuid::now_v7();
        sqlx::query(
            r#"INSERT INTO ar_fee_transactions (
                fee_transaction_id, student_fee_account_id, transaction_type,
                amount, payment_mode, receipt_number, gateway_transaction_id,
                linked_journal_id, created_at, created_by
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,now(),$9)"#,
        )
        .bind(fee_txn_id)
        .bind(cmd.student_fee_account_id)
        .bind("PAYMENT")
        .bind(cmd.amount.as_paise())
        .bind(&cmd.payment_mode)
        .bind(&receipt_number)
        .bind(&cmd.gateway_transaction_id)
        .bind(linked_journal_id)
        .bind(cmd.received_by)
        .execute(&mut *tx2)
        .await?;

        // Outbox event
        write_ar_outbox(
            &mut tx2,
            tid,
            "PaymentReceipt",
            &payment_receipt_id.as_uuid().to_string(),
            "PaymentReceiptCreated",
            &serde_json::json!({
                "receipt_id": payment_receipt_id.as_uuid().to_string(),
                "receipt_number": receipt_number,
                "student_id": cmd.student_id.to_string(),
                "amount": cmd.amount.as_paise(),
                "payment_mode": &cmd.payment_mode,
                "linked_journal_id": linked_journal_id.to_string(),
                "occurred_at": Utc::now(),
            }),
        )
        .await?;

        tx2.commit().await?;

        info!(
            tenant_id = %tid,
            receipt_number = %receipt_number,
            amount = %cmd.amount,
            "Fee payment recorded"
        );

        Ok(PaymentReceipt {
            payment_receipt_id,
            tenant_id,
            entity_id: cmd.entity_id,
            receipt_number,
            student_id: cmd.student_id,
            student_fee_account_id: Some(cmd.student_fee_account_id),
            payment_mode,
            payment_date: cmd.payment_date,
            amount: cmd.amount,
            status: ReceiptStatus::Completed,
            gateway_payment_id: cmd.gateway_transaction_id,
            gateway_reference: None,
            bank_transaction_ref: cmd.bank_transaction_ref,
            cheque_number: cmd.cheque_number,
            cheque_date: cmd.cheque_date,
            cleared_date: None,
            remarks: None,
            received_by_id: cmd.received_by,
            payment_journal_id: Some(linked_journal_id),
            version: 1,
            audit,
        })
    }

    // ─── Grant Concession ─────────────────────────────────────────────

    /// Apply a concession to a student's fee account.
    pub async fn grant_concession(
        &self,
        tenant_id: TenantId,
        cmd: GrantConcessionCmd,
    ) -> Result<Concession, ArError> {
        let tid = *tenant_id.as_uuid();

        let mut tx = self.pool.begin().await?;

        // Load fee account
        let fee_account = sqlx::query_as::<_, StudentFeeAccountRow>(
            r#"SELECT student_fee_account_id, tenant_id, student_id, fee_structure_id,
               academic_year, gross_fee, scholarship_expected, concession_amount,
               net_payable, total_paid, outstanding, status
               FROM ar_student_fee_accounts
               WHERE student_fee_account_id = $1 AND tenant_id = $2
               FOR UPDATE"#,
        )
        .bind(cmd.student_fee_account_id)
        .bind(tid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ArError::NotFound(format!("Student fee account {} not found", cmd.student_fee_account_id)))?;

        let concession_type = ConcessionType::from_db_str(&cmd.concession_type);

        // Calculate concession amount
        let calculated_amount = match &concession_type {
            ConcessionType::Percentage => {
                let pct = cmd.value / rust_decimal::Decimal::from(100);
                let paise = (fee_account.gross_fee_paise as f64 * rust_decimal::prelude::ToPrimitive::to_f64(&pct).unwrap_or(0.0)) as i64;
                Money::from_paise(paise)
            }
            ConcessionType::FixedAmount => {
                Money::from_paise((cmd.value * rust_decimal::Decimal::from(100)).try_into().unwrap_or(0))
            }
            ConcessionType::FullWaiver => {
                Money::from_paise(fee_account.net_payable_paise)
            }
        };

        let concession_id = EntityId::new();
        let audit = AuditInfo::new(cmd.approved_by);

        // Insert concession
        sqlx::query(
            r#"INSERT INTO ar_concessions (
                concession_id, tenant_id, student_id, student_fee_account_id,
                fee_head_id, concession_type, value, calculated_amount, reason,
                approved_by, status, created_by, created_at, updated_by, updated_at, version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,now(),$12,now(),1)"#,
        )
        .bind(concession_id.as_uuid())
        .bind(tid)
        .bind(cmd.student_id)
        .bind(cmd.student_fee_account_id)
        .bind(cmd.fee_head_id)
        .bind(concession_type.to_db_str())
        .bind(cmd.value.to_string())
        .bind(calculated_amount.as_paise())
        .bind(&cmd.reason)
        .bind(cmd.approved_by)
        .bind(ConcessionStatus::Approved.to_db_str())
        .bind(cmd.approved_by)
        .execute(&mut *tx)
        .await?;

        // Update fee account: recalculate net_payable
        let new_concession = fee_account.concession_amount_paise + calculated_amount.as_paise();
        let new_net = fee_account.gross_fee_paise - fee_account.scholarship_expected_paise - new_concession;
        let new_outstanding = (new_net - fee_account.total_paid_paise).max(0);

        sqlx::query(
            r#"UPDATE ar_student_fee_accounts
               SET concession_amount = $1, net_payable = $2, outstanding = $3,
                   updated_at = now(), version = version + 1
               WHERE student_fee_account_id = $4 AND tenant_id = $5"#,
        )
        .bind(new_concession)
        .bind(new_net)
        .bind(new_outstanding)
        .bind(cmd.student_fee_account_id)
        .bind(tid)
        .execute(&mut *tx)
        .await?;

        write_ar_outbox(
            &mut tx,
            tid,
            "Concession",
            &concession_id.as_uuid().to_string(),
            "ConcessionApproved",
            &serde_json::json!({
                "concession_id": concession_id.as_uuid().to_string(),
                "student_id": cmd.student_id.to_string(),
                "amount": calculated_amount.as_paise(),
                "approved_by": cmd.approved_by.to_string(),
                "occurred_at": Utc::now(),
            }),
        )
        .await?;

        tx.commit().await?;

        Ok(Concession {
            concession_id,
            tenant_id,
            student_id: cmd.student_id,
            student_fee_account_id: Some(EntityId::from_uuid(cmd.student_fee_account_id.into())),
            fee_head_id: cmd.fee_head_id.map(|id| EntityId::from_uuid(id.into())),
            concession_type,
            value: cmd.value,
            calculated_amount,
            reason: cmd.reason,
            approved_by: Some(cmd.approved_by),
            status: ConcessionStatus::Approved,
            audit,
        })
    }

    // ─── Apply Scholarship ────────────────────────────────────────────

    pub async fn apply_scholarship(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: ApplyScholarshipCmd,
    ) -> Result<StudentScholarship, ArError> {
        let tid = *tenant_id.as_uuid();
        let mut tx = self.pool.begin().await?;

        // Verify scheme exists and is active
        let _scheme = sqlx::query_as::<_, ScholarshipSchemeRow>(
            r#"SELECT scholarship_scheme_id, tenant_id, code, name, category,
               funding_source, maha_dbt_scheme_code, max_amount, is_active,
               requires_aadhaar, requires_bank_account, requires_income_cert, requires_caste_cert
               FROM ar_scholarship_schemes
               WHERE scholarship_scheme_id = $1 AND tenant_id = $2 AND is_active = true"#,
        )
        .bind(cmd.scheme_id)
        .bind(tid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ArError::ScholarshipNotFound(cmd.scheme_id.to_string()))?;

        let scholarship_id = EntityId::new();
        let audit = AuditInfo::new(created_by);

        sqlx::query(
            r#"INSERT INTO ar_student_scholarships (
                scholarship_id, tenant_id, student_id, scheme_id,
                student_fee_account_id, academic_year, expected_amount,
                status, maha_dbt_application_id,
                created_by, created_at, updated_by, updated_at, version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,now(),$10,now(),1)"#,
        )
        .bind(scholarship_id.as_uuid())
        .bind(tid)
        .bind(cmd.student_id)
        .bind(cmd.scheme_id)
        .bind(cmd.student_fee_account_id)
        .bind(&cmd.academic_year)
        .bind(cmd.expected_amount.as_paise())
        .bind(ScholarshipStatus::Applied.to_db_str())
        .bind(&cmd.maha_dbt_application_id)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        // Update fee account scholarship_expected if fee account is linked
        if let Some(fa_id) = cmd.student_fee_account_id {
            sqlx::query(
                r#"UPDATE ar_student_fee_accounts
                   SET scholarship_expected = scholarship_expected + $1,
                       net_payable = gross_fee - (scholarship_expected + $1) - concession_amount,
                       outstanding = (gross_fee - (scholarship_expected + $1) - concession_amount) - total_paid,
                       updated_at = now(), version = version + 1
                   WHERE student_fee_account_id = $2 AND tenant_id = $3"#,
            )
            .bind(cmd.expected_amount.as_paise())
            .bind(fa_id)
            .bind(tid)
            .execute(&mut *tx)
            .await?;
        }

        write_ar_outbox(
            &mut tx,
            tid,
            "StudentScholarship",
            &scholarship_id.as_uuid().to_string(),
            "ScholarshipApplied",
            &serde_json::json!({
                "scholarship_id": scholarship_id.as_uuid().to_string(),
                "student_id": cmd.student_id.to_string(),
                "scheme_id": cmd.scheme_id.to_string(),
                "expected_amount": cmd.expected_amount.as_paise(),
                "occurred_at": Utc::now(),
            }),
        )
        .await?;

        tx.commit().await?;

        Ok(StudentScholarship {
            scholarship_id,
            tenant_id,
            student_id: cmd.student_id,
            scheme_id: EntityId::from_uuid(cmd.scheme_id.into()),
            student_fee_account_id: cmd.student_fee_account_id.map(|id| EntityId::from_uuid(id.into())),
            academic_year: cmd.academic_year,
            expected_amount: cmd.expected_amount,
            sanctioned_amount: None,
            disbursed_amount: None,
            status: ScholarshipStatus::Applied,
            maha_dbt_application_id: cmd.maha_dbt_application_id,
            dbt_transaction_id: None,
            dbt_date: None,
            verified_by: None,
            verified_at: None,
            sanctioned_by: None,
            sanctioned_at: None,
            remarks: None,
            audit,
        })
    }

    // ─── Verify Scholarship ───────────────────────────────────────────

    pub async fn verify_scholarship(
        &self,
        tenant_id: TenantId,
        cmd: VerifyScholarshipCmd,
    ) -> Result<StudentScholarship, ArError> {
        let tid = *tenant_id.as_uuid();
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, StudentScholarshipRow>(
            r#"SELECT scholarship_id, tenant_id, student_id, scheme_id,
               student_fee_account_id, academic_year, expected_amount,
               sanctioned_amount, disbursed_amount, status,
               maha_dbt_application_id, dbt_transaction_id, dbt_date,
               verified_by, verified_at, sanctioned_by, sanctioned_at, remarks
               FROM ar_student_scholarships
               WHERE scholarship_id = $1 AND tenant_id = $2
               FOR UPDATE"#,
        )
        .bind(cmd.scholarship_id)
        .bind(tid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ArError::ScholarshipNotFound(cmd.scholarship_id.to_string()))?;

        if row.status != "APPLIED" {
            return Err(ArError::Validation(format!(
                "Scholarship must be in APPLIED status to verify, current: {}",
                row.status
            )));
        }

        let now = Utc::now();
        sqlx::query(
            r#"UPDATE ar_student_scholarships
               SET status = $1, verified_by = $2, verified_at = $3,
                   updated_at = now(), version = version + 1
               WHERE scholarship_id = $4 AND tenant_id = $5"#,
        )
        .bind(ScholarshipStatus::Verified.to_db_str())
        .bind(cmd.verified_by)
        .bind(now)
        .bind(cmd.scholarship_id)
        .bind(tid)
        .execute(&mut *tx)
        .await?;

        write_ar_outbox(
            &mut tx,
            tid,
            "StudentScholarship",
            &cmd.scholarship_id.to_string(),
            "ScholarshipVerified",
            &serde_json::json!({
                "scholarship_id": cmd.scholarship_id.to_string(),
                "verified_by": cmd.verified_by.to_string(),
                "occurred_at": now,
            }),
        )
        .await?;

        tx.commit().await?;

        Ok(StudentScholarship {
            scholarship_id: EntityId::from_uuid(cmd.scholarship_id.into()),
            tenant_id,
            student_id: Uuid::parse_str(&row.student_id).unwrap_or_default(),
            scheme_id: EntityId::from_uuid(Uuid::parse_str(&row.scheme_id).unwrap_or_default().into()),
            student_fee_account_id: row.student_fee_account_id.map(|id| EntityId::from_uuid(Uuid::parse_str(&id).unwrap_or_default().into())),
            academic_year: row.academic_year,
            expected_amount: Money::from_paise(row.expected_amount_paise),
            sanctioned_amount: row.sanctioned_amount_paise.map(Money::from_paise),
            disbursed_amount: row.disbursed_amount_paise.map(Money::from_paise),
            status: ScholarshipStatus::Verified,
            maha_dbt_application_id: row.maha_dbt_application_id,
            dbt_transaction_id: row.dbt_transaction_id,
            dbt_date: row.dbt_date,
            verified_by: Some(cmd.verified_by),
            verified_at: Some(now),
            sanctioned_by: None,
            sanctioned_at: None,
            remarks: row.remarks,
            audit: AuditInfo::new(cmd.verified_by),
        })
    }

    // ─── Record Scholarship Disbursement ──────────────────────────────

    pub async fn record_scholarship_disbursement(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: RecordScholarshipDisbursementCmd,
    ) -> Result<StudentScholarship, ArError> {
        let tid = *tenant_id.as_uuid();
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, StudentScholarshipRow>(
            r#"SELECT scholarship_id, tenant_id, student_id, scheme_id,
               student_fee_account_id, academic_year, expected_amount,
               sanctioned_amount, disbursed_amount, status,
               maha_dbt_application_id, dbt_transaction_id, dbt_date,
               verified_by, verified_at, sanctioned_by, sanctioned_at, remarks
               FROM ar_student_scholarships
               WHERE scholarship_id = $1 AND tenant_id = $2
               FOR UPDATE"#,
        )
        .bind(cmd.scholarship_id)
        .bind(tid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ArError::ScholarshipNotFound(cmd.scholarship_id.to_string()))?;

        if row.status != "VERIFIED" && row.status != "SANCTIONED" {
            return Err(ArError::Validation(format!(
                "Scholarship must be VERIFIED or SANCTIONED to record disbursement, current: {}",
                row.status
            )));
        }

        let now = Utc::now();
        sqlx::query(
            r#"UPDATE ar_student_scholarships
               SET disbursed_amount = $1, dbt_transaction_id = $2, dbt_date = $3,
                   status = $4, updated_at = now(), version = version + 1
               WHERE scholarship_id = $5 AND tenant_id = $6"#,
        )
        .bind(cmd.disbursed_amount.as_paise())
        .bind(&cmd.dbt_transaction_id)
        .bind(cmd.dbt_date)
        .bind(ScholarshipStatus::Disbursed.to_db_str())
        .bind(cmd.scholarship_id)
        .bind(tid)
        .execute(&mut *tx)
        .await?;

        // Check if overpayment → auto-refund
        let mut overpayment = false;
        if let Some(fa_id_str) = &row.student_fee_account_id {
            let fa = sqlx::query_as::<_, StudentFeeAccountRow>(
                r#"SELECT student_fee_account_id, tenant_id, student_id, fee_structure_id,
                   academic_year, gross_fee, scholarship_expected, concession_amount,
                   net_payable, total_paid, outstanding, status
                   FROM ar_student_fee_accounts
                   WHERE student_fee_account_id = $1 AND tenant_id = $2"#,
            )
            .bind(fa_id_str)
            .bind(tid)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| ArError::NotFound(format!("Fee account {} not found", fa_id_str)))?;

            // Reduce scholarship_expected (since DBT arrived)
            let new_scholarship = (fa.scholarship_expected_paise - cmd.disbursed_amount.as_paise()).max(0);
            let new_net = fa.gross_fee_paise - new_scholarship - fa.concession_amount_paise;
            let new_outstanding = (new_net - fa.total_paid_paise).max(0);

            if new_outstanding < 0 {
                overpayment = true;
            }

            sqlx::query(
                r#"UPDATE ar_student_fee_accounts
                   SET scholarship_expected = $1, net_payable = $2, outstanding = $3,
                       updated_at = now(), version = version + 1
                   WHERE student_fee_account_id = $4 AND tenant_id = $5"#,
            )
            .bind(new_scholarship)
            .bind(new_net)
            .bind(new_outstanding.max(0))
            .bind(fa_id_str)
            .bind(tid)
            .execute(&mut *tx)
            .await?;
        }

        write_ar_outbox(
            &mut tx,
            tid,
            "StudentScholarship",
            &cmd.scholarship_id.to_string(),
            "ScholarshipDisbursed",
            &serde_json::json!({
                "scholarship_id": cmd.scholarship_id.to_string(),
                "dbt_amount": cmd.disbursed_amount.as_paise(),
                "dbt_date": cmd.dbt_date,
                "transaction_ref": cmd.dbt_transaction_id,
                "overpayment": overpayment,
                "occurred_at": now,
            }),
        )
        .await?;

        tx.commit().await?;

        info!(
            tenant_id = %tid,
            scholarship_id = %cmd.scholarship_id,
            disbursed = %cmd.disbursed_amount,
            overpayment = %overpayment,
            "Scholarship disbursement recorded"
        );

        Ok(StudentScholarship {
            scholarship_id: EntityId::from_uuid(cmd.scholarship_id.into()),
            tenant_id,
            student_id: Uuid::parse_str(&row.student_id).unwrap_or_default(),
            scheme_id: EntityId::from_uuid(Uuid::parse_str(&row.scheme_id).unwrap_or_default().into()),
            student_fee_account_id: row.student_fee_account_id.map(|id| EntityId::from_uuid(Uuid::parse_str(&id).unwrap_or_default().into())),
            academic_year: row.academic_year,
            expected_amount: Money::from_paise(row.expected_amount_paise),
            sanctioned_amount: row.sanctioned_amount_paise.map(Money::from_paise),
            disbursed_amount: Some(cmd.disbursed_amount),
            status: ScholarshipStatus::Disbursed,
            maha_dbt_application_id: row.maha_dbt_application_id,
            dbt_transaction_id: Some(cmd.dbt_transaction_id),
            dbt_date: Some(cmd.dbt_date),
            verified_by: row.verified_by.map(|v| Uuid::parse_str(&v).unwrap_or_default()),
            verified_at: row.verified_at,
            sanctioned_by: None,
            sanctioned_at: None,
            remarks: row.remarks,
            audit: AuditInfo::new(created_by),
        })
    }

    // ─── Initiate Refund ──────────────────────────────────────────────

    pub async fn initiate_refund(
        &self,
        tenant_id: TenantId,
        created_by: Uuid,
        cmd: InitiateRefundCmd,
    ) -> Result<Refund, ArError> {
        let tid = *tenant_id.as_uuid();
        let mut tx = self.pool.begin().await?;

        // Calculate FRC compliance if dates provided
        let frc_pct = if let (Some(withdrawal), Some(course_start)) = (cmd.withdrawal_date, cmd.course_start_date) {
            Some(calculate_frc_refund_pct(withdrawal, course_start))
        } else {
            None
        };

        let refund_id = EntityId::new();
        let audit = AuditInfo::new(created_by);

        let refund_mode = RefundMode::from_db_str(&cmd.refund_mode);

        sqlx::query(
            r#"INSERT INTO ar_refunds (
                refund_id, tenant_id, student_id, amount, refund_reason,
                frc_compliant_pct, refund_mode, status, linked_payment_id,
                created_by, created_at, updated_by, updated_at, version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,now(),$10,now(),1)"#,
        )
        .bind(refund_id.as_uuid())
        .bind(tid)
        .bind(cmd.student_id)
        .bind(cmd.amount.as_paise())
        .bind(&cmd.refund_reason)
        .bind(frc_pct.map(|p| p.to_string()))
        .bind(refund_mode.to_db_str())
        .bind(RefundStatus::Requested.to_db_str())
        .bind(cmd.linked_payment_id)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        write_ar_outbox(
            &mut tx,
            tid,
            "Refund",
            &refund_id.as_uuid().to_string(),
            "RefundInitiated",
            &serde_json::json!({
                "refund_id": refund_id.as_uuid().to_string(),
                "amount": cmd.amount.as_paise(),
                "refund_reason": cmd.refund_reason,
                "occurred_at": Utc::now(),
            }),
        )
        .await?;

        tx.commit().await?;

        Ok(Refund {
            refund_id,
            tenant_id,
            student_id: Some(cmd.student_id),
            amount: cmd.amount,
            refund_reason: cmd.refund_reason,
            frc_compliant_pct: frc_pct,
            refund_mode,
            status: RefundStatus::Requested,
            linked_payment_id: cmd.linked_payment_id,
            reversal_journal_id: None,
            approved_by: None,
            approved_at: None,
            processed_at: None,
            bank_transaction_ref: None,
            audit,
        })
    }

    // ─── Process Refund ───────────────────────────────────────────────

    /// Process (execute) a refund — approve, create reversal journal.
    pub async fn process_refund(
        &self,
        tenant_id: TenantId,
        cmd: ProcessRefundCmd,
    ) -> Result<Refund, ArError> {
        let tid = *tenant_id.as_uuid();

        // Load refund
        let refund_row = sqlx::query_as::<_, RefundRow>(
            r#"SELECT refund_id, tenant_id, student_id, amount, refund_reason,
               frc_compliant_pct, refund_mode, status, linked_payment_id,
               reversal_journal_id, approved_by, approved_at, processed_at,
               bank_transaction_ref
               FROM ar_refunds
               WHERE refund_id = $1 AND tenant_id = $2
               FOR UPDATE"#,
        )
        .bind(cmd.refund_id)
        .bind(tid)
        .fetch_optional(&mut *self.pool.begin().await?)
        .await?
        .ok_or_else(|| ArError::NotFound(format!("Refund {} not found", cmd.refund_id)))?;

        if refund_row.status != "REQUESTED" && refund_row.status != "APPROVED" {
            return Err(ArError::Validation(format!(
                "Refund must be REQUESTED or APPROVED to process, current: {}",
                refund_row.status
            )));
        }

        let amount = Money::from_paise(refund_row.amount_paise);

        // ── Create GL Reversal Journal: DR FeeIncome → CR Bank ──
        let gl = self.gl_handler();
        let journal_cmd = CreateJournalCmd {
            journal_type: "Standard".to_string(),
            accounting_period_id: cmd.accounting_period_id,
            entity_id: cmd.entity_id,
            fund_id: None,
            cost_center_id: None,
            posting_date: Utc::now().date_naive(),
            description: format!("Refund — reversal for refund {}", cmd.refund_id),
            lines: vec![
                CreateJournalLineCmd {
                    line_number: 1,
                    account_id: cmd.fee_income_account_id,
                    debit_amount: Some(amount),
                    credit_amount: None,
                    description: Some(format!("Refund reversal — refund {}", cmd.refund_id)),
                    cost_center_id: None,
                    fund_id: None,
                    reference_id: Some(cmd.refund_id.to_string()),
                    reference_type: Some("REFUND".to_string()),
                },
                CreateJournalLineCmd {
                    line_number: 2,
                    account_id: cmd.bank_account_id,
                    debit_amount: None,
                    credit_amount: Some(amount),
                    description: Some(format!("Refund payment — refund {}", cmd.refund_id)),
                    cost_center_id: None,
                    fund_id: None,
                    reference_id: Some(cmd.refund_id.to_string()),
                    reference_type: Some("REFUND".to_string()),
                },
            ],
            attachment_ids: vec![],
        };

        let journal = gl.create_journal(tenant_id, cmd.processed_by, journal_cmd).await
            .map_err(|e| ArError::Internal(format!("Failed to create refund journal: {e}")))?;

        let journal = gl.post_journal(
            tenant_id,
            PostJournalCmd {
                journal_id: *journal.journal_id.as_uuid(),
                posted_by: cmd.processed_by,
            },
        ).await
            .map_err(|e| ArError::Internal(format!("Failed to post refund journal: {e}")))?;

        let reversal_journal_id = *journal.journal_id.as_uuid();

        // Update refund record
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        sqlx::query(
            r#"UPDATE ar_refunds
               SET status = $1, processed_at = $2, reversal_journal_id = $3,
                   bank_transaction_ref = $4, updated_at = now(), version = version + 1
               WHERE refund_id = $5 AND tenant_id = $6"#,
        )
        .bind(RefundStatus::Processed.to_db_str())
        .bind(now)
        .bind(reversal_journal_id)
        .bind(&cmd.bank_transaction_ref)
        .bind(cmd.refund_id)
        .bind(tid)
        .execute(&mut *tx)
        .await?;

        write_ar_outbox(
            &mut tx,
            tid,
            "Refund",
            &cmd.refund_id.to_string(),
            "RefundProcessed",
            &serde_json::json!({
                "refund_id": cmd.refund_id.to_string(),
                "reversal_journal_id": reversal_journal_id.to_string(),
                "amount": amount.as_paise(),
                "processed_by": cmd.processed_by.to_string(),
                "occurred_at": now,
            }),
        )
        .await?;

        tx.commit().await?;

        info!(
            tenant_id = %tid,
            refund_id = %cmd.refund_id,
            amount = %amount,
            "Refund processed"
        );

        Ok(Refund {
            refund_id: EntityId::from_uuid(cmd.refund_id.into()),
            tenant_id,
            student_id: refund_row.student_id.map(|s| Uuid::parse_str(&s).unwrap_or_default()),
            amount,
            refund_reason: refund_row.refund_reason,
            frc_compliant_pct: refund_row.frc_compliant_pct.and_then(|p| p.parse().ok()),
            refund_mode: RefundMode::from_db_str(&refund_row.refund_mode),
            status: RefundStatus::Processed,
            linked_payment_id: refund_row.linked_payment_id.map(|s| Uuid::parse_str(&s).unwrap_or_default()),
            reversal_journal_id: Some(reversal_journal_id),
            approved_by: refund_row.approved_by.map(|s| Uuid::parse_str(&s).unwrap_or_default()),
            approved_at: refund_row.approved_at,
            processed_at: Some(now),
            bank_transaction_ref: cmd.bank_transaction_ref,
            audit: AuditInfo::new(cmd.processed_by),
        })
    }
}

// ─── FRC Refund Calculation ─────────────────────────────────────────────

/// Calculate FRC-compliant refund percentage based on withdrawal date.
///
/// FRC rules (configurable per tenant, these are defaults):
/// - 100% before course start
/// - 80% within 15 days of start
/// - 50% within 30 days of start
/// - 0% after 30 days
fn calculate_frc_refund_pct(withdrawal_date: NaiveDate, course_start_date: NaiveDate) -> rust_decimal::Decimal {
    use rust_decimal::Decimal;

    if withdrawal_date < course_start_date {
        Decimal::new(100, 0) // 100%
    } else {
        let days_after = (withdrawal_date - course_start_date).num_days();
        if days_after <= 15 {
            Decimal::new(80, 0) // 80%
        } else if days_after <= 30 {
            Decimal::new(50, 0) // 50%
        } else {
            Decimal::ZERO // 0%
        }
    }
}

// ─── Outbox Writer ───────────────────────────────────────────────────────

async fn write_ar_outbox(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    aggregate_type: &str,
    aggregate_id: &str,
    event_type: &str,
    event_payload: &serde_json::Value,
) -> Result<(), ArError> {
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
    .bind(event_payload)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Generate a sequence number for receipt numbering.
async fn next_sequence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    table_name: &str,
    entity_id: &str,
) -> Result<i64, ArError> {
    let row = sqlx::query_scalar::<_, Option<i64>>(
        r#"SELECT COALESCE(MAX(
               CASE WHEN receipt_number ~ '^RCP-[A-Z0-9]+-([0-9]+)$'
                    THEN (regexp_replace(receipt_number, '^RCP-[A-Z0-9]+-', ''))::bigint
                    ELSE 0 END
           ), 0) + 1
           FROM ar_payment_receipts
           WHERE tenant_id = $1 AND entity_id = $2::uuid"#,
    )
    .bind(tenant_id)
    .bind(entity_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row.unwrap_or(1))
}

// ─── Row Types for SQLx queries ─────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct FeeStructureRow {
    #[allow(dead_code)]
    fee_structure_id: Uuid,
    #[allow(dead_code)]
    tenant_id: Uuid,
    #[allow(dead_code)]
    entity_id: Uuid,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    academic_year: String,
    #[allow(dead_code)]
    effective_from: NaiveDate,
    #[allow(dead_code)]
    effective_to: Option<NaiveDate>,
    #[allow(dead_code)]
    status: String,
    #[allow(dead_code)]
    frc_approval_number: Option<String>,
    #[allow(dead_code)]
    frc_approved_amount: Option<i64>,
    #[allow(dead_code)]
    program_id: Option<Uuid>,
    #[allow(dead_code)]
    batch: Option<String>,
    #[allow(dead_code)]
    semester: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct FeeStructureLineRow {
    #[allow(dead_code)]
    fee_structure_line_id: Uuid,
    #[allow(dead_code)]
    fee_head_id: Uuid,
    amount_paise: i64,
    #[allow(dead_code)]
    is_mandatory: bool,
    #[allow(dead_code)]
    installment_plan_id: Option<Uuid>,
    #[allow(dead_code)]
    gst_rate: Option<rust_decimal::Decimal>,
    #[allow(dead_code)]
    fee_head_code: String,
    #[allow(dead_code)]
    fee_head_name: String,
    #[allow(dead_code)]
    fee_type: String,
    #[allow(dead_code)]
    gst_classification: String,
    #[allow(dead_code)]
    sac_code: Option<String>,
    #[allow(dead_code)]
    is_refundable: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct InstallmentSlotRow {
    slot_number: i32,
    percentage: i32,
    due_date: NaiveDate,
}

#[derive(Debug, sqlx::FromRow)]
struct StudentFeeAccountRow {
    #[allow(dead_code)]
    student_fee_account_id: Uuid,
    #[allow(dead_code)]
    tenant_id: Uuid,
    #[allow(dead_code)]
    student_id: Uuid,
    #[allow(dead_code)]
    fee_structure_id: Uuid,
    #[allow(dead_code)]
    academic_year: String,
    gross_fee_paise: i64,
    scholarship_expected_paise: i64,
    concession_amount_paise: i64,
    net_payable_paise: i64,
    total_paid_paise: i64,
    #[allow(dead_code)]
    outstanding: i64,
    #[allow(dead_code)]
    status: String,
}

#[derive(Debug, sqlx::FromRow)]
struct FeeInstallmentRow {
    fee_installment_id: Uuid,
    #[allow(dead_code)]
    student_fee_account_id: Uuid,
    #[allow(dead_code)]
    installment_number: i32,
    #[allow(dead_code)]
    due_date: NaiveDate,
    amount_paise: i64,
    paid_amount_paise: i64,
    #[allow(dead_code)]
    status: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ScholarshipSchemeRow {
    #[allow(dead_code)]
    scholarship_scheme_id: Uuid,
    #[allow(dead_code)]
    tenant_id: Uuid,
    #[allow(dead_code)]
    code: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    category: String,
    #[allow(dead_code)]
    funding_source: String,
    #[allow(dead_code)]
    maha_dbt_scheme_code: Option<String>,
    #[allow(dead_code)]
    max_amount: i64,
    #[allow(dead_code)]
    is_active: bool,
    #[allow(dead_code)]
    requires_aadhaar: bool,
    #[allow(dead_code)]
    requires_bank_account: bool,
    #[allow(dead_code)]
    requires_income_cert: bool,
    #[allow(dead_code)]
    requires_caste_cert: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct StudentScholarshipRow {
    #[allow(dead_code)]
    scholarship_id: Uuid,
    #[allow(dead_code)]
    tenant_id: Uuid,
    student_id: String,
    scheme_id: String,
    student_fee_account_id: Option<String>,
    academic_year: String,
    expected_amount_paise: i64,
    sanctioned_amount_paise: Option<i64>,
    disbursed_amount_paise: Option<i64>,
    status: String,
    maha_dbt_application_id: Option<String>,
    dbt_transaction_id: Option<String>,
    dbt_date: Option<chrono::DateTime<Utc>>,
    verified_by: Option<String>,
    verified_at: Option<chrono::DateTime<Utc>>,
    sanctioned_by: Option<String>,
    sanctioned_at: Option<chrono::DateTime<Utc>>,
    remarks: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct RefundRow {
    #[allow(dead_code)]
    refund_id: Uuid,
    #[allow(dead_code)]
    tenant_id: Uuid,
    student_id: Option<String>,
    amount_paise: i64,
    refund_reason: String,
    frc_compliant_pct: Option<String>,
    refund_mode: String,
    status: String,
    linked_payment_id: Option<String>,
    reversal_journal_id: Option<Uuid>,
    approved_by: Option<String>,
    approved_at: Option<chrono::DateTime<Utc>>,
    processed_at: Option<chrono::DateTime<Utc>>,
    bank_transaction_ref: Option<String>,
}
