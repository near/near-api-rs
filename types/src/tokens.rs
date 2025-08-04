use near_token::NearToken;
use serde::{Deserialize, Serialize};

use crate::errors::DecimalNumberParsingError;

/// Static instance of [FTBalance] for USDT token with correct decimals and symbol.
pub const USDT_BALANCE: FTBalance = FTBalance::with_decimals_and_symbol(6, "USDT");
/// Static instance of [FTBalance] for USDC token with correct decimals and symbol.
pub const USDC_BALANCE: FTBalance = FTBalance::with_decimals_and_symbol(6, "USDC");
/// Static instance of [FTBalance] for wNEAR token with correct decimals and symbol.
pub const W_NEAR_BALANCE: FTBalance = FTBalance::with_decimals_and_symbol(24, "wNEAR");

/// The cost of storage per byte in NEAR.
pub const STORAGE_COST_PER_BYTE: NearToken = NearToken::from_yoctonear(10u128.pow(19));

/// A helper type that represents the fungible token balance with a given precision.
///
/// The type is created to simplify the usage of fungible tokens in similar way as the [NearToken] type does.
///
/// The symbol is used only for display purposes.
///
/// The type has static instances for some of the most popular tokens with correct decimals and symbol.
/// * [USDT_BALANCE] - USDT token with 6 decimals
/// * [USDC_BALANCE] - USDC token with 6 decimals
/// * [W_NEAR_BALANCE] - wNEAR token with 24 decimals
///
/// # Examples
///
/// ## Defining 2.5 USDT
/// ```rust
/// use near_api_types::tokens::FTBalance;
///
/// let usdt_balance = FTBalance::with_decimals(6).with_float_str("2.5").unwrap();
///
/// assert_eq!(usdt_balance.amount(), 2_500_000);
/// ```
///
/// ## Defining 3 USDT using smaller precision
/// ```rust
/// use near_api_types::tokens::FTBalance;
///
/// let usdt = FTBalance::with_decimals(6);
///
/// let usdt_balance = usdt.with_amount(3 * 10u128.pow(6));
///
/// assert_eq!(usdt_balance, usdt.with_whole_amount(3));
/// ```
///
/// ## Defining 3 wETH using 18 decimals
/// ```rust
/// use near_api_types::tokens::FTBalance;
///
/// let weth = FTBalance::with_decimals_and_symbol(18, "wETH");
/// let weth_balance = weth.with_whole_amount(3);
///
/// assert_eq!(weth_balance, weth.with_amount(3 * 10u128.pow(18)));
/// ```
#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct FTBalance {
    amount: u128,
    decimals: u8,
    symbol: &'static str,
}

impl FTBalance {
    /// Creates a new [FTBalance] with a given precision.
    ///
    /// The balance is initialized to 0.
    pub const fn with_decimals(decimals: u8) -> Self {
        Self {
            amount: 0,
            decimals,
            symbol: "FT",
        }
    }

    /// Creates a new [FTBalance] with a given precision and symbol.
    ///
    /// The balance is initialized to 0.
    pub const fn with_decimals_and_symbol(decimals: u8, symbol: &'static str) -> Self {
        Self {
            amount: 0,
            decimals,
            symbol,
        }
    }

    /// Stores the given amount without any transformations.
    ///
    /// The [NearToken] equivalent to this method is [NearToken::from_yoctonear].
    ///
    /// ## Example
    /// ```rust
    /// use near_api_types::tokens::FTBalance;
    ///
    /// let usdt_balance = FTBalance::with_decimals(6).with_amount(2_500_000);
    /// assert_eq!(usdt_balance.amount(), 2_500_000);
    /// assert_eq!(usdt_balance.to_whole(), 2);
    /// ```
    pub const fn with_amount(&self, amount: u128) -> Self {
        Self {
            amount,
            decimals: self.decimals,
            symbol: self.symbol,
        }
    }

    /// Stores the number as an integer token value utilizing the given precision.
    ///
    /// The [NearToken] equivalent to this method is [NearToken::from_near].
    ///
    /// ## Example
    /// ```rust
    /// use near_api_types::tokens::FTBalance;
    ///
    /// let usdt_balance = FTBalance::with_decimals(6).with_whole_amount(3);
    /// assert_eq!(usdt_balance.amount(), 3 * 10u128.pow(6));
    /// assert_eq!(usdt_balance.to_whole(), 3);
    /// ```
    pub const fn with_whole_amount(&self, amount: u128) -> Self {
        Self {
            amount: amount * 10u128.pow(self.decimals as u32),
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
    /// use near_api_types::tokens::FTBalance;
    ///
    /// let usdt_balance = FTBalance::with_decimals(6).with_float_str("2.515").unwrap();
    ///
    /// assert_eq!(usdt_balance.amount(), 2_515_000);
    /// ```
    pub fn with_float_str(&self, float_str: &str) -> Result<Self, DecimalNumberParsingError> {
        parse_decimal_number(float_str, 10u128.pow(self.decimals as u32))
            .map(|amount| self.with_amount(amount))
    }

    /// Returns the amount without any transformations.
    ///
    /// The [NearToken] equivalent to this method is [NearToken::as_yoctonear].
    pub const fn amount(&self) -> u128 {
        self.amount
    }

    /// Returns the amount as a whole number in the integer precision.
    ///
    /// The method rounds down the fractional part, so 2.5 USDT will be 2.
    ///
    /// The [NearToken] equivalent to this method is [NearToken::as_near].
    pub const fn to_whole(&self) -> u128 {
        self.amount / 10u128.pow(self.decimals as u32)
    }

    /// Returns the number of decimals used by the token.
    pub const fn decimals(&self) -> u8 {
        self.decimals
    }
}

impl PartialOrd for FTBalance {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.decimals != other.decimals || self.symbol != other.symbol {
            return None;
        }

        Some(self.amount.cmp(&other.amount))
    }
}

impl std::fmt::Display for FTBalance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let whole_part = self.to_whole();
        let fractional_part = self.amount % 10u128.pow(self.decimals as u32);

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
    ///
    /// The storage lock equal to [Self::storage_usage] * [STORAGE_COST_PER_BYTE]
    pub storage_locked: NearToken,
    /// The storage usage by the account in bytes.
    pub storage_usage: u64,
    /// The amount of NEAR tokens staked on a protocol level.
    /// Applicable for staking pools only in 99.99% of the cases.
    ///
    /// The PoS allows particular users to stake funds to become a validator, but the protocol itself
    /// doesn't allow other users to delegate tokens to the validator.
    /// This is why, the [NEP-27](https://github.com/near/core-contracts/tree/master/staking-pool) defines a Staking Pool smart contract
    /// that allows other users to delegate tokens to the validator.
    ///
    /// Even though, the user can stake and become validator itself, it's highly unlikely and this field will be 0
    /// for almost all the users, and not 0 for StakingPool contracts.
    ///
    /// Please note that this is not related to your delegations into the staking pools.
    /// To get your delegation information in the staking pools, use [near-api::Delegation]
    pub locked: NearToken,
}

/// Parsing decimal numbers from `&str` type in `u128`.
/// Function also takes a value of metric prefix in u128 type.
/// `parse_str` use the `u128` type, and have the same max and min values.
///
/// If the fractional part is longer than several zeros in the prefix, it will return the error `DecimalNumberParsingError::LongFractional`.
///
/// If the string slice has invalid chars, it will return the error `DecimalNumberParsingError::InvalidNumber`.
///
/// If the whole part of the number has a value more than the `u64` maximum value, it will return the error `DecimalNumberParsingError::LongWhole`.
fn parse_decimal_number(s: &str, pref_const: u128) -> Result<u128, DecimalNumberParsingError> {
    let (int, fraction) = if let Some((whole, fractional)) = s.trim().split_once('.') {
        let int: u128 = whole
            .parse()
            .map_err(|_| DecimalNumberParsingError::InvalidNumber(s.to_owned()))?;
        let mut fraction: u128 = fractional
            .parse()
            .map_err(|_| DecimalNumberParsingError::InvalidNumber(s.to_owned()))?;
        let len = u32::try_from(fractional.len())
            .map_err(|_| DecimalNumberParsingError::InvalidNumber(s.to_owned()))?;
        fraction = fraction
            .checked_mul(
                pref_const
                    .checked_div(10u128.checked_pow(len).ok_or_else(|| {
                        DecimalNumberParsingError::LongFractional(fractional.to_owned())
                    })?)
                    .filter(|n| *n != 0u128)
                    .ok_or_else(|| {
                        DecimalNumberParsingError::LongFractional(fractional.to_owned())
                    })?,
            )
            .ok_or_else(|| DecimalNumberParsingError::LongFractional(fractional.to_owned()))?;
        (int, fraction)
    } else {
        let int: u128 = s
            .parse()
            .map_err(|_| DecimalNumberParsingError::InvalidNumber(s.to_owned()))?;
        (int, 0)
    };
    let result = fraction
        .checked_add(
            int.checked_mul(pref_const)
                .ok_or_else(|| DecimalNumberParsingError::LongWhole(int.to_string()))?,
        )
        .ok_or_else(|| DecimalNumberParsingError::LongWhole(int.to_string()))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::{errors::DecimalNumberParsingError, tokens::parse_decimal_number};

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

    const TEST: [(u128, &str, u128); 6] = [
        (129_380_000_001_u128, "129.380000001", 10u128.pow(9)),
        (
            12_938_000_000_100_000_000_u128,
            "12938000000.1",
            10u128.pow(9),
        ),
        (129_380_000_001_u128, "0.129380000001", 10u128.pow(12)),
        (129_380_000_001_000_u128, "129.380000001000", 10u128.pow(12)),
        (
            9_488_129_380_000_001_u128,
            "9488.129380000001",
            10u128.pow(12),
        ),
        (129_380_000_001_u128, "00.129380000001", 10u128.pow(12)),
    ];

    #[test]
    fn parse_test() {
        for (expected_value, str_value, precision) in TEST {
            let parsed_value = parse_decimal_number(str_value, precision).unwrap();
            assert_eq!(parsed_value, expected_value)
        }
    }

    #[test]
    fn test_long_fraction() {
        let data = "1.23456";
        let prefix = 10000u128;
        assert_eq!(
            parse_decimal_number(data, prefix),
            Err(DecimalNumberParsingError::LongFractional(23456.to_string()))
        );
    }

    #[test]
    fn invalid_number_whole() {
        let num = "1h4.7859";
        let prefix: u128 = 10000;
        assert_eq!(
            parse_decimal_number(num, prefix),
            Err(DecimalNumberParsingError::InvalidNumber(
                "1h4.7859".to_owned()
            ))
        );
    }
    #[test]
    fn invalid_number_fraction() {
        let num = "14.785h9";
        let prefix: u128 = 10000;
        assert_eq!(
            parse_decimal_number(num, prefix),
            Err(DecimalNumberParsingError::InvalidNumber(
                "14.785h9".to_owned()
            ))
        );
    }

    #[test]
    fn max_long_fraction() {
        let max_data = 10u128.pow(17) + 1;
        let data = "1.".to_string() + max_data.to_string().as_str();
        let prefix = 10u128.pow(17);
        assert_eq!(
            parse_decimal_number(data.as_str(), prefix),
            Err(DecimalNumberParsingError::LongFractional(
                max_data.to_string()
            ))
        );
    }

    #[test]
    fn parse_u128_error_test() {
        let test_data = u128::MAX.to_string();
        let gas = parse_decimal_number(&test_data, 10u128.pow(9));
        assert_eq!(
            gas,
            Err(DecimalNumberParsingError::LongWhole(u128::MAX.to_string()))
        );
    }

    #[test]
    fn test() {
        let data = "1.000000000000000000000000000000000000001";
        let prefix = 100u128;
        assert_eq!(
            parse_decimal_number(data, prefix),
            Err(DecimalNumberParsingError::LongFractional(
                "000000000000000000000000000000000000001".to_string()
            ))
        );
    }
}
