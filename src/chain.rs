use near_primitives::types::{BlockHeight, BlockReference};

use crate::{
    common::query::{BlockQueryBuilder, PostprocessHandler, RpcBlockHandler, SimpleBlockRpc},
    types::{views::Block, CryptoHash},
};

#[derive(Debug, Clone, Copy)]
pub struct Chain;

impl Chain {
    pub fn block_number() -> BlockQueryBuilder<PostprocessHandler<BlockHeight, RpcBlockHandler>> {
        BlockQueryBuilder::new(
            SimpleBlockRpc,
            BlockReference::latest(),
            PostprocessHandler::new(RpcBlockHandler, Box::new(|data: Block| data.header.height)),
        )
    }

    pub fn block_hash() -> BlockQueryBuilder<PostprocessHandler<CryptoHash, RpcBlockHandler>> {
        BlockQueryBuilder::new(
            SimpleBlockRpc,
            BlockReference::latest(),
            PostprocessHandler::new(RpcBlockHandler, Box::new(|data: Block| data.header.hash)),
        )
    }

    pub fn block() -> BlockQueryBuilder<RpcBlockHandler> {
        BlockQueryBuilder::new(SimpleBlockRpc, BlockReference::latest(), RpcBlockHandler)
    }
}
