//! Tax Engine module errors.
use thiserror::Error;

/// Errors raised by the tax engine.
///
/// Every variant maps to a stable, machine-readable code so the API layer
/// can return the same error envelope used by the other finance modules
/// (see `crates/api/src/routes/treasury.rs::err_from`: NotFound variants →
/// 404, `Database` → 500, everything else → 422).
#[derive(Debug, Error)]
pub enum TaxError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("journal engine error: {0}")]
    Gl(String),

    // ── GST registration & rate master ────────────────────────────────
    #[error("GST registration {0} not found")]
    GstRegistrationNotFound(String),

    #[error("invalid GSTIN: {0} (expected 15 characters: 2 state + 10 PAN + 1 entity + 1 check + 1 'Z')")]
    InvalidGstin(String),

    #[error("GSTIN {0} is already registered for this tenant")]
    DuplicateGstin(String),

    #[error("registration state code {actual} does not match the entity's registered state {expected}")]
    GstStateCodeMismatch { expected: String, actual: String },

    #[error("GST rate for HSN/SAC {hsn_sac_code} not found (effective {as_of})")]
    GstRateNotFound { hsn_sac_code: String, as_of: String },

    #[error("invalid GST rate {0} — rate must be one of 0, 5, 12, 18, 28")]
    InvalidGstRate(i64),

    #[error("GST rate for HSN/SAC {hsn_sac_code} (supply_type {supply_type}) overlaps an existing effective period ({existing_from} to {existing_to})")]
    GstRateOverlap {
        hsn_sac_code: String,
        supply_type: String,
        existing_from: String,
        existing_to: String,
    },

    #[error("unknown GST classification for HSN/SAC {0} — tax engine refuses unclassified combinations (Notification 12/2017-CT(R))")]
    UnknownGstClassification(String),

    // ── GST returns ───────────────────────────────────────────────────
    #[error("GST return {0} not found")]
    GstReturnNotFound(String),

    #[error("GST return already exists for registration {registration_id}, type {return_type}, period {period}")]
    DuplicateGstReturn {
        registration_id: String,
        return_type: String,
        period: String,
    },

    #[error("period {0} is not open for computation")]
    PeriodNotOpen(String),

    #[error("GST return is not in the required state (current: {current}, expected: {expected})")]
    GstReturnStateViolation { current: String, expected: String },

    #[error("invalid acknowledgment number: {0}")]
    InvalidAcknowledgmentNo(String),

    #[error("GSTR-3B inconsistency: 3.1(d) RCM output ({rcm_output}) does not match 4(B)(2) RCM ITC ({rcm_itc})")]
    Gstr3bRcmMismatch { rcm_output: i64, rcm_itc: i64 },

    #[error("GSTR-1 vs GSTR-3B cross-check failed: outward taxable value differs (GSTR-1 {gstr1_txval} vs GSTR-3B 3.1(a) {gstr3b_txval}) — both returns must use the same tax-exclusive `net_amount` convention")]
    GstrConsistencyMismatch {
        period: String,
        gstr1_txval: i64,
        gstr3b_txval: i64,
        gstr1_tax: i64,
        gstr3b_tax: i64,
    },

    #[error("section {0} is not valid for return type {1}")]
    InvalidReturnSection(String, String),

    #[error("RCM entry already exists for invoice {0} (idempotency key collision)")]
    RcmEntryAlreadyExists(String),

    // ── ITC register ──────────────────────────────────────────────────
    #[error("ITC register for registration {0}, period {1} not found")]
    ItcRegisterNotFound(String, String),

    #[error("period {0} already computed for registration {1}")]
    PeriodAlreadyComputed(String, String),

    #[error("ITC register is not in the required state (current: {current}, expected: {expected})")]
    ItcRegisterStateViolation { current: String, expected: String },

    #[error("ITC register line {0} not found")]
    ItcLineNotFound(String),

    #[error("ITC eligibility mismatch on invoice {invoice_id}: account eligibility is {account_eligibility} but invoice line eligibility is {line_eligibility}")]
    ItcEligibilityMismatch {
        invoice_id: String,
        account_eligibility: String,
        line_eligibility: String,
    },

    #[error("reversal amount {reversal_amount} exceeds eligible ITC {eligible_amount} for register line {line_id}")]
    ItcReversalExceedsEligible {
        line_id: String,
        reversal_amount: i64,
        eligible_amount: i64,
    },

    #[error("exempt turnover {exempt_turnover} exceeds total turnover {total_turnover} (Rule 42 apportionment)")]
    ExemptTurnoverExceedsTotal { exempt_turnover: i64, total_turnover: i64 },

    // ── TDS ───────────────────────────────────────────────────────────
    #[error("TDS section {0} not found (effective {1})")]
    TdsSectionNotFound(String, String),

    #[error("TDS section {0} is inactive")]
    TdsSectionInactive(String),

    #[error("TDS rate {rate} for section {section_code} is outside the Finance Act band")]
    TdsRateOutOfBand { section_code: String, rate: String },

    #[error("TDS threshold must be >= 0")]
    TdsThresholdNegative,

    #[error("TDS deduction {0} not found")]
    TdsDeductionNotFound(String),

    #[error("TDS deduction is not in the required deposit state (current: {current}, expected: {expected})")]
    TdsDepositStateViolation { current: String, expected: String },

    #[error("no valid Section 197 certificate on file for vendor {0} — cannot apply lower rate")]
    Section197CertificateInvalid(String),

    #[error("PAN missing for deductee {0} — deduct at 20% under s.206AA")]
    PanMissing(String),

    #[error("TDS deposit {0} not found")]
    TdsDepositNotFound(String),

    #[error("TDS return {0} not found")]
    TdsReturnNotFound(String),

    #[error("TDS return already exists for entity {entity_id}, type {return_type}, quarter {quarter}, FY {fiscal_year}")]
    DuplicateTdsReturn {
        entity_id: String,
        return_type: String,
        quarter: String,
        fiscal_year: String,
    },

    #[error("TDS return is not in the required state (current: {current}, expected: {expected})")]
    TdsReturnStateViolation { current: String, expected: String },

    #[error("no DEPOSITED TDS deductions in quarter {0} — TDS return cannot be generated")]
    TdsReturnNotGeneratable(String),

    #[error("Form 24Q requires salary_month on all rows (missing on {0} deduction(s))")]
    Form24qSalaryMonthMissing(i64),

    #[error("Form 16 requires complete 24Q data for all 12 months of FY {0}")]
    Form16DataIncomplete(String),

    #[error("Form 16/16A certificate {0} not found")]
    Form16CertificateNotFound(String),

    #[error("certificate already generated for this {subject} and FY {fiscal_year}")]
    DuplicateCertificate { subject: String, fiscal_year: String },

    // ── Trust exemption & income tax compliance ───────────────────────
    #[error("trust exemption under {0} not found for entity {1}")]
    TrustExemptionNotFound(String, String),

    #[error("trust exemption under section {0} already exists for this entity")]
    DuplicateTrustExemption(String),

    #[error("trust exemption for entity {0} has expired ({1}) — exemption lost for the year")]
    TrustExemptionExpired(String, String),

    #[error("trust exemption cannot be renewed from status {0} (renewal requires ACTIVE or RENEWAL_PENDING)")]
    TrustExemptionNotRenewable(String),

    #[error("renewal must be applied before expiry; {0} days remain")]
    ExemptionRenewalTooLate(i64),

    #[error("income application for fiscal year {0} not found")]
    IncomeApplicationNotFound(String),

    #[error("income application for fiscal year {fiscal_year_id} already computed (status {status})")]
    IncomeApplicationAlreadyComputed { fiscal_year_id: String, status: String },

    #[error("income application line amount must be > 0 (category {0})")]
    InvalidApplicationLineAmount(String),

    #[error("Section 11(5) investment check failed: {0}")]
    Section115Breach(String),

    #[error("FCRA registration {0} not found")]
    FcraRegistrationNotFound(String),

    #[error("FCRA registration {0} already exists for this tenant")]
    DuplicateFcraRegistration(String),

    #[error("bank account {0} is not an FCRA-designated account (FCRA 2010 s.17)")]
    FcraBankNotFcraType(String),

    #[error("FCRA administrative expenses exceed the {max_allowed}% cap (actual {expense_ratio}%)")]
    FcraAdminExpenseExceeded { expense_ratio: String, max_allowed: i64 },

    // ── Generic ───────────────────────────────────────────────────────
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("event publish error: {0}")]
    EventPublish(String),

    #[error("internal error: {0}")]
    Internal(String),
}
