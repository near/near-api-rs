//! Transaction signing functionality for NEAR Protocol
//!
//! The [`Signer`] provides various ways to sign transactions on NEAR, including:
//! - Secret key signing
//! - Seed phrase (mnemonic) signing
//! - Access key file signing
//! - Hardware wallet (`Ledger`) signing
//! - System keychain signing
//!
//! # Examples
//!
//! ## Creating a signer using a secret key
//! ```rust,no_run
//! use near_api::*;
//! use near_crypto::SecretKey;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let secret_key: SecretKey = "ed25519:2vVTQWpoZvYZBS4HYFZtzU2rxpoQSrhyFWdaHLqSdyaEfgjefbSKiFpuVatuRqax3HFvVq2tkkqWH2h7tso2nK8q".parse()?;
//! let signer = Signer::new(Signer::from_secret_key(secret_key))?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating a signer using a seed phrase
//! ```rust,no_run
//! use near_api::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let seed_phrase = "witch collapse practice feed shame open despair creek road again ice least";
//! let signer = Signer::new(Signer::from_seed_phrase(seed_phrase, None)?)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating a `Ledger` signer
//! ```rust,no_run
//! # #[cfg(feature = "ledger")]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use near_api::*;
//!
//! let signer = Signer::new(Signer::from_ledger())?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating a `keystore` signer
//! ```rust,no_run
//! # #[cfg(feature = "keystore")]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use near_api::*;
//!
//! let preloaded_keychain = Signer::from_keystore_with_search_for_keys("account_id.testnet".parse()?, &NetworkConfig::testnet()).await?;
//! let signer = Signer::new(preloaded_keychain)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Example signing with [Signer](`Signer`)
//!
//! ```rust,no_run
//! # use near_api::*;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let signer = Signer::new(Signer::from_secret_key("ed25519:2vVTQWpoZvYZBS4HYFZtzU2rxpoQSrhyFWdaHLqSdyaEfgjefbSKiFpuVatuRqax3HFvVq2tkkqWH2h7tso2nK8q".parse()?))?;
//! let transaction_result = Tokens::account("alice.testnet".parse()?)
//!     .send_to("bob.testnet".parse()?)
//!     .near(NearToken::from_near(1))
//!     .with_signer(signer)
//!     .send_to_testnet()
//!     .await?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! # Advanced: [Access Key Pooling](https://github.com/akorchyn/near-api/issues/2)
//!
//! The signer supports pooling multiple access keys for improved transaction throughput.
//! It helps to mitigate concurrency issues that arise when multiple transactions are signed but the
//! transaction with the highest nonce arrives first which would fail transaction with a lower nonce.
//!
//! By using, account key pooling, each transaction is signed with a different key, so that the nonce issue
//! is mitigated as long as the keys are more or equal to the number of signed transactions.
//! ```rust,no_run
//! use near_api::*;
//! use near_crypto::SecretKey;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let signer = Signer::new(Signer::from_secret_key("ed25519:2vVTQWpoZvYZBS4HYFZtzU2rxpoQSrhyFWdaHLqSdyaEfgjefbSKiFpuVatuRqax3HFvVq2tkkqWH2h7tso2nK8q".parse()?))?;
//!
//! // Add additional keys to the pool
//! signer.add_signer_to_pool(Signer::from_seed_phrase("witch collapse practice feed shame open despair creek road again ice least", None)?).await?;
//! signer.add_signer_to_pool(Signer::from_seed_phrase("return cactus real attack meat pitch trash found autumn upgrade mystery pupil", None)?).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Nonce Management
//!
//! The signer automatically manages nonces for transactions:
//! - Caches nonces per (account_id, public_key) pair
//! - Automatically increments nonces for sequential transactions
//! - Supports concurrent transactions as long as the `Arc<Signer>` is same
//!
//! # Secret generation
//! The crate provides utility functions to generate new secret keys and seed phrases
//!
//! See [functions](#functions) section for details
//!
//! # Custom signer
//! The user can instantiate [`Signer`] with a custom signing logic by utilizing the [`SignerTrait`] trait.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use borsh::{BorshDeserialize, BorshSerialize};
use near_crypto::{ED25519SecretKey, PublicKey, SecretKey, Signature};
use near_primitives::{
    action::delegate::SignedDelegateAction,
    hash::hash,
    transaction::{SignedTransaction, Transaction},
    types::{AccountId, BlockHeight, Nonce},
};
use serde::{Deserialize, Serialize};
use slipped10::BIP32Path;
use tracing::{debug, info, instrument, trace, warn};

use crate::{
    config::NetworkConfig,
    errors::{AccessKeyFileError, MetaSignError, SecretError, SignerError},
    types::{transactions::PrepopulateTransaction, CryptoHash},
};

use secret_key::SecretKeySigner;

#[cfg(feature = "keystore")]
pub mod keystore;
#[cfg(feature = "ledger")]
pub mod ledger;
pub mod secret_key;

const SIGNER_TARGET: &str = "near_api::signer";
/// Default HD path for seed phrases and secret keys generation
pub const DEFAULT_HD_PATH: &str = "m/44'/397'/0'";
/// Default word count for seed phrases generation
pub const DEFAULT_WORD_COUNT: usize = 12;

/// A struct representing a pair of public and private keys for an account.
/// This might be useful for getting keys from a file. E.g. `~/.near-credentials`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountKeyPair {
    pub public_key: near_crypto::PublicKey,
    pub private_key: near_crypto::SecretKey,
}

impl AccountKeyPair {
    fn load_access_key_file(path: &Path) -> Result<Self, AccessKeyFileError> {
        let data = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }
}

/// [NEP413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md) input for the signing message.
#[derive(Debug, Clone, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct NEP413Payload {
    /// The message that wants to be transmitted.
    pub message: String,
    /// A nonce that uniquely identifies this instance of the message, denoted as a 32 bytes array.
    pub nonce: [u8; 32],
    /// The recipient to whom the message is destined (e.g. "alice.near" or "myapp.com").
    pub recipient: String,
    /// A callback URL that will be called with the signed message as a query parameter.
    pub callback_url: Option<String>,
}

#[cfg(feature = "ledger")]
impl From<NEP413Payload> for near_ledger::NEP413Payload {
    fn from(payload: NEP413Payload) -> Self {
        Self {
            // TODO: fix after https://github.com/khorolets/near-ledger-rs/pull/25 is merged
            //cspell:disable
            messsage: payload.message,
            //cspell:enable
            nonce: payload.nonce,
            recipient: payload.recipient,
            callback_url: payload.callback_url,
        }
    }
}

/// A trait for implementing custom signing logic.
///
/// This trait provides the core functionality needed to sign transactions and delegate actions.
/// It is used by the [`Signer`] to abstract over different signing methods (secret key, `ledger`, `keystore`, etc.).
///
/// # Examples
///
/// ## Implementing a custom signer
/// ```rust,no_run
/// use near_api::{AccountId, signer::*, types::{transactions::PrepopulateTransaction, CryptoHash}, errors::SignerError};
/// use near_crypto::{PublicKey, SecretKey};
/// use near_primitives::transaction::Transaction;
///
/// struct CustomSigner {
///     secret_key: SecretKey,
/// }
///
/// #[async_trait::async_trait]
/// impl SignerTrait for CustomSigner {
///     async fn get_secret_key(
///         &self,
///         _signer_id: &AccountId,
///         _public_key: &PublicKey
///     ) -> Result<SecretKey, SignerError> {
///         Ok(self.secret_key.clone())
///     }
///
///     fn get_public_key(&self) -> Result<PublicKey, SignerError> {
///         Ok(self.secret_key.public_key())
///     }
/// }
/// ```
///
/// ## Using a custom signer
/// ```rust,no_run
/// # use near_api::{AccountId, signer::*, types::{transactions::PrepopulateTransaction, CryptoHash}, errors::SignerError};
/// # use near_crypto::{PublicKey, SecretKey};
/// # struct CustomSigner;
/// # impl CustomSigner {
/// #     fn new(_: SecretKey) -> Self { Self }
/// # }
/// # #[async_trait::async_trait]
/// # impl SignerTrait for CustomSigner {
/// #     async fn get_secret_key(&self, _: &AccountId, _: &near_crypto::PublicKey) -> Result<near_crypto::SecretKey, near_api::errors::SignerError> { unimplemented!() }
/// #     fn get_public_key(&self) -> Result<PublicKey, SignerError> { unimplemented!() }
/// # }
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let secret_key = "ed25519:2vVTQWpoZvYZBS4HYFZtzU2rxpoQSrhyFWdaHLqSdyaEfgjefbSKiFpuVatuRqax3HFvVq2tkkqWH2h7tso2nK8q".parse()?;
/// let custom_signer = CustomSigner::new(secret_key);
/// let signer = Signer::new(custom_signer)?;
/// # Ok(())
/// # }
/// ```
///
/// ## Example of implementing `sign_meta` and `sign` methods
/// The default implementation of `sign_meta` and `sign` methods should work for most cases.
/// If you need to implement custom logic, you can override these methods.
/// See [`near_ledger`](`ledger::LedgerSigner`) implementation for an example.
#[async_trait::async_trait]
pub trait SignerTrait {
    /// Signs a delegate action for meta transactions.
    ///
    /// This method is used for meta-transactions where one account can delegate transaction delivery and gas payment to another account.
    /// The delegate action is signed with a maximum block height to ensure the delegation expiration after some point in time.
    ///
    /// The default implementation should work for most cases.
    #[instrument(skip(self, tr), fields(signer_id = %tr.signer_id, receiver_id = %tr.receiver_id))]
    async fn sign_meta(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
        max_block_height: BlockHeight,
    ) -> Result<SignedDelegateAction, MetaSignError> {
        let signer_secret_key = self.get_secret_key(&tr.signer_id, &public_key).await?;
        let mut unsigned_transaction = Transaction::new_v0(
            tr.signer_id.clone(),
            public_key,
            tr.receiver_id,
            nonce,
            block_hash.into(),
        );
        *unsigned_transaction.actions_mut() = tr.actions;

        get_signed_delegate_action(unsigned_transaction, signer_secret_key, max_block_height)
    }

    /// Signs a regular transaction.
    ///
    /// This method is used for standard transactions. It creates a signed transaction
    /// that can be sent to the `NEAR` network.
    ///
    /// The default implementation should work for most cases.
    #[instrument(skip(self, tr), fields(signer_id = %tr.signer_id, receiver_id = %tr.receiver_id))]
    async fn sign(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> Result<SignedTransaction, SignerError> {
        let signer_secret_key = self.get_secret_key(&tr.signer_id, &public_key).await?;
        let mut unsigned_transaction = Transaction::new_v0(
            tr.signer_id.clone(),
            public_key,
            tr.receiver_id,
            nonce,
            block_hash.into(),
        );
        *unsigned_transaction.actions_mut() = tr.actions;

        let signature = signer_secret_key.sign(unsigned_transaction.get_hash_and_size().0.as_ref());

        Ok(SignedTransaction::new(signature, unsigned_transaction))
    }

    /// Signs a [NEP413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md) message that is widely used for the [authentication](https://docs.near.org/build/web3-apps/backend/)
    /// and off-chain proof of account ownership.
    ///
    /// The default implementation should work for most cases.
    #[instrument(skip(self), fields(signer_id = %signer_id, receiver_id = %payload.recipient, message = %payload.message))]
    async fn sign_message_nep413(
        &self,
        signer_id: AccountId,
        public_key: PublicKey,
        payload: NEP413Payload,
    ) -> Result<Signature, SignerError> {
        const NEP413_413_SIGN_MESSAGE_PREFIX: u32 = (1u32 << 31u32) + 413u32;
        let mut bytes = NEP413_413_SIGN_MESSAGE_PREFIX.to_le_bytes().to_vec();
        borsh::to_writer(&mut bytes, &payload)?;
        let hash = hash(&bytes);
        let secret = self.get_secret_key(&signer_id, &public_key).await?;
        let signature = secret.sign(hash.as_ref());
        Ok(signature)
    }

    /// Returns the secret key associated with this signer.
    /// This is a `helper` method that should be implemented by the signer or fail with [`SignerError`].
    /// As long as this method works, the default implementation of the [sign_meta](`SignerTrait::sign_meta`) and [sign](`SignerTrait::sign`) methods should work.
    ///
    /// If you can't provide a [`SecretKey`] for some reason (E.g. `Ledger``),
    /// you can fail with SignerError and override `sign_meta` and `sign`, `sign_message_nep413` methods.
    async fn get_secret_key(
        &self,
        signer_id: &AccountId,
        public_key: &PublicKey,
    ) -> Result<SecretKey, SignerError>;

    /// Returns the public key associated with this signer.
    ///
    /// This method is used by the [`Signer`] to manage the pool of signing keys.
    fn get_public_key(&self) -> Result<PublicKey, SignerError>;
}

/// A [Signer](`Signer`) is a wrapper around a single or multiple signer implementations
/// of [SignerTrait](`SignerTrait`).
///
/// It provides an access key pooling and a nonce caching mechanism to improve transaction throughput.
pub struct Signer {
    pool: tokio::sync::RwLock<HashMap<PublicKey, Box<dyn SignerTrait + Send + Sync + 'static>>>,
    nonce_cache: tokio::sync::RwLock<HashMap<(AccountId, PublicKey), AtomicU64>>,
    current_public_key: AtomicUsize,
}

impl Signer {
    /// Creates a new signer and instantiates nonce cache.
    #[instrument(skip(signer))]
    pub fn new<T: SignerTrait + Send + Sync + 'static>(
        signer: T,
    ) -> Result<Arc<Self>, SignerError> {
        let public_key = signer.get_public_key()?;
        Ok(Arc::new(Self {
            pool: tokio::sync::RwLock::new(HashMap::from([(
                public_key,
                Box::new(signer) as Box<dyn SignerTrait + Send + Sync + 'static>,
            )])),
            nonce_cache: tokio::sync::RwLock::new(HashMap::new()),
            current_public_key: AtomicUsize::new(0),
        }))
    }

    /// Adds a signer to the pool of signers.
    /// The [Signer](`Signer`) will rotate the provided implementation of [SignerTrait](`SignerTrait`) on each call to [get_public_key](`Signer::get_public_key`).
    #[instrument(skip(self, signer))]
    pub async fn add_signer_to_pool<T: SignerTrait + Send + Sync + 'static>(
        &self,
        signer: T,
    ) -> Result<(), SignerError> {
        let public_key = signer.get_public_key()?;
        debug!(target: SIGNER_TARGET, "Adding signer to pool");
        self.pool.write().await.insert(public_key, Box::new(signer));
        Ok(())
    }

    /// Fetches the transaction nonce and block hash associated to the access key. Internally
    /// caches the nonce as to not need to query for it every time, and ending up having to run
    /// into contention with others.
    #[instrument(skip(self, network), fields(account_id = %account_id))]
    pub async fn fetch_tx_nonce(
        &self,
        account_id: AccountId,
        public_key: PublicKey,
        network: &NetworkConfig,
    ) -> Result<(Nonce, CryptoHash, BlockHeight), SignerError> {
        debug!(target: SIGNER_TARGET, "Fetching transaction nonce");
        let nonce_data = crate::account::Account(account_id.clone())
            .access_key(public_key.clone())
            .fetch_from(network)
            .await?;
        let nonce_cache = self.nonce_cache.read().await;

        if let Some(nonce) = nonce_cache.get(&(account_id.clone(), public_key.clone())) {
            let nonce = nonce.fetch_add(1, Ordering::SeqCst);
            drop(nonce_cache);
            trace!(target: SIGNER_TARGET, "Nonce fetched from cache");
            return Ok((nonce + 1, nonce_data.block_hash, nonce_data.block_height));
        } else {
            drop(nonce_cache);
        }

        // It's initialization, so it's better to take write lock, so other will wait

        // case where multiple writers end up at the same lock acquisition point and tries
        // to overwrite the cached value that a previous writer already wrote.
        let nonce = self
            .nonce_cache
            .write()
            .await
            .entry((account_id.clone(), public_key.clone()))
            .or_insert_with(|| AtomicU64::new(nonce_data.data.nonce + 1))
            .fetch_max(nonce_data.data.nonce + 1, Ordering::SeqCst)
            .max(nonce_data.data.nonce + 1);

        info!(target: SIGNER_TARGET, "Nonce fetched and cached");
        Ok((nonce, nonce_data.block_hash, nonce_data.block_height))
    }

    /// Creates a [SecretKeySigner](`SecretKeySigner`) using seed phrase with default HD path.
    pub fn from_seed_phrase(
        seed_phrase: &str,
        password: Option<&str>,
    ) -> Result<SecretKeySigner, SecretError> {
        Self::from_seed_phrase_with_hd_path(
            seed_phrase,
            BIP32Path::from_str("m/44'/397'/0'").expect("Valid HD path"),
            password,
        )
    }

    /// Creates a [SecretKeySigner](`SecretKeySigner`) using a secret key.
    pub fn from_secret_key(secret_key: SecretKey) -> SecretKeySigner {
        SecretKeySigner::new(secret_key)
    }

    /// Creates a [SecretKeySigner](`SecretKeySigner`) using seed phrase with a custom HD path.
    pub fn from_seed_phrase_with_hd_path(
        seed_phrase: &str,
        hd_path: BIP32Path,
        password: Option<&str>,
    ) -> Result<SecretKeySigner, SecretError> {
        let secret_key = get_secret_key_from_seed(hd_path, seed_phrase, password)?;
        Ok(SecretKeySigner::new(secret_key))
    }

    /// Creates a [SecretKeySigner](`secret_key::SecretKeySigner`) using a path to the access key file.
    pub fn from_access_keyfile(path: PathBuf) -> Result<SecretKeySigner, AccessKeyFileError> {
        let keypair = AccountKeyPair::load_access_key_file(&path)?;
        debug!(target: SIGNER_TARGET, "Access key file loaded successfully");

        if keypair.public_key != keypair.private_key.public_key() {
            return Err(AccessKeyFileError::PrivatePublicKeyMismatch);
        }

        Ok(SecretKeySigner::new(keypair.private_key))
    }

    /// Creates a [LedgerSigner](`ledger::LedgerSigner`) using default HD path.
    #[cfg(feature = "ledger")]
    pub fn from_ledger() -> ledger::LedgerSigner {
        ledger::LedgerSigner::new(BIP32Path::from_str("44'/397'/0'/0'/1'").expect("Valid HD path"))
    }

    /// Creates a [LedgerSigner](`ledger::LedgerSigner`) using a custom HD path.
    #[cfg(feature = "ledger")]
    pub const fn from_ledger_with_hd_path(hd_path: BIP32Path) -> ledger::LedgerSigner {
        ledger::LedgerSigner::new(hd_path)
    }

    /// Creates a [KeystoreSigner](`keystore::KeystoreSigner`) with predefined public key.
    #[cfg(feature = "keystore")]
    pub fn from_keystore(pub_key: PublicKey) -> keystore::KeystoreSigner {
        keystore::KeystoreSigner::new_with_pubkey(pub_key)
    }

    /// Creates a [KeystoreSigner](`keystore::KeystoreSigner`). The provided function will query provided account for public keys and search
    /// in the system keychain for the corresponding secret keys.
    #[cfg(feature = "keystore")]
    pub async fn from_keystore_with_search_for_keys(
        account_id: AccountId,
        network: &NetworkConfig,
    ) -> Result<keystore::KeystoreSigner, crate::errors::KeyStoreError> {
        keystore::KeystoreSigner::search_for_keys(account_id, network).await
    }

    /// Creates a [SecretKeySigner](`secret_key::SecretKeySigner`) from a [near_workspaces::Account](`near_workspaces::Account`) for testing purposes.
    #[cfg(feature = "workspaces")]
    pub fn from_workspace(account: &near_workspaces::Account) -> SecretKeySigner {
        SecretKeySigner::new(account.secret_key().to_string().parse().unwrap())
    }

    /// Retrieves the public key from the pool of signers.
    /// The public key is rotated on each call.
    #[instrument(skip(self))]
    pub async fn get_public_key(&self) -> Result<PublicKey, SignerError> {
        let index = self.current_public_key.fetch_add(1, Ordering::SeqCst);
        let public_key = {
            let pool = self.pool.read().await;
            pool.keys()
                .nth(index % pool.len())
                .ok_or(SignerError::PublicKeyIsNotAvailable)?
                .clone()
        };
        debug!(target: SIGNER_TARGET, "Public key retrieved");
        Ok(public_key)
    }

    #[instrument(skip(self, tr), fields(signer_id = %tr.signer_id, receiver_id = %tr.receiver_id))]
    pub async fn sign_meta(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
        max_block_height: BlockHeight,
    ) -> Result<SignedDelegateAction, MetaSignError> {
        let signer = self.pool.read().await;

        signer
            .get(&public_key)
            .ok_or(SignerError::PublicKeyIsNotAvailable)?
            .sign_meta(tr, public_key, nonce, block_hash, max_block_height)
            .await
    }

    #[instrument(skip(self, tr), fields(signer_id = %tr.signer_id, receiver_id = %tr.receiver_id))]
    pub async fn sign(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> Result<SignedTransaction, SignerError> {
        let pool = self.pool.read().await;

        pool.get(&public_key)
            .ok_or(SignerError::PublicKeyIsNotAvailable)?
            .sign(tr, public_key, nonce, block_hash)
            .await
    }
}

#[instrument(skip(unsigned_transaction, private_key))]
fn get_signed_delegate_action(
    unsigned_transaction: Transaction,
    private_key: SecretKey,
    max_block_height: u64,
) -> core::result::Result<SignedDelegateAction, MetaSignError> {
    use near_primitives::signable_message::{SignableMessage, SignableMessageType};

    let mut delegate_action = near_primitives::action::delegate::DelegateAction {
        sender_id: unsigned_transaction.signer_id().clone(),
        receiver_id: unsigned_transaction.receiver_id().clone(),
        actions: vec![],
        nonce: unsigned_transaction.nonce(),
        max_block_height,
        public_key: unsigned_transaction.public_key().clone(),
    };

    let actions = unsigned_transaction
        .take_actions()
        .into_iter()
        .map(near_primitives::action::delegate::NonDelegateAction::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| MetaSignError::DelegateActionIsNotSupported)?;

    delegate_action.actions = actions;

    // create a new signature here signing the delegate action + discriminant
    let signable = SignableMessage::new(&delegate_action, SignableMessageType::DelegateAction);
    let signer = near_crypto::InMemorySigner::from_secret_key(
        delegate_action.sender_id.clone(),
        private_key,
    );
    let signature = signable.sign(&near_crypto::Signer::InMemory(signer));

    Ok(near_primitives::action::delegate::SignedDelegateAction {
        delegate_action,
        signature,
    })
}

/// Generates a secret key from a seed phrase.
///
/// Prefer using [generate_secret_key_from_seed_phrase](`generate_secret_key_from_seed_phrase`) if you don't need to customize the HD path and passphrase.
#[instrument(skip(seed_phrase_hd_path, master_seed_phrase, password))]
pub fn get_secret_key_from_seed(
    seed_phrase_hd_path: BIP32Path,
    master_seed_phrase: &str,
    password: Option<&str>,
) -> Result<SecretKey, SecretError> {
    let master_seed =
        bip39::Mnemonic::parse(master_seed_phrase)?.to_seed(password.unwrap_or_default());
    let derived_private_key = slipped10::derive_key_from_path(
        &master_seed,
        slipped10::Curve::Ed25519,
        &seed_phrase_hd_path,
    )
    .map_err(|_| SecretError::DeriveKeyInvalidIndex)?;

    let signing_key = ed25519_dalek::SigningKey::from_bytes(&derived_private_key.key);
    let secret_key = ED25519SecretKey(signing_key.to_keypair_bytes());

    Ok(SecretKey::ED25519(secret_key))
}

/// Generates a new seed phrase with optional customization.
///
/// Prefer using [generate_seed_phrase](`generate_seed_phrase`) or [generate_secret_key](`generate_secret_key`) if you don't need to customize the seed phrase.
pub fn generate_seed_phrase_custom(
    word_count: Option<usize>,
    hd_path: Option<BIP32Path>,
    passphrase: Option<&str>,
) -> Result<(String, PublicKey), SecretError> {
    let mnemonic = bip39::Mnemonic::generate(word_count.unwrap_or(DEFAULT_WORD_COUNT))?;
    let seed_phrase = mnemonic.words().collect::<Vec<&str>>().join(" ");

    let secret_key = get_secret_key_from_seed(
        hd_path.unwrap_or_else(|| DEFAULT_HD_PATH.parse().expect("Valid HD path")),
        &seed_phrase,
        passphrase,
    )?;

    Ok((seed_phrase, secret_key.public_key()))
}

/// Generates a new seed phrase with default settings (12 words, [default HD path](`DEFAULT_HD_PATH`))
pub fn generate_seed_phrase() -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(None, None, None)
}

/// Generates a new 12-words seed phrase with a custom HD path
pub fn generate_seed_phrase_with_hd_path(
    hd_path: BIP32Path,
) -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(None, Some(hd_path), None)
}

/// Generates a new 12-words seed phrase with a custom passphrase and [default HD path](`DEFAULT_HD_PATH`)
pub fn generate_seed_phrase_with_passphrase(
    passphrase: &str,
) -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(None, None, Some(passphrase))
}

/// Generates a new seed phrase with a custom word count and [default HD path](`DEFAULT_HD_PATH`)
pub fn generate_seed_phrase_with_word_count(
    word_count: usize,
) -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(Some(word_count), None, None)
}

/// Generates a secret key from a new seed phrase using default settings
pub fn generate_secret_key() -> Result<SecretKey, SecretError> {
    let (seed_phrase, _) = generate_seed_phrase()?;
    let secret_key = get_secret_key_from_seed(
        DEFAULT_HD_PATH.parse().expect("Valid HD path"),
        &seed_phrase,
        None,
    )?;
    Ok(secret_key)
}

/// Generates a secret key from a seed phrase using [default HD path](`DEFAULT_HD_PATH`)
pub fn generate_secret_key_from_seed_phrase(seed_phrase: String) -> Result<SecretKey, SecretError> {
    get_secret_key_from_seed(
        DEFAULT_HD_PATH.parse().expect("Valid HD path"),
        &seed_phrase,
        None,
    )
}

#[cfg(test)]
mod nep_413_tests {
    use near_crypto::Signature;
    use near_primitives::serialize::from_base64;

    use crate::SignerTrait;

    use super::{NEP413Payload, Signer};

    // The mockup data is created using the sender/my-near-wallet NEP413 implementation
    // The meteor wallet ignores the callback url on time of writing.
    #[tokio::test]
    pub async fn with_callback_url() {
        let payload: NEP413Payload = NEP413Payload {
            message: "Hello NEAR!".to_string(),
            nonce: from_base64("KNV0cOpvJ50D5vfF9pqWom8wo2sliQ4W+Wa7uZ3Uk6Y=")
                .unwrap()
                .try_into()
                .unwrap(),
            recipient: "example.near".to_string(),
            // callback_url: None,
            callback_url: Some("http://localhost:3000".to_string()),
        };

        let signer = Signer::from_seed_phrase(
            "fatal edge jacket cash hard pass gallery fabric whisper size rain biology",
            None,
        )
        .unwrap();
        let signature = signer
            .sign_message_nep413(
                "round-toad.testnet".parse().unwrap(),
                signer.get_public_key().unwrap(),
                payload,
            )
            .await
            .unwrap();

        let expected_signature =
            from_base64("zzZQ/GwAjrZVrTIFlvmmQbDQHllfzrr8urVWHaRt5cPfcXaCSZo35c5LDpPpTKivR6BxLyb3lcPM0FfCW5lcBQ==").unwrap();
        let expected_signature =
            Signature::from_parts(near_crypto::KeyType::ED25519, &expected_signature).unwrap();
        assert_eq!(signature, expected_signature);
    }

    // The mockup data is created using the sender/meteor NEP413 implementation.
    // My near wallet adds the callback url to the payload if it is not provided on time of writing.
    #[tokio::test]
    pub async fn without_callback_url() {
        let payload: NEP413Payload = NEP413Payload {
            message: "Hello NEAR!".to_string(),
            nonce: from_base64("KNV0cOpvJ50D5vfF9pqWom8wo2sliQ4W+Wa7uZ3Uk6Y=")
                .unwrap()
                .try_into()
                .unwrap(),
            recipient: "example.near".to_string(),
            callback_url: None,
        };

        let signer = Signer::from_seed_phrase(
            "fatal edge jacket cash hard pass gallery fabric whisper size rain biology",
            None,
        )
        .unwrap();
        let signature = signer
            .sign_message_nep413(
                "round-toad.testnet".parse().unwrap(),
                signer.get_public_key().unwrap(),
                payload,
            )
            .await
            .unwrap();

        let expected_signature =
            from_base64("NnJgPU1Ql7ccRTITIoOVsIfElmvH1RV7QAT4a9Vh6ShCOnjIzRwxqX54JzoQ/nK02p7VBMI2vJn48rpImIJwAw==").unwrap();
        let expected_signature =
            Signature::from_parts(near_crypto::KeyType::ED25519, &expected_signature).unwrap();
        assert_eq!(signature, expected_signature);
    }
}
