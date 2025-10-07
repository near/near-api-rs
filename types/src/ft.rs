use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::utils::base64_bytes;

#[derive(Clone, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct FungibleTokenMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub reference: Option<String>,
    #[serde(with = "base64_bytes")]
    pub reference_hash: Vec<u8>,
    pub decimals: u8,
}
