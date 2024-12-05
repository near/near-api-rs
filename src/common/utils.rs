// https://github.com/near/near-token-rs/blob/3feafec624e7d1028ed00695f2acf87e1d823fa7/src/utils.rs#L1-L49

use crate::errors::{DecimalNumberParsingError, RetryError};

/// Parsing decimal numbers from `&str` type in `u128`.
/// Function also takes a value of metric prefix in u128 type.
/// `parse_str` use the `u128` type, and have the same max and min values.
///
/// If the fractional part is longer than several zeros in the prefix, it will return the error `DecimalNumberParsingError::LongFractional`.
///
/// If the string slice has invalid chars, it will return the error `DecimalNumberParsingError::InvalidNumber`.
///
/// If the whole part of the number has a value more than the `u64` maximum value, it will return the error `DecimalNumberParsingError::LongWhole`.
pub fn parse_decimal_number(s: &str, pref_const: u128) -> Result<u128, DecimalNumberParsingError> {
    let (int, fract) = if let Some((whole, fractional)) = s.trim().split_once('.') {
        let int: u128 = whole
            .parse()
            .map_err(|_| DecimalNumberParsingError::InvalidNumber(s.to_owned()))?;
        let mut fract: u128 = fractional
            .parse()
            .map_err(|_| DecimalNumberParsingError::InvalidNumber(s.to_owned()))?;
        let len = u32::try_from(fractional.len())
            .map_err(|_| DecimalNumberParsingError::InvalidNumber(s.to_owned()))?;
        fract = fract
            .checked_mul(
                pref_const
                    .checked_div(10u128.checked_pow(len).ok_or_else(|| {
                        DecimalNumberParsingError::LongFractional(fractional.to_owned())
                    })?)
                    .filter(|n| *n != 0u128)
                    .ok_or_else(|| {
                        DecimalNumberParsingError::LongFractional(fractional.to_owned())
                    })?,
            )
            .ok_or_else(|| DecimalNumberParsingError::LongFractional(fractional.to_owned()))?;
        (int, fract)
    } else {
        let int: u128 = s
            .parse()
            .map_err(|_| DecimalNumberParsingError::InvalidNumber(s.to_owned()))?;
        (int, 0)
    };
    let result = fract
        .checked_add(
            int.checked_mul(pref_const)
                .ok_or_else(|| DecimalNumberParsingError::LongWhole(int.to_string()))?,
        )
        .ok_or_else(|| DecimalNumberParsingError::LongWhole(int.to_string()))?;
    Ok(result)
}

#[derive(Debug)]
pub enum RetryResponse<R, E> {
    Ok(R),
    Retry(E),
    Critical(E),
}

impl<R, E> From<Result<R, E>> for RetryResponse<R, E> {
    fn from(value: Result<R, E>) -> Self {
        match value {
            Ok(value) => Self::Ok(value),
            Err(value) => Self::Retry(value),
        }
    }
}

pub async fn retry<R, E, T, F>(
    mut task: F,
    retries: u8,
    initial_sleep: std::time::Duration,
    exponential_backoff: bool,
) -> Result<R, RetryError<E>>
where
    F: FnMut() -> T + Send,
    T: core::future::Future<Output = RetryResponse<R, E>> + Send,
    T::Output: Send,
{
    let mut retries = (1..=retries).rev();
    let mut sleep_duration = initial_sleep;
    loop {
        let result = task().await;
        match result {
            RetryResponse::Ok(result) => return Ok(result),
            RetryResponse::Retry(_) if retries.next().is_some() => {
                tokio::time::sleep(sleep_duration).await;
                sleep_duration = if exponential_backoff {
                    sleep_duration * 2
                } else {
                    sleep_duration
                };
            }
            RetryResponse::Retry(err) => {
                return Err(RetryError::RetriesExhausted(err));
            }
            RetryResponse::Critical(err) => return Err(RetryError::Critical(err)),
        }
    }
}

pub fn is_critical_blocks_error(
    err: &near_jsonrpc_client::errors::JsonRpcError<
        near_jsonrpc_primitives::types::blocks::RpcBlockError,
    >,
) -> bool {
    is_critical_json_rpc_error(err, |err| match err {
        near_jsonrpc_client::methods::block::RpcBlockError::UnknownBlock { .. }
        | near_jsonrpc_client::methods::block::RpcBlockError::NotSyncedYet
        | near_jsonrpc_client::methods::block::RpcBlockError::InternalError { .. } => true,
    })
}

pub fn is_critical_validator_error(
    err: &near_jsonrpc_client::errors::JsonRpcError<
        near_jsonrpc_primitives::types::validator::RpcValidatorError,
    >,
) -> bool {
    is_critical_json_rpc_error(err, |err| match err {
        near_jsonrpc_primitives::types::validator::RpcValidatorError::UnknownEpoch
        | near_jsonrpc_primitives::types::validator::RpcValidatorError::ValidatorInfoUnavailable
        | near_jsonrpc_primitives::types::validator::RpcValidatorError::InternalError { .. } => {
            true
        }
    })
}

pub fn is_critical_query_error(
    err: &near_jsonrpc_client::errors::JsonRpcError<
        near_jsonrpc_primitives::types::query::RpcQueryError,
    >,
) -> bool {
    is_critical_json_rpc_error(err, |err| match err {
        near_jsonrpc_primitives::types::query::RpcQueryError::NoSyncedBlocks
        | near_jsonrpc_primitives::types::query::RpcQueryError::UnavailableShard { .. }
        | near_jsonrpc_primitives::types::query::RpcQueryError::GarbageCollectedBlock { .. }
        | near_jsonrpc_primitives::types::query::RpcQueryError::UnknownBlock { .. }
        | near_jsonrpc_primitives::types::query::RpcQueryError::InvalidAccount { .. }
        | near_jsonrpc_primitives::types::query::RpcQueryError::UnknownAccount { .. }
        | near_jsonrpc_primitives::types::query::RpcQueryError::NoContractCode { .. }
        | near_jsonrpc_primitives::types::query::RpcQueryError::TooLargeContractState { .. }
        | near_jsonrpc_primitives::types::query::RpcQueryError::UnknownAccessKey { .. }
        | near_jsonrpc_primitives::types::query::RpcQueryError::ContractExecutionError { .. }
        | near_jsonrpc_primitives::types::query::RpcQueryError::InternalError { .. } => true,
    })
}

pub fn is_critical_transaction_error(
    err: &near_jsonrpc_client::errors::JsonRpcError<
        near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError,
    >,
) -> bool {
    is_critical_json_rpc_error(err, |err| {
        match err {
            near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::TimeoutError => {
                false
            }
            near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::InvalidTransaction { .. } |
                near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::DoesNotTrackShard |
                    near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::RequestRouted{..} |
                        near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::UnknownTransaction{..} |
                            near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::InternalError{..} => {
                true
            }
        }
    })
}

fn is_critical_json_rpc_error<T>(
    err: &near_jsonrpc_client::errors::JsonRpcError<T>,
    is_critical_t: impl Fn(&T) -> bool,
) -> bool {
    match err {
        near_jsonrpc_client::errors::JsonRpcError::TransportError(_rpc_transport_error) => {
            false
        }
        near_jsonrpc_client::errors::JsonRpcError::ServerError(rpc_server_error) => match rpc_server_error {
            near_jsonrpc_client::errors::JsonRpcServerError::HandlerError(rpc_transaction_error) => is_critical_t(rpc_transaction_error),
            near_jsonrpc_client::errors::JsonRpcServerError::RequestValidationError(..) |
            near_jsonrpc_client::errors::JsonRpcServerError::NonContextualError(..) => {
                true
            }
            near_jsonrpc_client::errors::JsonRpcServerError::InternalError{ .. } => {
                false
            }
            near_jsonrpc_client::errors::JsonRpcServerError::ResponseStatusError(json_rpc_server_response_status_error) => match json_rpc_server_response_status_error {
                near_jsonrpc_client::errors::JsonRpcServerResponseStatusError::Unauthorized |
                near_jsonrpc_client::errors::JsonRpcServerResponseStatusError::Unexpected{..} |
                near_jsonrpc_client::errors::JsonRpcServerResponseStatusError::BadRequest => {
                    true
                }
                near_jsonrpc_client::errors::JsonRpcServerResponseStatusError::TimeoutError |
                near_jsonrpc_client::errors::JsonRpcServerResponseStatusError::ServiceUnavailable |
                near_jsonrpc_client::errors::JsonRpcServerResponseStatusError::TooManyRequests => {
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST: [(u128, &str, u128); 6] = [
        (129_380_000_001_u128, "129.380000001", 10u128.pow(9)),
        (
            12_938_000_000_100_000_000_u128,
            "12938000000.1",
            10u128.pow(9),
        ),
        (129_380_000_001_u128, "0.129380000001", 10u128.pow(12)),
        (129_380_000_001_000_u128, "129.380000001000", 10u128.pow(12)),
        (
            9_488_129_380_000_001_u128,
            "9488.129380000001",
            10u128.pow(12),
        ),
        (129_380_000_001_u128, "00.129380000001", 10u128.pow(12)),
    ];

    #[test]
    fn parse_test() {
        for (expected_value, str_value, precision) in TEST {
            let parsed_value = parse_decimal_number(str_value, precision).unwrap();
            assert_eq!(parsed_value, expected_value)
        }
    }

    #[test]
    fn test_long_fract() {
        let data = "1.23456";
        let prefix = 10000u128;
        assert_eq!(
            parse_decimal_number(data, prefix),
            Err(DecimalNumberParsingError::LongFractional(23456.to_string()))
        );
    }

    #[test]
    fn invalidnumber_whole() {
        let num = "1h4.7859";
        let prefix: u128 = 10000;
        assert_eq!(
            parse_decimal_number(num, prefix),
            Err(DecimalNumberParsingError::InvalidNumber(
                "1h4.7859".to_owned()
            ))
        );
    }
    #[test]
    fn invalidnumber_fract() {
        let num = "14.785h9";
        let prefix: u128 = 10000;
        assert_eq!(
            parse_decimal_number(num, prefix),
            Err(DecimalNumberParsingError::InvalidNumber(
                "14.785h9".to_owned()
            ))
        );
    }

    #[test]
    fn max_long_fract() {
        let max_data = 10u128.pow(17) + 1;
        let data = "1.".to_string() + max_data.to_string().as_str();
        let prefix = 10u128.pow(17);
        assert_eq!(
            parse_decimal_number(data.as_str(), prefix),
            Err(DecimalNumberParsingError::LongFractional(
                max_data.to_string()
            ))
        );
    }

    #[test]
    fn parse_u128_errortest() {
        let test_data = u128::MAX.to_string();
        let gas = parse_decimal_number(&test_data, 10u128.pow(9));
        assert_eq!(
            gas,
            Err(DecimalNumberParsingError::LongWhole(u128::MAX.to_string()))
        );
    }

    #[test]
    fn test() {
        let data = "1.000000000000000000000000000000000000001";
        let prefix = 100u128;
        assert_eq!(
            parse_decimal_number(data, prefix),
            Err(DecimalNumberParsingError::LongFractional(
                "000000000000000000000000000000000000001".to_string()
            ))
        );
    }
}
