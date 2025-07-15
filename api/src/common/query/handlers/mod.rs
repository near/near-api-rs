use near_openapi_client::types::RpcQueryResponse;
use near_types::{
    AccessKey, Account, AccountView, ContractCodeView, Data, RpcBlockResponse,
    RpcValidatorResponse, ViewStateResult, json::U64, transaction::actions::AccessKeyInfo,
};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use tracing::{info, trace, warn};

use crate::{
    advanced::{
        RpcType, block_rpc::SimpleBlockRpc, query_rpc::SimpleQueryRpc,
        validator_rpc::SimpleValidatorRpc,
    },
    common::query::{QUERY_EXECUTOR_TARGET, ResultWithMethod},
    errors::QueryError,
};
pub mod transformers;
pub use transformers::*;

const fn query_to_kind(response: &RpcQueryResponse) -> &'static str {
    match response {
        RpcQueryResponse::Variant0 { .. } => "ViewAccount",
        RpcQueryResponse::Variant1 { .. } => "ViewCode",
        RpcQueryResponse::Variant2 { .. } => "ViewState",
        RpcQueryResponse::Variant3 { .. } => "CallResult",
        RpcQueryResponse::Variant4 { .. } => "AccessKey",
        RpcQueryResponse::Variant5 { .. } => "AccessKeyList",
    }
}

pub trait ResponseHandler {
    type Response;
    type Query: RpcType;

    /// NOTE: responses should always >= 1
    fn process_response(
        &self,
        responses: Vec<<Self::Query as RpcType>::Response>,
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error>;
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
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;

        if let RpcQueryResponse::Variant3 {
            result,
            logs: _logs,
            block_height,
            block_hash,
        } = response
        {
            trace!(target: QUERY_EXECUTOR_TARGET, "Deserializing CallResult, result size: {} bytes", result.len());
            let data: Response = serde_json::from_slice(&result)?;
            Ok(Data {
                data,
                block_height,
                block_hash: block_hash
                    .try_into()
                    .map_err(|e| QueryError::ConversionError(Box::new(e)))?,
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response);
            Err(QueryError::UnexpectedResponse {
                expected: "CallResult",
                got: query_to_kind(&response),
            })
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AccountViewHandler;

impl ResponseHandler for AccountViewHandler {
    type Query = SimpleQueryRpc;
    type Response = Data<Account>;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;

        if let RpcQueryResponse::Variant0 {
            amount,
            locked,
            code_hash,
            storage_usage,
            storage_paid_at,
            block_hash,
            block_height,
            global_contract_account_id,
            global_contract_hash,
        } = response
        {
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed ViewAccount response: balance: {}, locked: {}",
                amount, locked
            );
            Ok(Data {
                data: AccountView {
                    amount,
                    locked,
                    code_hash,
                    storage_usage,
                    storage_paid_at,
                    global_contract_account_id,
                    global_contract_hash,
                }
                .try_into()
                .map_err(|e| QueryError::ConversionError(Box::new(e)))?,
                block_height,
                block_hash: block_hash
                    .try_into()
                    .map_err(|e| QueryError::ConversionError(Box::new(e)))?,
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response);
            Err(QueryError::UnexpectedResponse {
                expected: "ViewAccount",
                got: query_to_kind(&response),
            })
        }
    }

    fn request_amount(&self) -> usize {
        1
    }
}

#[derive(Default, Debug, Clone)]
pub struct AccessKeyListHandler;

impl ResponseHandler for AccessKeyListHandler {
    type Response = Data<Vec<AccessKeyInfo>>;
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;
        if let RpcQueryResponse::Variant5 {
            keys,
            block_height,
            block_hash,
        } = response
        {
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed AccessKeyList response, keys count: {}",
                keys.len()
            );
            Ok(Data {
                data: keys
                    .into_iter()
                    .filter_map(|key| key.try_into().ok())
                    .collect(),
                block_height,
                block_hash: block_hash
                    .try_into()
                    .map_err(|e| QueryError::ConversionError(Box::new(e)))?,
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response);
            Err(QueryError::UnexpectedResponse {
                expected: "AccessKeyList",
                got: query_to_kind(&response),
            })
        }
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
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;
        if let RpcQueryResponse::Variant4 {
            block_hash,
            nonce,
            block_height,
            permission,
        } = response
        {
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed AccessKey response, nonce: {}, permission: {:?}",
                nonce,
                permission
            );
            Ok(Data {
                data: AccessKey {
                    nonce: U64(nonce),
                    permission: permission
                        .try_into()
                        .map_err(|e| QueryError::ConversionError(Box::new(e)))?,
                },
                block_height,
                block_hash: block_hash
                    .try_into()
                    .map_err(|e| QueryError::ConversionError(Box::new(e)))?,
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response);
            Err(QueryError::UnexpectedResponse {
                expected: "AccessKey",
                got: query_to_kind(&response),
            })
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ViewStateHandler;

impl ResponseHandler for ViewStateHandler {
    type Response = Data<ViewStateResult>;
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;
        if let RpcQueryResponse::Variant2 {
            proof,
            values,
            block_height,
            block_hash,
        } = response
        {
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed ViewState response, values count: {}, proof nodes: {}",
                values.len(),
                proof.len()
            );
            Ok(Data {
                data: ViewStateResult { proof, values },
                block_height,
                block_hash: block_hash
                    .try_into()
                    .map_err(|e| QueryError::ConversionError(Box::new(e)))?,
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response);
            Err(QueryError::UnexpectedResponse {
                expected: "ViewState",
                got: query_to_kind(&response),
            })
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ViewCodeHandler;

impl ResponseHandler for ViewCodeHandler {
    type Response = Data<ContractCodeView>;
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;
        if let RpcQueryResponse::Variant1 {
            code_base64,
            hash,
            block_height,
            block_hash,
        } = response
        {
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed ViewCode response, code size: {} bytes, hash: {:?}",
                code_base64.len(),
                hash
            );
            Ok(Data {
                data: ContractCodeView { code_base64, hash },
                block_height,
                block_hash: block_hash
                    .try_into()
                    .map_err(|e| QueryError::ConversionError(Box::new(e)))?,
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response);
            Err(QueryError::UnexpectedResponse {
                expected: "ViewCode",
                got: query_to_kind(&response),
            })
        }
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
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;

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
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;

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

impl ResponseHandler for () {
    type Response = ();
    type Query = SimpleQueryRpc;

    fn process_response(
        &self,
        _response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error> {
        trace!(target: QUERY_EXECUTOR_TARGET, "Processed empty response handler");
        Ok(())
    }
}
