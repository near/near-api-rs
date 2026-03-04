use near_openrpc_client::RpcError;
use serde::{Deserialize, Serialize};

pub use near_openrpc_client::errors;

/// Thin JSON-RPC 2.0 client wrapping `reqwest::Client`.
///
/// This replaces the OpenAPI-generated client with a minimal transport
/// that sends raw JSON-RPC requests and deserializes responses using
/// the OpenRPC-generated types.
#[derive(Debug, Clone)]
pub struct RpcClient {
    pub(crate) client: reqwest::Client,
    pub(crate) url: String,
}

/// JSON-RPC request envelope.
#[derive(Debug, Serialize)]
struct RpcRequest<P> {
    jsonrpc: &'static str,
    id: &'static str,
    method: &'static str,
    params: P,
}

/// Raw JSON-RPC response envelope — deserialized as `Value` first to avoid
/// issues with `serde(flatten)` + `serde(untagged)`.
#[derive(Debug, Deserialize)]
struct RpcResponse {
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<RpcError>,
}

/// Errors that can occur when making a JSON-RPC call.
#[derive(Debug, thiserror::Error)]
pub enum RpcCallError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("RPC error: {0}")]
    Rpc(RpcError),
    #[error("JSON deserialization error: {0}")]
    Deserialize(serde_json::Error),
}

impl RpcClient {
    pub fn new(url: String, client: reqwest::Client) -> Self {
        Self { client, url }
    }

    /// Make a JSON-RPC call with the given method and params.
    pub async fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &'static str,
        params: P,
    ) -> Result<R, RpcCallError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: "0",
            method,
            params,
        };

        let bytes = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await?
            .bytes()
            .await?;

        let response: RpcResponse =
            serde_json::from_slice(&bytes).map_err(RpcCallError::Deserialize)?;

        if let Some(error) = response.error {
            return Err(RpcCallError::Rpc(error));
        }

        match response.result {
            Some(value) => serde_json::from_value(value).map_err(RpcCallError::Deserialize),
            None => Err(RpcCallError::Deserialize(serde::de::Error::custom(
                "response has neither result nor error",
            ))),
        }
    }
}
