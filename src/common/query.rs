use std::{marker::PhantomData, sync::Arc};

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

use crate::{config::NetworkConfig, errors::QueryError, types::Data};

type ResultWithMethod<T, Method> = core::result::Result<T, QueryError<Method>>;

pub trait ResponseHandler
where
    <Self::Method as RpcMethod>::Error: std::fmt::Display + std::fmt::Debug,
{
    type QueryResponse;
    type Response;
    type Method: RpcMethod;

    // TODO: Add error type

    /// NOTE: responses should always >= 1
    fn process_response(
        &self,
        responses: Vec<Self::QueryResponse>,
    ) -> ResultWithMethod<Self::Response, Self::Method>;
    fn request_amount(&self) -> usize {
        1
    }
}

pub trait QueryCreator<Method: RpcMethod>
where
    Method::Error: std::fmt::Display + std::fmt::Debug + Sync + Send,
{
    type RpcReference;
    fn create_query(
        &self,
        network: &NetworkConfig,
        reference: Self::RpcReference,
    ) -> ResultWithMethod<Method, Method>;
}

#[derive(Clone, Debug)]
pub struct SimpleQuery {
    pub request: QueryRequest,
}

impl QueryCreator<RpcQueryRequest> for SimpleQuery {
    type RpcReference = BlockReference;
    fn create_query(
        &self,
        _network: &NetworkConfig,
        reference: BlockReference,
    ) -> ResultWithMethod<RpcQueryRequest, RpcQueryRequest> {
        Ok(RpcQueryRequest {
            block_reference: reference,
            request: self.request.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SimpleValidatorRpc;

impl QueryCreator<RpcValidatorRequest> for SimpleValidatorRpc {
    type RpcReference = EpochReference;
    fn create_query(
        &self,
        _network: &NetworkConfig,
        reference: EpochReference,
    ) -> ResultWithMethod<RpcValidatorRequest, RpcValidatorRequest> {
        Ok(RpcValidatorRequest {
            epoch_reference: reference,
        })
    }
}

pub type QueryBuilder<T> = RpcBuilder<T, RpcQueryRequest, BlockReference>;
pub type MultiQueryBuilder<T> = MultiRpcBuilder<T, RpcQueryRequest, BlockReference>;

pub type ValidatorQueryBuilder<T> = RpcBuilder<T, RpcValidatorRequest, EpochReference>;

pub struct MultiRpcBuilder<ResponseHandler, Method, Reference>
where
    Reference: Send + Sync,
    ResponseHandler: Send + Sync,
{
    reference: Reference,
    requests: Vec<Arc<dyn QueryCreator<Method, RpcReference = Reference> + Send + Sync>>,
    handler: ResponseHandler,
}

impl<Handler, Method, Reference> MultiRpcBuilder<Handler, Method, Reference>
where
    Handler: ResponseHandler<QueryResponse = Method::Response, Method = Method> + Send + Sync,
    Method: RpcMethod + Send + Sync + 'static,
    Method::Response: Send + Sync,
    Method::Error: std::fmt::Display + std::fmt::Debug + Sync + Send,
    Reference: Clone + Send + Sync,
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
        request: Arc<dyn QueryCreator<Method, RpcReference = Reference> + Send + Sync>,
    ) -> Self {
        self.requests.push(request);
        self
    }

    pub fn add_query_builder<T>(mut self, query_builder: RpcBuilder<T, Method, Reference>) -> Self {
        self.requests.push(query_builder.request);
        self
    }

    pub fn at(self, block_reference: Reference) -> Self {
        Self {
            reference: block_reference,
            ..self
        }
    }

    pub async fn fetch_from_mainnet(self) -> ResultWithMethod<Handler::Response, Method> {
        let network = NetworkConfig::mainnet();
        self.fetch_from(&network).await
    }

    pub async fn fetch_from_testnet(self) -> ResultWithMethod<Handler::Response, Method> {
        let network = NetworkConfig::testnet();
        self.fetch_from(&network).await
    }

    pub async fn fetch_from(
        self,
        network: &NetworkConfig,
    ) -> ResultWithMethod<Handler::Response, Method> {
        let json_rpc_client = network.json_rpc_client();

        let requests: Vec<_> = self
            .requests
            .into_iter()
            .map(|request| request.create_query(network, self.reference.clone()))
            .collect::<Result<_, _>>()?;
        let requests = requests
            .into_iter()
            .map(|request| json_rpc_client.call(request));

        let requests: Vec<_> = join_all(requests)
            .await
            .into_iter()
            .collect::<Result<_, _>>()?;
        if requests.is_empty() {
            return Err(QueryError::InternalErrorNoResponse);
        }

        self.handler.process_response(requests)
    }
}

pub struct RpcBuilder<Handler, Method, Reference> {
    reference: Reference,
    request: Arc<dyn QueryCreator<Method, RpcReference = Reference> + Send + Sync>,
    handler: Handler,
}

impl<Handler, Method, Reference> RpcBuilder<Handler, Method, Reference>
where
    Handler: ResponseHandler<QueryResponse = Method::Response, Method = Method> + Send + Sync,
    Method: RpcMethod + Send + Sync + 'static,
    Method::Response: Send + Sync,
    Method: RpcMethod + 'static,
    Method::Error: std::fmt::Display + std::fmt::Debug + Sync + Send,
    Reference: Send + Sync,
{
    pub fn new(
        request: impl QueryCreator<Method, RpcReference = Reference> + 'static + Send + Sync,
        reference: Reference,
        handler: Handler,
    ) -> Self {
        Self {
            reference,
            request: Arc::new(request),
            handler,
        }
    }

    pub fn at(self, reference: Reference) -> Self {
        Self { reference, ..self }
    }

    pub async fn fetch_from_mainnet(self) -> ResultWithMethod<Handler::Response, Method> {
        let network = NetworkConfig::mainnet();
        self.fetch_from(&network).await
    }

    pub async fn fetch_from_testnet(self) -> ResultWithMethod<Handler::Response, Method> {
        let network = NetworkConfig::testnet();
        self.fetch_from(&network).await
    }

    pub async fn fetch_from(
        self,
        network: &NetworkConfig,
    ) -> ResultWithMethod<Handler::Response, Method> {
        let json_rpc_client = network.json_rpc_client();

        let query_response = json_rpc_client
            .call(self.request.create_query(network, self.reference)?)
            .await?;

        self.handler.process_response(vec![query_response])
    }
}

#[derive(Clone, Debug)]
pub struct MultiQueryHandler<Handlers> {
    handlers: Handlers,
}

impl<QR, Method, H1, H2, R1, R2> ResponseHandler for MultiQueryHandler<(H1, H2)>
where
    Method: RpcMethod,
    H1: ResponseHandler<QueryResponse = QR, Response = R1, Method = Method>,
    H2: ResponseHandler<QueryResponse = QR, Response = R2, Method = Method>,
    Method::Error: std::fmt::Display + std::fmt::Debug,
{
    type Response = (R1, R2);
    type QueryResponse = QR;
    type Method = Method;

    fn process_response(&self, mut responses: Vec<QR>) -> ResultWithMethod<Self::Response, Method> {
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

impl<QR, Method, H1, H2, H3, R1, R2, R3> ResponseHandler for MultiQueryHandler<(H1, H2, H3)>
where
    Method: RpcMethod,
    Method::Error: std::fmt::Display + std::fmt::Debug,
    H1: ResponseHandler<QueryResponse = QR, Response = R1, Method = Method>,
    H2: ResponseHandler<QueryResponse = QR, Response = R2, Method = Method>,
    H3: ResponseHandler<QueryResponse = QR, Response = R3, Method = Method>,
{
    type Response = (R1, R2, R3);
    type QueryResponse = QR;
    type Method = Method;

    fn process_response(&self, mut responses: Vec<QR>) -> ResultWithMethod<Self::Response, Method> {
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
    pub const fn new(handlers: Handlers) -> Self {
        Self { handlers }
    }
}
pub struct PostprocessHandler<PostProcessed, Handler: ResponseHandler>
where
    <Handler::Method as RpcMethod>::Error: std::fmt::Display + std::fmt::Debug,
{
    post_process: Box<dyn Fn(Handler::Response) -> PostProcessed + Send + Sync>,
    handler: Handler,
}

impl<PostProcessed, Handler: ResponseHandler> PostprocessHandler<PostProcessed, Handler>
where
    <Handler::Method as RpcMethod>::Error: std::fmt::Display + std::fmt::Debug,
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

impl<PostProcessed, Handler> ResponseHandler for PostprocessHandler<PostProcessed, Handler>
where
    Handler: ResponseHandler,
    <Handler::Method as RpcMethod>::Error: std::fmt::Display + std::fmt::Debug,
{
    type Response = PostProcessed;
    type QueryResponse = Handler::QueryResponse;
    type Method = Handler::Method;

    fn process_response(
        &self,
        response: Vec<Self::QueryResponse>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
        Handler::process_response(&self.handler, response).map(|data| (self.post_process)(data))
    }

    fn request_amount(&self) -> usize {
        self.handler.request_amount()
    }
}

#[derive(Default, Debug, Clone)]
pub struct CallResultHandler<Response: Send + Sync>(pub PhantomData<Response>);

impl<Response> ResponseHandler for CallResultHandler<Response>
where
    Response: DeserializeOwned + Send + Sync,
{
    type Response = Data<Response>;
    type QueryResponse = RpcQueryResponse;
    type Method = RpcQueryRequest;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;

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
            Err(QueryError::UnexpectedResponse {
                expected: "CallResult",
                got: response.kind,
            })
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AccountViewHandler;

impl ResponseHandler for AccountViewHandler {
    type QueryResponse = RpcQueryResponse;
    type Response = Data<AccountView>;
    type Method = RpcQueryRequest;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;

        if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewAccount(account) =
            response.kind
        {
            Ok(Data {
                data: account,
                block_height: response.block_height,
                block_hash: response.block_hash,
            })
        } else {
            Err(QueryError::UnexpectedResponse {
                expected: "ViewAccount",
                got: response.kind,
            })
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AccessKeyListHandler;

impl ResponseHandler for AccessKeyListHandler {
    type Response = AccessKeyList;
    type QueryResponse = RpcQueryResponse;
    type Method = RpcQueryRequest;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKeyList(account) =
            response.kind
        {
            Ok(account)
        } else {
            Err(QueryError::UnexpectedResponse {
                expected: "AccessKeyList",
                got: response.kind,
            })
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AccessKeyHandler;

impl ResponseHandler for AccessKeyHandler {
    type Response = Data<AccessKeyView>;
    type QueryResponse = RpcQueryResponse;
    type Method = RpcQueryRequest;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKey(key) =
            response.kind
        {
            Ok(Data {
                data: key,
                block_height: response.block_height,
                block_hash: response.block_hash,
            })
        } else {
            Err(QueryError::UnexpectedResponse {
                expected: "AccessKey",
                got: response.kind,
            })
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ViewStateHandler;

impl ResponseHandler for ViewStateHandler {
    type Response = Data<ViewStateResult>;
    type QueryResponse = RpcQueryResponse;
    type Method = RpcQueryRequest;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewState(data) =
            response.kind
        {
            Ok(Data {
                data,
                block_height: response.block_height,
                block_hash: response.block_hash,
            })
        } else {
            Err(QueryError::UnexpectedResponse {
                expected: "ViewState",
                got: response.kind,
            })
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ViewCodeHandler;

impl ResponseHandler for ViewCodeHandler {
    type Response = Data<ContractCodeView>;
    type QueryResponse = RpcQueryResponse;
    type Method = RpcQueryRequest;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewCode(code) =
            response.kind
        {
            Ok(Data {
                data: code,
                block_height: response.block_height,
                block_hash: response.block_hash,
            })
        } else {
            Err(QueryError::UnexpectedResponse {
                expected: "ViewCode",
                got: response.kind,
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct RpcValidatorHandler;

impl ResponseHandler for RpcValidatorHandler {
    type Response = EpochValidatorInfo;
    type QueryResponse = EpochValidatorInfo;
    type Method = RpcValidatorRequest;

    fn process_response(
        &self,
        response: Vec<EpochValidatorInfo>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
        let response = response
            .into_iter()
            .next()
            .ok_or(QueryError::InternalErrorNoResponse)?;

        Ok(response)
    }
}

impl ResponseHandler for () {
    type Response = ();
    type QueryResponse = RpcQueryResponse;
    type Method = RpcQueryRequest;

    fn process_response(
        &self,
        _response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
        Ok(())
    }
}
