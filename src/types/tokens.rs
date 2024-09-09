use near_token::NearToken;
use serde::{Deserialize, Serialize};

use crate::errors::DecimalNumberParsingError;

pub const USDT_BALANCE: FTBalance = FTBalance::with_decimals_and_symbol(4, "USDT");
pub const W_NEAR_BALANCE: FTBalance = FTBalance::with_decimals_and_symbol(24, "wNEAR");

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct FTBalance {
    balance: u128,
    decimals: u8,
    symbol: &'static str,
}

impl FTBalance {
    pub const fn with_decimals(decimals: u8) -> Self {
        Self {
            balance: 0,
            decimals,
            symbol: "FT",
        }
    }

    pub const fn with_decimals_and_symbol(decimals: u8, symbol: &'static str) -> Self {
        Self {
            balance: 0,
            decimals,
            symbol,
        }
    }

    pub const fn with_amount(&self, amount: u128) -> Self {
        Self {
            balance: amount,
            decimals: self.decimals,
            symbol: self.symbol,
        }
    }

    pub const fn with_whole_amount(&self, amount: u128) -> Self {
        Self {
            balance: amount * 10u128.pow(self.decimals as u32),
            decimals: self.decimals,
            symbol: self.symbol,
        }
    }

    pub fn with_float_str(&self, float_str: &str) -> Result<Self, DecimalNumberParsingError> {
        crate::common::utils::parse_decimal_number(float_str, 10u128.pow(self.decimals as u32))
            .map(|amount| self.with_amount(amount))
    }

    pub const fn amount(&self) -> u128 {
        self.balance
    }

    pub const fn to_whole(&self) -> u128 {
        self.balance / 10u128.pow(self.decimals as u32)
    }

    pub const fn decimals(&self) -> u8 {
        self.decimals
    }
}

impl std::fmt::Display for FTBalance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let whole_part = self.to_whole();
        let fractional_part = self.balance % 10u128.pow(self.decimals as u32);

        let fractional_part_str = format!(
            "{:0width$}",
            fractional_part,
            width = self.decimals as usize
        );
        let fractional_part_str = fractional_part_str.trim_end_matches('0');

        if fractional_part_str.is_empty() {
            return write!(f, "{} {}", whole_part, self.symbol);
        }

        write!(f, "{}.{} {}", whole_part, fractional_part_str, self.symbol)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct UserBalance {
    pub liquid: NearToken,
    pub locked: NearToken,
    pub storage_usage: u64,
}

#[cfg(test)]
mod tests {
    use super::FTBalance;

    #[test]
    fn ft_balance_default() {
        assert_eq!(
            FTBalance::with_decimals(5).with_whole_amount(5).amount(),
            500000
        );
        assert_eq!(FTBalance::with_decimals(5).with_amount(5).amount(), 5);

        assert_eq!(
            FTBalance::with_decimals(5).with_whole_amount(5).to_whole(),
            5
        );
    }

    #[test]
    fn ft_balance_str() {
        assert_eq!(
            FTBalance::with_decimals(5)
                .with_float_str("5")
                .unwrap()
                .amount(),
            500000
        );
        assert_eq!(
            FTBalance::with_decimals(5)
                .with_float_str("5.00001")
                .unwrap()
                .amount(),
            500001
        );
        assert_eq!(
            FTBalance::with_decimals(5)
                .with_float_str("5.55")
                .unwrap()
                .amount(),
            555000
        );
    }
}
