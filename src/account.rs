use anyhow::bail;
use near_crypto::PublicKey;
use near_gas::NearGas;
use near_primitives::{
    account::{AccessKey, AccessKeyPermission},
    action::{AddKeyAction, DeleteKeyAction},
    types::AccountId,
};
use near_token::NearToken;
use reqwest::Response;
use serde_json::json;
use url::Url;

use crate::common::{
    query::{AccessKeyHandler, AccessKeyListHandler, AccountViewHandler, QueryBuilder},
    secret::SecretBuilder,
};
use crate::{
    common::send::Transactionable,
    config::NetworkConfig,
    transactions::{ConstructTransaction, PrepopulateTransaction, TransactionWithSign},
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

    pub fn create_account() -> CreateAccountBuilder {
        CreateAccountBuilder
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

pub struct CreateAccountBuilder;

impl CreateAccountBuilder {
    pub fn fund_myself(
        self,
        account_id: AccountId,
        signer_account_id: AccountId,
        initial_balance: NearToken,
    ) -> SecretBuilder<TransactionWithSign<CreateAccountFundMyselfTx>> {
        SecretBuilder::new(Box::new(move |public_key| {
            let (actions, receiver_id) = if account_id.is_sub_account_of(&signer_account_id) {
                (
                    vec![
                        near_primitives::transaction::Action::CreateAccount(
                            near_primitives::transaction::CreateAccountAction {},
                        ),
                        near_primitives::transaction::Action::Transfer(
                            near_primitives::transaction::TransferAction {
                                deposit: initial_balance.as_yoctonear(),
                            },
                        ),
                        near_primitives::transaction::Action::AddKey(Box::new(
                            near_primitives::transaction::AddKeyAction {
                                public_key,
                                access_key: near_primitives::account::AccessKey {
                                    nonce: 0,
                                    permission:
                                        near_primitives::account::AccessKeyPermission::FullAccess,
                                },
                            },
                        )),
                    ],
                    account_id.clone(),
                )
            } else {
                let args = serde_json::to_vec(&json!({
                    "new_account_id": account_id.to_string(),
                    "new_public_key": public_key.to_string(),
                }))?;

                if let Some(linkdrop_account_id) = account_id.get_parent_account_id() {
                    (
                        vec![near_primitives::transaction::Action::FunctionCall(
                            Box::new(near_primitives::transaction::FunctionCallAction {
                                method_name: "create_account".to_string(),
                                args,
                                gas: NearGas::from_tgas(30).as_gas(),
                                deposit: initial_balance.as_yoctonear(),
                            }),
                        )],
                        linkdrop_account_id.to_owned(),
                    )
                } else {
                    bail!("Can't create top-level account")
                }
            };

            let prepopulated = ConstructTransaction::new(signer_account_id.clone(), receiver_id)
                .add_actions(actions)
                .prepopulated();

            Ok(TransactionWithSign {
                tx: CreateAccountFundMyselfTx { prepopulated },
            })
        }))
    }

    pub fn sponsor_by_faucet_service(
        self,
        account_id: AccountId,
    ) -> SecretBuilder<CreateAccountByFaucet> {
        SecretBuilder::new(Box::new(move |public_key| {
            Ok(CreateAccountByFaucet {
                new_account_id: account_id,
                public_key,
            })
        }))
    }

    pub fn implicit(self) -> SecretBuilder<PublicKey> {
        SecretBuilder::new(Box::new(Ok))
    }
}

pub struct CreateAccountByFaucet {
    pub new_account_id: AccountId,
    pub public_key: PublicKey,
}

impl CreateAccountByFaucet {
    pub async fn send_to_testnet_faucet(self) -> anyhow::Result<Response> {
        let testnet = NetworkConfig::testnet();
        self.send_to_config_faucet(&testnet).await
    }

    pub async fn send_to_config_faucet(self, config: &NetworkConfig) -> anyhow::Result<Response> {
        let faucet_service_url = match &config.faucet_url {
            Some(url) => url,
            None => bail!(
                "The <{}> network config does not have a defined faucet (helper service) that can sponsor the creation of an account.",
                &config.network_name
            )
        };

        self.send_to_faucet(faucet_service_url).await
    }

    pub async fn send_to_faucet(self, url: &Url) -> anyhow::Result<Response> {
        let mut data = std::collections::HashMap::new();
        data.insert("newAccountId", self.new_account_id.to_string());
        data.insert("newAccountPublicKey", self.public_key.to_string());

        let client = reqwest::Client::new();

        Ok(client.post(url.clone()).json(&data).send().await?)
    }
}

pub struct CreateAccountFundMyselfTx {
    prepopulated: PrepopulateTransaction,
}

impl Transactionable for CreateAccountFundMyselfTx {
    fn prepopulated(&self) -> PrepopulateTransaction {
        self.prepopulated.clone()
    }

    fn validate_with_network(
        tx: &PrepopulateTransaction,
        network: &NetworkConfig,
    ) -> anyhow::Result<()> {
        if tx.receiver_id.is_sub_account_of(&tx.signer_id) {
            return Ok(());
        }

        match &network.linkdrop_account_id {
            Some(linkdrop) => {
                if &tx.receiver_id != linkdrop {
                    bail!("Account can be created either under signer account or under linkdrop account. Expected: {:?}, got: {:?}", linkdrop, tx.receiver_id.get_parent_account_id().map(ToString::to_string).unwrap_or_default());
                }
            }
            None => bail!("Can't create top-level account"),
        }

        Ok(())
    }
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
    use near_primitives::types::{AccountId, BlockReference};
    use near_token::NearToken;

    use crate::sign::Signer;

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
                Signer::seed_phrase(include_str!("../seed_phrase").to_string(), None).unwrap(),
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
