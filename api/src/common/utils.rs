// New errors can be added to the codebase, so we want to handle them gracefully
#![allow(unreachable_patterns)]

use base64::{Engine, prelude::BASE64_STANDARD};
use near_api_types::NearToken;

use crate::{
    config::RetryResponse,
    errors::SendRequestError,
    rpc_client::{RpcCallError, RpcError},
};

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

pub fn to_retry_error<T>(
    err: SendRequestError,
    is_critical_t: impl Fn(&SendRequestError) -> bool,
) -> RetryResponse<T, SendRequestError> {
    if is_critical_t(&err) {
        RetryResponse::Critical(err)
    } else {
        RetryResponse::Retry(err)
    }
}

pub fn is_critical_blocks_error(err: &SendRequestError) -> bool {
    is_critical_json_rpc_error(err, |rpc_err| {
        match rpc_err.cause_name() {
            Some("UNKNOWN_BLOCK") | Some("NOT_SYNCED_YET") | Some("INTERNAL_ERROR") => false,
            _ => false,
        }
    })
}

pub fn is_critical_validator_error(err: &SendRequestError) -> bool {
    is_critical_json_rpc_error(err, |rpc_err| {
        match rpc_err.cause_name() {
            Some("UNKNOWN_EPOCH") | Some("VALIDATOR_INFO_UNAVAILABLE") | Some("INTERNAL_ERROR") => {
                false
            }
            _ => false,
        }
    })
}

pub fn is_critical_query_error(err: &SendRequestError) -> bool {
    is_critical_json_rpc_error(err, |rpc_err| {
        match rpc_err.cause_name() {
            Some("NO_SYNCED_BLOCKS") | Some("UNAVAILABLE_SHARD") | Some("UNKNOWN_BLOCK")
            | Some("INTERNAL_ERROR") => false,

            Some("GARBAGE_COLLECTED_BLOCK")
            | Some("INVALID_ACCOUNT")
            | Some("UNKNOWN_ACCOUNT")
            | Some("NO_CONTRACT_CODE")
            | Some("TOO_LARGE_CONTRACT_STATE")
            | Some("UNKNOWN_ACCESS_KEY")
            | Some("CONTRACT_EXECUTION_ERROR")
            | Some("UNKNOWN_GAS_KEY") => true,

            // Might be critical, but also might not yet propagated across the network, so we will retry
            Some("NO_GLOBAL_CONTRACT_CODE") => false,
            _ => false,
        }
    })
}

pub fn is_critical_transaction_error(err: &SendRequestError) -> bool {
    is_critical_json_rpc_error(err, |rpc_err| {
        match rpc_err.cause_name() {
            Some("TIMEOUT_ERROR") | Some("REQUEST_ROUTED") => false,
            Some("INVALID_TRANSACTION")
            | Some("DOES_NOT_TRACK_SHARD")
            | Some("UNKNOWN_TRANSACTION")
            | Some("INTERNAL_ERROR") => true,
            _ => false,
        }
    })
}

fn is_critical_json_rpc_error(
    err: &SendRequestError,
    is_critical_handler: impl Fn(&RpcError) -> bool,
) -> bool {
    match err {
        SendRequestError::ServerError(rpc_error) => is_critical_handler(rpc_error),
        SendRequestError::RequestCreationError(_) => true,
        SendRequestError::TransportError(err) => match err {
            RpcCallError::Http(e) => {
                use reqwest::StatusCode;
                // Check HTTP status for retryable errors
                e.status().map_or(false, |s| {
                    !matches!(
                        s,
                        StatusCode::REQUEST_TIMEOUT
                            | StatusCode::TOO_MANY_REQUESTS
                            | StatusCode::INTERNAL_SERVER_ERROR
                            | StatusCode::BAD_GATEWAY
                            | StatusCode::SERVICE_UNAVAILABLE
                            | StatusCode::GATEWAY_TIMEOUT
                    )
                })
            }
            RpcCallError::Deserialize(_) => true,
            RpcCallError::Rpc(_) => {
                unreachable!("Rpc errors are converted to ServerError in From<RpcCallError>")
            }
        },
    }
}
