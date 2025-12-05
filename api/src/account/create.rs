use std::convert::Infallible;

use near_api_types::{
    near_account_id::{ParseAccountError, TryIntoAccountId},
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
    errors::{AccountCreationError, ArgumentValidationError, FaucetError, ValidationError},
    transactions::{ConstructTransaction, TransactionWithSign},
    Contract, NetworkConfig,
};

#[derive(Clone, Debug)]
pub struct CreateAccountBuilder {
    pub(crate) account_id: Result<AccountId, ParseAccountError>,
}

impl CreateAccountBuilder {
    /// Create an NEAR account and fund it by your own
    ///
    /// You can only create an sub-account of your own account or sub-account of the linkdrop account ([near](https://nearblocks.io/address/near) on mainnet , [testnet](https://testnet.nearblocks.io/address/testnet) on testnet)
    pub fn fund_myself(
        self,
        signer_account_id: impl TryIntoAccountId,
        initial_balance: NearToken,
    ) -> FundMyselfBuilder {
        FundMyselfBuilder {
            new_account_id: self.account_id,
            signer_account_id: signer_account_id.try_into_account_id(),
            initial_balance,
        }
    }

    /// Create an account sponsored by faucet service
    ///
    /// This is a way to create an account without having to fund it. It works only on testnet.
    /// You can only create an sub-account of the [testnet](https://testnet.nearblocks.io/address/testnet) account
    pub fn sponsor_by_faucet_service(self) -> SponsorByFaucetServiceBuilder {
        SponsorByFaucetServiceBuilder {
            new_account_id: self.account_id,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FundMyselfBuilder {
    new_account_id: Result<AccountId, ParseAccountError>,
    signer_account_id: Result<AccountId, ParseAccountError>,
    initial_balance: NearToken,
}

impl FundMyselfBuilder {
    /// Provide a public key that will be used as full access key.
    ///
    /// Please ensure that you have a private key.
    pub fn with_public_key(
        self,
        pk: impl Into<PublicKey>,
    ) -> TransactionWithSign<CreateAccountFundMyselfTx> {
        let public_key = pk.into();
        let transaction = self
            .new_account_id
            .and_then(|id| {
                self.signer_account_id
                    .clone()
                    .map(|signer_id| (id, signer_id))
            })
            .clone()
            .map_err(Into::into)
            .map(|(new_account_id, signer_account_id)| {
                if new_account_id.is_sub_account_of(&signer_account_id) {
                    ConstructTransaction::new(signer_account_id, new_account_id)
                        .add_actions(vec![
                            Action::CreateAccount(CreateAccountAction {}),
                            Action::Transfer(TransferAction {
                                deposit: self.initial_balance,
                            }),
                            Action::AddKey(Box::new(AddKeyAction {
                                public_key,
                                access_key: AccessKey {
                                    nonce: 0.into(),
                                    permission: AccessKeyPermission::FullAccess,
                                },
                            })),
                        ])
                        .transaction
                } else if let Some(linkdrop_account_id) = new_account_id.get_parent_account_id() {
                    Contract::from_id(linkdrop_account_id.to_owned())
                        .call_function(
                            "create_account",
                            json!({
                                "new_account_id": new_account_id.to_string(),
                                "new_public_key": public_key,
                            }),
                        )
                        .transaction()
                        .gas(NearGas::from_tgas(30))
                        .deposit(self.initial_balance)
                        .with_signer_account(signer_account_id)
                        .transaction
                } else {
                    Err(AccountCreationError::TopLevelAccountIsNotAllowed.into())
                }
            })
            .flatten();

        TransactionWithSign {
            tx: CreateAccountFundMyselfTx {
                prepopulated: transaction,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct SponsorByFaucetServiceBuilder {
    new_account_id: Result<AccountId, ParseAccountError>,
}

impl SponsorByFaucetServiceBuilder {
    /// Provide a public key that will be used as full access key.
    ///
    /// Please ensure that you have a private key.
    pub fn with_public_key(
        self,
        pk: impl Into<PublicKey>,
    ) -> Result<CreateAccountByFaucet, Infallible> {
        Ok(CreateAccountByFaucet {
            new_account_id: self.new_account_id,
            public_key: pk.into(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct CreateAccountByFaucet {
    pub new_account_id: Result<AccountId, ParseAccountError>,
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
        data.insert("newAccountId", self.new_account_id?.to_string());
        data.insert("newAccountPublicKey", self.public_key.to_string());

        let client = reqwest::Client::new();

        Ok(client.post(url.clone()).json(&data).send().await?)
    }
}

/// The [CreateAccountFundMyselfTx] is used to validate the transaction before sending it to the network.
///
/// It validates that:
/// - The account is created as a sub-account of the signer
/// - The account is created as a sub-account of the linkdrop account defined in the network config
#[derive(Clone, Debug)]
pub struct CreateAccountFundMyselfTx {
    prepopulated: Result<PrepopulateTransaction, ArgumentValidationError>,
}

#[async_trait::async_trait]
impl Transactionable for CreateAccountFundMyselfTx {
    fn prepopulated(&self) -> Result<PrepopulateTransaction, ArgumentValidationError> {
        self.prepopulated.clone()
    }

    async fn validate_with_network(&self, network: &NetworkConfig) -> Result<(), ValidationError> {
        let prepopulated = self.prepopulated()?;

        if prepopulated
            .receiver_id
            .is_sub_account_of(&prepopulated.signer_id)
        {
            return Ok(());
        }

        match &network.linkdrop_account_id {
            Some(linkdrop) => {
                if &prepopulated.receiver_id != linkdrop {
                    Err(AccountCreationError::AccountShouldBeSubAccountOfSignerOrLinkdrop)?;
                }
            }
            None => Err(AccountCreationError::LinkdropIsNotDefined)?,
        }

        Ok(())
    }
}
