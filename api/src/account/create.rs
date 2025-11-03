use std::convert::Infallible;

use near_api_types::{
    transaction::{
        actions::{AddKeyAction, CreateAccountAction, TransferAction},
        PrepopulateTransaction,
    },
    AccessKey, AccessKeyPermission, AccountId, Action, NearGas, NearToken, PublicKey,
};
use reqwest::Response;
use serde_json::json;
use url::Url;

use crate::{
    common::send::Transactionable,
    errors::{AccountCreationError, FaucetError, ValidationError},
    transactions::{ConstructTransaction, TransactionWithSign},
    Contract, NetworkConfig,
};

#[derive(Clone, Debug)]
pub struct CreateAccountBuilder {
    pub account_id: AccountId,
}

impl CreateAccountBuilder {
    /// Create an NEAR account and fund it by your own
    ///
    /// You can only create an sub-account of your own account or sub-account of the linkdrop account ([near](https://nearblocks.io/address/near) on mainnet , [testnet](https://testnet.nearblocks.io/address/testnet) on testnet)
    pub fn fund_myself(
        self,
        signer_account_id: AccountId,
        initial_balance: NearToken,
    ) -> PublicKeyProvider<TransactionWithSign<CreateAccountFundMyselfTx>, AccountCreationError>
    {
        PublicKeyProvider::new(Box::new(move |public_key| {
            let (actions, receiver_id) = if self.account_id.is_sub_account_of(&signer_account_id) {
                (
                    vec![
                        Action::CreateAccount(CreateAccountAction {}),
                        Action::Transfer(TransferAction {
                            deposit: initial_balance,
                        }),
                        Action::AddKey(Box::new(AddKeyAction {
                            public_key,
                            access_key: AccessKey {
                                nonce: 0.into(),
                                permission: AccessKeyPermission::FullAccess,
                            },
                        })),
                    ],
                    self.account_id.clone(),
                )
            } else if let Some(linkdrop_account_id) = self.account_id.get_parent_account_id() {
                (
                    Contract(linkdrop_account_id.to_owned())
                        .call_function(
                            "create_account",
                            json!({
                                "new_account_id": self.account_id.to_string(),
                                "new_public_key": public_key,
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

            let prepopulated = ConstructTransaction::new(signer_account_id, receiver_id)
                .add_actions(actions)
                .prepopulated();

            Ok(TransactionWithSign {
                tx: CreateAccountFundMyselfTx { prepopulated },
            })
        }))
    }

    /// Create an account sponsored by faucet service
    ///
    /// This is a way to create an account without having to fund it. It works only on testnet.
    /// You can only create an sub-account of the [testnet](https://testnet.nearblocks.io/address/testnet) account
    pub fn sponsor_by_faucet_service(self) -> PublicKeyProvider<CreateAccountByFaucet, Infallible> {
        PublicKeyProvider::new(Box::new(move |public_key| {
            Ok(CreateAccountByFaucet {
                new_account_id: self.account_id,
                public_key,
            })
        }))
    }
}

#[derive(Clone, Debug)]
pub struct CreateAccountByFaucet {
    pub new_account_id: AccountId,
    pub public_key: PublicKey,
}

impl CreateAccountByFaucet {
    /// Sends the account creation request to the default testnet faucet service.
    ///
    /// The account will be created as a sub-account of the [testnet](https://testnet.nearblocks.io/address/testnet) account
    pub async fn send_to_testnet_faucet(self) -> Result<Response, FaucetError> {
        let testnet = NetworkConfig::testnet();
        self.send_to_config_faucet(&testnet).await
    }

    /// Sends the account creation request to the faucet service specified in the network config.
    /// This way you can specify your own faucet service.
    ///
    /// The function sends the request in the following format:
    /// ```json
    /// {
    ///     "newAccountId": "new_account_id",
    ///     "newAccountPublicKey": "new_account_public_key"
    /// }
    /// ```
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

    /// Sends the account creation request to the faucet service specified by the URL.
    ///
    /// The function sends the request in the following format:
    /// ```json
    /// {
    ///     "newAccountId": "new_account_id",
    ///     "newAccountPublicKey": "new_account_public_key"
    /// }
    /// ```
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
                    Err(AccountCreationError::AccountShouldBeSubAccountOfSignerOrLinkdrop)?;
                }
            }
            None => Err(AccountCreationError::LinkdropIsNotDefined)?,
        }

        Ok(())
    }
}

pub type PublicKeyCallback<T, E> = dyn FnOnce(PublicKey) -> Result<T, E>;

pub struct PublicKeyProvider<T, E> {
    next_step: Box<PublicKeyCallback<T, E>>,
}

impl<T, E> PublicKeyProvider<T, E> {
    pub const fn new(next_step: Box<PublicKeyCallback<T, E>>) -> Self {
        Self { next_step }
    }

    pub fn public_key(self, pk: impl Into<PublicKey>) -> Result<T, E> {
        (self.next_step)(pk.into())
    }
}
