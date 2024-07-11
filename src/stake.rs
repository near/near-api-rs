use near_gas::NearGas;
use near_primitives::types::AccountId;
use near_token::NearToken;

use crate::{
    common::query::{CallResultHandler, QueryBuilder, ViewStateHandler},
    contract::Contract,
    transactions::ConstructTransaction,
};

// TODO: Would be nice to have aggregated info from staking pool. That would return staked, unstaked, total.
pub struct Delegation(pub AccountId);

impl Delegation {
    pub fn view_staked_balance(
        self,
        pool: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<u128, NearToken>>> {
        let args = serde_json::to_vec(&serde_json::json!({
            "account_id": self.0,
        }))?;
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: pool,
            method_name: "get_account_staked_balance".to_owned(),
            args: near_primitives::types::FunctionArgs::from(args),
        };

        Ok(QueryBuilder::new(
            request,
            CallResultHandler::with_postprocess(NearToken::from_yoctonear),
        ))
    }

    pub fn view_unstaked_balance(
        self,
        pool: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<u128, NearToken>>> {
        let args = serde_json::to_vec(&serde_json::json!({
            "account_id": self.0,
        }))?;
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: pool,
            method_name: "get_account_unstaked_balance".to_owned(),
            args: near_primitives::types::FunctionArgs::from(args),
        };

        Ok(QueryBuilder::new(
            request,
            CallResultHandler::with_postprocess(NearToken::from_yoctonear),
        ))
    }

    pub fn view_total_balance(
        self,
        pool: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<u128, NearToken>>> {
        let args = serde_json::to_vec(&serde_json::json!({
            "account_id": self.0,
        }))?;
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: pool,
            method_name: "get_account_total_balance".to_owned(),
            args: near_primitives::types::FunctionArgs::from(args),
        };

        Ok(QueryBuilder::new(
            request,
            CallResultHandler::with_postprocess(NearToken::from_yoctonear),
        ))
    }

    pub fn is_account_unstaked_balance_available_for_withdrawal(
        self,
        pool: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<bool, bool>>> {
        let args = serde_json::to_vec(&serde_json::json!({
            "account_id": self.0,
        }))?;

        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: pool,
            method_name: "is_account_unstaked_balance_available".to_owned(),
            args: near_primitives::types::FunctionArgs::from(args),
        };

        Ok(QueryBuilder::new(request, CallResultHandler::default()))
    }

    pub fn deposit(
        self,
        pool: AccountId,
        amount: NearToken,
    ) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("deposit", ())?
            .as_transaction()
            .gas(NearGas::from_tgas(50))
            .deposit(amount)
            .with_signer_account(self.0))
    }

    pub fn deposit_and_stake(
        self,
        pool: AccountId,
        amount: NearToken,
    ) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("deposit_and_stake", ())?
            .as_transaction()
            .gas(NearGas::from_tgas(50))
            .deposit(amount)
            .with_signer_account(self.0))
    }

    pub fn stake(self, pool: AccountId, amount: NearToken) -> anyhow::Result<ConstructTransaction> {
        let args = serde_json::json!({
            "amount": amount.as_yoctonear(),
        });

        Ok(Contract(pool)
            .call_function("stake", args)?
            .as_transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0))
    }

    pub fn stake_all(self, pool: AccountId) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("stake_all", ())?
            .as_transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0))
    }

    pub fn unstake(
        self,
        pool: AccountId,
        amount: NearToken,
    ) -> anyhow::Result<ConstructTransaction> {
        let args = serde_json::json!({
            "amount": amount.as_yoctonear(),
        });

        Ok(Contract(pool)
            .call_function("unstake", args)?
            .as_transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0))
    }

    pub fn unstake_all(self, pool: AccountId) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("unstake_all", ())?
            .as_transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0))
    }

    pub fn withdraw(
        self,
        pool: AccountId,
        amount: NearToken,
    ) -> anyhow::Result<ConstructTransaction> {
        let args = serde_json::json!({
            "amount": amount.as_yoctonear(),
        });

        Ok(Contract(pool)
            .call_function("withdraw", args)?
            .as_transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0))
    }

    pub fn withdraw_all(self, pool: AccountId) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("withdraw_all", ())?
            .as_transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0))
    }
}

pub struct Staking {}

impl Staking {
    pub fn staking_pools(
        &self,
        factory: AccountId,
    ) -> QueryBuilder<ViewStateHandler<std::collections::BTreeSet<AccountId>>> {
        let request = near_primitives::views::QueryRequest::ViewState {
            account_id: factory,
            prefix: near_primitives::types::StoreKey::from(b"se".to_vec()),
            include_proof: false,
        };

        QueryBuilder::new(
            request,
            ViewStateHandler::with_postprocess(|query_result| {
                query_result
                    .values
                    .into_iter()
                    .flat_map(|item| String::from_utf8(item.value.into()))
                    .flat_map(|result| result.parse())
                    .collect()
            }),
        )
    }

    pub fn delegation(account_id: AccountId) -> Delegation {
        Delegation(account_id)
    }
}

#[cfg(test)]
mod tests {

    use crate::config::NetworkConfig;

    #[tokio::test]
    async fn get_pools() {
        let staking = super::Staking {};
        let pools = staking
            .staking_pools(
                NetworkConfig::mainnet()
                    .staking_pools_factory_account_id
                    .unwrap(),
            )
            .fetch_from_mainnet()
            .await
            .unwrap();

        for pool in pools.data.iter() {
            println!("{}", pool);
        }
    }
}
