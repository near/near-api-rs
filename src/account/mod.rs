use std::convert::Infallible;

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

#[derive(Clone, Debug)]
pub struct Account(pub AccountId);

impl Account {
    pub fn view(&self) -> QueryBuilder<AccountViewHandler> {
        let request = near_primitives::views::QueryRequest::ViewAccount {
            account_id: self.0.clone(),
        };
        QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            Default::default(),
        )
    }

    pub fn access_key(&self, signer_public_key: PublicKey) -> QueryBuilder<AccessKeyHandler> {
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

    pub fn list_keys(&self) -> QueryBuilder<AccessKeyListHandler> {
        let request = near_primitives::views::QueryRequest::ViewAccessKeyList {
            account_id: self.0.clone(),
        };
        RpcBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            Default::default(),
        )
    }

    pub fn add_key(
        &self,
        permission: AccessKeyPermission,
    ) -> SecretBuilder<ConstructTransaction, Infallible> {
        let account_id = self.0.clone();
        SecretBuilder::new(move |public_key| {
            Ok(
                ConstructTransaction::new(account_id.clone(), account_id.clone()).add_action(
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

    pub fn delete_key(&self, public_key: PublicKey) -> ConstructTransaction {
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(
            near_primitives::transaction::Action::DeleteKey(Box::new(DeleteKeyAction {
                public_key,
            })),
        )
    }

    pub fn delete_keys(&self, public_keys: Vec<PublicKey>) -> ConstructTransaction {
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
        &self,
        beneficiary_id: AccountId,
    ) -> ConstructTransaction {
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(
            near_primitives::transaction::Action::DeleteAccount(
                near_primitives::transaction::DeleteAccountAction { beneficiary_id },
            ),
        )
    }

    pub const fn create_account() -> CreateAccountBuilder {
        CreateAccountBuilder
    }
}
