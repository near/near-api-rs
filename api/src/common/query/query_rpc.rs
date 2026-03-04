use async_trait::async_trait;

use near_openrpc_client::{RpcError, RpcErrorCause};

use crate::{
    NetworkConfig,
    advanced::{RpcType, query_request::QueryRequest},
    common::utils::{is_critical_query_error, to_retry_error},
    config::RetryResponse,
    errors::SendRequestError,
    rpc_client::{RpcCallError, RpcClient},
};
use near_api_types::Reference;

#[derive(Clone, Debug)]
pub struct SimpleQueryRpc {
    pub request: QueryRequest,
}

/// Synthesize a `SendRequestError` for query-embedded execution errors.
///
/// NEAR's query RPC returns HTTP 200 with an `"error"` string field in the
/// result body for WASM execution failures. We convert this into a proper
/// `RpcError` with a `CONTRACT_EXECUTION_ERROR` cause so the retry logic
/// and typed error matching work uniformly.
fn query_execution_error(error_msg: &str) -> SendRequestError {
    SendRequestError::from(RpcCallError::Rpc(RpcError {
        code: -32000,
        message: "Server error".to_string(),
        data: Some(serde_json::Value::String(error_msg.to_string())),
        name: Some("HANDLER_ERROR".to_string()),
        cause: Some(RpcErrorCause {
            name: "CONTRACT_EXECUTION_ERROR".to_string(),
            info: Some(serde_json::json!({ "error_message": error_msg })),
        }),
    }))
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
                        query_execution_error(error_msg),
                        is_critical_query_error,
                    );
                }
                RetryResponse::Ok(value)
            }
            Err(err) => to_retry_error(SendRequestError::from(err), is_critical_query_error),
        }
    }
}
