// TODO: root level doc might be needed here. It's pretty complicated.
use async_trait::async_trait;
use futures::future::join_all;
use near_openapi_client::Client;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

use crate::{
    config::{retry, NetworkConfig, RetryResponse},
    errors::{ArgumentSerializationError, QueryError, SendRequestError},
};

pub mod block_rpc;
pub mod handlers;
pub mod query_request;
pub mod query_rpc;
pub mod validator_rpc;

pub use handlers::*;

const QUERY_EXECUTOR_TARGET: &str = "near_api::query::executor";

type ResultWithMethod<T, E> = core::result::Result<T, QueryError<E>>;

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

pub type RequestBuilder<T> = RpcBuilder<<T as ResponseHandler>::Query, T>;
pub type MultiRequestBuilder<T> = MultiRpcBuilder<<T as ResponseHandler>::Query, T>;

/// A builder for querying multiple items at once.
///
/// Sometimes to construct some complex type, you would need to query multiple items at once, and combine them into one.
/// This is where this builder comes in handy. Almost every time, you would want to use [Self::map] method to combine the responses into your desired type.
///
/// Currently, `MultiQueryHandler` supports tuples of sizes 2 and 3.
/// For single responses, use `RequestBuilder` instead.
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
    #[allow(clippy::type_complexity)]
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

    deferred_errors: Vec<ArgumentSerializationError>,
}

impl<Query, Handler> MultiRpcBuilder<Query, Handler>
where
    Handler: Default + Send + Sync,
    Query: RpcType,
    Query::Response: std::fmt::Debug + Send + Sync,
    Query::Error: std::fmt::Debug + Send + Sync,
{
    pub fn with_reference(reference: impl Into<Query::RpcReference>) -> Self {
        Self {
            reference: reference.into(),
            requests: vec![],
            handler: Default::default(),
            deferred_errors: vec![],
        }
    }
}

impl<Query, Handler> MultiRpcBuilder<Query, Handler>
where
    Handler: ResponseHandler<Query = Query> + Send + Sync,
    Query: RpcType,
    Query::Response: std::fmt::Debug + Send + Sync,
    Query::Error: std::fmt::Debug + Send + Sync,
{
    pub fn new(handler: Handler, reference: impl Into<Query::RpcReference>) -> Self {
        Self {
            reference: reference.into(),
            requests: vec![],
            handler,
            deferred_errors: vec![],
        }
    }

    pub fn with_deferred_error(mut self, error: ArgumentSerializationError) -> Self {
        self.deferred_errors.push(error);
        self
    }

    /// Map response of the queries to another type. The `map` function is executed after the queries are fetched.
    ///
    /// The `Handler::Response` is the type returned by the handler's `process_response` method.
    ///
    /// For single responses, use `RequestBuilder` instead.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::advanced::{MultiQueryHandler, CallResultHandler, MultiRpcBuilder};
    /// use near_api::types::{Data, Reference};
    /// use std::marker::PhantomData;
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
            deferred_errors: self.deferred_errors,
        }
    }

    /// Post-process the response of the query with error handling
    ///
    /// This is useful if you want to convert one type to another but your function might fail.
    ///
    /// The error will be wrapped in a `QueryError::ConversionError` and returned to the caller.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let balance: NearToken = Contract("some_contract.testnet".parse()?)
    ///         .call_function("get_balance", ())
    ///         .read_only()
    ///         .and_then(|balance: Data<String>| Ok(NearToken::from_yoctonear(balance.data.parse()?)))
    ///         .fetch_from_testnet()
    ///         .await?;
    /// println!("Balance: {}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_then<MappedType>(
        self,
        map: impl Fn(Handler::Response) -> Result<MappedType, Box<dyn std::error::Error + Send + Sync>>
            + Send
            + Sync
            + 'static,
    ) -> MultiRpcBuilder<Query, AndThenHandler<MappedType, Handler>> {
        MultiRpcBuilder {
            handler: AndThenHandler::new(self.handler, map),
            requests: self.requests,
            reference: self.reference,
            deferred_errors: self.deferred_errors,
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
        self.deferred_errors.extend(query_builder.deferred_error);
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
        if !self.deferred_errors.is_empty() {
            return Err(QueryError::ArgumentSerializationError(
                ArgumentSerializationError::multiple(self.deferred_errors),
            ));
        }

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
    deferred_error: Option<ArgumentSerializationError>,
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
            deferred_error: None,
        }
    }

    pub fn with_deferred_error(mut self, error: ArgumentSerializationError) -> Self {
        self.deferred_error = Some(error);
        self
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
    ///         .call_function("get_balance", ())
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
            deferred_error: self.deferred_error,
        }
    }

    /// Post-process the response of the query with error handling
    ///
    /// This is useful if you want to convert one type to another but your function might fail.
    ///
    /// The error will be wrapped in a `QueryError::ConversionError` and returned to the caller.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let balance: NearToken = Contract("some_contract.testnet".parse()?)
    ///         .call_function("get_balance", ())
    ///         .read_only()
    ///         .and_then(|balance: Data<String>| Ok(NearToken::from_yoctonear(balance.data.parse()?)))
    ///         .fetch_from_testnet()
    ///         .await?;
    /// println!("Balance: {}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_then<MappedType>(
        self,
        map: impl Fn(Handler::Response) -> Result<MappedType, Box<dyn std::error::Error + Send + Sync>>
            + Send
            + Sync
            + 'static,
    ) -> RpcBuilder<Query, AndThenHandler<MappedType, Handler>> {
        RpcBuilder {
            handler: AndThenHandler::new(self.handler, map),
            request: self.request,
            reference: self.reference,
            deferred_error: self.deferred_error,
        }
    }

    /// Fetch the query from the provided network.
    #[instrument(skip(self, network))]
    pub async fn fetch_from(
        self,
        network: &NetworkConfig,
    ) -> ResultWithMethod<Handler::Response, Query::Error> {
        if let Some(err) = self.deferred_error {
            return Err(QueryError::ArgumentSerializationError(err));
        }

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
