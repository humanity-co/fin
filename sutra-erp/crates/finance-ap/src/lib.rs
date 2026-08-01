//! SutraERP — Accounts Payable Module
//!
//! Vendor management, procurement (PR→PO→GRN→Invoice),
//! vendor payments with TDS, and employee reimbursements.

pub mod commands;
pub mod errors;
pub mod events;
pub mod models;
pub mod queries;
pub mod repository;

pub use commands::{
    ApCommandHandler, CreatePurchaseOrderCmd, CreateVendorCmd,
    CreateVendorPaymentCmd, IssuePurchaseOrderCmd, MatchInvoiceCmd,
    PostInvoiceCmd, ProcessPaymentCmd, RecordGoodsReceiptCmd,
    RecordVendorInvoiceCmd, UpdateVendorCmd,
};
pub use errors::ApError;
pub use events::ApEventData;
pub use models::vendor::{
    BankAccountType, BankValidationStatus, GstinStatus, MsmeType, PanStatus,
    RegistrationType, Section197Certificate, Vendor, VendorBankAccount, VendorType,
};
pub use models::purchase_order::{
    PoStatus, PurchaseOrder, PurchaseOrderLine, TaxType,
};
pub use models::goods_receipt::{GoodsReceiptNote, GoodsReceiptNoteLine, GrnStatus};
pub use models::vendor_invoice::{
    InvoiceLine, InvoiceStatus, MatchingStatus, PaymentStatus, VendorInvoice,
};
pub use models::vendor_payment::{
    PaymentAllocation, PaymentMode, PaymentType, TdsDeduction, TdsDepositStatus,
    VendorPayment, VpStatus,
};
