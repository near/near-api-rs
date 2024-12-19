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

/// A wrapper struct that simplifies interactions with NEAR storage management standard.
///
/// Contracts on NEAR Protocol often implement a [standard interface](https://nomicon.io/Standards/StorageManagement) for managing storage deposits,
/// which are required for storing data on the blockchain. This struct provides convenient methods
/// to interact with these storage-related functions.
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
