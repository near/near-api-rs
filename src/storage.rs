use near_primitives::types::AccountId;
use near_socialdb_client::StorageBalance;
use near_token::NearToken;
use serde_json::json;

use crate::{
    common::query::{CallResultHandler, QueryBuilder},
    contract::{Contract, ContractTransactBuilder},
    errors::BuilderError,
    transactions::ConstructTransaction,
};

#[derive(Clone, Debug)]
pub struct StorageDeposit(AccountId);

impl StorageDeposit {
    pub fn on_contract(contract_id: AccountId) -> Self {
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
