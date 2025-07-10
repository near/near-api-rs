use near_sdk::NearToken;
use serde::de::{Deserialize, Deserializer};

/// A type that represents the storage balance. Please note that this type is not part of the NEP-145 standard.
/// This type provides a more detailed view of the storage balance on the contract.
///
/// [StorageBalanceInternal] is a [NEP-145](https://github.com/near/NEPs/blob/master/neps/nep-0145.md) standard type.
/// This type is used internally to parse the storage balance from the contract and
/// to convert it into the [StorageBalance] type.
///
/// As a storing data on-chain requires storage staking, the contracts require users to deposit NEAR to store user-rel.
#[derive(Debug, Clone)]
pub struct StorageBalance {
    /// The available balance that might be used for storage.
    ///
    /// The user can withdraw this balance from the contract.
    pub available: NearToken,
    /// The total user balance on the contract for storage.
    ///
    /// This is a sum of the `available` and `locked` balances.
    pub total: NearToken,

    /// The storage deposit that is locked for the account
    ///
    /// The user can unlock some funds by removing the data from the contract.
    /// Though, it's contract-specific on how much can be unlocked.
    pub locked: NearToken,
}

/// Used internally to parse the storage balance from the contract and
/// to convert it into the [StorageBalance] type.
///
/// This type is a part of the [NEP-145](https://github.com/near/NEPs/blob/master/neps/nep-0145.md) standard.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StorageBalanceInternal {
    #[serde(deserialize_with = "parse_u128_string")]
    pub available: NearToken,
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
