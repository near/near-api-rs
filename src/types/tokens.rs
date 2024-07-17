use near_token::NearToken;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct FungibleToken {
    pub balance: u128,
    pub decimals: u8,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct UserBalance {
    pub liquid: NearToken,
    pub locked: NearToken,
    pub storage_usage: u64,
}
