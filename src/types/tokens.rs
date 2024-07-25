use near_token::NearToken;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct FTBalance {
    balance: u128,
    decimals: u8,
}

impl FTBalance {
    pub fn from_smallest(balance: u128, decimals: u8) -> Self {
        Self { balance, decimals }
    }

    pub fn from_millis(balance: u128, decimals: u8) -> Self {
        Self {
            balance: balance * 10u128.pow(decimals as u32 - 3),
            decimals,
        }
    }

    pub fn from_whole(balance: u128, decimals: u8) -> Self {
        Self {
            balance: balance * 10u128.pow(decimals as u32),
            decimals,
        }
    }

    pub fn to_smallest(&self) -> u128 {
        self.balance
    }

    pub fn to_millis(&self) -> u128 {
        self.balance / 10u128.pow(self.decimals as u32 - 3)
    }

    pub fn to_whole(&self) -> u128 {
        self.balance / 10u128.pow(self.decimals as u32)
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }
}

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct UserBalance {
    pub liquid: NearToken,
    pub locked: NearToken,
    pub storage_usage: u64,
}
