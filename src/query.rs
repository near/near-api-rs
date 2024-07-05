use std::marker::PhantomData;

use near_jsonrpc_client::methods::query::RpcQueryResponse;
use near_primitives::{
    types::{BlockReference, Finality},
    views::{AccessKeyList, AccountView, QueryRequest},
};
use serde::de::DeserializeOwned;

use crate::config::NetworkConfig;

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
            block_reference: Finality::Final.into(),
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
        self.fetch_from(network).await
    }

    pub async fn fetch_from_testnet(self) -> anyhow::Result<Handler::Response> {
        let network = NetworkConfig::testnet();
        self.fetch_from(network).await
    }

    pub async fn fetch_from(self, network: NetworkConfig) -> anyhow::Result<Handler::Response> {
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
    type Response = PostProcessed;

    fn process_response(&self, response: RpcQueryResponse) -> anyhow::Result<Self::Response> {
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) =
            response.kind
        {
            let raw: Response = serde_json::from_slice(&result.result)?;
            Ok((self.post_process)(raw))
        } else {
            Err(anyhow::anyhow!(
                "Received unexpected query kind in response to a view-function query call"
            ))
        }
    }
}

#[derive(Default)]
pub struct AccountViewHandler;

impl ResponseHandler for AccountViewHandler {
    type Response = AccountView;

    fn process_response(&self, response: RpcQueryResponse) -> anyhow::Result<Self::Response> {
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewAccount(account) =
            response.kind
        {
            Ok(account)
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
                "Received unexpected query kind in response to a view-account query call"
            ))
        }
    }
}
