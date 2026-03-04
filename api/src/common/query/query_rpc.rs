use async_trait::async_trait;

use crate::{
    NetworkConfig,
    advanced::{RpcType, query_request::QueryRequest},
    common::utils::{is_critical_query_error, to_retry_error},
    config::RetryResponse,
    errors::SendRequestError,
    rpc_client::RpcClient,
};
use near_api_types::Reference;

#[derive(Clone, Debug)]
pub struct SimpleQueryRpc {
    pub request: QueryRequest,
}

#[async_trait]
impl RpcType for SimpleQueryRpc {
    type RpcReference = Reference;
    type Response = serde_json::Value;
    async fn send_query(
        &self,
        client: &RpcClient,
        _network: &NetworkConfig,
        reference: &Reference,
    ) -> RetryResponse<serde_json::Value, SendRequestError> {
        let request = self.request.clone().to_rpc_query_request(reference.clone());
        match client.call::<_, serde_json::Value>("query", request).await {
            Ok(value) => {
                if let Some(error_msg) = value.get("error").and_then(|e| e.as_str()) {
                    return to_retry_error(
                        SendRequestError::ContractExecutionError(error_msg.to_string()),
                        is_critical_query_error,
                    );
                }
                RetryResponse::Ok(value)
            }
            Err(err) => to_retry_error(SendRequestError::from(err), is_critical_query_error),
        }
    }
}
