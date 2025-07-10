use tracing::trace;

use crate::{
    advanced::{RpcType, handlers::ResponseHandler},
    common::query::{QUERY_EXECUTOR_TARGET, ResultWithMethod},
};

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
