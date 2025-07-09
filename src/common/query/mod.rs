// TODO: root level doc might be needed here. It's pretty complicated.
use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use futures::future::join_all;
use near_openapi_client::Client;
use near_openapi_types::{
    AccessKey, AccessKeyList, AccessKeyPermission, AccessKeyPermissionView, AccountView,
    ContractCodeView, CurrentEpochValidatorInfo, FunctionCallPermission, RpcBlockResponse,
    RpcQueryResponse, RpcValidatorResponse, ViewStateResult,
};
use serde::de::DeserializeOwned;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::{
    advanced::{
        block_rpc::SimpleBlockRpc, query_rpc::SimpleQueryRpc, validator_rpc::SimpleValidatorRpc,
    },
    common::utils::query_to_kind,
    config::{NetworkConfig, RetryResponse, retry},
    errors::{QueryError, SendRequestError},
    types::Data,
};

pub mod block_rpc;
pub mod query_rpc;
pub mod validator_rpc;

const QUERY_EXECUTOR_TARGET: &str = "near_api::query::executor";

type ResultWithMethod<T, E> = core::result::Result<T, QueryError<E>>;

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

#[async_trait]
pub trait RpcType: Send + Sync + std::fmt::Debug {
    type RpcReference: Send + Sync + Clone;
    type Response;
    type Error: std::fmt::Debug + Send + Sync;
    async fn send_query(
        &self,
        client: &Client,
        network: &NetworkConfig,
        reference: &Self::RpcReference,
    ) -> RetryResponse<Self::Response, SendRequestError<Self::Error>>;
}

pub type QueryBuilder<T: ResponseHandler> = RpcBuilder<T::Query, T>;
pub type MultiQueryBuilder<T: ResponseHandler> = MultiRpcBuilder<T::Query, T>;

pub type ValidatorQueryBuilder<T: ResponseHandler> = RpcBuilder<T::Query, T>;
pub type BlockQueryBuilder<T: ResponseHandler> = RpcBuilder<T::Query, T>;

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
pub struct MultiRpcBuilder<Query, Handler>
where
    Query: RpcType,
    Query::Response: std::fmt::Debug + Send + Sync,
    Query::Error: std::fmt::Debug + Send + Sync,
    Handler: Send + Sync,
{
    reference: Query::RpcReference,
    requests: Vec<
        Arc<
            dyn RpcType<
                    Response = Query::Response,
                    RpcReference = Query::RpcReference,
                    Error = Query::Error,
                > + Send
                + Sync,
        >,
    >,
    handler: Handler,
}

impl<Query, Handler> MultiRpcBuilder<Query, Handler>
where
    Handler: Default + Send + Sync,
    Query: RpcType,
    Query::Response: std::fmt::Debug + Send + Sync,
    Query::Error: std::fmt::Debug + Send + Sync,
    Handler: Send + Sync,
{
    pub fn with_reference(reference: impl Into<Query::RpcReference>) -> Self {
        Self {
            reference: reference.into(),
            requests: vec![],
            handler: Default::default(),
        }
    }
}

impl<Query, Handler> MultiRpcBuilder<Query, Handler>
where
    Handler: ResponseHandler<Query = Query> + Send + Sync,
    Query: RpcType,
    Query::Response: std::fmt::Debug + Send + Sync,
    Query::Error: std::fmt::Debug + Send + Sync,
    Handler: Send + Sync,
{
    pub fn new(handler: Handler, reference: impl Into<Query::RpcReference>) -> Self {
        Self {
            reference: reference.into(),
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
    /// let builder = MultiRpcBuilder::new(handler, Reference::Optimistic);
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
    ) -> MultiRpcBuilder<Query, PostprocessHandler<MappedType, Handler>> {
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
        request: Arc<
            dyn RpcType<
                    Response = Query::Response,
                    RpcReference = Query::RpcReference,
                    Error = Query::Error,
                > + Send
                + Sync,
        >,
    ) -> Self {
        self.requests.push(request);
        self
    }

    /// It might be easier to use this method to add a query builder to the queried items.
    pub fn add_query_builder<Handler2>(mut self, query_builder: RpcBuilder<Query, Handler2>) -> Self
    where
        Handler2: ResponseHandler<Query = Query> + Send + Sync,
    {
        self.requests.push(query_builder.request);
        self
    }

    /// Set the block reference for the queries.
    pub fn at(self, reference: impl Into<Query::RpcReference>) -> Self {
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
    ) -> ResultWithMethod<Handler::Response, Query::Error> {
        debug!(target: QUERY_EXECUTOR_TARGET, "Preparing queries");

        info!(target: QUERY_EXECUTOR_TARGET, "Sending {} queries", self.requests.len());
        let requests = self.requests.into_iter().map(|request| {
            let reference = &self.reference;
            async move {
                retry(network.clone(), |client| {
                    let request = &request;

                    async move {
                        let result = request.send_query(&client, network, reference).await;

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
            }
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
    pub async fn fetch_from_mainnet(self) -> ResultWithMethod<Handler::Response, Query::Error> {
        let network = NetworkConfig::mainnet();
        self.fetch_from(&network).await
    }

    /// Fetch the queries from the default testnet network configuration.
    pub async fn fetch_from_testnet(self) -> ResultWithMethod<Handler::Response, Query::Error> {
        let network = NetworkConfig::testnet();
        self.fetch_from(&network).await
    }
}

pub struct RpcBuilder<Query, Handler>
where
    Query: RpcType,
    Query::Response: std::fmt::Debug + Send + Sync,
    Query::Error: std::fmt::Debug + Send + Sync,
    Handler: Send + Sync,
{
    reference: Query::RpcReference,
    request: Arc<
        dyn RpcType<
                Response = Query::Response,
                RpcReference = Query::RpcReference,
                Error = Query::Error,
            > + Send
            + Sync,
    >,
    handler: Handler,
}

impl<Query, Handler> RpcBuilder<Query, Handler>
where
    Handler: ResponseHandler<Query = Query> + Send + Sync,
    Query: RpcType + 'static,
    Query::Response: std::fmt::Debug + Send + Sync,
    Query::Error: std::fmt::Debug + Send + Sync,
{
    pub fn new(
        request: Query,
        reference: impl Into<Query::RpcReference>,
        handler: Handler,
    ) -> Self {
        Self {
            reference: reference.into(),
            request: Arc::new(request),
            handler,
        }
    }

    /// Set the block reference for the query.
    pub fn at(self, reference: impl Into<Query::RpcReference>) -> Self {
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
    ) -> RpcBuilder<Query, PostprocessHandler<MappedType, Handler>> {
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
    ) -> ResultWithMethod<Handler::Response, Query::Error> {
        debug!(target: QUERY_EXECUTOR_TARGET, "Preparing query");

        let query_response = retry(network.clone(), |client| {
            let request = &self.request;
            let reference = &self.reference;
            async move {
                let result = request.send_query(&client, network, reference).await;
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
    pub async fn fetch_from_mainnet(self) -> ResultWithMethod<Handler::Response, Query::Error> {
        let network = NetworkConfig::mainnet();
        self.fetch_from(&network).await
    }

    /// Fetch the query from the default testnet network configuration.
    pub async fn fetch_from_testnet(self) -> ResultWithMethod<Handler::Response, Query::Error> {
        let network = NetworkConfig::testnet();
        self.fetch_from(&network).await
    }
}

#[derive(Clone, Debug)]
pub struct MultiQueryHandler<Handlers> {
    handlers: Handlers,
}

impl<Query, H1, H2, R1, R2> ResponseHandler for MultiQueryHandler<(H1, H2)>
where
    Query: RpcType,
    H1: ResponseHandler<Response = R1, Query = Query>,
    H2: ResponseHandler<Response = R2, Query = Query>,
{
    type Response = (R1, R2);
    type Query = H1::Query;

    fn process_response(
        &self,
        mut responses: Vec<<H1::Query as RpcType>::Response>,
    ) -> ResultWithMethod<Self::Response, <H1::Query as RpcType>::Error> {
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

impl<Query, H1, H2, H3, R1, R2, R3> ResponseHandler for MultiQueryHandler<(H1, H2, H3)>
where
    Query: RpcType,
    H1: ResponseHandler<Response = R1, Query = Query>,
    H2: ResponseHandler<Response = R2, Query = Query>,
    H3: ResponseHandler<Response = R3, Query = Query>,
{
    type Response = (R1, R2, R3);
    type Query = Query;

    fn process_response(
        &self,
        mut responses: Vec<<Query as RpcType>::Response>,
    ) -> ResultWithMethod<Self::Response, <Query as RpcType>::Error> {
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
    type Query = Handler::Query;

    fn process_response(
        &self,
        response: Vec<<Self::Query as RpcType>::Response>,
    ) -> ResultWithMethod<Self::Response, <Self::Query as RpcType>::Error> {
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
                block_hash: block_hash.into(),
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
    type Response = Data<AccountView>;

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
                },
                block_height,
                block_hash: block_hash.into(),
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response);
            Err(QueryError::UnexpectedResponse {
                expected: "ViewAccount",
                got: query_to_kind(&response),
            })
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AccessKeyListHandler;

impl ResponseHandler for AccessKeyListHandler {
    type Response = Data<AccessKeyList>;
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
                data: AccessKeyList { keys },
                block_height,
                block_hash: block_hash.into(),
            })
        } else {
            warn!(target: QUERY_EXECUTOR_TARGET, "Unexpected response kind: {:?}", response);
            Err(QueryError::UnexpectedResponse {
                expected: "AccessKeyList",
                got: query_to_kind(&response),
            })
        }
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
                    nonce,
                    permission: match permission {
                        AccessKeyPermissionView::FullAccess => AccessKeyPermission::FullAccess,
                        AccessKeyPermissionView::FunctionCall {
                            allowance,
                            method_names,
                            receiver_id,
                        } => AccessKeyPermission::FunctionCall(FunctionCallPermission {
                            allowance,
                            method_names,
                            receiver_id,
                        }),
                    },
                },
                block_height,
                block_hash: block_hash.into(),
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
                block_hash: block_hash.into(),
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
                block_hash: block_hash.into(),
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
