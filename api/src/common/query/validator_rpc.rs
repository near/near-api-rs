use near_api_types::EpochReference;
use near_openapi_client::types::{
    BlockId, EpochId, JsonRpcRequestForValidators, JsonRpcRequestForValidatorsMethod,
    JsonRpcResponseForRpcValidatorResponseAndRpcError, RpcError, RpcValidatorRequest,
    RpcValidatorResponse,
};
use near_openapi_client::Client;

use crate::{
    advanced::RpcType, common::utils::is_critical_validator_error, config::RetryResponse,
    errors::SendRequestError, NetworkConfig,
};

#[derive(Clone, Debug)]
pub struct SimpleValidatorRpc;

#[async_trait::async_trait]
impl RpcType for SimpleValidatorRpc {
    type RpcReference = EpochReference;
    type Response = RpcValidatorResponse;
    type Error = RpcError;
    async fn send_query(
        &self,
        client: &Client,
        _network: &NetworkConfig,
        reference: &EpochReference,
    ) -> RetryResponse<RpcValidatorResponse, SendRequestError<RpcError>> {
        let request = match reference {
            EpochReference::Latest => RpcValidatorRequest::Latest,
            EpochReference::AtEpoch(epoch) => {
                RpcValidatorRequest::EpochId(EpochId((*epoch).into()))
            }
            EpochReference::AtBlock(block) => {
                RpcValidatorRequest::BlockId(BlockId::BlockHeight(*block))
            }
            EpochReference::AtBlockHash(block_hash) => {
                RpcValidatorRequest::BlockId(BlockId::CryptoHash((*block_hash).into()))
            }
        };
        let response = client
            .validators(&JsonRpcRequestForValidators {
                id: "0".to_string(),
                jsonrpc: "2.0".to_string(),
                method: JsonRpcRequestForValidatorsMethod::Validators,
                params: request,
            })
            .await
            .map(|r| r.into_inner());
        match response {
            Ok(JsonRpcResponseForRpcValidatorResponseAndRpcError::Variant0 { result, .. }) => {
                RetryResponse::Ok(result)
            }
            Ok(JsonRpcResponseForRpcValidatorResponseAndRpcError::Variant1 { error, .. }) => {
                if is_critical_validator_error(&error) {
                    RetryResponse::Critical(SendRequestError::ServerError(error))
                } else {
                    RetryResponse::Retry(SendRequestError::ServerError(error))
                }
            }
            Err(err) => RetryResponse::Critical(SendRequestError::ClientError(err)),
        }
    }
}
