use anyhow::bail;
use near_crypto::PublicKey;
use near_gas::NearGas;
use near_primitives::types::AccountId;
use near_token::NearToken;
use reqwest::Response;
use serde_json::json;
use url::Url;

use crate::{
    common::{query::QueryBuilder, secret::SecretBuilder, send::Transactionable},
    config::NetworkConfig,
    transactions::{ConstructTransaction, TransactionWithSign},
    types::transactions::PrepopulateTransaction,
};

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
    type Handler = ();

    fn prepopulated(&self) -> PrepopulateTransaction {
        self.prepopulated.clone()
    }

    fn validate_with_network(
        &self,
        network: &NetworkConfig,
        _query_response: Option<()>,
    ) -> anyhow::Result<()> {
        if self
            .prepopulated
            .receiver_id
            .is_sub_account_of(&self.prepopulated.signer_id)
        {
            return Ok(());
        }

        match &network.linkdrop_account_id {
            Some(linkdrop) => {
                if &self.prepopulated.receiver_id != linkdrop {
                    bail!("Account can be created either under signer account or under linkdrop account. Expected: {:?}, got: {:?}", linkdrop, self.prepopulated.receiver_id.get_parent_account_id().map(ToString::to_string).unwrap_or_default());
                }
            }
            None => bail!("Can't create top-level account"),
        }

        Ok(())
    }

    fn prequery(&self) -> Option<QueryBuilder<()>> {
        None
    }
}
