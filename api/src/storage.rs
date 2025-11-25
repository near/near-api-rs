use std::sync::Arc;

use near_api_types::{AccountId, Data, NearToken, StorageBalance, StorageBalanceInternal};
use serde_json::json;

use crate::{
    common::query::{CallResultHandler, PostprocessHandler, RequestBuilder},
    contract::ContractTransactBuilder,
    errors::BuilderError,
    transactions::ConstructTransaction,
    Signer,
};

///A wrapper struct that simplifies interactions with the [Storage Management](https://github.com/near/NEPs/blob/master/neps/nep-0145.md) standard
///
/// Contracts on NEAR Protocol often implement a [NEP-145](https://github.com/near/NEPs/blob/master/neps/nep-0145.md) for managing storage deposits,
/// which are required for storing data on the blockchain. This struct provides convenient methods
/// to interact with these storage-related functions on the contract.
///
/// # Example
/// ```
/// use near_api::*;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = StorageDeposit::on_contract("contract.testnet".parse()?);
///
/// // Check storage balance
/// let balance = storage.view_account_storage("alice.testnet".parse()?)?.fetch_from_testnet().await?;
/// println!("Storage balance: {:?}", balance);
///
/// // Bob pays for Alice's storage on the contract contract.testnet
/// let deposit_tx = storage.deposit("alice.testnet".parse()?, NearToken::from_near(1))
///     .with_signer("bob.testnet".parse()?, Signer::new(Signer::from_ledger())?)?
///     .send_to_testnet()
///     .await
///     .unwrap();
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct StorageDeposit(crate::Contract);

impl StorageDeposit {
    pub const fn on_contract(contract_id: AccountId) -> Self {
        Self(crate::Contract(contract_id))
    }

    /// Returns the underlying contract account ID for this storage deposit wrapper.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = StorageDeposit::on_contract("contract.testnet".parse()?);
    /// let contract_id = storage.contract_id();
    /// println!("Contract ID: {}", contract_id);
    /// # Ok(())
    /// # }
    /// ```
    pub const fn contract_id(&self) -> &AccountId {
        self.0.account_id()
    }

    /// Converts this storage deposit wrapper to a Contract for other contract operations.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = StorageDeposit::on_contract("usdt.tether-token.near".parse()?);
    /// let contract = storage.as_contract();
    ///
    /// // Now you can call other contract methods
    /// let metadata: serde_json::Value = contract.call_function("ft_metadata", ())?.read_only().fetch_from_mainnet().await?.data;
    /// println!("Token metadata: {:?}", metadata);
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_contract(&self) -> crate::contract::Contract {
        self.0.clone()
    }

    /// Prepares a new contract query (`storage_balance_of`) for fetching the storage balance (Option<[StorageBalance]>) of the account on the contract.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let balance = StorageDeposit::on_contract("contract.testnet".parse()?)
    ///     .view_account_storage("alice.testnet".parse()?)?
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Storage balance: {:?}", balance);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn view_account_storage(
        &self,
        account_id: AccountId,
    ) -> Result<
        RequestBuilder<
            PostprocessHandler<
                Data<Option<StorageBalance>>,
                CallResultHandler<Option<StorageBalanceInternal>>,
            >,
        >,
        BuilderError,
    > {
        Ok(self
            .0
            .call_function(
                "storage_balance_of",
                json!({
                    "account_id": account_id,
                }),
            )?
            .read_only()
            .map(|storage: Data<Option<StorageBalanceInternal>>| {
                storage.map(|option_storage| {
                    option_storage.map(|data| StorageBalance {
                        available: data.available,
                        total: data.total,
                        locked: NearToken::from_yoctonear(
                            data.total.as_yoctonear() - data.available.as_yoctonear(),
                        ),
                    })
                })
            }))
    }

    /// Prepares a new transaction contract call (`storage_deposit`) for depositing storage on the contract.
    ///
    /// Returns a [`StorageDepositBuilder`] that allows configuring the deposit behavior
    /// with [`registration_only()`](StorageDepositBuilder::registration_only).
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Basic deposit for another account
    /// let tx = StorageDeposit::on_contract("contract.testnet".parse()?)
    ///     .deposit("alice.testnet".parse()?, NearToken::from_near(1))
    ///     .with_signer("bob.testnet".parse()?, Signer::new(Signer::from_ledger())?)?
    ///     .send_to_testnet()
    ///     .await?;
    ///
    /// // Registration-only deposit (refunds excess above minimum)
    /// let tx = StorageDeposit::on_contract("contract.testnet".parse()?)
    ///     .deposit("alice.testnet".parse()?, NearToken::from_near(1))
    ///     .registration_only()
    ///     .with_signer("bob.testnet".parse()?, Signer::new(Signer::from_ledger())?)?
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn deposit(
        &self,
        receiver_account_id: AccountId,
        amount: NearToken,
    ) -> StorageDepositBuilder {
        StorageDepositBuilder {
            contract: self.0.clone(),
            account_id: receiver_account_id,
            amount,
            registration_only: false,
        }
    }

    /// Prepares a new transaction contract call (`storage_withdraw`) for withdrawing storage from the contract.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tx = StorageDeposit::on_contract("contract.testnet".parse()?)
    ///     .withdraw("alice.testnet".parse()?, NearToken::from_near(1))?
    ///     .with_signer( Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn withdraw(
        &self,
        account_id: AccountId,
        amount: NearToken,
    ) -> Result<ConstructTransaction, BuilderError> {
        Ok(self
            .0
            .call_function(
                "storage_withdraw",
                json!({
                    "amount": amount
                }),
            )?
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .with_signer_account(account_id))
    }

    /// Prepares a new transaction contract call (`storage_unregister`) for unregistering
    /// the predecessor account and returning the storage NEAR deposit.
    ///
    /// If the predecessor account is not registered, the function returns `false` without panic.
    ///
    /// By default, the contract will panic if the caller has existing account data (such as
    /// a positive token balance). Use [`force()`](StorageUnregisterBuilder::force) to ignore
    /// existing account data and force unregistering (which may burn token balances).
    ///
    /// **Note:** Requires exactly 1 yoctoNEAR attached for security purposes.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Normal unregister (fails if account has data like token balance)
    /// let tx = StorageDeposit::on_contract("contract.testnet".parse()?)
    ///     .unregister()
    ///     .with_signer("alice.testnet".parse()?, Signer::new(Signer::from_ledger())?)?
    ///     .send_to_testnet()
    ///     .await?;
    ///
    /// // Force unregister (burns any remaining token balance)
    /// let tx = StorageDeposit::on_contract("contract.testnet".parse()?)
    ///     .unregister()
    ///     .force()
    ///     .with_signer("alice.testnet".parse()?, Signer::new(Signer::from_ledger())?)?
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unregister(&self) -> StorageUnregisterBuilder {
        StorageUnregisterBuilder {
            contract: self.0.clone(),
            force: false,
        }
    }
}

/// Builder for configuring a `storage_deposit` transaction.
///
/// Created by [`StorageDeposit::deposit`].
#[derive(Clone, Debug)]
pub struct StorageDepositBuilder {
    contract: crate::Contract,
    account_id: AccountId,
    amount: NearToken,
    registration_only: bool,
}

impl StorageDepositBuilder {
    /// Sets `registration_only=true` for the deposit.
    ///
    /// When enabled, the contract will refund any deposit above the minimum balance
    /// if the account wasn't registered, and refund the full deposit if already registered.
    pub const fn registration_only(mut self) -> Self {
        self.registration_only = true;
        self
    }

    /// Builds and returns the transaction builder for this storage deposit.
    pub fn into_transaction(self) -> Result<ContractTransactBuilder, BuilderError> {
        let args = if self.registration_only {
            json!({
                "account_id": self.account_id.to_string(),
                "registration_only": true,
            })
        } else {
            json!({
                "account_id": self.account_id.to_string(),
            })
        };

        Ok(self
            .contract
            .call_function("storage_deposit", args)?
            .transaction()
            .deposit(self.amount))
    }

    /// Adds a signer to the transaction.
    ///
    /// This is a convenience method that calls `into_transaction()` and then `with_signer()`.
    pub fn with_signer(
        self,
        signer_id: AccountId,
        signer: Arc<Signer>,
    ) -> Result<crate::common::send::ExecuteSignedTransaction, BuilderError> {
        Ok(self.into_transaction()?.with_signer(signer_id, signer))
    }
}

/// Builder for configuring a `storage_unregister` transaction.
///
/// Created by [`StorageDeposit::unregister`].
#[derive(Clone, Debug)]
pub struct StorageUnregisterBuilder {
    contract: crate::Contract,
    force: bool,
}

impl StorageUnregisterBuilder {
    /// Sets `force=true` for the unregistering.
    ///
    /// When enabled, the contract will ignore existing account data (such as non-zero
    /// token balances) and close the account anyway, potentially burning those balances.
    ///
    /// **Warning:** This may result in permanent loss of tokens or other account data.
    pub const fn force(mut self) -> Self {
        self.force = true;
        self
    }

    /// Builds and returns the transaction builder for this storage unregister.
    pub fn into_transaction(self) -> Result<ContractTransactBuilder, BuilderError> {
        let args = if self.force {
            json!({ "force": true })
        } else {
            json!({})
        };

        Ok(self
            .contract
            .call_function("storage_unregister", args)?
            .transaction()
            .deposit(NearToken::from_yoctonear(1)))
    }

    /// Adds a signer to the transaction.
    ///
    /// This is a convenience method that calls `into_transaction()` and then `with_signer()`.
    pub fn with_signer(
        self,
        signer_id: AccountId,
        signer: Arc<Signer>,
    ) -> Result<crate::common::send::ExecuteSignedTransaction, BuilderError> {
        Ok(self.into_transaction()?.with_signer(signer_id, signer))
    }
}
