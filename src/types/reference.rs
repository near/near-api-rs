// Source: <https://github.com/near/near-workspaces-rs/blob/10a6c1a00b2b6c937242043312455e05f0d4a125/workspaces/src/types/mod.rs#L513C1-L537C2>

use crate::types::{BlockHeight, CryptoHash};

/// A reference to a specific block. This type is used to specify the block for most queries.
///
/// It represents the finality of a transaction or block in which transaction is included in. For more info
/// go to the [NEAR finality](https://docs.near.org/docs/concepts/transaction#finality) docs.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Reference {
    /// Optimistic finality. The latest block recorded on the node that responded to our query
    /// (<1 second delay after the transaction is submitted).
    Optimistic,
    /// Near-final finality. Similarly to `Final` finality, but delay should be roughly 1 second.
    NearFinal,
    /// Final finality. The block that has been validated on at least 66% of the nodes in the
    /// network. (At max, should be 2 second delay after the transaction is submitted.)
    Final,
    /// Reference to a specific block.
    AtBlock(BlockHeight),
    /// Reference to a specific block hash.
    AtBlockHash(CryptoHash),
}

/// A reference to a specific epoch. This type is used to specify the epoch for some queries.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum EpochReference {
    /// Reference to a specific Epoch Id
    AtEpoch(CryptoHash),
    /// Reference to an epoch at a specific block height.
    AtBlock(BlockHeight),
    /// Reference to an epoch at a specific block hash.
    AtBlockHash(CryptoHash),
    /// Latest epoch on the node
    Latest,
}
