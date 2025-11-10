use near_api_types::AccountId;

use crate::{account::Account, contract::Contract, stake::Delegation, tokens::Tokens};

/// Extension trait for AccountId that provides convenient conversions to various NEAR API types.
///
/// This trait allows you to easily convert an AccountId into different wrapper types
/// for interacting with various aspects of the NEAR Protocol.
///
/// # Example
/// ```rust,no_run
/// use near_api::*;
/// use near_api::AccountIdExt;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let account_id: AccountId = "alice.near".parse()?;
///
/// // Get Account wrapper for account operations
/// let account = account_id.account();
/// let info = account.view().fetch_from_mainnet().await?;
///
/// // Get Contract wrapper for contract calls
/// let contract = account_id.contract();
/// let result: String = contract.call_function("get_status", ())?.read_only().fetch_from_mainnet().await?.data;
///
/// // Get tokens wrapper for token operations
/// let tokens = account_id.tokens();
/// let balance = tokens.near_balance().fetch_from_mainnet().await?;
///
/// // Get delegation wrapper for staking operations
/// let delegation = account_id.delegation();
/// # Ok(())
/// # }
/// ```
pub trait AccountIdExt {
    /// Creates an Account wrapper for account-related operations.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use near_api::AccountIdExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let account_id: AccountId = "alice.near".parse()?;
    /// let account = account_id.account();
    /// let info = account.view().fetch_from_mainnet().await?;
    /// println!("Account balance: {}", info.data.amount);
    /// # Ok(())
    /// # }
    /// ```
    fn account(&self) -> Account;

    /// Creates a Contract wrapper for contract-related operations.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use near_api::AccountIdExt;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract_id: AccountId = "contract.near".parse()?;
    /// let contract = contract_id.contract();
    /// let result: String = contract.call_function("get_value", ())?.read_only().fetch_from_mainnet().await?.data;
    /// println!("Contract value: {:?}", result);
    /// # Ok(())
    /// # }
    /// ```
    fn contract(&self) -> Contract;

    /// Creates a Tokens wrapper for token-related operations on this account.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use near_api::AccountIdExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let account_id: AccountId = "alice.near".parse()?;
    /// let tokens = account_id.tokens();
    /// let balance = tokens.near_balance().fetch_from_mainnet().await?;
    /// println!("NEAR balance: {}", balance.total);
    /// # Ok(())
    /// # }
    /// ```
    fn tokens(&self) -> Tokens;

    /// Creates a Delegation wrapper for staking-related operations on this account.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use near_api::AccountIdExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let account_id: AccountId = "alice.near".parse()?;
    /// let delegation = account_id.delegation();
    /// let staked = delegation.view_staked_balance("pool.near".parse()?)?.fetch_from_mainnet().await?;
    /// println!("Staked balance: {:?}", staked);
    /// # Ok(())
    /// # }
    /// ```
    fn delegation(&self) -> Delegation;
}

impl AccountIdExt for AccountId {
    fn account(&self) -> Account {
        Account(self.clone())
    }

    fn contract(&self) -> Contract {
        Contract(self.clone())
    }

    fn tokens(&self) -> Tokens {
        Tokens::account(self.clone())
    }

    fn delegation(&self) -> Delegation {
        Delegation(self.clone())
    }
}
