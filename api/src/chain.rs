use near_types::{BlockHeight, CryptoHash, Reference};

use crate::{
    advanced::{AndThenHandler, block_rpc::SimpleBlockRpc},
    common::query::{BlockQueryBuilder, PostprocessHandler, RpcBlockHandler},
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
    /// Set ups a query to fetch the [BlockHeight] of the current block
    ///
    /// ## Fetching the latest block number
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
    ///
    /// ## Fetching the final block number
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let block_number = Chain::block_number().at(Reference::Final).fetch_from_testnet().await?;
    /// println!("Final block number: {}", block_number);
    /// # Ok(())
    /// # }
    /// ```
    pub fn block_number() -> BlockQueryBuilder<PostprocessHandler<BlockHeight, RpcBlockHandler>> {
        BlockQueryBuilder::new(SimpleBlockRpc, Reference::Optimistic, RpcBlockHandler)
            .map(|data| data.header.height)
    }

    /// Set ups a query to fetch the [CryptoHash] of the block
    ///
    /// ## Fetching the latest block hash
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let block_hash = Chain::block_hash().fetch_from_testnet().await?;
    /// println!("Current block hash: {}", block_hash);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Fetching the hash at a specific block number
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let block_hash = Chain::block_hash().at(Reference::AtBlock(1000000)).fetch_from_testnet().await?;
    /// println!("Block hash at block number 1000000: {}", block_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub fn block_hash() -> BlockQueryBuilder<AndThenHandler<CryptoHash, RpcBlockHandler>> {
        BlockQueryBuilder::new(SimpleBlockRpc, Reference::Optimistic, RpcBlockHandler)
            .and_then(|data| Ok(CryptoHash::try_from(data.header.hash)?))
    }

    /// Set ups a query to fetch the [RpcBlockResponse][near_types::RpcBlockResponse]
    ///
    /// ## Fetching the latest block
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let block = Chain::block().fetch_from_testnet().await?;
    /// println!("Current block: {:?}", block);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Fetching the block at a specific block number
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let block = Chain::block().at(Reference::AtBlock(1000000)).fetch_from_testnet().await?;
    /// println!("Block at block number 1000000: {:?}", block);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Fetching the block at a specific block hash
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let block_hash = near_api::types::CryptoHash::default();       
    /// let block = Chain::block().at(Reference::AtBlockHash(block_hash)).fetch_from_testnet().await?;
    /// println!("Block at block hash: {:?}", block);
    /// # Ok(())
    /// # }
    /// ```
    pub fn block() -> BlockQueryBuilder<RpcBlockHandler> {
        BlockQueryBuilder::new(SimpleBlockRpc, Reference::Optimistic, RpcBlockHandler)
    }

    // TODO: chunk info
}
