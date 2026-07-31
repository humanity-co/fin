//! AP repository traits and implementations.
//! For the modular monolith pattern, direct SQLx queries are used in command handlers.
//! Repository traits defined here for future abstraction.

use async_trait::async_trait;
use uuid::Uuid;

use crate::errors::ApError;
use crate::models::vendor::Vendor;

/// Repository trait for vendor persistence.
#[async_trait]
pub trait VendorRepository {
    async fn find_by_id(&self, tenant_id: Uuid, vendor_id: Uuid) -> Result<Option<Vendor>, ApError>;
    async fn find_by_code(&self, tenant_id: Uuid, code: &str) -> Result<Option<Vendor>, ApError>;
    async fn save(&self, vendor: &Vendor) -> Result<(), ApError>;
}

/// Repository trait for purchase order persistence.
#[async_trait]
pub trait PurchaseOrderRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        po_id: Uuid,
    ) -> Result<Option<crate::models::purchase_order::PurchaseOrder>, ApError>;
    async fn save(
        &self,
        po: &crate::models::purchase_order::PurchaseOrder,
    ) -> Result<(), ApError>;
}
