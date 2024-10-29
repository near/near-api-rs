use near_primitives::{
    hash::CryptoHash,
    types::{BlockHeight, BlockReference},
    views::BlockView,
};

use crate::common::query::{
    BlockQueryBuilder, PostprocessHandler, RpcBlockHandler, SimpleBlockRpc,
};

#[derive(Debug, Clone, Copy)]
pub struct Chain;

impl Chain {
    pub fn block_number(
        &self,
    ) -> BlockQueryBuilder<PostprocessHandler<BlockHeight, RpcBlockHandler>> {
        BlockQueryBuilder::new(
            SimpleBlockRpc,
            BlockReference::latest(),
            PostprocessHandler::new(
                RpcBlockHandler,
                Box::new(|data: BlockView| data.header.height),
            ),
        )
    }

    pub fn block_hash(&self) -> BlockQueryBuilder<PostprocessHandler<CryptoHash, RpcBlockHandler>> {
        BlockQueryBuilder::new(
            SimpleBlockRpc,
            BlockReference::latest(),
            PostprocessHandler::new(
                RpcBlockHandler,
                Box::new(|data: BlockView| data.header.hash),
            ),
        )
    }

    pub fn block(&self) -> BlockQueryBuilder<RpcBlockHandler> {
        BlockQueryBuilder::new(SimpleBlockRpc, BlockReference::latest(), RpcBlockHandler)
    }
}
