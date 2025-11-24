use near_api_types::EpochReference;
use near_openapi_client::types::{
    BlockId, EpochId, ErrorWrapperForRpcValidatorError, JsonRpcRequestForValidators,
    JsonRpcRequestForValidatorsMethod, JsonRpcResponseForRpcValidatorResponseAndRpcValidatorError,
    RpcValidatorError, RpcValidatorRequest, RpcValidatorResponse,
};
use near_openapi_client::Client;

use crate::common::utils::to_retry_error;
use crate::errors::SendRequestError;
use crate::{
    advanced::RpcType, common::utils::is_critical_validator_error, config::RetryResponse,
    NetworkConfig,
};

#[derive(Clone, Debug)]
pub struct SimpleValidatorRpc;

#[async_trait::async_trait]
impl RpcType for SimpleValidatorRpc {
    type RpcReference = EpochReference;
    type Response = RpcValidatorResponse;
    type Error = RpcValidatorError;
    async fn send_query(
        &self,
        client: &Client,
        _network: &NetworkConfig,
        reference: &EpochReference,
    ) -> RetryResponse<RpcValidatorResponse, SendRequestError<RpcValidatorError>> {
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
            .map(|r| r.into_inner())
            .map_err(SendRequestError::from);

        match response {
            Ok(JsonRpcResponseForRpcValidatorResponseAndRpcValidatorError::Variant0 {
                result,
                ..
            }) => RetryResponse::Ok(result),
            Ok(JsonRpcResponseForRpcValidatorResponseAndRpcValidatorError::Variant1 {
                error,
                ..
            }) => {
                let error: SendRequestError<RpcValidatorError> = SendRequestError::from(error);
                to_retry_error(error, is_critical_validator_error)
            }
            Err(err) => to_retry_error(err, is_critical_validator_error),
        }
    }
}

impl From<ErrorWrapperForRpcValidatorError> for SendRequestError<RpcValidatorError> {
    fn from(err: ErrorWrapperForRpcValidatorError) -> Self {
        match err {
            ErrorWrapperForRpcValidatorError::InternalError(internal_error) => {
                Self::InternalError(internal_error)
            }
            ErrorWrapperForRpcValidatorError::RequestValidationError(
                rpc_request_validation_error_kind,
            ) => Self::RequestValidationError(rpc_request_validation_error_kind),
            ErrorWrapperForRpcValidatorError::HandlerError(server_error) => {
                Self::ServerError(server_error)
            }
        }
    }
}
