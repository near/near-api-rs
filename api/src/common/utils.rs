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

// TODO: this is a temporary solution to check if an error is critical
// we had previously a full scale support for that
// but auto generated code doesn't support errors yet, so we would need to leave it as is for now
// We default to false as we can't know if an error is critical or not without the types
// so to keep it safe it's better to retry

pub fn is_critical_blocks_error(err: &RpcError) -> bool {
    is_critical_json_rpc_error(err, |_| false)
}

pub fn is_critical_validator_error(err: &RpcError) -> bool {
    is_critical_json_rpc_error(err, |_| false)
}

pub fn is_critical_query_error(rpc_error: &RpcError) -> bool {
    is_critical_json_rpc_error(rpc_error, |_| false)
}

pub fn is_critical_transaction_error(err: &RpcError) -> bool {
    is_critical_json_rpc_error(err, |_| false)
}

fn is_critical_json_rpc_error(
    err: &RpcError,
    is_critical_t: impl Fn(&serde_json::Value) -> bool,
) -> bool {
    match err {
        RpcError::Variant0 { .. } => true,
        RpcError::Variant1 { cause, .. } => is_critical_t(cause),
        RpcError::Variant2 { .. } => false,
    }
}
