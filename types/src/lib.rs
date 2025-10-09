use sha2::Digest;
use std::fmt;

pub mod account;
pub mod contract;
pub mod crypto;
pub mod errors;
pub mod ft;
pub mod json;
pub mod nft;
pub mod reference;
pub mod signable_message;
pub mod stake;
pub mod storage;
pub mod tokens;
pub mod transaction;
pub mod utils;

#[cfg(feature = "sandbox")]
pub mod sandbox;

pub use near_abi as abi;
pub use near_account_id::AccountId;
pub use near_gas::NearGas;
pub use near_openapi_types::{
    AccountView, ContractCodeView, FunctionArgs, RpcBlockResponse, RpcTransactionResponse,
    RpcValidatorResponse, StoreKey, StoreValue, TxExecutionStatus, ViewStateResult,
};
pub use near_token::NearToken;
pub use reference::{EpochReference, Reference};
pub use storage::{StorageBalance, StorageBalanceInternal};

pub use account::Account;
pub use crypto::public_key::PublicKey;
pub use crypto::secret_key::SecretKey;
pub use crypto::signature::Signature;
pub use transaction::actions::{AccessKey, AccessKeyPermission, Action};
pub use transaction::receipt::{DelayedReceipt, Receipt};

use crate::errors::DataConversionError;

pub type BlockHeight = u64;
pub type Nonce = u64;
pub type StorageUsage = u64;

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    borsh::BorshDeserialize,
    borsh::BorshSerialize,
)]
pub struct ShardId(pub u64);

impl From<near_openapi_types::ShardId> for ShardId {
    fn from(value: near_openapi_types::ShardId) -> Self {
        Self(value.0)
    }
}

impl From<ShardId> for near_openapi_types::ShardId {
    fn from(value: ShardId) -> Self {
        Self(value.0)
    }
}

/// A wrapper around a generic query result that includes the block height and block hash
/// at which the query was executed
#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    borsh::BorshDeserialize,
    borsh::BorshSerialize,
)]
pub struct Data<T> {
    /// The data returned by the query
    pub data: T,
    /// The block height at which the query was executed
    pub block_height: BlockHeight,
    /// The block hash at which the query was executed
    pub block_hash: CryptoHash,
}

impl<T> Data<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Data<U> {
        Data {
            data: f(self.data),
            block_height: self.block_height,
            block_hash: self.block_hash,
        }
    }
}

/// A type that represents a hash of the data.
///
/// This type is copy of the [crate::CryptoHash]
/// as part of the [decoupling initiative](https://github.com/near/near-api-rs/issues/5)
#[derive(
    Copy,
    Clone,
    Default,
    Hash,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    serde::Serialize,
    serde::Deserialize,
    borsh::BorshDeserialize,
    borsh::BorshSerialize,
)]
pub struct CryptoHash(pub [u8; 32]);

impl CryptoHash {
    pub fn hash(bytes: &[u8]) -> Self {
        Self(sha2::Sha256::digest(bytes).into())
    }
}

impl std::str::FromStr for CryptoHash {
    type Err = DataConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(s).into_vec()?;
        Self::try_from(bytes)
    }
}

impl TryFrom<&[u8]> for CryptoHash {
    type Error = DataConversionError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 32 {
            return Err(DataConversionError::IncorrectLength(bytes.len()));
        }
        let mut buf = [0; 32];
        buf.copy_from_slice(bytes);
        Ok(Self(buf))
    }
}

impl TryFrom<Vec<u8>> for CryptoHash {
    type Error = DataConversionError;

    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        <Self as TryFrom<&[u8]>>::try_from(v.as_ref())
    }
}

impl TryFrom<near_openapi_types::CryptoHash> for CryptoHash {
    type Error = DataConversionError;

    fn try_from(value: near_openapi_types::CryptoHash) -> Result<Self, Self::Error> {
        let near_openapi_types::CryptoHash(hash) = value;
        let bytes = bs58::decode(hash).into_vec()?;
        Self::try_from(bytes)
    }
}

impl std::fmt::Debug for CryptoHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Display for CryptoHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(&bs58::encode(self.0).into_string(), f)
    }
}

impl From<CryptoHash> for near_openapi_types::CryptoHash {
    fn from(hash: CryptoHash) -> Self {
        Self(hash.to_string())
    }
}
