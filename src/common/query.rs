// TODO: root level doc might be needed here. It's pretty complicated.
use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use futures::future::join_all;
use near_openapi_client::Client;
use near_openapi_types::{
    AccessKey, AccessKeyList, AccountView, BlockId, ContractCodeView, CurrentEpochValidatorInfo,
    EpochId, Error, Finality, JsonRpcRequestForBlock, JsonRpcRequestForBlockMethod,
    JsonRpcRequestForQuery, JsonRpcRequestForQueryMethod, JsonRpcRequestForValidators,
    JsonRpcRequestForValidatorsMethod, JsonRpcResponseForRpcBlockResponseAndRpcError,
    JsonRpcResponseForRpcQueryResponseAndRpcError,
    JsonRpcResponseForRpcValidatorResponseAndRpcError, RpcBlockRequest, RpcBlockResponse, RpcError,
    RpcQueryRequest, RpcQueryResponse, RpcValidatorRequest, RpcValidatorResponse, ViewStateResult,
};
use serde::de::DeserializeOwned;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::{
    EpochReference, Reference,
    common::utils::overwrite_reference,
    config::{NetworkConfig, RetryResponse, retry},
    errors::QueryError,
    types::Data,
};

use super::utils::{
    is_critical_blocks_error, is_critical_query_error, is_critical_validator_error,
};

const QUERY_EXECUTOR_TARGET: &str = "near_api::query::executor";

type ResultWithMethod<T, Method> = core::result::Result<T, QueryError<Method>>;

pub trait ResponseHandler {
    type QueryResponse;
    type Response;
    type Method;

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

#[async_trait]
pub trait QueryCreator<Method> {
    type RpcReference;
    type Response;
    async fn send_query(
        &self,
        client: &Client,
        reference: Self::RpcReference,
    ) -> Result<Self::Response, Error<()>>;
    fn is_critical_error(&self, error: &RpcError) -> bool;
}

#[derive(Clone, Debug)]
pub struct SimpleQuery {
    pub request: RpcQueryRequest,
}

#[async_trait]
impl QueryCreator<RpcQueryRequest> for SimpleQuery {
    type RpcReference = Reference;
    type Response = JsonRpcResponseForRpcQueryResponseAndRpcError;
    async fn send_query(
        &self,
        client: &Client,
        reference: Reference,
    ) -> Result<JsonRpcResponseForRpcQueryResponseAndRpcError, Error<()>> {
        client
            .query(&JsonRpcRequestForQuery {
                id: 0,
                jsonrpc: "2.0".to_string(),
                method: JsonRpcRequestForQueryMethod::Query,
                params: overwrite_reference(&self.request, reference),
            })
            .await
    }

    fn is_critical_error(&self, error: &RpcError) -> bool {
        // TODO: implement this
        // is_critical_query_error(error)
        false
    }
}

#[derive(Clone, Debug)]
pub struct SimpleValidatorRpc;

#[async_trait]
impl QueryCreator<RpcValidatorRequest> for SimpleValidatorRpc {
    type RpcReference = EpochReference;
    type Response = JsonRpcResponseForRpcValidatorResponseAndRpcError;
    async fn send_query(
        &self,
        client: &Client,
        reference: EpochReference,
    ) -> Result<JsonRpcResponseForRpcValidatorResponseAndRpcError, Error<()>> {
        let request = match reference {
            EpochReference::Latest => RpcValidatorRequest::Latest,
            EpochReference::AtEpoch(epoch) => RpcValidatorRequest::EpochId(EpochId(epoch.into())),
            EpochReference::AtBlock(block) => {
                RpcValidatorRequest::BlockId(BlockId::BlockHeight(block.into()))
            }
            EpochReference::AtBlockHash(block_hash) => {
                RpcValidatorRequest::BlockId(BlockId::CryptoHash(block_hash.into()))
            }
        };
        client
            .query(&JsonRpcRequestForValidators {
                id: 0,
                jsonrpc: "2.0".to_string(),
                method: JsonRpcRequestForValidatorsMethod::Validators,
                params: request,
            })
            .await
    }

    fn is_critical_error(&self, error: &RpcError) -> bool {
        // TODO: implement this
        // is_critical_validator_error(error)
        false
    }
}

#[derive(Clone, Debug)]
pub struct SimpleBlockRpc;

#[async_trait]
impl QueryCreator<RpcBlockRequest> for SimpleBlockRpc {
    type RpcReference = Reference;
    type Response = JsonRpcResponseForRpcBlockResponseAndRpcError;
    async fn send_query(
        &self,
        client: &Client,
        reference: Reference,
    ) -> Result<JsonRpcResponseForRpcBlockResponseAndRpcError, Error<()>> {
        let request = match reference {
            Reference::Optimistic => RpcBlockRequest::Finality(Finality::Optimistic),
            Reference::DoomSlug => RpcBlockRequest::Finality(Finality::NearFinal),
            Reference::Final => RpcBlockRequest::Finality(Finality::Final),
            Reference::AtBlock(block) => {
                RpcBlockRequest::BlockId(BlockId::BlockHeight(block.into()))
            }
            Reference::AtBlockHash(block_hash) => {
                RpcBlockRequest::BlockId(BlockId::CryptoHash(block_hash.into()))
            }
        };
        client
            .query(&JsonRpcRequestForBlock {
                id: 0,
                jsonrpc: "2.0".to_string(),
                method: JsonRpcRequestForBlockMethod::Block,
                params: request,
            })
            .await
    }

    fn is_critical_error(&self, error: &RpcError) -> bool {
        // TODO: implement this
        // is_critical_blocks_error(error)
        false
    }
}

pub type QueryBuilder<T> =
    RpcBuilder<T, RpcQueryRequest, JsonRpcResponseForRpcQueryResponseAndRpcError, Reference>;
pub type MultiQueryBuilder<T> =
    MultiRpcBuilder<T, RpcQueryRequest, JsonRpcResponseForRpcQueryResponseAndRpcError, Reference>;

pub type ValidatorQueryBuilder<T> = RpcBuilder<
    T,
    RpcValidatorRequest,
    JsonRpcResponseForRpcValidatorResponseAndRpcError,
    EpochReference,
>;
pub type BlockQueryBuilder<T> =
    RpcBuilder<T, RpcBlockRequest, JsonRpcResponseForRpcBlockResponseAndRpcError, Reference>;

/// A builder for querying multiple items at once.
///
/// Sometimes to construct some complex type, you would need to query multiple items at once, and combine them into one.
/// This is where this builder comes in handy. Almost every time, you would want to use [Self::map] method to combine the responses into your desired type.
///
/// Currently, `MultiQueryHandler` supports tuples of sizes 2 and 3.
/// For single responses, use `QueryBuilder` instead.
///
/// Here is a list of examples on how to use this:
/// - [Tokens::ft_balance](crate::tokens::Tokens::ft_balance)
/// - [StakingPool::staking_pool_info](crate::stake::Staking::staking_pool_info)
pub struct MultiRpcBuilder<Handler, Method, Response, Reference>
where
    Reference: Send + Sync,
    Handler: Send + Sync,
{
    reference: Reference,
    requests: Vec<
        Arc<dyn QueryCreator<Method, Response = Response, RpcReference = Reference> + Send + Sync>,
    >,
    handler: Handler,
}

impl<Handler, Method, Response, Reference> MultiRpcBuilder<Handler, Method, Response, Reference>
where
    Reference: Send + Sync,
    Handler: Default + Send + Sync,
{
    pub fn with_reference(reference: impl Into<Reference>) -> Self {
        Self {
            reference: reference.into(),
            requests: vec![],
            handler: Default::default(),
        }
    }
}

impl<Handler, Method, Response, Reference> MultiRpcBuilder<Handler, Method, Response, Reference>
where
    Handler: ResponseHandler<QueryResponse = Response, Method = Method> + Send + Sync,
    Method: std::fmt::Debug + Send + Sync + 'static,
    Response: std::fmt::Debug + Send + Sync,
    Reference: Clone + Send + Sync,
{
    pub fn new(handler: Handler, reference: Reference) -> Self {
        Self {
            reference,
            requests: vec![],
            handler,
        }
    }

    /// Map response of the queries to another type. The `map` function is executed after the queries are fetched.
    ///
    /// The `Handler::Response` is the type returned by the handler's `process_response` method.
    ///
    /// For single responses, use `QueryBuilder` instead.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::advanced::{MultiQueryHandler, CallResultHandler, MultiRpcBuilder};
    /// use near_api::types::Data;
    /// use std::marker::PhantomData;
    /// use near_primitives::types::BlockReference;
    ///
    /// // Create a handler for multiple query responses and specify the types of the responses
    /// let handler = MultiQueryHandler::new((
    ///     CallResultHandler::<String>::new(),
    ///     CallResultHandler::<u128>::new(),
    /// ));
    ///
    /// // Create the builder with the handler
    /// let builder = MultiRpcBuilder::new(handler, BlockReference::latest());
    ///
    /// // Add queries to the builder
    /// builder.add_query(todo!());
    ///
    /// // Map the tuple of responses to a combined type
    /// let mapped_builder = builder.map(|(response1, response2): (Data<String>, Data<u128>)| {
    ///     // Process the combined data
    ///     format!("{}: {}", response1.data, response2.data)
    /// });
    /// ```
    ///
    /// See [Tokens::ft_balance](crate::tokens::Tokens::ft_balance) implementation for a real-world example.
    pub fn map<MappedType>(
        self,
        map: impl Fn(Handler::Response) -> MappedType + Send + Sync + 'static,
    ) -> MultiRpcBuilder<PostprocessHandler<MappedType, Handler>, Method, Reference> {
        MultiRpcBuilder {
            handler: PostprocessHandler::new(self.handler, map),
            requests: self.requests,
            reference: self.reference,
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

        info!(target: QUERY_EXECUTOR_TARGET, "Sending {} queries", self.requests.len());
        let requests = self.requests.into_iter().map(|request| async move {
            retry(network.clone(), |client| {
                let request = &request;

                async move {
                    let result = match request.send_query(&client, self.reference.clone()).await {
                        Ok(result) => match result {
                            JsonRpcResponseForRpcQueryResponseAndRpcError::Variant0 {
                                id,
                                jsonrpc,
                                result,
                            } => RetryResponse::Ok(result),
                            JsonRpcResponseForRpcQueryResponseAndRpcError::Variant1 {
                                id,
                                jsonrpc,
                                error,
                            } => {
                                if request.is_critical_error(&error) {
                                    RetryResponse::Critical(error)
                                } else {
                                    RetryResponse::Retry(error)
                                }
                            }
                        },
                        Err(err) => RetryResponse::Critical(err),
                    };
                    tracing::debug!(
                        target: QUERY_EXECUTOR_TARGET,
                        "Querying RPC with {:?} resulted in {:?}",
                        request,
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

pub struct RpcBuilder<Handler, Method, Response, Reference> {
    reference: Reference,
    request:
        Arc<dyn QueryCreator<Method, Response = Response, RpcReference = Reference> + Send + Sync>,
    handler: Handler,
}

impl<Handler, Method, Response, Reference> RpcBuilder<Handler, Method, Response, Reference>
where
    Handler: ResponseHandler<QueryResponse = Response, Method = Method> + Send + Sync,
    Method: std::fmt::Debug + Send + Sync + 'static,
    Response: std::fmt::Debug + Send + Sync,
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

    /// Post-process the response of the query.
    ///
    /// This is useful if you want to convert one type to another.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let balance: NearToken = Contract("some_contract.testnet".parse()?)
    ///         .call_function("get_balance", ())?
    ///         .read_only()
    ///         .map(|balance: Data<u128>| NearToken::from_yoctonear(balance.data))
    ///         .fetch_from_testnet()
    ///         .await?;
    /// println!("Balance: {}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn map<MappedType>(
        self,
        map: impl Fn(Handler::Response) -> MappedType + Send + Sync + 'static,
    ) -> RpcBuilder<PostprocessHandler<MappedType, Handler>, Method, Reference> {
        RpcBuilder {
            handler: PostprocessHandler::new(self.handler, map),
            request: self.request,
            reference: self.reference,
        }
    }

    /// Fetch the query from the provided network.
    #[instrument(skip(self, network))]
    pub async fn fetch_from(
        self,
        network: &NetworkConfig,
    ) -> ResultWithMethod<Handler::Response, Method> {
        debug!(target: QUERY_EXECUTOR_TARGET, "Preparing query");

        let query_response = retry(network.clone(), |client| {
            let request = &self.request;
            async move {
                let result = match request.send_query(&client, self.reference.clone()).await {
                    Ok(result) => match result {
                        JsonRpcResponseForRpcQueryResponseAndRpcError::Variant0 {
                            id,
                            jsonrpc,
                            result,
                        } => RetryResponse::Ok(result),
                        JsonRpcResponseForRpcQueryResponseAndRpcError::Variant1 {
                            id,
                            jsonrpc,
                            error,
                        } => {
                            if request.is_critical_error(&error) {
                                RetryResponse::Critical(error)
                            } else {
                                RetryResponse::Retry(error)
                            }
                        }
                    },
                    Err(err) => RetryResponse::Critical(err),
                };
                tracing::debug!(
                    target: QUERY_EXECUTOR_TARGET,
                    "Querying RPC with {:?} resulted in {:?}",
                    request,
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
    H1: ResponseHandler<QueryResponse = QR, Response = R1, Method = Method>,
    H2: ResponseHandler<QueryResponse = QR, Response = R2, Method = Method>,
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

impl<Handlers: Default> Default for MultiQueryHandler<Handlers> {
    fn default() -> Self {
        Self::new(Default::default())
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

        if let RpcQueryResponse::Variant3 {
            result,
            logs,
            block_height,
            block_hash,
        } = response.kind
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
                got: Box::new(response.kind),
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
        } = response.kind
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
                },
                block_height,
                block_hash,
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
            Err(QueryError::UnexpectedResponse {
                expected: "ViewAccount",
                got: Box::new(response.kind),
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
        if let RpcQueryResponse::Variant5 {
            keys,
            block_height,
            block_hash,
        } = response.kind
        {
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed AccessKeyList response, keys count: {}",
                keys.len()
            );
            Ok(keys)
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
            Err(QueryError::UnexpectedResponse {
                expected: "AccessKeyList",
                got: Box::new(response.kind),
            })
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AccessKeyHandler;

impl ResponseHandler for AccessKeyHandler {
    type Response = Data<AccessKey>;
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
        if let RpcQueryResponse::Variant4 {
            block_hash,
            nonce,
            block_height,
            permission,
        } = response.kind
        {
            info!(
                target: QUERY_EXECUTOR_TARGET,
                "Processed AccessKey response, nonce: {}, permission: {:?}",
                permission.nonce,
                permission
            );
            Ok(Data {
                data: AccessKey { nonce, permission },
                block_height,
                block_hash,
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
            Err(QueryError::UnexpectedResponse {
                expected: "AccessKey",
                got: Box::new(response.kind),
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
        if let RpcQueryResponse::Variant2 {
            proof,
            values,
            block_height,
            block_hash,
        } = response.kind
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
                block_hash,
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
            Err(QueryError::UnexpectedResponse {
                expected: "ViewState",
                got: Box::new(response.kind),
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
        if let RpcQueryResponse::Variant1 {
            code_base64,
            hash,
            block_height,
            block_hash,
        } = response.kind
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
                block_hash,
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response.kind);
            Err(QueryError::UnexpectedResponse {
                expected: "ViewCode",
                got: Box::new(response.kind),
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct RpcValidatorHandler;

impl ResponseHandler for RpcValidatorHandler {
    type Response = Vec<CurrentEpochValidatorInfo>;
    type QueryResponse = RpcValidatorResponse;
    type Method = RpcValidatorRequest;

    fn process_response(
        &self,
        response: Vec<RpcValidatorResponse>,
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
        Ok(response.current_validators)
    }
}

#[derive(Clone, Debug)]
pub struct RpcBlockHandler;

impl ResponseHandler for RpcBlockHandler {
    type Response = RpcBlockResponse;
    type QueryResponse = RpcBlockResponse;
    type Method = RpcBlockRequest;

    fn process_response(
        &self,
        response: Vec<RpcBlockResponse>,
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
