//! gRPC service layer — wires tonic-generated services to business logic.

pub mod convert;
pub mod lifecycle_service;
pub mod policy_service;

pub use lifecycle_service::AgentLifecycleServiceImpl;
pub use policy_service::PolicyServiceImpl;
