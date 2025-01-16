use near_crypto::PublicKey;
use near_primitives::{
    account::{AccessKey, AccessKeyPermission},
    action::{AddKeyAction, DeleteKeyAction},
    types::{AccountId, BlockReference},
};

use crate::common::query::{
    AccessKeyHandler, AccessKeyListHandler, AccountViewHandler, QueryBuilder, RpcBuilder,
    SimpleQuery,
};
use crate::transactions::ConstructTransaction;

use self::create::CreateAccountBuilder;

mod create;

/// Account management related interactions with the NEAR Protocol
///
/// The [`Account`] struct provides methods to interact with NEAR accounts, including querying account information, managing access keys, and creating new accounts.
///
/// # Examples
///
/// ```rust,no_run
/// use near_api::*;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let account_info = Account("alice.testnet".parse()?).view().fetch_from_testnet().await?;
/// println!("Account: {:?}", account_info);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Account(pub AccountId);

impl Account {
    /// Returns the account information for the given account ID.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let account_info = Account("alice.testnet".parse()?).view().fetch_from_testnet().await?;
    /// println!("Account: {:?}", account_info);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Returns the access key information for the given account public key.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use near_crypto::PublicKey;
    /// use std::str::FromStr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let access_key = Account("alice.testnet".parse()?)
    ///     .access_key(PublicKey::from_str("ed25519:H4sIAAAAAAAAA+2X0Q6CMBAAtVlJQgYAAAA=")?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Access key: {:?}", access_key);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Returns the list of access keys for the given account ID.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let access_keys = Account("alice.testnet".parse()?).list_keys().fetch_from_testnet().await?;
    /// println!("Access keys: {:?}", access_keys);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Adds a new access key to the given account ID.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use near_primitives::account::AccessKeyPermission;
    /// use near_crypto::PublicKey;
    /// use std::str::FromStr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let pk = PublicKey::from_str("ed25519:H4sIAAAAAAAAA+2X0Q6CMBAAtVlJQgYAAAA=")?;
    /// Account("alice.testnet".parse()?)
    ///     .add_key(AccessKeyPermission::FullAccess, pk)
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_key(
        &self,
        permission: AccessKeyPermission,
        public_key: PublicKey,
    ) -> ConstructTransaction {
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(
            near_primitives::transaction::Action::AddKey(Box::new(AddKeyAction {
                access_key: AccessKey {
                    nonce: 0,
                    permission,
                },
                public_key,
            })),
        )
    }

    /// Deletes an access key from the given account ID.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use near_crypto::PublicKey;
    /// use std::str::FromStr;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Account("alice.testnet".parse()?)
    ///     .delete_key(PublicKey::from_str("ed25519:H4sIAAAAAAAAA+2X0Q6CMBAAtVlJQgYAAAA=")?)
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete_key(&self, public_key: PublicKey) -> ConstructTransaction {
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(
            near_primitives::transaction::Action::DeleteKey(Box::new(DeleteKeyAction {
                public_key,
            })),
        )
    }

    /// Deletes multiple access keys from the given account ID.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use near_crypto::PublicKey;
    /// use std::str::FromStr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Account("alice.testnet".parse()?)
    ///     .delete_keys(vec![PublicKey::from_str("ed25519:H4sIAAAAAAAAA+2X0Q6CMBAAtVlJQgYAAAA=")?])
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
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

    /// Deletes the account with the given beneficiary ID. The account balance will be transfered to the beneficiary.
    ///
    /// Please note that this action is irreversible. Also, you have to understand that another person could potentially
    /// get access to the named account and pretend to be the owner of the account on other websites.
    ///
    /// Do not use it unless you understand the consequences.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Account("alice.testnet".parse()?)
    ///     .delete_account_with_beneficiary("bob.testnet".parse()?)
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
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

    /// Creates a new account builder for the given account ID.
    ///
    /// Please note that you can create an account inhereted from root account (near, testnet) or sub-account only.
    /// You can't create an account that is sub-account of other account.
    ///
    /// E.g you are `alice.testnet`, you can't create `subaccount.bob.testnet`, but you can create `subaccount.alice.testnet`.
    ///
    /// ## Creating account sponsored by faucet service
    ///
    /// This is a way to create an account without having to fund it. It works only on testnet.
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let secret = near_api::signer::generate_secret_key()?;
    /// let account = Account::create_account("alice.testnet".parse()?)
    ///     .sponsor_by_faucet_service()
    ///     .public_key(secret.public_key())?
    ///     .send_to_testnet_faucet()
    ///     .await?;
    /// // You have to save the secret key somewhere safe
    /// std::fs::write("secret.key", secret.to_string())?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Creating account inhereted from root account funding by your own
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let secret = near_api::signer::generate_secret_key()?;
    /// let bob_signer = Signer::new(Signer::from_seed_phrase("lucky barrel fall come bottom can rib join rough around subway cloth ", None)?)?;
    /// let account = Account::create_account("alice.testnet".parse()?)
    ///     .fund_myself("bob.testnet".parse()?, NearToken::from_near(1))
    ///     .public_key(secret.public_key())?
    ///     .with_signer(bob_signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Creating sub-account funded by your own
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let secret = near_api::signer::generate_secret_key()?;
    /// let bob_signer = Signer::new(Signer::from_seed_phrase("lucky barrel fall come bottom can rib join rough around subway cloth ", None)?)?;
    /// let account = Account::create_account("subaccount.bob.testnet".parse()?)
    ///     .fund_myself("bob.testnet".parse()?, NearToken::from_near(1))
    ///     .public_key(secret.public_key())?
    ///     .with_signer(bob_signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub const fn create_account(account_id: AccountId) -> CreateAccountBuilder {
        CreateAccountBuilder { account_id }
    }
}
