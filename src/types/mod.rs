use near_primitives::{hash::CryptoHash, types::BlockHeight};
use reqwest::header::InvalidHeaderValue;

pub mod contract;
pub mod reference;
pub mod stake;
pub mod storage;
pub mod tokens;
pub mod transactions;

#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    borsh::BorshDeserialize,
    borsh::BorshSerialize,
)]
pub struct Data<T> {
    pub data: T,
    pub block_height: BlockHeight,
    pub block_hash: CryptoHash,
}

#[derive(Eq, Hash, Clone, Debug, PartialEq)]
pub struct ApiKey(pub near_jsonrpc_client::auth::ApiKey);

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
