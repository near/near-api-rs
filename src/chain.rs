use near_primitives::{
    types::{BlockHeight, BlockReference},
    views::BlockView,
};

use crate::{
    common::query::{BlockQueryBuilder, PostprocessHandler, RpcBlockHandler, SimpleBlockRpc},
    types::CryptoHash,
};

/// Chain-related interactions with the NEAR Protocol
///
/// The [`Chain`] struct provides methods to interact with the NEAR blockchain
///
/// # Examples
///
/// ```rust,no_run
/// use near_api::*;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let block_number = Chain::block_number().fetch_from_testnet().await?;
/// println!("Current block number: {}", block_number);
/// # Ok(())
/// # }
/// ```
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

    // TODO: fetch transaction status
    // TODO: fetch transaction receipt
    // TODO: fetch transaction proof
    // TODO: fetch epoch id
    // TODO: fetch epoch info
}
