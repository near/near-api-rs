use async_trait::async_trait;
use near_openapi_client::types::{
    JsonRpcRequestForQuery, JsonRpcRequestForQueryMethod,
    JsonRpcResponseForRpcQueryResponseAndRpcError, RpcError, RpcQueryResponse,
};

use crate::{
    NetworkConfig,
    advanced::{RpcType, query_request::QueryRequest},
    common::utils::is_critical_query_error,
    config::RetryResponse,
    errors::SendRequestError,
};
use near_types::Reference;

#[derive(Clone, Debug)]
pub struct SimpleQueryRpc {
    pub request: QueryRequest,
}

#[async_trait]
impl RpcType for SimpleQueryRpc {
    type RpcReference = Reference;
    type Response = RpcQueryResponse;
    type Error = RpcError;
    async fn send_query(
        &self,
        client: &near_openapi_client::Client,
        _network: &NetworkConfig,
        reference: &Reference,
    ) -> RetryResponse<RpcQueryResponse, SendRequestError<RpcError>> {
        let request = self.request.clone().to_rpc_query_request(reference.clone());
        let response = client
            .query(&JsonRpcRequestForQuery {
                id: "0".to_string(),
                jsonrpc: "2.0".to_string(),
                method: JsonRpcRequestForQueryMethod::Query,
                params: request,
            })
            .await
            .map(|r| r.into_inner());
        match response {
            Ok(JsonRpcResponseForRpcQueryResponseAndRpcError::Variant0 { result, .. }) => {
                RetryResponse::Ok(result)
            }
            Ok(JsonRpcResponseForRpcQueryResponseAndRpcError::Variant1 { error, .. }) => {
                if is_critical_query_error(&error) {
                    RetryResponse::Critical(SendRequestError::ServerError(error))
                } else {
                    RetryResponse::Retry(SendRequestError::ServerError(error))
                }
            }
            Err(err) => RetryResponse::Critical(SendRequestError::ClientError(err)),
        }
    }
}
