use near_api_types::Reference;
use near_openapi_client::Client;
use near_openapi_client::types::{
    BlockId, Finality, JsonRpcRequestForBlock, JsonRpcRequestForBlockMethod,
    JsonRpcResponseForRpcBlockResponseAndRpcError, RpcBlockRequest, RpcBlockResponse, RpcError,
};

use crate::{
    NetworkConfig, advanced::RpcType, common::utils::is_critical_blocks_error,
    config::RetryResponse, errors::SendRequestError,
};

#[derive(Clone, Debug)]
pub struct SimpleBlockRpc;

#[async_trait::async_trait]
impl RpcType for SimpleBlockRpc {
    type RpcReference = Reference;
    type Response = RpcBlockResponse;
    type Error = RpcError;
    async fn send_query(
        &self,
        client: &Client,
        _network: &NetworkConfig,
        reference: &Reference,
    ) -> RetryResponse<RpcBlockResponse, SendRequestError<RpcError>> {
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
            .map(|r| r.into_inner());
        match response {
            Ok(JsonRpcResponseForRpcBlockResponseAndRpcError::Variant0 { result, .. }) => {
                RetryResponse::Ok(result)
            }
            Ok(JsonRpcResponseForRpcBlockResponseAndRpcError::Variant1 { error, .. }) => {
                if is_critical_blocks_error(&error) {
                    RetryResponse::Critical(SendRequestError::ServerError(error))
                } else {
                    RetryResponse::Retry(SendRequestError::ServerError(error))
                }
            }
            Err(err) => RetryResponse::Critical(SendRequestError::ClientError(err)),
        }
    }
}
