// https://github.com/near/near-token-rs/blob/3feafec624e7d1028ed00695f2acf87e1d823fa7/src/utils.rs#L1-L49

use base64::{prelude::BASE64_STANDARD, Engine};
use near_api_types::NearToken;
use near_openapi_client::types::{
    RpcBlockError, RpcQueryError, RpcTransactionError, RpcValidatorError,
};
use reqwest::StatusCode;

use crate::{config::RetryResponse, errors::SendRequestError};

pub fn to_base64(input: &[u8]) -> String {
    BASE64_STANDARD.encode(input)
}

pub fn from_base64(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    BASE64_STANDARD.decode(encoded)
}

/// Converts [crate::Data]<[u128]>] to [crate::NearToken].
pub const fn near_data_to_near_token(data: near_api_types::Data<u128>) -> NearToken {
    NearToken::from_yoctonear(data.data)
}

// TODO: this is a temporary solution to check if an error is critical
// we had previously a full scale support for that
// but auto generated code doesn't support errors yet, so we would need to leave it as is for now
// We default to false as we can't know if an error is critical or not without the types
// so to keep it safe it's better to retry

pub fn to_retry_error<T, E: std::fmt::Debug + Send + Sync>(
    err: SendRequestError<E>,
    is_critical_t: impl Fn(&SendRequestError<E>) -> bool,
) -> RetryResponse<T, SendRequestError<E>> {
    if is_critical_t(&err) {
        RetryResponse::Critical(err)
    } else {
        RetryResponse::Retry(err)
    }
}

pub fn is_critical_blocks_error(err: &SendRequestError<RpcBlockError>) -> bool {
    is_critical_json_rpc_error(err, |err| match err {
        RpcBlockError::UnknownBlock { .. }
        | RpcBlockError::NotSyncedYet
        | RpcBlockError::InternalError { .. } => false,
    })
}

pub fn is_critical_validator_error(err: &SendRequestError<RpcValidatorError>) -> bool {
    is_critical_json_rpc_error(err, |err| match err {
        RpcValidatorError::UnknownEpoch
        | RpcValidatorError::ValidatorInfoUnavailable
        | RpcValidatorError::InternalError { .. } => false,
    })
}
pub fn is_critical_query_error(err: &SendRequestError<RpcQueryError>) -> bool {
    is_critical_json_rpc_error(err, |err| match err {
        RpcQueryError::NoSyncedBlocks
        | RpcQueryError::UnavailableShard { .. }
        | RpcQueryError::UnknownBlock { .. }
        | RpcQueryError::InternalError { .. } => false,

        RpcQueryError::GarbageCollectedBlock { .. }
        | RpcQueryError::InvalidAccount { .. }
        | RpcQueryError::UnknownAccount { .. }
        | RpcQueryError::NoContractCode { .. }
        | RpcQueryError::TooLargeContractState { .. }
        | RpcQueryError::UnknownAccessKey { .. }
        | RpcQueryError::ContractExecutionError { .. }
        | RpcQueryError::UnknownGasKey { .. } => true,

        // Might be critical, but also might not yet propagated across the network, so we will retry
        RpcQueryError::NoGlobalContractCode { .. } => false,
    })
}

pub fn is_critical_transaction_error(err: &SendRequestError<RpcTransactionError>) -> bool {
    is_critical_json_rpc_error(err, |err| match err {
        RpcTransactionError::TimeoutError | RpcTransactionError::RequestRouted { .. } => false,
        RpcTransactionError::InvalidTransaction { .. }
        | RpcTransactionError::DoesNotTrackShard
        | RpcTransactionError::UnknownTransaction { .. }
        | RpcTransactionError::InternalError { .. } => true,
    })
}

fn is_critical_json_rpc_error<RpcError: std::fmt::Debug + Send + Sync>(
    err: &SendRequestError<RpcError>,
    is_critical_t: impl Fn(&RpcError) -> bool,
) -> bool {
    match err {
        SendRequestError::ServerError(rpc_error) => is_critical_t(rpc_error),
        SendRequestError::WasmExecutionError(_) => true,
        SendRequestError::InternalError { .. } => false,
        SendRequestError::RequestValidationError(_) => true,
        SendRequestError::RequestCreationError(_) => true,
        SendRequestError::TransportError(error) => match error {
            near_openapi_client::Error::InvalidRequest(_)
            | near_openapi_client::Error::CommunicationError(_)
            | near_openapi_client::Error::InvalidUpgrade(_)
            | near_openapi_client::Error::ResponseBodyError(_)
            | near_openapi_client::Error::InvalidResponsePayload(_, _)
            | near_openapi_client::Error::UnexpectedResponse(_)
            | near_openapi_client::Error::Custom(_) => true,

            near_openapi_client::Error::ErrorResponse(response_value) => {
                // It's more readable to use a match statement than a macro
                #[allow(clippy::match_like_matches_macro)]
                match response_value.status() {
                    StatusCode::REQUEST_TIMEOUT
                    | StatusCode::TOO_MANY_REQUESTS
                    | StatusCode::INTERNAL_SERVER_ERROR
                    | StatusCode::BAD_GATEWAY
                    | StatusCode::SERVICE_UNAVAILABLE
                    | StatusCode::GATEWAY_TIMEOUT => false,
                    _ => true,
                }
            }
        },
    }
}
