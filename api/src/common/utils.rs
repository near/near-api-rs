// https://github.com/near/near-token-rs/blob/3feafec624e7d1028ed00695f2acf87e1d823fa7/src/utils.rs#L1-L49

use base64::{Engine, prelude::BASE64_STANDARD};
use near_openapi_client::types::RpcError;
use near_types::NearToken;

pub fn to_base64(input: &[u8]) -> String {
    BASE64_STANDARD.encode(input)
}

pub fn from_base64(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    BASE64_STANDARD.decode(encoded)
}

/// Converts [crate::Data]<[u128]>] to [crate::NearToken].
pub const fn near_data_to_near_token(data: near_types::Data<u128>) -> NearToken {
    NearToken::from_yoctonear(data.data)
}

pub fn is_critical_blocks_error(err: &RpcError) -> bool {
    // is_critical_json_rpc_error(err, |err| match err {
    //     near_openapi_client::methods::block::RpcBlockError::UnknownBlock { .. }
    //     | near_openapi_client::methods::block::RpcBlockError::NotSyncedYet
    //     | near_openapi_client::methods::block::RpcBlockError::InternalError { .. } => true,
    // })

    // TODO: implement this
    false
}

pub fn is_critical_validator_error(err: &RpcError) -> bool {
    // is_critical_json_rpc_error(err, |err| match err {
    //     near_jsonrpc_primitives::types::validator::RpcValidatorError::UnknownEpoch
    //     | near_jsonrpc_primitives::types::validator::RpcValidatorError::ValidatorInfoUnavailable
    //     | near_jsonrpc_primitives::types::validator::RpcValidatorError::InternalError { .. } => {
    //         true
    //     }
    // })
    // TODO: implement this
    false
}

pub fn is_critical_query_error(rpc_error: &RpcError) -> bool {
    // is_critical_json_rpc_error(err, |err| match err {
    //     near_jsonrpc_primitives::types::query::RpcQueryError::NoSyncedBlocks
    //     | near_jsonrpc_primitives::types::query::RpcQueryError::UnavailableShard { .. }
    //     | near_jsonrpc_primitives::types::query::RpcQueryError::GarbageCollectedBlock { .. }
    //     | near_jsonrpc_primitives::types::query::RpcQueryError::UnknownBlock { .. }
    //     | near_jsonrpc_primitives::types::query::RpcQueryError::InvalidAccount { .. }
    //     | near_jsonrpc_primitives::types::query::RpcQueryError::UnknownAccount { .. }
    //     | near_jsonrpc_primitives::types::query::RpcQueryError::NoContractCode { .. }
    //     | near_jsonrpc_primitives::types::query::RpcQueryError::TooLargeContractState { .. }
    //     | near_jsonrpc_primitives::types::query::RpcQueryError::UnknownAccessKey { .. }
    //     | near_jsonrpc_primitives::types::query::RpcQueryError::ContractExecutionError { .. }
    //     | near_jsonrpc_primitives::types::query::RpcQueryError::InternalError { .. } => true,
    // })
    // TODO: implement this
    false
}

pub fn is_critical_transaction_error(err: &RpcError) -> bool {
    // is_critical_json_rpc_error(err, |err| {
    //     match err {
    //         near_openapi_client::methods::broadcast_tx_commit::RpcTransactionError::TimeoutError => {
    //             false
    //         }
    //         near_openapi_client::methods::broadcast_tx_commit::RpcTransactionError::InvalidTransaction { .. } |
    //             near_openapi_client::methods::broadcast_tx_commit::RpcTransactionError::DoesNotTrackShard |
    //                 near_openapi_client::methods::broadcast_tx_commit::RpcTransactionError::RequestRouted{..} |
    //                     near_openapi_client::methods::broadcast_tx_commit::RpcTransactionError::UnknownTransaction{..} |
    //                         near_openapi_client::methods::broadcast_tx_commit::RpcTransactionError::InternalError{..} => {
    //             true
    //         }
    //     }
    // })
    // TODO: implement this
    false
}

fn is_critical_json_rpc_error<T>(err: &RpcError, is_critical_t: impl Fn(&T) -> bool) -> bool {
    // match err {
    //     near_openapi_client::errors::JsonRpcError::TransportError(_rpc_transport_error) => {
    //         false
    //     }
    //     near_openapi_client::errors::JsonRpcError::ServerError(rpc_server_error) => match rpc_server_error {
    //         near_openapi_client::errors::JsonRpcServerError::HandlerError(rpc_transaction_error) => is_critical_t(rpc_transaction_error),
    //         near_openapi_client::errors::JsonRpcServerError::RequestValidationError(..) |
    //         near_openapi_client::errors::JsonRpcServerError::NonContextualError(..) => {
    //             true
    //         }
    //         near_openapi_client::errors::JsonRpcServerError::InternalError{ .. } => {
    //             false
    //         }
    //         near_openapi_client::errors::JsonRpcServerError::ResponseStatusError(json_rpc_server_response_status_error) => match json_rpc_server_response_status_error {
    //             near_openapi_client::errors::JsonRpcServerResponseStatusError::Unauthorized |
    //             near_openapi_client::errors::JsonRpcServerResponseStatusError::Unexpected{..} |
    //             near_openapi_client::errors::JsonRpcServerResponseStatusError::BadRequest => {
    //                 true
    //             }
    //             near_openapi_client::errors::JsonRpcServerResponseStatusError::TimeoutError |
    //             near_openapi_client::errors::JsonRpcServerResponseStatusError::ServiceUnavailable |
    //             near_openapi_client::errors::JsonRpcServerResponseStatusError::TooManyRequests => {
    //                 false
    //             }
    //         }
    //     }
    // }
    // TODO: implement this
    false
}
