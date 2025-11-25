use async_trait::async_trait;
use near_openapi_client::types::{
    ErrorWrapperForRpcQueryError, JsonRpcRequestForQuery, JsonRpcRequestForQueryMethod,
    JsonRpcResponseForRpcQueryResponseAndRpcQueryError, RpcQueryError, RpcQueryResponse,
};

use crate::{
    advanced::{query_request::QueryRequest, RpcType},
    common::utils::{is_critical_query_error, to_retry_error},
    config::RetryResponse,
    errors::SendRequestError,
    NetworkConfig,
};
use near_api_types::Reference;

#[derive(Clone, Debug)]
pub struct SimpleQueryRpc {
    pub request: QueryRequest,
}

#[async_trait]
impl RpcType for SimpleQueryRpc {
    type RpcReference = Reference;
    type Response = RpcQueryResponse;
    type Error = RpcQueryError;
    async fn send_query(
        &self,
        client: &near_openapi_client::Client,
        _network: &NetworkConfig,
        reference: &Reference,
    ) -> RetryResponse<RpcQueryResponse, SendRequestError<RpcQueryError>> {
        let request = self.request.clone().to_rpc_query_request(reference.clone());
        let response = client
            .query(&JsonRpcRequestForQuery {
                id: "0".to_string(),
                jsonrpc: "2.0".to_string(),
                method: JsonRpcRequestForQueryMethod::Query,
                params: request,
            })
            .await
            .map(|r| r.into_inner())
            .map_err(SendRequestError::from);

        match response {
            Ok(JsonRpcResponseForRpcQueryResponseAndRpcQueryError::Variant0 { result, .. }) => {
                RetryResponse::Ok(result)
            }
            Ok(JsonRpcResponseForRpcQueryResponseAndRpcQueryError::Variant1 { error, .. }) => {
                let error = SendRequestError::from(error);
                to_retry_error(error, is_critical_query_error)
            }
            Err(err) => to_retry_error(err, is_critical_query_error),
        }
    }
}

impl From<ErrorWrapperForRpcQueryError> for SendRequestError<RpcQueryError> {
    fn from(err: ErrorWrapperForRpcQueryError) -> Self {
        match err {
            ErrorWrapperForRpcQueryError::InternalError(internal_error) => {
                Self::InternalError(internal_error)
            }
            ErrorWrapperForRpcQueryError::RequestValidationError(
                rpc_request_validation_error_kind,
            ) => Self::RequestValidationError(rpc_request_validation_error_kind),
            ErrorWrapperForRpcQueryError::HandlerError(server_error) => {
                Self::ServerError(server_error)
            }
        }
    }
}
