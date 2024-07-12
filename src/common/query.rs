use std::marker::PhantomData;

use anyhow::{anyhow, bail};
use futures::future::join_all;
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

    /// NOTE: responses should always > 1
    fn process_response(&self, responses: Vec<RpcQueryResponse>) -> anyhow::Result<Self::Response>;
    fn request_amount(&self) -> usize {
        1
    }
}

pub struct MultiQueryBuilder<ResponseHandler> {
    block_reference: BlockReference,
    requests: Vec<QueryRequest>,
    handler: ResponseHandler,
}

impl<Handler> MultiQueryBuilder<Handler>
where
    Handler: ResponseHandler,
{
    pub fn new(handler: Handler) -> Self {
        Self {
            block_reference: BlockReference::latest(),
            requests: vec![],
            handler,
        }
    }

    pub fn add_query(mut self, request: QueryRequest) -> Self {
        self.requests.push(request);
        self
    }

    pub fn add_query_builder<T>(mut self, query_builder: QueryBuilder<T>) -> Self {
        self.requests.push(query_builder.request);
        self
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

        let requests = self.requests.into_iter().map(|request| {
            json_rpc_client.call(near_jsonrpc_client::methods::query::RpcQueryRequest {
                block_reference: self.block_reference.clone(),
                request,
            })
        });

        let requests: Vec<_> = join_all(requests)
            .await
            .into_iter()
            .collect::<Result<_, _>>()?;
        if requests.is_empty() {
            bail!("Zero length response that should be possible")
        }

        self.handler.process_response(requests)
    }
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

        self.handler.process_response(vec![query_response])
    }
}

pub struct MultiQueryHandler<Handlers> {
    handlers: Handlers,
}

impl<H1, H2, R1, R2> ResponseHandler for MultiQueryHandler<(H1, H2)>
where
    H1: ResponseHandler<Response = R1>,
    H2: ResponseHandler<Response = R2>,
{
    type Response = (R1, R2);

    fn process_response(
        &self,
        mut responses: Vec<RpcQueryResponse>,
    ) -> anyhow::Result<Self::Response> {
        let (h1, h2) = &self.handlers;

        let first_response =
            h1.process_response(responses.drain(0..h1.request_amount()).collect())?;
        let second_response = h2.process_response(responses)?;

        Ok((first_response, second_response))
    }

    fn request_amount(&self) -> usize {
        self.handlers.0.request_amount() + self.handlers.1.request_amount()
    }
}

impl<H1, H2, H3, R1, R2, R3> ResponseHandler for MultiQueryHandler<(H1, H2, H3)>
where
    H1: ResponseHandler<Response = R1>,
    H2: ResponseHandler<Response = R2>,
    H3: ResponseHandler<Response = R3>,
{
    type Response = (R1, R2, R3);

    fn process_response(
        &self,
        mut responses: Vec<RpcQueryResponse>,
    ) -> anyhow::Result<Self::Response> {
        let (h1, h2, h3) = &self.handlers;

        let first_response =
            h1.process_response(responses.drain(0..h1.request_amount()).collect())?;
        let second_response = h2.process_response(
            responses
                .drain(h1.request_amount()..h2.request_amount())
                .collect(),
        )?;
        let third_response = h3.process_response(responses)?;

        Ok((first_response, second_response, third_response))
    }

    fn request_amount(&self) -> usize {
        self.handlers.0.request_amount() + self.handlers.1.request_amount()
    }
}

impl<Handlers> MultiQueryHandler<Handlers> {
    pub fn new(handlers: Handlers) -> Self {
        Self { handlers }
    }
}

pub struct PostprocessHandler<PostProcessed, Handler: ResponseHandler> {
    post_process: Box<dyn Fn(Handler::Response) -> PostProcessed + Send + Sync>,
    handler: Handler,
}

impl<PostProcessed, Handler: ResponseHandler> PostprocessHandler<PostProcessed, Handler> {
    pub fn new<F>(handler: Handler, post_process: F) -> Self
    where
        F: Fn(Handler::Response) -> PostProcessed + Send + Sync + 'static,
    {
        Self {
            post_process: Box::new(post_process),
            handler,
        }
    }
}

impl<PostProcessed, Handler> ResponseHandler for PostprocessHandler<PostProcessed, Handler>
where
    Handler: ResponseHandler,
{
    type Response = PostProcessed;

    fn process_response(&self, response: Vec<RpcQueryResponse>) -> anyhow::Result<Self::Response> {
        Handler::process_response(&self.handler, response).map(|data| (self.post_process)(data))
    }

    fn request_amount(&self) -> usize {
        self.handler.request_amount()
    }
}

#[derive(Default)]
pub struct CallResultHandler<Response>(pub PhantomData<Response>);

impl<Response> ResponseHandler for CallResultHandler<Response>
where
    Response: DeserializeOwned,
{
    type Response = Data<Response>;

    fn process_response(&self, response: Vec<RpcQueryResponse>) -> anyhow::Result<Self::Response> {
        let response = response
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No response for the call result handler"))?;

        if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) =
            response.kind
        {
            let data: Response = serde_json::from_slice(&result.result)?;
            Ok(Data {
                data,
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

    fn process_response(&self, response: Vec<RpcQueryResponse>) -> anyhow::Result<Self::Response> {
        let response = response
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No response for the account view handler"))?;

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

    fn process_response(&self, response: Vec<RpcQueryResponse>) -> anyhow::Result<Self::Response> {
        let response = response
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No response for the access key list handler"))?;
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

    fn process_response(&self, response: Vec<RpcQueryResponse>) -> anyhow::Result<Self::Response> {
        let response = response
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No response for the access key handler"))?;
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

#[derive(Default)]
pub struct ViewStateHandler;

impl ResponseHandler for ViewStateHandler {
    type Response = Data<ViewStateResult>;

    fn process_response(&self, response: Vec<RpcQueryResponse>) -> anyhow::Result<Self::Response> {
        let response = response
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No response for the view state handler"))?;
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewState(data) =
            response.kind
        {
            Ok(Data {
                data,
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

    fn process_response(&self, response: Vec<RpcQueryResponse>) -> anyhow::Result<Self::Response> {
        let response = response
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No response for the view code handler"))?;
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
