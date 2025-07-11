use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::fmt;

use crate::errors::CryptoHashError;

pub mod actions;
pub mod contract;
pub mod delegate_action;
pub mod errors;
pub mod public_key;
pub mod reference;
pub mod signable_message;
pub mod signature;
pub mod stake;
pub mod storage;
pub mod tokens;
pub mod transactions;

pub use near_abi as abi;
pub use near_account_id::AccountId;
pub use near_contract_standards::{fungible_token, non_fungible_token};
pub use near_crypto::{ED25519SecretKey, InMemorySigner, SecretKey};
pub use near_gas::NearGas;
pub use near_openapi_types::{
    AccountView, ContractCodeView, FunctionArgs, RpcBlockResponse, RpcTransactionResponse,
    RpcValidatorResponse, StoreKey, ViewStateResult,
};
pub use near_sdk::json_types::U128;
pub use near_token::NearToken;
pub use reference::{EpochReference, Reference};
pub use storage::{StorageBalance, StorageBalanceInternal};

pub use actions::{AccessKey, AccessKeyPermission, Action};
pub use public_key::PublicKey;
pub use signature::Signature;
pub mod integers;

pub type BlockHeight = u64;
pub type Nonce = u64;

pub fn hash(bytes: &[u8]) -> CryptoHash {
    CryptoHash(sha2::Sha256::digest(bytes).into())
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

/// A wrapper around [near_openapi_client::auth::ApiKey]
///
/// This type is used to authenticate requests to the RPC node
///
/// ## Creating an API key
///
/// ```
/// use near_api::types::ApiKey;
/// use std::str::FromStr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let api_key = ApiKey("your_api_key".to_string());
/// # Ok(())
/// # }
#[derive(Eq, Hash, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiKey(String);

impl std::fmt::Display for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A type that represents a hash of the data.
///
/// This type is copy of the [near_primitives::hash::CryptoHash]
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

impl std::str::FromStr for CryptoHash {
    type Err = CryptoHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(s).into_vec()?;
        Self::try_from(bytes)
    }
}

impl TryFrom<&[u8]> for CryptoHash {
    type Error = CryptoHashError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 32 {
            return Err(CryptoHashError::IncorrectHashLength(bytes.len()));
        }
        let mut buf = [0; 32];
        buf.copy_from_slice(bytes);
        Ok(Self(buf))
    }
}

impl TryFrom<Vec<u8>> for CryptoHash {
    type Error = CryptoHashError;

    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        <Self as TryFrom<&[u8]>>::try_from(v.as_ref())
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

impl From<near_openapi_types::CryptoHash> for CryptoHash {
    fn from(hash: near_openapi_types::CryptoHash) -> Self {
        // TODO: handle error
        hash.0.parse().unwrap()
    }
}

impl From<CryptoHash> for near_openapi_types::CryptoHash {
    fn from(hash: CryptoHash) -> Self {
        near_openapi_types::CryptoHash(hash.to_string())
    }
}
