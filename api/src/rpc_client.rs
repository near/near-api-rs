use serde::{Deserialize, Serialize};

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

/// Error from a JSON-RPC call.
///
/// NEAR's RPC extends standard JSON-RPC errors with `name` and `cause` fields
/// that carry structured, typed error information.
#[derive(Debug, Clone, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    /// Deprecated by nearcore. Prefer `cause` for structured error data.
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    /// Error category: `HANDLER_ERROR`, `REQUEST_VALIDATION_ERROR`, or `INTERNAL_ERROR`.
    #[serde(default)]
    pub name: Option<String>,
    /// Structured error detail with per-method error variant name and info.
    #[serde(default)]
    pub cause: Option<RpcErrorCause>,
}

/// Structured cause of an RPC error.
#[derive(Debug, Clone, Deserialize)]
pub struct RpcErrorCause {
    /// The error variant name (e.g., `UNKNOWN_BLOCK`, `INVALID_ACCOUNT`).
    pub name: String,
    /// Additional structured information about the error.
    #[serde(default)]
    pub info: Option<serde_json::Value>,
}

impl RpcError {
    pub fn is_handler_error(&self) -> bool {
        self.name.as_deref() == Some("HANDLER_ERROR")
    }

    pub fn is_request_validation_error(&self) -> bool {
        self.name.as_deref() == Some("REQUEST_VALIDATION_ERROR")
    }

    pub fn is_internal_error(&self) -> bool {
        self.name.as_deref() == Some("INTERNAL_ERROR")
    }

    pub fn cause_name(&self) -> Option<&str> {
        self.cause.as_ref().map(|c| c.name.as_str())
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RPC error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

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
