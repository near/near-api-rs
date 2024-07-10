use std::marker::PhantomData;

use near_jsonrpc_client::methods::query::RpcQueryResponse;
use near_primitives::{
    hash::CryptoHash,
    types::{BlockHeight, BlockReference},
    views::{
        AccessKeyList, AccessKeyView, AccountView, ContractCodeView, QueryRequest, ViewStateResult,
    },
};
use serde::de::DeserializeOwned;

use crate::config::NetworkConfig;

pub struct Data<T> {
    pub data: T,
    pub block_height: BlockHeight,
    pub block_hash: CryptoHash,
}

pub trait ResponseHandler {
    type Response;

    // TODO: Add error type
    fn process_response(&self, response: RpcQueryResponse) -> anyhow::Result<Self::Response>;
}

pub struct QueryBuilder<ResponseHandler> {
    block_reference: BlockReference,
    request: QueryRequest,
    handler: ResponseHandler,
}

impl<Handler> QueryBuilder<Handler>
where
    Handler: ResponseHandler,
{
    pub fn new(request: QueryRequest, handler: Handler) -> Self {
        Self {
            block_reference: BlockReference::latest(),
            request,
            handler,
        }
    }

    pub fn as_of(self, block_reference: near_primitives::types::BlockReference) -> Self {
        Self {
            block_reference,
            ..self
        }
    }

    pub async fn fetch_from_mainnet(self) -> anyhow::Result<Handler::Response> {
        let network = NetworkConfig::mainnet();
        self.fetch_from(&network).await
    }

    pub async fn fetch_from_testnet(self) -> anyhow::Result<Handler::Response> {
        let network = NetworkConfig::testnet();
        self.fetch_from(&network).await
    }

    pub async fn fetch_from(self, network: &NetworkConfig) -> anyhow::Result<Handler::Response> {
        let json_rpc_client = network.json_rpc_client();

        let query_response = json_rpc_client
            .call(near_jsonrpc_client::methods::query::RpcQueryRequest {
                block_reference: self.block_reference,
                request: self.request,
            })
            .await?;

        self.handler.process_response(query_response)
    }
}

pub struct CallResultHandler<Response, PostProcessed>
where
    Response: DeserializeOwned,
{
    post_process: Box<dyn Fn(Response) -> PostProcessed + Send + Sync>,
    _phantom: PhantomData<Response>,
}

impl<Response, PostProcessed> CallResultHandler<Response, PostProcessed>
where
    Response: DeserializeOwned,
{
    pub fn with_postprocess<F>(post_process: F) -> Self
    where
        F: Fn(Response) -> PostProcessed + Send + Sync + 'static,
    {
        Self {
            post_process: Box::new(post_process),
            _phantom: PhantomData,
        }
    }
}

impl<Response> Default for CallResultHandler<Response, Response>
where
    Response: DeserializeOwned,
{
    fn default() -> Self {
        Self {
            post_process: Box::new(|response| response),
            _phantom: PhantomData,
        }
    }
}

impl<Response, PostProcessed> ResponseHandler for CallResultHandler<Response, PostProcessed>
where
    Response: DeserializeOwned,
{
    type Response = Data<PostProcessed>;

    fn process_response(&self, response: RpcQueryResponse) -> anyhow::Result<Self::Response> {
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) =
            response.kind
        {
            let raw: Response = serde_json::from_slice(&result.result)?;
            Ok(Data {
                data: (self.post_process)(raw),
                block_height: response.block_height,
                block_hash: response.block_hash,
            })
        } else {
            Err(anyhow::anyhow!(
                "Received unexpected query kind in response to a view-call-result query call"
            ))
        }
    }
}

#[derive(Default)]
pub struct AccountViewHandler;

impl ResponseHandler for AccountViewHandler {
    type Response = Data<AccountView>;

    fn process_response(&self, response: RpcQueryResponse) -> anyhow::Result<Self::Response> {
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewAccount(account) =
            response.kind
        {
            Ok(Data {
                data: account,
                block_height: response.block_height,
                block_hash: response.block_hash,
            })
        } else {
            Err(anyhow::anyhow!(
                "Received unexpected query kind in response to a view-account query call"
            ))
        }
    }
}

#[derive(Default)]
pub struct AccessKeyListHandler;

impl ResponseHandler for AccessKeyListHandler {
    type Response = AccessKeyList;

    fn process_response(&self, response: RpcQueryResponse) -> anyhow::Result<Self::Response> {
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKeyList(account) =
            response.kind
        {
            Ok(account)
        } else {
            Err(anyhow::anyhow!(
                "Received unexpected query kind in response to a view-access-key-list query call"
            ))
        }
    }
}

#[derive(Default)]
pub struct AccessKeyHandler;

impl ResponseHandler for AccessKeyHandler {
    type Response = Data<AccessKeyView>;

    fn process_response(&self, response: RpcQueryResponse) -> anyhow::Result<Self::Response> {
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKey(key) =
            response.kind
        {
            Ok(Data {
                data: key,
                block_height: response.block_height,
                block_hash: response.block_hash,
            })
        } else {
            Err(anyhow::anyhow!(
                "Received unexpected query kind in response to a view-access-key query call"
            ))
        }
    }
}

pub struct ViewStateHandler<PostProcessed> {
    post_process: Box<dyn Fn(ViewStateResult) -> PostProcessed + Send + Sync>,
}

impl<PostProcessed> ViewStateHandler<PostProcessed> {
    pub fn with_postprocess<F>(post_process: F) -> Self
    where
        F: Fn(ViewStateResult) -> PostProcessed + Send + Sync + 'static,
    {
        Self {
            post_process: Box::new(post_process),
        }
    }
}

impl<PostProcessed> ResponseHandler for ViewStateHandler<PostProcessed> {
    type Response = Data<PostProcessed>;

    fn process_response(&self, response: RpcQueryResponse) -> anyhow::Result<Self::Response> {
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewState(view) =
            response.kind
        {
            Ok(Data {
                data: (self.post_process)(view),
                block_height: response.block_height,
                block_hash: response.block_hash,
            })
        } else {
            Err(anyhow::anyhow!(
                "Received unexpected query kind in response to a view-state query call"
            ))
        }
    }
}

#[derive(Default)]
pub struct ViewCodeHandler;

impl ResponseHandler for ViewCodeHandler {
    type Response = Data<ContractCodeView>;

    fn process_response(&self, response: RpcQueryResponse) -> anyhow::Result<Self::Response> {
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewCode(code) =
            response.kind
        {
            Ok(Data {
                data: code,
                block_height: response.block_height,
                block_hash: response.block_hash,
            })
        } else {
            Err(anyhow::anyhow!(
                "Received unexpected query kind in response to a view-code query call"
            ))
        }
    }
}
