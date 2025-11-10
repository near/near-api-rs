use near_api_types::{
    json::U64,
    transaction::actions::{AccessKey, AddKeyAction, DeleteAccountAction, DeleteKeyAction},
    AccessKeyPermission, AccountId, Action, PublicKey, Reference,
};

use crate::advanced::{query_request::QueryRequest, query_rpc::SimpleQueryRpc};
use crate::common::query::{
    AccessKeyHandler, AccessKeyListHandler, AccountViewHandler, RequestBuilder, RpcBuilder,
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
    /// Returns the underlying account ID for this account.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let account = Account("alice.testnet".parse()?);
    /// let account_id = account.account_id();
    /// println!("Account ID: {}", account_id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn account_id(&self) -> &AccountId {
        &self.0
    }

    /// Converts this account to a Contract for contract-related operations.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let account = Account("contract.testnet".parse()?);
    /// let contract = account.as_contract();
    /// let result = contract.call_function("get_value", ())?.read_only().fetch_from_testnet().await?;
    /// println!("Contract value: {:?}", result);
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_contract(&self) -> crate::contract::Contract {
        crate::contract::Contract(self.0.clone())
    }

    /// Creates a Tokens wrapper for token-related operations on this account.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let account = Account("alice.testnet".parse()?);
    /// let tokens = account.tokens();
    /// let balance = tokens.near_balance().fetch_from_testnet().await?;
    /// println!("NEAR balance: {}", balance.total);
    /// # Ok(())
    /// # }
    /// ```
    pub fn tokens(&self) -> crate::tokens::Tokens {
        crate::tokens::Tokens::account(self.0.clone())
    }

    /// Creates a Delegation wrapper for staking-related operations on this account.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let account = Account("alice.testnet".parse()?);
    /// let delegation = account.delegation();
    /// let staked = delegation.view_staked_balance("pool.testnet".parse()?)?.fetch_from_testnet().await?;
    /// println!("Staked balance: {:?}", staked);
    /// # Ok(())
    /// # }
    /// ```
    pub fn delegation(&self) -> crate::stake::Delegation {
        crate::stake::Delegation(self.0.clone())
    }

    /// Prepares a query to fetch the [Data](crate::Data)<[AccountView](near_api_types::AccountView)> with the account information for the given account ID.
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
    pub fn view(&self) -> RequestBuilder<AccountViewHandler> {
        let request = QueryRequest::ViewAccount {
            account_id: self.0.clone(),
        };
        RequestBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            Default::default(),
        )
    }

    /// Prepares a query to fetch the [Data](crate::Data)<[AccessKey]> with the access key information for the given account public key.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
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
    pub fn access_key(
        &self,
        signer_public_key: impl Into<PublicKey>,
    ) -> RequestBuilder<AccessKeyHandler> {
        let request = QueryRequest::ViewAccessKey {
            account_id: self.0.clone(),
            public_key: signer_public_key.into().into(),
        };
        RpcBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            Default::default(),
        )
    }

    /// Prepares a query to fetch the Vec<([`PublicKey`], [`AccessKey`])> list of access keys for the given account ID.
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
    pub fn list_keys(&self) -> RequestBuilder<AccessKeyListHandler> {
        let request = QueryRequest::ViewAccessKeyList {
            account_id: self.0.clone(),
        };
        RpcBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            Default::default(),
        )
    }

    /// Adds a new access key to the given account ID.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::{*, types::AccessKeyPermission};
    /// use std::str::FromStr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let pk = PublicKey::from_str("ed25519:H4sIAAAAAAAAA+2X0Q6CMBAAtVlJQgYAAAA=")?;
    /// let result = Account("alice.testnet".parse()?)
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
        public_key: impl Into<PublicKey>,
    ) -> ConstructTransaction {
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(Action::AddKey(
            Box::new(AddKeyAction {
                access_key: AccessKey {
                    nonce: U64::from(0),
                    permission,
                },
                public_key: public_key.into(),
            }),
        ))
    }

    /// Deletes an access key from the given account ID.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use std::str::FromStr;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Account("alice.testnet".parse()?)
    ///     .delete_key(PublicKey::from_str("ed25519:H4sIAAAAAAAAA+2X0Q6CMBAAtVlJQgYAAAA=")?)
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete_key(&self, public_key: impl Into<PublicKey>) -> ConstructTransaction {
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(Action::DeleteKey(
            Box::new(DeleteKeyAction {
                public_key: public_key.into(),
            }),
        ))
    }

    /// Deletes multiple access keys from the given account ID.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use std::str::FromStr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Account("alice.testnet".parse()?)
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
            .map(|public_key| Action::DeleteKey(Box::new(DeleteKeyAction { public_key })))
            .collect();

        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_actions(actions)
    }

    /// Deletes the account with the given beneficiary ID. The account balance will be transferred to the beneficiary.
    ///
    /// Please note that this action is irreversible. Also, you have to understand that another person could potentially
    /// re-create the account with the same name and pretend to be you on other websites that use your account ID as a unique identifier.
    /// (near.social, devhub proposal, etc)
    ///
    /// Do not use it unless you understand the consequences.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Account("alice.testnet".parse()?)
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
        ConstructTransaction::new(self.0.clone(), self.0.clone()).add_action(Action::DeleteAccount(
            DeleteAccountAction { beneficiary_id },
        ))
    }

    /// Creates a new account builder for the given account ID.
    ///
    /// ## Creating account sponsored by faucet service
    ///
    /// This is a way to create an account without having to fund it. It works only on testnet.
    /// The account should be created as a sub-account of the [testnet](https://testnet.nearblocks.io/address/testnet) account
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let secret = near_api::signer::generate_secret_key()?;
    /// let result: reqwest::Response = Account::create_account("alice.testnet".parse()?)
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
    /// ## Creating sub-account of the linkdrop root account funded by your own NEAR and signed by your account
    ///
    /// There is a few linkdrop root accounts that you can use to create sub-accounts.
    /// * For mainnet, you can use the [near](https://explorer.near.org/accounts/near) account.
    /// * For testnet, you can use the [testnet](https://testnet.nearblocks.io/address/testnet) account.
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let secret = near_api::signer::generate_secret_key()?;
    /// let bob_signer = Signer::new(Signer::from_seed_phrase("lucky barrel fall come bottom can rib join rough around subway cloth ", None)?)?;
    /// let result = Account::create_account("alice.testnet".parse()?)
    ///     .fund_myself("bob.testnet".parse()?, NearToken::from_near(1))
    ///     .public_key(secret.public_key())?
    ///     .with_signer(bob_signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Creating sub-account of your own account funded by your NEAR
    ///
    /// You can create only one level deep of sub-accounts.
    ///
    /// E.g you are `alice.testnet`, you can't create `sub.sub.alice.testnet`, but you can create `sub.alice.testnet`.
    /// Though, 'sub.alice.testnet' can create sub-accounts of its own.
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let secret = near_api::signer::generate_secret_key()?;
    /// let bob_signer = Signer::new(Signer::from_seed_phrase("lucky barrel fall come bottom can rib join rough around subway cloth ", None)?)?;
    /// let result = Account::create_account("sub.bob.testnet".parse()?)
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
