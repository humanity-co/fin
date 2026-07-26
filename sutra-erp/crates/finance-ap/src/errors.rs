use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApError {
    #[error("vendor {0} not found")]
    VendorNotFound(String),
    #[error("purchase order {0} not found")]
    PONotFound(String),
    #[error("vendor is blacklisted")]
    VendorBlacklisted,
}
