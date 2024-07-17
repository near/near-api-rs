use std::marker::PhantomData;

use anyhow::{anyhow, bail};
use futures::future::join_all;
use near_jsonrpc_client::methods::{
    query::{RpcQueryRequest, RpcQueryResponse},
    validators::RpcValidatorRequest,
    RpcMethod,
};
use near_primitives::{
    types::{BlockReference, EpochReference},
    views::{
        AccessKeyList, AccessKeyView, AccountView, ContractCodeView, EpochValidatorInfo,
        QueryRequest, ViewStateResult,
    },
};
use serde::de::DeserializeOwned;

use crate::{config::NetworkConfig, types::Data};

pub trait ResponseHandler<QueryResponse> {
    type Response;

    // TODO: Add error type

    /// NOTE: responses should always > 1
    fn process_response(&self, responses: Vec<QueryResponse>) -> anyhow::Result<Self::Response>;
    fn request_amount(&self) -> usize {
        1
    }
}

pub trait QueryCreator<Method: RpcMethod> {
    type RpcReference;
    fn create_query(
        &self,
        network: &NetworkConfig,
        reference: Self::RpcReference,
    ) -> anyhow::Result<Method>;
}

pub struct SimpleQuery {
    pub request: QueryRequest,
}

impl QueryCreator<RpcQueryRequest> for SimpleQuery {
    type RpcReference = BlockReference;
    fn create_query(
        &self,
        _network: &NetworkConfig,
        reference: BlockReference,
    ) -> anyhow::Result<RpcQueryRequest> {
        Ok(RpcQueryRequest {
            block_reference: reference,
            request: self.request.clone(),
        })
    }
}

pub struct SimpleValidatorRpc;

impl QueryCreator<RpcValidatorRequest> for SimpleValidatorRpc {
    type RpcReference = EpochReference;
    fn create_query(
        &self,
        _network: &NetworkConfig,
        reference: EpochReference,
    ) -> anyhow::Result<RpcValidatorRequest> {
        Ok(RpcValidatorRequest {
            epoch_reference: reference,
        })
    }
}

pub type QueryBuilder<T> = RpcBuilder<T, RpcQueryRequest, BlockReference>;
pub type MultiQueryBuilder<T> = MultiRpcBuilder<T, RpcQueryRequest, BlockReference>;

pub type ValidatorQueryBuilder<T> = RpcBuilder<T, RpcValidatorRequest, EpochReference>;

pub struct MultiRpcBuilder<ResponseHandler, Method, Reference> {
    reference: Reference,
    requests: Vec<Box<dyn QueryCreator<Method, RpcReference = Reference>>>,
    handler: ResponseHandler,
}

impl<Handler, Method: RpcMethod, Reference> MultiRpcBuilder<Handler, Method, Reference>
where
    Handler: ResponseHandler<Method::Response>,
    Method: RpcMethod + 'static,
    Method::Error: std::fmt::Display + std::fmt::Debug + Sync + Send,
    Reference: Clone,
{
    pub fn new(handler: Handler, reference: Reference) -> Self {
        Self {
            reference,
            requests: vec![],
            handler,
        }
    }

    pub fn add_query(
        mut self,
        request: Box<dyn QueryCreator<Method, RpcReference = Reference>>,
    ) -> Self {
        self.requests.push(request);
        self
    }

    pub fn add_query_builder<T>(mut self, query_builder: RpcBuilder<T, Method, Reference>) -> Self {
        self.requests.push(query_builder.request);
        self
    }

    pub fn as_of(self, block_reference: Reference) -> Self {
        Self {
            reference: block_reference,
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

        let requests: Vec<_> = self
            .requests
            .into_iter()
            .map(|request| anyhow::Ok(request.create_query(network, self.reference.clone())?))
            .collect::<Result<_, _>>()?;
        let requests = requests
            .into_iter()
            .map(|request| json_rpc_client.call(request));

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

pub struct RpcBuilder<ResponseHandler, Method, Reference> {
    reference: Reference,
    request: Box<dyn QueryCreator<Method, RpcReference = Reference>>,
    handler: ResponseHandler,
}

impl<Handler, Method, Reference> RpcBuilder<Handler, Method, Reference>
where
    Handler: ResponseHandler<Method::Response>,
    Method: RpcMethod + 'static,
    Method::Error: std::fmt::Display + std::fmt::Debug + Sync + Send,
{
    pub fn new(
        request: impl QueryCreator<Method, RpcReference = Reference> + 'static,
        reference: Reference,
        handler: Handler,
    ) -> Self {
        Self {
            reference,
            request: Box::new(request),
            handler,
        }
    }

    pub fn as_of(self, reference: Reference) -> Self {
        Self { reference, ..self }
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
            .call(self.request.create_query(network, self.reference)?)
            .await?;

        self.handler.process_response(vec![query_response])
    }
}

pub struct MultiQueryHandler<Handlers> {
    handlers: Handlers,
}

impl<QR, H1, H2, R1, R2> ResponseHandler<QR> for MultiQueryHandler<(H1, H2)>
where
    H1: ResponseHandler<QR, Response = R1>,
    H2: ResponseHandler<QR, Response = R2>,
{
    type Response = (R1, R2);

    fn process_response(&self, mut responses: Vec<QR>) -> anyhow::Result<Self::Response> {
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

impl<QR, H1, H2, H3, R1, R2, R3> ResponseHandler<QR> for MultiQueryHandler<(H1, H2, H3)>
where
    H1: ResponseHandler<QR, Response = R1>,
    H2: ResponseHandler<QR, Response = R2>,
    H3: ResponseHandler<QR, Response = R3>,
{
    type Response = (R1, R2, R3);

    fn process_response(&self, mut responses: Vec<QR>) -> anyhow::Result<Self::Response> {
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

pub struct PostprocessHandler<PostProcessed, Response, Handler: ResponseHandler<Response>> {
    post_process: Box<dyn Fn(Handler::Response) -> PostProcessed + Send + Sync>,
    handler: Handler,
}

impl<PostProcessed, Response, Handler: ResponseHandler<Response>>
    PostprocessHandler<PostProcessed, Response, Handler>
{
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

impl<PostProcessed, QueryResponse, Handler> ResponseHandler<QueryResponse>
    for PostprocessHandler<PostProcessed, QueryResponse, Handler>
where
    Handler: ResponseHandler<QueryResponse>,
{
    type Response = PostProcessed;

    fn process_response(&self, response: Vec<QueryResponse>) -> anyhow::Result<Self::Response> {
        Handler::process_response(&self.handler, response).map(|data| (self.post_process)(data))
    }

    fn request_amount(&self) -> usize {
        self.handler.request_amount()
    }
}

#[derive(Default)]
pub struct CallResultHandler<Response>(pub PhantomData<Response>);

impl<Response> ResponseHandler<RpcQueryResponse> for CallResultHandler<Response>
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

impl ResponseHandler<RpcQueryResponse> for AccountViewHandler {
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

impl ResponseHandler<RpcQueryResponse> for AccessKeyListHandler {
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

impl ResponseHandler<RpcQueryResponse> for AccessKeyHandler {
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

impl ResponseHandler<RpcQueryResponse> for ViewStateHandler {
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

impl ResponseHandler<RpcQueryResponse> for ViewCodeHandler {
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

pub struct RpcValidatorHandler;

impl ResponseHandler<EpochValidatorInfo> for RpcValidatorHandler {
    type Response = EpochValidatorInfo;

    fn process_response(
        &self,
        response: Vec<EpochValidatorInfo>,
    ) -> anyhow::Result<Self::Response> {
        let response = response
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No response for the view code handler"))?;

        Ok(response)
    }
}
