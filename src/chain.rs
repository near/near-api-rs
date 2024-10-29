use near_primitives::{
    types::{BlockHeight, BlockReference},
    views::BlockView,
};

use crate::{
    common::query::{BlockQueryBuilder, PostprocessHandler, RpcBlockHandler, SimpleBlockRpc},
    types::CryptoHash,
};

#[derive(Debug, Clone, Copy)]
pub struct Chain;

impl Chain {
    pub fn block_number() -> BlockQueryBuilder<PostprocessHandler<BlockHeight, RpcBlockHandler>> {
        BlockQueryBuilder::new(
            SimpleBlockRpc,
            BlockReference::latest(),
            PostprocessHandler::new(
                RpcBlockHandler,
                Box::new(|data: BlockView| data.header.height),
            ),
        )
    }

    pub fn block_hash() -> BlockQueryBuilder<PostprocessHandler<CryptoHash, RpcBlockHandler>> {
        BlockQueryBuilder::new(
            SimpleBlockRpc,
            BlockReference::latest(),
            PostprocessHandler::new(
                RpcBlockHandler,
                Box::new(|data: BlockView| data.header.hash.into()),
            ),
        )
    }

    pub fn block() -> BlockQueryBuilder<RpcBlockHandler> {
        BlockQueryBuilder::new(SimpleBlockRpc, BlockReference::latest(), RpcBlockHandler)
    }
}
