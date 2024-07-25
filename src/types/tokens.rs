use near_token::NearToken;
use serde::{Deserialize, Serialize};

pub const USDT_BALANCE: FTBalance = FTBalance::with_decimals(4);
pub const W_NEAR_BALANCE: FTBalance = FTBalance::with_decimals(24);

#[derive(Debug, Copy, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct FTBalance {
    balance: u128,
    decimals: u8,
}

impl FTBalance {
    pub const fn with_decimals(decimals: u8) -> Self {
        Self {
            balance: 0,
            decimals,
        }
    }

    pub fn with_amount(&self, amount: u128) -> Self {
        Self {
            balance: amount,
            decimals: self.decimals,
        }
    }

    pub fn with_whole_amount(&self, amount: u128) -> Self {
        Self {
            balance: amount * 10u128.pow(self.decimals as u32),
            decimals: self.decimals,
        }
    }

    pub fn with_scaled_amount(&self, amount: u128, scale: u8) -> Self {
        let balance = if scale > self.decimals {
            amount / 10u128.pow((scale - self.decimals) as u32)
        } else {
            amount * 10u128.pow((self.decimals - scale) as u32)
        };
        Self {
            balance,
            decimals: self.decimals,
        }
    }

    pub fn amount(&self) -> u128 {
        self.balance
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
