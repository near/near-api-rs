use base64::{Engine, prelude::BASE64_STANDARD};
use near_api_types::NearToken;
use near_openrpc_client::{RpcError, RpcTransactionError};

use crate::{
    config::RetryResponse,
    errors::SendRequestError,
    rpc_client::RpcCallError,
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

/// Generic RPC error criticality check: an error is critical unless `is_retryable()` says otherwise.
/// Used for blocks, validators, and other RPC methods where all known causes are retryable.
pub fn is_critical_rpc_error(err: &SendRequestError) -> bool {
    is_critical_json_rpc_error(err, |rpc_err| !rpc_err.is_retryable())
}

/// Query errors: retryable causes (NO_SYNCED_BLOCKS, UNAVAILABLE_SHARD, UNKNOWN_BLOCK, INTERNAL_ERROR)
/// are not critical, but permanent errors (INVALID_ACCOUNT, UNKNOWN_ACCOUNT, etc.) are.
/// NO_GLOBAL_CONTRACT_CODE is treated as retryable since it may not have propagated yet.
/// ContractExecutionError is always critical (handled by `is_critical_json_rpc_error`).
pub fn is_critical_query_error(err: &SendRequestError) -> bool {
    is_critical_json_rpc_error(err, |rpc_err| !rpc_err.is_retryable())
}

/// Transaction errors: TIMEOUT_ERROR and REQUEST_ROUTED are retryable.
/// INVALID_TRANSACTION, DOES_NOT_TRACK_SHARD, UNKNOWN_TRANSACTION are critical.
/// INTERNAL_ERROR is treated as critical for transactions (different from queries).
pub fn is_critical_transaction_error(err: &SendRequestError) -> bool {
    is_critical_json_rpc_error(err, |rpc_err| {
        match rpc_err.try_cause_as::<RpcTransactionError>() {
            Some(Ok(RpcTransactionError::TimeoutError | RpcTransactionError::RequestRouted { .. })) => false,
            _ => true,
        }
    })
}

/// Transaction status errors: TIMEOUT_ERROR, REQUEST_ROUTED, UNKNOWN_TRANSACTION,
/// DOES_NOT_TRACK_SHARD, and INTERNAL_ERROR are retryable.
/// Only INVALID_TRANSACTION is critical.
pub fn is_critical_transaction_status_error(err: &SendRequestError) -> bool {
    is_critical_json_rpc_error(err, |rpc_err| {
        match rpc_err.try_cause_as::<RpcTransactionError>() {
            Some(Ok(
                RpcTransactionError::TimeoutError
                | RpcTransactionError::RequestRouted { .. }
                | RpcTransactionError::UnknownTransaction { .. }
                | RpcTransactionError::DoesNotTrackShard { .. }
                | RpcTransactionError::InternalError { .. },
            )) => false,
            _ => true,
        }
    })
}

/// Receipt errors: INTERNAL_ERROR is retryable, everything else is critical.
pub fn is_critical_receipt_error(err: &SendRequestError) -> bool {
    is_critical_json_rpc_error(err, |rpc_err| {
        // UNKNOWN_RECEIPT is critical, INTERNAL_ERROR is retryable
        match rpc_err.cause_name() {
            Some("INTERNAL_ERROR") => false,
            _ => !rpc_err.is_retryable(),
        }
    })
}

/// Light client proof errors: UNKNOWN_BLOCK, INTERNAL_ERROR, UNAVAILABLE_SHARD are retryable.
/// INCONSISTENT_STATE, NOT_CONFIRMED, UNKNOWN_TRANSACTION_OR_RECEIPT are critical.
pub fn is_critical_light_client_proof_error(err: &SendRequestError) -> bool {
    is_critical_json_rpc_error(err, |rpc_err| {
        match rpc_err.cause_name() {
            Some("UNKNOWN_BLOCK" | "INTERNAL_ERROR" | "UNAVAILABLE_SHARD") => false,
            _ => true,
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
        SendRequestError::ContractExecutionError(_) => true,
        SendRequestError::TransportError(err) => match err {
            RpcCallError::Http(e) => {
                use reqwest::StatusCode;
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
