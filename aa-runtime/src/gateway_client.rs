//! Optional gRPC client for forwarding policy checks to `aa-gateway`.
//!
//! When the runtime is configured with a `gateway_endpoint`, policy queries
//! are forwarded over gRPC to the governance gateway instead of being
//! evaluated locally. This enables the full 7-stage policy pipeline.

use aa_proto::assembly::policy::v1::policy_service_client::PolicyServiceClient;
use aa_proto::assembly::policy::v1::{CheckActionRequest, CheckActionResponse};
use tonic::transport::Channel;

/// gRPC client wrapper for the governance gateway's `PolicyService`.
pub struct GatewayClient {
    client: PolicyServiceClient<Channel>,
}

impl GatewayClient {
    /// Connect to the gateway at the given endpoint.
    ///
    /// `endpoint` should be a URI like `"http://127.0.0.1:50051"` (TCP) or
    /// a UDS path handled by a custom connector.
    pub async fn connect(endpoint: &str) -> Result<Self, tonic::transport::Error> {
        let client = PolicyServiceClient::connect(endpoint.to_string()).await?;
        Ok(Self { client })
    }

    /// Forward a `CheckActionRequest` to the gateway and return the response.
    pub async fn check_action(&mut self, req: CheckActionRequest) -> Result<CheckActionResponse, tonic::Status> {
        let resp = self.client.check_action(req).await?;
        Ok(resp.into_inner())
    }
}
