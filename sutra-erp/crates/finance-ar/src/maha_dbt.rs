//! MahaDBT Integration Stub
//!
//! This module provides a stub implementation for the Maharashtra
//! Direct Benefit Transfer (MahaDBT) portal integration.
//! Real API integration will replace these stubs in a future iteration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sutra_core::Money;
use uuid::Uuid;

/// Result of verifying a student on MahaDBT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MahaDbtVerificationResult {
    pub verified: bool,
    pub student_name: String,
    pub aadhaar_linked: bool,
    pub bank_account_verified: bool,
    pub scheme_eligible: bool,
    pub remarks: Option<String>,
}

/// Status of a DBT disbursement from MahaDBT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MahaDbtDisbursementStatus {
    pub application_id: String,
    pub scheme_code: String,
    pub status: String, // PENDING, SANCTIONED, DISBURSED, REJECTED
    pub sanctioned_amount: Option<Money>,
    pub disbursed_amount: Option<Money>,
    pub dbt_transaction_id: Option<String>,
    pub dbt_date: Option<DateTime<Utc>>,
    pub bank_reference: Option<String>,
}

/// A stub client for MahaDBT integration.
///
/// In production, this will be replaced with actual HTTP calls to the
/// MahaDBT API endpoints.
#[derive(Debug, Clone)]
pub struct MahaDbtClient {
    /// Base URL for the MahaDBT API (not used in stub).
    #[allow(dead_code)]
    base_url: String,
    /// API key for authentication (not used in stub).
    #[allow(dead_code)]
    api_key: Option<String>,
}

impl MahaDbtClient {
    /// Create a new MahaDBT client stub.
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        MahaDbtClient { base_url, api_key }
    }

    /// Stub: Verify a student's details against MahaDBT records.
    ///
    /// Returns mock verification data. Will be replaced with actual API call.
    pub async fn verify_student(
        &self,
        _student_id: Uuid,
        _aadhaar_number: &str,
        _scheme_code: &str,
    ) -> Result<MahaDbtVerificationResult, String> {
        // Stub: return mock success
        Ok(MahaDbtVerificationResult {
            verified: true,
            student_name: "Mock Student".to_string(),
            aadhaar_linked: true,
            bank_account_verified: true,
            scheme_eligible: true,
            remarks: Some("Stub verification — real API integration pending".to_string()),
        })
    }

    /// Stub: Check disbursement status for a scholarship application.
    ///
    /// Returns mock disbursement data. Will be replaced with actual API call.
    pub async fn check_disbursement_status(
        &self,
        _application_id: &str,
    ) -> Result<MahaDbtDisbursementStatus, String> {
        // Stub: return mock disbursed status
        Ok(MahaDbtDisbursementStatus {
            application_id: _application_id.to_string(),
            scheme_code: "MOCK-SCHEME".to_string(),
            status: "DISBURSED".to_string(),
            sanctioned_amount: Some(Money::from_paise(2_500_000)), // ₹25,000
            disbursed_amount: Some(Money::from_paise(2_500_000)),
            dbt_transaction_id: Some(format!("DBT-{}", Uuid::new_v4())),
            dbt_date: Some(Utc::now()),
            bank_reference: Some("MOCK-BANK-REF-12345".to_string()),
        })
    }
}

impl Default for MahaDbtClient {
    fn default() -> Self {
        MahaDbtClient {
            base_url: "https://mahadbtmahait.gov.in/api".to_string(),
            api_key: None,
        }
    }
}
