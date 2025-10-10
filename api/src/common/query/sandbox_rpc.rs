use near_api_types::sandbox::StateRecord;
use near_openapi_client::ClientInfo;

use crate::{NetworkConfig, advanced::RpcType, config::RetryResponse, errors::SendRequestError};

#[derive(Clone, Debug)]
pub enum SandboxAction {
    PatchState(Vec<StateRecord>),
    FastForward(u64),
}

impl SandboxAction {
    pub const fn method_name(&self) -> &str {
        match self {
            Self::PatchState(_) => "sandbox_patch_state",
            Self::FastForward(_) => "sandbox_fast_forward",
        }
    }

    pub fn params(&self) -> serde_json::Value {
        match self {
            Self::PatchState(state) => serde_json::json!({
                "records": state,
            }),
            Self::FastForward(height) => serde_json::json!({
                "delta_height": height,
            }),
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
    type Error = String;
    async fn send_query(
        &self,
        client: &near_openapi_client::Client,
        _network: &NetworkConfig,
        _reference: &(),
    ) -> RetryResponse<(), SendRequestError<String>> {
        let result = client
            .client()
            .post(client.baseurl())
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": "0",
                "method": self.action.method_name(),
                "params": self.action.params(),
            }))
            .send()
            .await;

        match result {
            Ok(response) => {
                let Ok(body) = response.json::<serde_json::Value>().await else {
                    return RetryResponse::Critical(SendRequestError::ServerError(
                        "Invalid response".to_string(),
                    ));
                };

                if body["error"].is_object() {
                    return RetryResponse::Critical(SendRequestError::ServerError(
                        body["error"].to_string(),
                    ));
                }

                RetryResponse::Ok(())
            }
            Err(error) => RetryResponse::Critical(SendRequestError::ServerError(error.to_string())),
        }
    }
}
