use near_sdk::NearToken;
use serde::de::{Deserialize, Deserializer};

/// A type that represents the storage balance from the [NEP-145](https://nomicon.io/Standards/StorageManagement) standard
/// on some NEAR contract.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StorageBalance {
    /// The available balance that might be used for storage.
    ///
    /// The user can withdraw this balance.
    #[serde(deserialize_with = "parse_u128_string")]
    pub available: NearToken,
    /// The total user balance on the contract for storage.
    /// This is a sum of the `available` and `locked` balances.
    #[serde(deserialize_with = "parse_u128_string")]
    pub total: NearToken,
}

fn parse_u128_string<'de, D>(deserializer: D) -> Result<NearToken, D::Error>
where
    D: Deserializer<'de>,
{
    <std::string::String as Deserialize>::deserialize(deserializer)?
        .parse::<u128>()
        .map(NearToken::from_yoctonear)
        .map_err(serde::de::Error::custom)
}
