use std::fmt;

use near_primitives::types::BlockHeight;
use reqwest::header::InvalidHeaderValue;

use crate::errors::CryptoHashError;

pub mod contract;
pub mod reference;
pub mod signed_delegate_action;
pub mod stake;
pub mod storage;
pub mod tokens;
pub mod transactions;

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

/// A wrapper around [near_jsonrpc_client::auth::ApiKey]
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
/// let api_key = ApiKey::from_str("your_api_key")?;
/// # Ok(())
/// # }
/// ```
#[derive(Eq, Hash, Clone, Debug, PartialEq)]
pub struct ApiKey(near_jsonrpc_client::auth::ApiKey);

impl From<ApiKey> for near_jsonrpc_client::auth::ApiKey {
    fn from(api_key: ApiKey) -> Self {
        api_key.0
    }
}

impl std::fmt::Display for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.to_str().map_err(|_| std::fmt::Error)?)
    }
}

impl std::str::FromStr for ApiKey {
    type Err = InvalidHeaderValue;

    fn from_str(api_key: &str) -> Result<Self, Self::Err> {
        Ok(Self(near_jsonrpc_client::auth::ApiKey::new(api_key)?))
    }
}

impl serde::ser::Serialize for ApiKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.0.to_str().map_err(serde::ser::Error::custom)?)
    }
}

impl<'de> serde::de::Deserialize<'de> for ApiKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(|err: InvalidHeaderValue| serde::de::Error::custom(err.to_string()))
    }
}

fn from_base58(s: &str) -> Result<Vec<u8>, bs58::decode::Error> {
    bs58::decode(s).into_vec()
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
        let bytes = from_base58(s)?;
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
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for CryptoHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(&bs58::encode(self.0).into_string(), f)
    }
}

impl From<near_primitives::hash::CryptoHash> for CryptoHash {
    fn from(hash: near_primitives::hash::CryptoHash) -> Self {
        Self(hash.0)
    }
}

impl From<CryptoHash> for near_primitives::hash::CryptoHash {
    fn from(hash: CryptoHash) -> Self {
        Self(hash.0)
    }
}
