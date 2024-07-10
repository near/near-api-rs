use near_crypto::PublicKey;
use near_primitives::types::AccountId;
use near_token::NearToken;

use crate::query::{
    AccessKeyHandler, AccessKeyListHandler, AccountViewHandler, CallResultHandler, QueryBuilder,
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
