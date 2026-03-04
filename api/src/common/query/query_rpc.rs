use async_trait::async_trait;

use crate::{
    NetworkConfig,
    advanced::{RpcType, query_request::QueryRequest},
    common::utils::{is_critical_query_error, to_retry_error},
    config::RetryResponse,
    errors::SendRequestError,
    rpc_client::{RpcCallError, RpcClient, RpcError, RpcErrorCause},
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
                // NEAR's query method returns a successful JSON-RPC response even for
                // WASM execution errors. The error is embedded as an "error" string field
                // in the result body (e.g., {"error": "wasm execution failed...", "logs": []}).
                // We need to detect this and convert it to a proper RPC error.
                if let Some(error_msg) = value.get("error").and_then(|e| e.as_str()) {
                    let cause_name = if error_msg.contains("CodeDoesNotExist") {
                        "CONTRACT_EXECUTION_ERROR"
                    } else if error_msg.contains("MethodNotFound")
                        || error_msg.contains("MethodResolveError")
                    {
                        "CONTRACT_EXECUTION_ERROR"
                    } else {
                        "CONTRACT_EXECUTION_ERROR"
                    };

                    let err = SendRequestError::from(RpcCallError::Rpc(RpcError {
                        code: -32000,
                        message: "Server error".to_string(),
                        data: Some(serde_json::Value::String(error_msg.to_string())),
                        name: Some("HANDLER_ERROR".to_string()),
                        cause: Some(RpcErrorCause {
                            name: cause_name.to_string(),
                            info: Some(serde_json::json!({
                                "error_message": error_msg,
                            })),
                        }),
                    }));
                    return to_retry_error(err, is_critical_query_error);
                }
                RetryResponse::Ok(value)
            }
            Err(err) => {
                let err = SendRequestError::from(err);
                to_retry_error(err, is_critical_query_error)
            }
        }
    }
}
