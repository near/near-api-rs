use near_api_types::sandbox::StateRecord;
use near_openapi_client::ClientInfo;

use crate::{NetworkConfig, advanced::RpcType, config::RetryResponse, errors::SendRequestError};

#[derive(Clone, Debug)]
pub enum SandboxAction {
    PatchState(Vec<StateRecord>),
    FastForward(u64),
}

impl SandboxAction {
    pub fn method_name(&self) -> &str {
        match self {
            Self::PatchState(_) => "sandbox_patch_state",
            Self::FastForward(_) => "sandbox_fast_forward",
        }
    }

    pub fn params(&self) -> Result<serde_json::Value, serde_json::Error> {
        match self {
            Self::PatchState(state) => serde_json::to_value(state),
            Self::FastForward(height) => serde_json::to_value(height),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SimpleSandboxRpc {
    pub action: SandboxAction,
}

#[async_trait::async_trait]
impl RpcType for SimpleSandboxRpc {
    type RpcReference = ();
    type Response = ();
    type Error = ();
    async fn send_query(
        &self,
        client: &near_openapi_client::Client,
        _network: &NetworkConfig,
        _reference: &(),
    ) -> RetryResponse<(), SendRequestError<()>> {
        let Ok(params) = self.action.params() else {
            return RetryResponse::Critical(SendRequestError::ClientError(
                near_openapi_client::Error::InvalidRequest("Serialization error".to_string()),
            ));
        };

        let result = client
            .client()
            .post(format!("{}", client.baseurl()))
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": "0",
                "method": self.action.method_name(),
                "params": params,
            }))
            .send()
            .await;

        match result {
            Ok(_) => RetryResponse::Ok(()),
            Err(_) => RetryResponse::Critical(SendRequestError::ServerError(())),
        }
    }
}
