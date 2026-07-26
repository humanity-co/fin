use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArError {
    #[error("fee structure {0} not found")]
    FeeStructureNotFound(String),
    #[error("payment receipt {0} not found")]
    ReceiptNotFound(String),
    #[error("scholarship {0} not found")]
    ScholarshipNotFound(String),
}
