use near_primitives::types::AccountId;
use near_token::NearToken;
use serde_json::json;

use crate::{
    common::query::{CallResultHandler, QueryBuilder},
    contract::{Contract, ContractTransactBuilder},
    errors::BuilderError,
    transactions::ConstructTransaction,
    types::storage::StorageBalance,
};

///A wrapper struct that simplifies interactions with the [Storage Management](https://nomicon.io/Standards/StorageManagement) standard
///
/// Contracts on NEAR Protocol often implement a [NEP-145](https://nomicon.io/Standards/StorageManagement) for managing storage deposits,
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
/// let deposit_tx = storage.deposit("alice.testnet".parse()?, NearToken::from_near(1))?
///     .with_signer("bob.testnet".parse()?, Signer::new(Signer::from_ledger())?)
///     .send_to_testnet()
///     .await
///     .unwrap();
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct StorageDeposit(AccountId);

impl StorageDeposit {
    pub const fn on_contract(contract_id: AccountId) -> Self {
        Self(contract_id)
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
    pub fn view_account_storage(
        &self,
        account_id: AccountId,
    ) -> Result<QueryBuilder<CallResultHandler<Option<StorageBalance>>>, BuilderError> {
        Ok(Contract(self.0.clone())
            .call_function(
                "storage_balance_of",
                json!({
                "account_id": account_id,
                }),
            )?
            .read_only())
    }

    /// Prepares a new transaction contract call (`storage_deposit`) for depositing storage on the contract.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tx: near_primitives::views::FinalExecutionOutcomeView = StorageDeposit::on_contract("contract.testnet".parse()?)
    ///     .deposit("alice.testnet".parse()?, NearToken::from_near(1))?
    ///     .with_signer("bob.testnet".parse()?, Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn deposit(
        &self,
        receiver_account_id: AccountId,
        amount: NearToken,
    ) -> Result<ContractTransactBuilder, BuilderError> {
        Ok(Contract(self.0.clone())
            .call_function(
                "storage_deposit",
                json!({
                    "account_id": receiver_account_id.to_string(),
                }),
            )?
            .transaction()
            .deposit(amount))
    }

    /// Prepares a new transaction contract call (`storage_withdraw`) for withdrawing storage from the contract.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tx: near_primitives::views::FinalExecutionOutcomeView = StorageDeposit::on_contract("contract.testnet".parse()?)
    ///     .withdraw("alice.testnet".parse()?, NearToken::from_near(1))?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
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
        Ok(Contract(self.0.clone())
            .call_function(
                "storage_withdraw",
                json!({
                    "amount": amount.as_yoctonear()
                }),
            )?
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .with_signer_account(account_id))
    }
}
