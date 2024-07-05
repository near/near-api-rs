use near_primitives::types::AccountId;
use near_token::NearToken;

use crate::query::{AccessKeyListHandler, AccountViewHandler, CallResultHandler, QueryBuilder};

pub struct Account {
    account_id: AccountId,
}

impl Account {
    pub fn new(account_id: AccountId) -> Self {
        Self { account_id }
    }

    pub fn view(&self) -> QueryBuilder<AccountViewHandler> {
        QueryBuilder::new(
            near_primitives::views::QueryRequest::ViewAccount {
                account_id: self.account_id.clone(),
            },
            Default::default(),
        )
    }

    pub fn list_keys(&self) -> QueryBuilder<AccessKeyListHandler> {
        QueryBuilder::new(
            near_primitives::views::QueryRequest::ViewAccessKeyList {
                account_id: self.account_id.clone(),
            },
            Default::default(),
        )
    }

    pub async fn delegation_in_pool(
        &self,
        pool: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<u128, NearToken>>> {
        let args = serde_json::to_vec(&serde_json::json!({
            "account_id": self.account_id.clone(),
        }))?;
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: pool,
            method_name: "get_account_staked_balance".to_owned(),
            args: near_primitives::types::FunctionArgs::from(args),
        };

        Ok(QueryBuilder::new(
            request,
            CallResultHandler::with_postprocess(|balance| NearToken::from_yoctonear(balance)),
        ))
    }

    // pub async fn delegations(&self) -> anyhow::Result<BTreeMap<AccountId, NearToken>> {
    //     let validators = if let Ok(fastnear) = self.client.fastnear() {
    //         fastnear.account_delegated_in(&self.account_id).await?
    //     } else if let Ok(staking) = self.client.stake() {
    //         staking.staking_pools().await?
    //     } else {
    //         bail!("FastNear and Staking pool factory are not set");
    //     };

    //     futures::stream::iter(validators)
    //         .map(|validator_account_id| async {
    //             let balance = self.delegation_in_pool(&validator_account_id).await?;
    //             Ok::<_, anyhow::Error>((validator_account_id, balance))
    //         })
    //         .buffer_unordered(self.client.concurrency_limit)
    //         .filter(|balance_result| {
    //             futures::future::ready(if let Ok((_, balance)) = balance_result {
    //                 !balance.is_zero()
    //             } else {
    //                 true
    //             })
    //         })
    //         .try_collect()
    //         .await
    // }
}

#[cfg(test)]
mod tests {
    use near_primitives::types::BlockReference;

    const TESTNET_ACCOUNT: &str = "yurtur.testnet";

    #[tokio::test]
    async fn load_account() {
        let account = super::Account::new(TESTNET_ACCOUNT.parse().unwrap());
        assert!(account
            .view()
            .as_of(BlockReference::latest())
            .fetch_from_testnet()
            .await
            .is_ok());
        assert!(account.list_keys().fetch_from_testnet().await.is_ok());
    }
}
