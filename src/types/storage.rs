// Taken from https://github.com/bos-cli-rs/near-socialdb-client-rs/blob/main/src/lib.rs
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StorageBalance {
    #[serde(deserialize_with = "parse_u128_string")]
    pub available: u128,
    #[serde(deserialize_with = "parse_u128_string")]
    pub total: u128,
}
