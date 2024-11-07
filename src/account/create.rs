use std::convert::Infallible;

use near_crypto::PublicKey;
use near_gas::NearGas;
use near_token::NearToken;
use reqwest::Response;
use serde_json::json;
use url::Url;

use crate::{
    common::{secret::SecretBuilder, send::Transactionable},
    errors::{AccountCreationError, FaucetError, ValidationError},
    prelude::*,
    transactions::{ConstructTransaction, TransactionWithSign},
    types::{transactions::PrepopulateTransaction, AccountId},
};

#[derive(Clone, Debug)]
pub struct CreateAccountBuilder;

impl CreateAccountBuilder {
    pub fn fund_myself(
        self,
        account_id: AccountId,
        signer_account_id: AccountId,
        initial_balance: NearToken,
    ) -> SecretBuilder<TransactionWithSign<CreateAccountFundMyselfTx>, AccountCreationError> {
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
            } else if let Some(linkdrop_account_id) = account_id.get_parent_account_id() {
                (
                    Contract(linkdrop_account_id.to_owned())
                        .call_function(
                            "create_account",
                            json!({
                                "new_account_id": account_id.to_string(),
                                "new_public_key": public_key.to_string(),
                            }),
                        )?
                        .transaction()
                        .gas(NearGas::from_tgas(30))
                        .deposit(initial_balance)
                        .with_signer_account(signer_account_id.clone())
                        .prepopulated()
                        .actions,
                    linkdrop_account_id.to_owned(),
                )
            } else {
                return Err(AccountCreationError::TopLevelAccountIsNotAllowed);
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
    ) -> SecretBuilder<CreateAccountByFaucet, Infallible> {
        SecretBuilder::new(Box::new(move |public_key| {
            Ok(CreateAccountByFaucet {
                new_account_id: account_id,
                public_key,
            })
        }))
    }

    pub fn implicit(self) -> SecretBuilder<PublicKey, Infallible> {
        SecretBuilder::new(Box::new(Ok))
    }
}

#[derive(Clone, Debug)]
pub struct CreateAccountByFaucet {
    pub new_account_id: AccountId,
    pub public_key: PublicKey,
}

impl CreateAccountByFaucet {
    pub async fn send_to_testnet_faucet(self) -> Result<Response, FaucetError> {
        let testnet = NetworkConfig::testnet();
        self.send_to_config_faucet(&testnet).await
    }

    pub async fn send_to_config_faucet(
        self,
        config: &NetworkConfig,
    ) -> Result<Response, FaucetError> {
        let faucet_service_url = match &config.faucet_url {
            Some(url) => url,
            None => return Err(FaucetError::FaucetIsNotDefined(config.network_name.clone())),
        };

        self.send_to_faucet(faucet_service_url).await
    }

    pub async fn send_to_faucet(self, url: &Url) -> Result<Response, FaucetError> {
        let mut data = std::collections::HashMap::new();
        data.insert("newAccountId", self.new_account_id.to_string());
        data.insert("newAccountPublicKey", self.public_key.to_string());

        let client = reqwest::Client::new();

        Ok(client.post(url.clone()).json(&data).send().await?)
    }
}

#[derive(Clone, Debug)]
pub struct CreateAccountFundMyselfTx {
    prepopulated: PrepopulateTransaction,
}

#[async_trait::async_trait]
impl Transactionable for CreateAccountFundMyselfTx {
    fn prepopulated(&self) -> PrepopulateTransaction {
        self.prepopulated.clone()
    }

    async fn validate_with_network(&self, network: &NetworkConfig) -> Result<(), ValidationError> {
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
                    Err(AccountCreationError::AccountShouldBeSubaccountOfSignerOrLinkdrop)?;
                }
            }
            None => Err(AccountCreationError::LinkdropIsNotDefined)?,
        }

        Ok(())
    }
}
