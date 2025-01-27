use near_token::NearToken;
use serde::{Deserialize, Serialize};

use crate::errors::DecimalNumberParsingError;

/// Static instance of [FTBalance] for USDT token with correct decimals and symbol.
pub const USDT_BALANCE: FTBalance = FTBalance::with_decimals_and_symbol(6, "USDT");
/// Static instance of [FTBalance] for USDC token with correct decimals and symbol.
pub const USDC_BALANCE: FTBalance = FTBalance::with_decimals_and_symbol(6, "USDC");
/// Static instance of [FTBalance] for wNEAR token with correct decimals and symbol.
pub const W_NEAR_BALANCE: FTBalance = FTBalance::with_decimals_and_symbol(24, "wNEAR");

/// A helper type that represents the fungible token balance with a given precision.
///
/// The type is created to simplify the usage of fungible tokens in similar way as the [NearToken] type does.
///
/// The symbol is used only for display purposes.
///
/// # Examples
///
/// ## Defining 2.5 USDT
/// ```rust
/// use near_api::USDT_BALANCE;
///
/// let usdt_balance = USDT_BALANCE.with_float_str("2.5").unwrap();
///
/// assert_eq!(usdt_balance.amount(), 2_500_000);
/// ```
///
/// ## Defining 3 wNEAR using yoctoNEAR
/// ```rust
/// use near_api::{W_NEAR_BALANCE, NearToken};
///
/// let wnear_balance = W_NEAR_BALANCE.with_amount(3 * 10u128.pow(24));
///
/// assert_eq!(wnear_balance.amount(), NearToken::from_near(3).as_yoctonear());
/// ```
///
/// ## Defining 3 wETH using 18 decimals
/// ```rust
/// use near_api::FTBalance;
///
/// let weth_balance = FTBalance::with_decimals_and_symbol(18, "wETH").with_whole_amount(3);
/// ```
#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct FTBalance {
    balance: u128,
    decimals: u8,
    symbol: &'static str,
}

impl FTBalance {
    /// Creates a new [FTBalance] with a given precision.
    ///
    /// The balance is initialized to 0.
    pub const fn with_decimals(decimals: u8) -> Self {
        Self {
            balance: 0,
            decimals,
            symbol: "FT",
        }
    }

    /// Creates a new [FTBalance] with a given precision and symbol.
    ///
    /// The balance is initialized to 0.
    pub const fn with_decimals_and_symbol(decimals: u8, symbol: &'static str) -> Self {
        Self {
            balance: 0,
            decimals,
            symbol,
        }
    }

    /// Stores the given amount without any transformations.
    ///
    /// The [NearToken] alternative is [NearToken::from_yoctonear].
    pub const fn with_amount(&self, amount: u128) -> Self {
        Self {
            balance: amount,
            decimals: self.decimals,
            symbol: self.symbol,
        }
    }

    /// Stores the number as an integer token value utilizing the given precision.
    ///
    /// The [NearToken] alternative is [NearToken::from_near].
    pub const fn with_whole_amount(&self, amount: u128) -> Self {
        Self {
            balance: amount * 10u128.pow(self.decimals as u32),
            decimals: self.decimals,
            symbol: self.symbol,
        }
    }

    /// Parses float string and stores the value in defined precision.
    ///
    /// # Examples
    ///
    /// ## Defining 2.5 USDT
    /// ```rust
    /// use near_api::USDT_BALANCE;
    ///
    /// let usdt_balance = USDT_BALANCE.with_float_str("2.515").unwrap();
    ///
    /// assert_eq!(usdt_balance.amount(), 2_515_000);
    /// ```
    pub fn with_float_str(&self, float_str: &str) -> Result<Self, DecimalNumberParsingError> {
        crate::common::utils::parse_decimal_number(float_str, 10u128.pow(self.decimals as u32))
            .map(|amount| self.with_amount(amount))
    }

    /// Returns the amount without any transformations.
    ///
    /// The [NearToken] alternative is [NearToken::as_yoctonear].
    pub const fn amount(&self) -> u128 {
        self.balance
    }

    /// Returns the amount as a whole number in the defined precision.
    ///
    /// The [NearToken] alternative is [NearToken::as_near].
    pub const fn to_whole(&self) -> u128 {
        self.balance / 10u128.pow(self.decimals as u32)
    }

    /// Returns the number of decimals used by the token.
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

/// Account balance on the NEAR blockchain.
///
/// This balance doesn't include staked NEAR tokens or storage
#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct UserBalance {
    /// The total amount of NEAR tokens in the account.
    ///
    /// Please note that this is the total amount of NEAR tokens in the account, not the amount available for use.
    pub total: NearToken,
    /// The amount of NEAR tokens locked in the account for storage usage.
    pub storage_locked: NearToken,
    /// The storage usage by the account in bytes.
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
