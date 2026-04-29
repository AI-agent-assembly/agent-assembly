//! gRPC service layer — wires tonic-generated `PolicyService` to `PolicyEngine`.

pub mod convert;
pub mod policy_service;

pub use policy_service::PolicyServiceImpl;
