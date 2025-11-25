use near_api_types::Reference;
use near_openapi_client::types::{
    BlockId, ErrorWrapperForRpcBlockError, Finality, JsonRpcRequestForBlock,
    JsonRpcRequestForBlockMethod, JsonRpcResponseForRpcBlockResponseAndRpcBlockError,
    RpcBlockError, RpcBlockRequest, RpcBlockResponse,
};
use near_openapi_client::Client;

use crate::common::utils::to_retry_error;
use crate::{
    advanced::RpcType, common::utils::is_critical_blocks_error, config::RetryResponse,
    errors::SendRequestError, NetworkConfig,
};

#[derive(Clone, Debug)]
pub struct SimpleBlockRpc;

#[async_trait::async_trait]
impl RpcType for SimpleBlockRpc {
    type RpcReference = Reference;
    type Response = RpcBlockResponse;
    type Error = RpcBlockError;
    async fn send_query(
        &self,
        client: &Client,
        _network: &NetworkConfig,
        reference: &Reference,
    ) -> RetryResponse<RpcBlockResponse, SendRequestError<RpcBlockError>> {
        let request = match reference {
            Reference::Optimistic => RpcBlockRequest::Finality(Finality::Optimistic),
            Reference::NearFinal => RpcBlockRequest::Finality(Finality::NearFinal),
            Reference::Final => RpcBlockRequest::Finality(Finality::Final),
            Reference::AtBlock(block) => RpcBlockRequest::BlockId(BlockId::BlockHeight(*block)),
            Reference::AtBlockHash(block_hash) => {
                RpcBlockRequest::BlockId(BlockId::CryptoHash((*block_hash).into()))
            }
        };
        let response = client
            .block(&JsonRpcRequestForBlock {
                id: "0".to_string(),
                jsonrpc: "2.0".to_string(),
                method: JsonRpcRequestForBlockMethod::Block,
                params: request,
            })
            .await
            .map(|r| r.into_inner())
            .map_err(SendRequestError::from);

        match response {
            Ok(JsonRpcResponseForRpcBlockResponseAndRpcBlockError::Variant0 { result, .. }) => {
                RetryResponse::Ok(result)
            }
            Ok(JsonRpcResponseForRpcBlockResponseAndRpcBlockError::Variant1 { error, .. }) => {
                let error = SendRequestError::from(error);
                to_retry_error(error, is_critical_blocks_error)
            }
            Err(err) => to_retry_error(err, is_critical_blocks_error),
        }
    }
}

impl From<ErrorWrapperForRpcBlockError> for SendRequestError<RpcBlockError> {
    fn from(err: ErrorWrapperForRpcBlockError) -> Self {
        match err {
            ErrorWrapperForRpcBlockError::InternalError(internal_error) => {
                Self::InternalError(internal_error)
            }
            ErrorWrapperForRpcBlockError::RequestValidationError(
                rpc_request_validation_error_kind,
            ) => Self::RequestValidationError(rpc_request_validation_error_kind),
            ErrorWrapperForRpcBlockError::HandlerError(server_error) => {
                Self::ServerError(server_error)
            }
        }
    }
}
