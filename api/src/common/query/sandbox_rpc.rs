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
    type Error = ();
    async fn send_query(
        &self,
        client: &near_openapi_client::Client,
        _network: &NetworkConfig,
        _reference: &(),
    ) -> RetryResponse<(), SendRequestError<()>> {
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
                let body = response.text().await.unwrap();
                println!("{}", body);
                RetryResponse::Ok(())
            }
            Err(error) => {
                println!("{:?}", error);
                RetryResponse::Critical(SendRequestError::ServerError(()))
            }
        }
    }
}
