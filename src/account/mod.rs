use near_crypto::PublicKey;
use near_primitives::{
    account::{AccessKey, AccessKeyPermission},
    action::{AddKeyAction, DeleteKeyAction},
    types::{AccountId, BlockReference},
};

use crate::common::{
    query::{
        AccessKeyHandler, AccessKeyListHandler, AccountViewHandler, QueryBuilder, RpcBuilder,
        SimpleQuery,
    },
    secret::SecretBuilder,
};
use crate::transactions::ConstructTransaction;

use self::create::CreateAccountBuilder;

mod create;

pub struct Account(pub AccountId);

impl Account {
    pub fn view(self) -> QueryBuilder<AccountViewHandler> {
        let request = near_primitives::views::QueryRequest::ViewAccount {
            account_id: self.0.clone(),
        };
        QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            Default::default(),
        )
    }

    pub fn access_key(self, signer_public_key: PublicKey) -> QueryBuilder<AccessKeyHandler> {
        let request = near_primitives::views::QueryRequest::ViewAccessKey {
            account_id: self.0.clone(),
            public_key: signer_public_key,
        };
        RpcBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            Default::default(),
        )
    }

    pub fn list_keys(self) -> QueryBuilder<AccessKeyListHandler> {
        let request = near_primitives::views::QueryRequest::ViewAccessKeyList {
            account_id: self.0.clone(),
        };
        RpcBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            Default::default(),
        )
    }

    pub fn add_key(self, permission: AccessKeyPermission) -> SecretBuilder<ConstructTransaction> {
        SecretBuilder::new(move |public_key| {
            Ok(
                ConstructTransaction::new(self.0.clone(), self.0).add_action(
                    near_primitives::transaction::Action::AddKey(Box::new(AddKeyAction {
                        access_key: AccessKey {
                            nonce: 0,
                            permission,
                        },
                        public_key,
                    })),
                ),
            )
        })
    }

    pub fn delete_key(self, public_key: PublicKey) -> ConstructTransaction {
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(
            near_primitives::transaction::Action::DeleteKey(Box::new(DeleteKeyAction {
                public_key,
            })),
        )
    }

    pub fn delete_keys(self, public_keys: Vec<PublicKey>) -> ConstructTransaction {
        let actions = public_keys
            .into_iter()
            .map(|public_key| {
                near_primitives::transaction::Action::DeleteKey(Box::new(DeleteKeyAction {
                    public_key,
                }))
            })
            .collect();

        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_actions(actions)
    }

    pub fn delete_account_with_beneficiary(
        self,
        beneficiary_id: AccountId,
    ) -> ConstructTransaction {
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(
            near_primitives::transaction::Action::DeleteAccount(
                near_primitives::transaction::DeleteAccountAction { beneficiary_id },
            ),
        )
    }

    pub fn create_account() -> CreateAccountBuilder {
        CreateAccountBuilder
    }
}

#[cfg(test)]
mod tests {
    use near_primitives::types::{AccountId, BlockReference};
    use near_token::NearToken;

    use crate::signer::Signer;

    const TESTNET_ACCOUNT: &str = "yurtur.testnet";

    #[tokio::test]
    async fn load_account() {
        let account = super::Account(TESTNET_ACCOUNT.parse().unwrap());

        assert!(account
            .view()
            .at(BlockReference::latest())
            .fetch_from_testnet()
            .await
            .is_ok());
        assert!(account.list_keys().fetch_from_testnet().await.is_ok());
    }

    #[tokio::test]
    async fn create_account() {
        super::Account::create_account()
            .fund_myself(
                "hahasdasdas.testnet".parse().unwrap(),
                "yurtur.testnet".parse().unwrap(),
                NearToken::from_millinear(100),
            )
            .new_keypair()
            .save_generated_seed_to_file("account_seed".into())
            .unwrap()
            .with_signer(
                Signer::seed_phrase(include_str!("../../seed_phrase").to_string(), None).unwrap(),
            )
            .send_to_testnet()
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn faucet() {
        let account: AccountId = "humblebee.testnet".parse().unwrap();
        let (key, tx) = super::Account::create_account()
            .sponsor_by_faucet_service(account.clone())
            .new_keypair()
            .generate_secret_key()
            .unwrap();

        tx.send_to_testnet_faucet()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        super::Account(account)
            .delete_account_with_beneficiary(TESTNET_ACCOUNT.parse().unwrap())
            .with_signer(Signer::secret_key(key))
            .send_to_testnet()
            .await
            .unwrap()
            .assert_success();
    }

    #[tokio::test]
    async fn implicit() {
        let _ = super::Account::create_account()
            .implicit()
            .new_keypair()
            .save_generated_seed_to_file("account_seed".into())
            .unwrap();
    }
}
