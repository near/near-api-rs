use near_crypto::PublicKey;
use near_primitives::{
    account::{AccessKey, AccessKeyPermission},
    action::{AddKeyAction, DeleteKeyAction},
    types::AccountId,
};
use near_token::NearToken;

use crate::{
    query::{
        AccessKeyHandler, AccessKeyListHandler, AccountViewHandler, CallResultHandler, QueryBuilder,
    },
    transactions::ConstructTransaction,
};

pub struct Account(pub AccountId);

impl Account {
    pub fn view(&self) -> QueryBuilder<AccountViewHandler> {
        QueryBuilder::new(
            near_primitives::views::QueryRequest::ViewAccount {
                account_id: self.0.clone(),
            },
            Default::default(),
        )
    }

    pub fn access_key(&self, signer_public_key: PublicKey) -> QueryBuilder<AccessKeyHandler> {
        QueryBuilder::new(
            near_primitives::views::QueryRequest::ViewAccessKey {
                account_id: self.0.clone(),
                public_key: signer_public_key,
            },
            Default::default(),
        )
    }

    pub fn list_keys(&self) -> QueryBuilder<AccessKeyListHandler> {
        QueryBuilder::new(
            near_primitives::views::QueryRequest::ViewAccessKeyList {
                account_id: self.0.clone(),
            },
            Default::default(),
        )
    }

    pub fn delegation_in_pool(
        &self,
        pool: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<u128, NearToken>>> {
        let args = serde_json::to_vec(&serde_json::json!({
            "account_id": self.0.clone(),
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

    pub fn add_key(&self, access_key: AccessKeyPermission) -> AddKeyBuilder {
        AddKeyBuilder {
            account_id: self.0.clone(),
            access_key,
        }
    }

    pub fn delete_key(&self, public_key: PublicKey) -> ConstructTransaction {
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(
            near_primitives::transaction::Action::DeleteKey(Box::new(DeleteKeyAction {
                public_key,
            })),
        )
    }

    pub fn delete_account(&self, beneficiary_id: AccountId) -> ConstructTransaction {
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(
            near_primitives::transaction::Action::DeleteAccount(
                near_primitives::transaction::DeleteAccountAction { beneficiary_id },
            ),
        )
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

pub struct AddKeyBuilder {
    account_id: AccountId,
    access_key: AccessKeyPermission,
}

impl AddKeyBuilder {
    pub fn with_public_key(self, public_key: PublicKey) -> ConstructTransaction {
        ConstructTransaction::new(self.account_id.clone(), self.account_id).add_action(
            near_primitives::transaction::Action::AddKey(Box::new(AddKeyAction {
                access_key: AccessKey {
                    nonce: 0,
                    permission: self.access_key,
                },
                public_key,
            })),
        )
    }
}

#[cfg(test)]
mod tests {
    use near_primitives::types::BlockReference;

    const TESTNET_ACCOUNT: &str = "yurtur.testnet";

    #[tokio::test]
    async fn load_account() {
        let account = super::Account(TESTNET_ACCOUNT.parse().unwrap());
        assert!(account
            .view()
            .as_of(BlockReference::latest())
            .fetch_from_testnet()
            .await
            .is_ok());
        assert!(account.list_keys().fetch_from_testnet().await.is_ok());
    }
}
