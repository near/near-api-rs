// TODO: root level doc might be needed here. It's pretty complicated.
use std::{marker::PhantomData, sync::Arc};

use futures::future::join_all;
use near_jsonrpc_client::methods::{
    block::{RpcBlockError, RpcBlockRequest},
    query::{RpcQueryError, RpcQueryRequest, RpcQueryResponse},
    validators::{RpcValidatorError, RpcValidatorRequest},
    RpcMethod,
};
use near_primitives::{
    types::{BlockReference, EpochReference},
    views::{
        AccessKeyList, AccessKeyView, AccountView, BlockView, ContractCodeView, EpochValidatorInfo,
        QueryRequest, ViewStateResult,
    },
};
use serde::de::DeserializeOwned;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::{
    config::{retry, NetworkConfig, RetryResponse},
    errors::QueryError,
    types::Data,
};

use super::utils::{
    is_critical_blocks_error, is_critical_query_error, is_critical_validator_error,
};

const QUERY_EXECUTOR_TARGET: &str = "near_api::query::executor";

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

    fn is_critical_error(
        &self,
        error: &near_jsonrpc_client::errors::JsonRpcError<Method::Error>,
    ) -> bool;
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

    fn is_critical_error(
        &self,
        error: &near_jsonrpc_client::errors::JsonRpcError<RpcQueryError>,
    ) -> bool {
        is_critical_query_error(error)
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

    fn is_critical_error(
        &self,
        error: &near_jsonrpc_client::errors::JsonRpcError<RpcValidatorError>,
    ) -> bool {
        is_critical_validator_error(error)
    }
}

#[derive(Clone, Debug)]
pub struct SimpleBlockRpc;

impl QueryCreator<RpcBlockRequest> for SimpleBlockRpc {
    type RpcReference = BlockReference;
    fn create_query(
        &self,
        _network: &NetworkConfig,
        reference: BlockReference,
    ) -> ResultWithMethod<RpcBlockRequest, RpcBlockRequest> {
        Ok(RpcBlockRequest {
            block_reference: reference,
        })
    }

    fn is_critical_error(
        &self,
        error: &near_jsonrpc_client::errors::JsonRpcError<RpcBlockError>,
    ) -> bool {
        is_critical_blocks_error(error)
    }
}

pub type QueryBuilder<T> = RpcBuilder<T, RpcQueryRequest, BlockReference>;
pub type MultiQueryBuilder<T> = MultiRpcBuilder<T, RpcQueryRequest, BlockReference>;

pub type ValidatorQueryBuilder<T> = RpcBuilder<T, RpcValidatorRequest, EpochReference>;
pub type BlockQueryBuilder<T> = RpcBuilder<T, RpcBlockRequest, BlockReference>;

pub struct MultiRpcBuilder<Handler, Method, Reference>
where
    Reference: Send + Sync,
    Handler: Send + Sync,
{
    reference: Reference,
    requests: Vec<Arc<dyn QueryCreator<Method, RpcReference = Reference> + Send + Sync>>,
    handler: Handler,
}

impl<Handler, Method, Reference> MultiRpcBuilder<Handler, Method, Reference>
where
    Handler: ResponseHandler<QueryResponse = Method::Response, Method = Method> + Send + Sync,
    Method: RpcMethod + std::fmt::Debug + Send + Sync + 'static,
    Method::Response: std::fmt::Debug + Send + Sync,
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

    /// Add a query to the queried items. Sometimes you might need to query multiple items at once.
    /// To combine the result of multiple queries into one.
    pub fn add_query(
        mut self,
        request: Arc<dyn QueryCreator<Method, RpcReference = Reference> + Send + Sync>,
    ) -> Self {
        self.requests.push(request);
        self
    }

    /// It might be easier to use this method to add a query builder to the queried items.
    pub fn add_query_builder<T>(mut self, query_builder: RpcBuilder<T, Method, Reference>) -> Self {
        self.requests.push(query_builder.request);
        self
    }

    /// Set the block reference for the queries.
    pub fn at(self, reference: impl Into<Reference>) -> Self {
        Self {
            reference: reference.into(),
            ..self
        }
    }

    /// Fetch the queries from the provided network.
    #[instrument(skip(self, network), fields(request_count = self.requests.len()))]
    pub async fn fetch_from(
        self,
        network: &NetworkConfig,
    ) -> ResultWithMethod<Handler::Response, Method> {
        debug!(target: QUERY_EXECUTOR_TARGET, "Preparing queries");
        let requests: Vec<_> = self
            .requests
            .into_iter()
            .map(|request| {
                request
                    .create_query(network, self.reference.clone())
                    .map(|query| (query, request))
            })
            .collect::<Result<_, _>>()?;

        info!(target: QUERY_EXECUTOR_TARGET, "Sending {} queries", requests.len());
        let requests = requests.into_iter().map(|(query, request)| async move {
            retry(network.clone(), |json_rpc_client| {
                let query = &query;
                let request = &request;

                async move {
                    let result = match json_rpc_client.call(&query).await {
                        Ok(result) => RetryResponse::Ok(result),
                        Err(err) if request.is_critical_error(&err) => RetryResponse::Critical(err),
                        Err(err) => RetryResponse::Retry(err),
                    };
                    tracing::debug!(
                        target: QUERY_EXECUTOR_TARGET,
                        "Querying RPC with {:?} resulted in {:?}",
                        query,
                        result
                    );
                    result
                }
            })
            .await
        });

        let requests: Vec<_> = join_all(requests)
            .await
            .into_iter()
            .collect::<Result<_, _>>()?;
        if requests.is_empty() {
            error!(target: QUERY_EXECUTOR_TARGET, "No responses received");
            return Err(QueryError::InternalErrorNoResponse);
        }

        debug!(target: QUERY_EXECUTOR_TARGET, "Processing {} responses", requests.len());
        self.handler.process_response(requests)
    }

    /// Fetch the queries from the default mainnet network configuration.
    pub async fn fetch_from_mainnet(self) -> ResultWithMethod<Handler::Response, Method> {
        let network = NetworkConfig::mainnet();
        self.fetch_from(&network).await
    }

    /// Fetch the queries from the default testnet network configuration.
    pub async fn fetch_from_testnet(self) -> ResultWithMethod<Handler::Response, Method> {
        let network = NetworkConfig::testnet();
        self.fetch_from(&network).await
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
    Method: RpcMethod + std::fmt::Debug + Send + Sync + 'static,
    Method::Response: std::fmt::Debug + Send + Sync,
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

    /// Set the block reference for the query.
    pub fn at(self, reference: impl Into<Reference>) -> Self {
        Self {
            reference: reference.into(),
            ..self
        }
    }

    /// Fetch the query from the provided network.
    #[instrument(skip(self, network))]
    pub async fn fetch_from(
        self,
        network: &NetworkConfig,
    ) -> ResultWithMethod<Handler::Response, Method> {
        debug!(target: QUERY_EXECUTOR_TARGET, "Preparing query");
        let query = self.request.create_query(network, self.reference)?;

        let query_response = retry(network.clone(), |json_rpc_client| {
            let query = &query;
            let request = &self.request;
            async move {
                let result = match json_rpc_client.call(&query).await {
                    Ok(result) => RetryResponse::Ok(result),
                    Err(err) if request.is_critical_error(&err) => RetryResponse::Critical(err),
                    Err(err) => RetryResponse::Retry(err),
                };
                tracing::debug!(
                    target: QUERY_EXECUTOR_TARGET,
                    "Querying RPC with {:?} resulted in {:?}",
                    query,
                    result
                );
                result
            }
        })
        .await?;

        debug!(target: QUERY_EXECUTOR_TARGET, "Processing query response");
        self.handler.process_response(vec![query_response])
    }

    /// Fetch the query from the default mainnet network configuration.
    pub async fn fetch_from_mainnet(self) -> ResultWithMethod<Handler::Response, Method> {
        let network = NetworkConfig::mainnet();
        self.fetch_from(&network).await
    }

    /// Fetch the query from the default testnet network configuration.
    pub async fn fetch_from_testnet(self) -> ResultWithMethod<Handler::Response, Method> {
        let network = NetworkConfig::testnet();
        self.fetch_from(&network).await
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
        trace!(target: QUERY_EXECUTOR_TARGET, "Processing response with postprocessing, response count: {}", response.len());
        Handler::process_response(&self.handler, response).map(|data| {
            trace!(target: QUERY_EXECUTOR_TARGET, "Applying postprocessing");
            (self.post_process)(data)
        })
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
            trace!(target: QUERY_EXECUTOR_TARGET, "Deserializing CallResult, result size: {} bytes", result.result.len());
            let data: Response = serde_json::from_slice(&result.result)?;
            Ok(Data {
                data,
                block_height: response.block_height,
                block_hash: response.block_hash.into(),
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
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
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed ViewAccount response: balance: {}, locked: {}",
                 account.amount, account.locked
            );
            Ok(Data {
                data: account,
                block_height: response.block_height,
                block_hash: response.block_hash.into(),
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
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
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKeyList(
            access_key_list,
        ) = response.kind
        {
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed AccessKeyList response, keys count: {}",
                access_key_list.keys.len()
            );
            Ok(access_key_list)
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
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
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed AccessKey response, nonce: {}, permission: {:?}",
                key.nonce,
                key.permission
            );
            Ok(Data {
                data: key,
                block_height: response.block_height,
                block_hash: response.block_hash.into(),
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
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
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed ViewState response, values count: {}, proof nodes: {}",
                data.values.len(),
                data.proof.len()
            );
            Ok(Data {
                data,
                block_height: response.block_height,
                block_hash: response.block_hash.into(),
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
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
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed ViewCode response, code size: {} bytes, hash: {:?}",
                code.code.len(),
                code.hash
            );
            Ok(Data {
                data: code,
                block_height: response.block_height,
                block_hash: response.block_hash.into(),
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
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
    type Response = BlockView;
    type QueryResponse = BlockView;
    type Method = RpcBlockRequest;

    fn process_response(
        &self,
        response: Vec<BlockView>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
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
}

impl ResponseHandler for () {
    type Response = ();
    type QueryResponse = RpcQueryResponse;
    type Method = RpcQueryRequest;

    fn process_response(
        &self,
        _response: Vec<RpcQueryResponse>,
    ) -> ResultWithMethod<Self::Response, Self::Method> {
        trace!(target: QUERY_EXECUTOR_TARGET, "Processed empty response handler");
        Ok(())
    }
}
