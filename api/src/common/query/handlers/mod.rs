use borsh::BorshDeserialize;
use near_api_types::{
    AccessKey, Account, Data, PublicKey, RpcBlockResponse, RpcReceiptResponse,
    RpcTransactionResponse, RpcValidatorResponse, json::U64,
    transaction::result::ExecutionFinalResult,
};
use near_openrpc_client::{
    RpcCallFunctionResponse, RpcViewAccessKeyListResponse,
    RpcViewAccessKeyResponse, RpcViewAccountResponse, RpcViewCodeResponse, RpcViewStateResponse,
};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use tracing::{info, trace};

use crate::{
    advanced::{
        RpcType, block_rpc::SimpleBlockRpc, query_rpc::SimpleQueryRpc,
        tx_rpc::TransactionStatusRpc, validator_rpc::SimpleValidatorRpc,
    },
    common::{
        query::{QUERY_EXECUTOR_TARGET, ResultWithMethod},
        send::to_final_execution_outcome,
    },
    errors::QueryError,
};
pub mod transformers;
pub use transformers::*;

fn take_single<T>(responses: Vec<T>) -> Result<T, QueryError> {
    responses
        .into_iter()
        .next()
        .ok_or(QueryError::InternalErrorNoResponse)
}

fn convert_block_hash(
    hash: near_openrpc_client::CryptoHash,
) -> Result<near_api_types::CryptoHash, QueryError> {
    hash.try_into()
        .map_err(|e| QueryError::ConversionError(Box::new(e)))
}

pub trait ResponseHandler {
    type Response;
    type Query: RpcType;

    /// NOTE: responses should always >= 1
    fn process_response(
        &self,
        responses: Vec<<Self::Query as RpcType>::Response>,
    ) -> ResultWithMethod<Self::Response>;
    fn request_amount(&self) -> usize {
        1
    }
}

#[derive(Default, Debug, Clone)]
pub struct CallResultHandler<Response: Send + Sync>(PhantomData<Response>);

impl<Response: Send + Sync> CallResultHandler<Response> {
    pub const fn new() -> Self {
        Self(PhantomData::<Response>)
    }
}

impl<Response> ResponseHandler for CallResultHandler<Response>
where
    Response: DeserializeOwned + Send + Sync,
{
    type Response = Data<Response>;
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        response: Vec<serde_json::Value>,
    ) -> ResultWithMethod<Self::Response> {
        let call_result: RpcCallFunctionResponse = serde_json::from_value(take_single(response)?)?;

        trace!(target: QUERY_EXECUTOR_TARGET, "Deserializing CallResult, result size: {} bytes", call_result.result.len());
        let data: Response = serde_json::from_slice(&call_result.result)?;
        Ok(Data {
            data,
            block_height: call_result.block_height,
            block_hash: convert_block_hash(call_result.block_hash)?,
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct CallResultRawHandler;

impl CallResultRawHandler {
    pub const fn new() -> Self {
        Self
    }
}

impl ResponseHandler for CallResultRawHandler {
    type Response = Data<Vec<u8>>;
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        response: Vec<serde_json::Value>,
    ) -> ResultWithMethod<Self::Response> {
        let call_result: RpcCallFunctionResponse = serde_json::from_value(take_single(response)?)?;

        trace!(target: QUERY_EXECUTOR_TARGET, "Returning CallResult raw bytes, result size: {} bytes", call_result.result.len());
        Ok(Data {
            data: call_result.result,
            block_height: call_result.block_height,
            block_hash: convert_block_hash(call_result.block_hash)?,
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct CallResultBorshHandler<Response: Send + Sync>(PhantomData<Response>);

impl<Response: Send + Sync> CallResultBorshHandler<Response> {
    pub const fn new() -> Self {
        Self(PhantomData::<Response>)
    }
}

impl<Response> ResponseHandler for CallResultBorshHandler<Response>
where
    Response: BorshDeserialize + Send + Sync,
{
    type Response = Data<Response>;
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        response: Vec<serde_json::Value>,
    ) -> ResultWithMethod<Self::Response> {
        let call_result: RpcCallFunctionResponse = serde_json::from_value(take_single(response)?)?;

        trace!(target: QUERY_EXECUTOR_TARGET, "Deserializing CallResult using Borsh, result size: {} bytes", call_result.result.len());
        let data: Response = Response::try_from_slice(&call_result.result)
            .map_err(|e| QueryError::ConversionError(Box::new(e)))?;
        Ok(Data {
            data,
            block_height: call_result.block_height,
            block_hash: convert_block_hash(call_result.block_hash)?,
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct AccountViewHandler;

impl ResponseHandler for AccountViewHandler {
    type Query = SimpleQueryRpc;
    type Response = Data<Account>;

    fn process_response(
        &self,
        response: Vec<serde_json::Value>,
    ) -> ResultWithMethod<Self::Response> {
        let account_view: RpcViewAccountResponse = serde_json::from_value(take_single(response)?)?;

        info!(
            target: QUERY_EXECUTOR_TARGET,
            "Processed ViewAccount response: balance: {}, locked: {}",
            account_view.amount, account_view.locked
        );

        let block_height = account_view.block_height;
        let block_hash = convert_block_hash(account_view.block_hash.clone())?;

        Ok(Data {
            data: Account::try_from(account_view)
                .map_err(|e| QueryError::ConversionError(Box::new(e)))?,
            block_height,
            block_hash,
        })
    }

    fn request_amount(&self) -> usize {
        1
    }
}

#[derive(Default, Debug, Clone)]
pub struct AccessKeyListHandler;

impl ResponseHandler for AccessKeyListHandler {
    type Response = Data<Vec<(PublicKey, AccessKey)>>;
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        response: Vec<serde_json::Value>,
    ) -> ResultWithMethod<Self::Response> {
        let key_list: RpcViewAccessKeyListResponse = serde_json::from_value(take_single(response)?)?;

        info!(
            target: QUERY_EXECUTOR_TARGET,
            "Processed AccessKeyList response, keys count: {}",
            key_list.keys.len()
        );
        Ok(Data {
            data: key_list
                .keys
                .into_iter()
                .filter_map(|key| {
                    let public_key = key.public_key.try_into().ok()?;
                    let access_key = key.access_key.try_into().ok()?;
                    Some((public_key, access_key))
                })
                .collect(),
            block_height: key_list.block_height,
            block_hash: convert_block_hash(key_list.block_hash)?,
        })
    }

    fn request_amount(&self) -> usize {
        1
    }
}

#[derive(Default, Debug, Clone)]
pub struct AccessKeyHandler;

impl ResponseHandler for AccessKeyHandler {
    type Response = Data<AccessKey>;
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        response: Vec<serde_json::Value>,
    ) -> ResultWithMethod<Self::Response> {
        let ak: RpcViewAccessKeyResponse = serde_json::from_value(take_single(response)?)?;

        info!(
            target: QUERY_EXECUTOR_TARGET,
            "Processed AccessKey response, nonce: {}, permission: {:?}",
            ak.nonce,
            ak.permission
        );
        Ok(Data {
            data: AccessKey {
                nonce: U64(ak.nonce),
                permission: ak
                    .permission
                    .try_into()
                    .map_err(|e| QueryError::ConversionError(Box::new(e)))?,
            },
            block_height: ak.block_height,
            block_hash: convert_block_hash(ak.block_hash)?,
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct ViewStateHandler;

impl ResponseHandler for ViewStateHandler {
    type Response = Data<RpcViewStateResponse>;
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        response: Vec<serde_json::Value>,
    ) -> ResultWithMethod<Self::Response> {
        let state: RpcViewStateResponse = serde_json::from_value(take_single(response)?)?;

        info!(
            target: QUERY_EXECUTOR_TARGET,
            "Processed ViewState response, values count: {}, proof nodes: {}",
            state.values.len(),
            state.proof.len()
        );
        Ok(Data {
            block_height: state.block_height,
            block_hash: convert_block_hash(state.block_hash.clone())?,
            data: state,
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct ViewCodeHandler;

impl ResponseHandler for ViewCodeHandler {
    type Response = Data<RpcViewCodeResponse>;
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        response: Vec<serde_json::Value>,
    ) -> ResultWithMethod<Self::Response> {
        let code: RpcViewCodeResponse = serde_json::from_value(take_single(response)?)?;

        info!(
            target: QUERY_EXECUTOR_TARGET,
            "Processed ViewCode response, code size: {} bytes, hash: {:?}",
            code.code_base64.len(),
            code.hash
        );
        Ok(Data {
            block_height: code.block_height,
            block_hash: convert_block_hash(code.block_hash.clone())?,
            data: code,
        })
    }
}

#[derive(Clone, Debug)]
pub struct RpcValidatorHandler;

impl ResponseHandler for RpcValidatorHandler {
    type Response = RpcValidatorResponse;
    type Query = SimpleValidatorRpc;

    fn process_response(
        &self,
        response: Vec<RpcValidatorResponse>,
    ) -> ResultWithMethod<Self::Response> {
        let response = take_single(response)?;
        info!(
            target: QUERY_EXECUTOR_TARGET,
            "Processed EpochValidatorInfo response, epoch height: {}, validators count: {}",
            response.epoch_height,
            response.current_validators.len()
        );
        Ok(response)
    }
}

#[derive(Clone, Debug)]
pub struct RpcBlockHandler;

impl ResponseHandler for RpcBlockHandler {
    type Response = RpcBlockResponse;
    type Query = SimpleBlockRpc;

    fn process_response(
        &self,
        response: Vec<RpcBlockResponse>,
    ) -> ResultWithMethod<Self::Response> {
        let response = take_single(response)?;
        info!(
            target: QUERY_EXECUTOR_TARGET,
            "Processed Block response, height: {}, hash: {:?}",
            response.header.height,
            response.header.hash
        );
        Ok(response)
    }

    fn request_amount(&self) -> usize {
        1
    }
}

/// Handler that converts an [`RpcTransactionResponse`] into an [`ExecutionFinalResult`].
///
/// This reuses the same conversion logic from transaction sending: it extracts the
/// `FinalExecutionOutcomeView` from the response and converts it using `TryFrom`.
#[derive(Clone, Debug)]
pub struct TransactionStatusHandler;

impl ResponseHandler for TransactionStatusHandler {
    type Response = ExecutionFinalResult;
    type Query = TransactionStatusRpc;

    fn process_response(
        &self,
        response: Vec<RpcTransactionResponse>,
    ) -> ResultWithMethod<Self::Response> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;

        let final_execution_outcome_view = to_final_execution_outcome(response);

        info!(
            target: QUERY_EXECUTOR_TARGET,
            "Processed TransactionStatus response, tx hash: {:?}",
            final_execution_outcome_view.transaction_outcome.id,
        );

        ExecutionFinalResult::try_from(final_execution_outcome_view)
            .map_err(|e| QueryError::ConversionError(Box::new(e)))
    }
}

/// Handler that passes through the raw [`RpcReceiptResponse`] without transformation.
#[derive(Clone, Debug)]
pub struct ReceiptHandler;

impl ResponseHandler for ReceiptHandler {
    type Response = RpcReceiptResponse;
    type Query = crate::advanced::tx_rpc::ReceiptRpc;

    fn process_response(
        &self,
        response: Vec<RpcReceiptResponse>,
    ) -> ResultWithMethod<Self::Response> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;

        info!(
            target: QUERY_EXECUTOR_TARGET,
            "Processed Receipt response, receipt_id: {:?}, receiver: {:?}",
            response.receipt_id,
            response.receiver_id,
        );

        Ok(response)
    }
}

impl<T: RpcType> ResponseHandler for T {
    type Response = <T as RpcType>::Response;
    type Query = T;

    fn process_response(
        &self,
        response: Vec<<Self::Query as RpcType>::Response>,
    ) -> ResultWithMethod<Self::Response> {
        let response = take_single(response)?;
        trace!(target: QUERY_EXECUTOR_TARGET, "Processed empty response handler");
        Ok(response)
    }
}
