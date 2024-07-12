use near_primitives::types::AccountId;
use near_socialdb_client::StorageBalance;
use near_token::NearToken;
use serde_json::json;

use crate::{
    common::query::{CallResultHandler, QueryBuilder},
    contract::Contract,
    transactions::ConstructTransaction,
};

pub struct StorageBuilder {
    pub account_id: AccountId,
    pub contract_id: AccountId,
}

impl StorageBuilder {
    pub fn view(self) -> anyhow::Result<QueryBuilder<CallResultHandler<StorageBalance>>> {
        Ok(Contract(self.contract_id)
            .call_function(
                "storage_balance_of",
                json!({
                    "account_id": self.account_id,
                }),
            )?
            .as_read_only())
    }

    pub fn deposit(
        self,
        receiver_account_id: AccountId,
        amount: NearToken,
    ) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(self.contract_id)
            .call_function(
                "storage_deposit",
                json!({
                    "account_id": receiver_account_id.to_string(),
                }),
            )?
            .as_transaction()
            .deposit(amount)
            .with_signer_account(self.account_id))
    }

    pub fn withdraw(self, amount: NearToken) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(self.contract_id)
            .call_function(
                "storage_withdraw",
                json!({
                    "amount": amount.as_yoctonear()
                }),
            )?
            .as_transaction()
            .deposit(NearToken::from_yoctonear(1))
            .with_signer_account(self.account_id))
    }
}
