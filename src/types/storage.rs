use near_sdk::NearToken;
use serde::de::{Deserialize, Deserializer};

// Taken from https://github.com/bos-cli-rs/near-socialdb-client-rs/blob/main/src/lib.rs
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StorageBalance {
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
