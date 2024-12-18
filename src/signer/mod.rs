//! Transaction signing functionality for NEAR Protocol
//!
//! The [`Signer`] provides various ways to sign transactions on NEAR, including:
//! - Secret key signing
//! - Seed phrase (mnemonic) signing
//! - Access key file signing
//! - Hardware wallet (Ledger) signing
//! - System keychain signing
//!
//! # Examples
//!
//! ## Sign with Secret Key
//! ```rust,no_run
//! use near_api::*;
//! use near_crypto::SecretKey;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let secret_key: SecretKey = "ed25519:...".parse()?;
//! let signer = Signer::new(Signer::secret_key(secret_key))?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Sign with Seed Phrase
//! ```rust,no_run
//! use near_api::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let seed_phrase = "witch collapse practice feed shame open despair creek road again ice least";
//! let signer = Signer::new(Signer::seed_phrase(seed_phrase, None)?)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Sign with Ledger
//! ```rust,no_run
//! # #[cfg(feature = "ledger")]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use near_api::*;
//!
//! let signer = Signer::new(Signer::ledger())?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Sign with a key from the system keychain
//! ```rust,no_run
//! # #[cfg(feature = "keystore")]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use near_api::*;
//!
//! let preloaded_keychain = Signer::keystore_search_for_keys("account_id.testnet".parse()?, &NetworkConfig::testnet()).await?;
//! let signer = Signer::new(preloaded_keychain)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Access Key Pooling
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
//! let signer = Signer::new(Signer::secret_key("ed25519:...".parse()?))?;
//!
//! // Add additional keys to the pool
//! signer.add_signer_to_pool(Signer::secret_key("ed25519:...".parse()?)).await?;
//! signer.add_signer_to_pool(Signer::secret_key("ed25519:...".parse()?)).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Nonce Management
//!
//! The signer automatically manages nonces for transactions:
//! - Caches nonces per (account_id, public_key) pair
//! - Handles concurrent transactions safely
//! - Automatically increments nonces for sequential transactions
//!
//! # Secret generation
//!
//! The crate provides utility functions to generate new secret keys and seed phrases:
//! - [`generate_seed_phrase`]: Generates a new seed phrase with default settings
//! - [`generate_seed_phrase_with_hd_path`]: Generates a new seed phrase with a custom HD path
//! - [`generate_seed_phrase_with_passphrase`]: Generates a new seed phrase with a custom passphrase
//! - [`generate_seed_phrase_with_word_count`]: Generates a new seed phrase with a custom word count
//! - [`generate_secret_key`]: Generates a new secret key from a new seed phrase
//! - [`generate_secret_key_from_seed_phrase`]: Generates a secret key from a seed phrase
//!
//! # Custom signer
//! The user can instantiate [`Signer`] with a custom signing logic by utilising the [`SignerTrait`] trait.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use near_crypto::{ED25519SecretKey, PublicKey, SecretKey};
use near_primitives::{
    action::delegate::SignedDelegateAction,
    transaction::{SignedTransaction, Transaction},
    types::{AccountId, BlockHeight, Nonce},
};
use serde::Deserialize;
use slipped10::BIP32Path;
use tracing::{debug, info, instrument, trace, warn};

use crate::{
    config::NetworkConfig,
    errors::{AccessKeyFileError, MetaSignError, SecretError, SignerError},
    types::{transactions::PrepopulateTransaction, CryptoHash},
};

use self::{access_keyfile_signer::AccessKeyFileSigner, secret_key::SecretKeySigner};

pub mod access_keyfile_signer;
#[cfg(feature = "keystore")]
pub mod keystore;
#[cfg(feature = "ledger")]
pub mod ledger;
pub mod secret_key;

const SIGNER_TARGET: &str = "near_api::signer";
const DEFAULT_HD_PATH: &str = "m/44'/397'/0'";
const DEFAULT_WORD_COUNT: usize = 12;

#[derive(Debug, Deserialize, Clone)]
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

#[async_trait::async_trait]
pub trait SignerTrait {
    async fn sign_meta(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
        max_block_height: BlockHeight,
    ) -> Result<SignedDelegateAction, MetaSignError> {
        let (unsigned_transaction, signer_secret_key) =
            self.tx_and_secret(tr, public_key, nonce, block_hash)?;

        get_signed_delegate_action(unsigned_transaction, signer_secret_key, max_block_height)
    }

    async fn sign(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> Result<SignedTransaction, SignerError> {
        let (unsigned_transaction, signer_secret_key) =
            self.tx_and_secret(tr, public_key, nonce, block_hash)?;
        let signature = signer_secret_key.sign(unsigned_transaction.get_hash_and_size().0.as_ref());

        Ok(SignedTransaction::new(signature, unsigned_transaction))
    }

    fn tx_and_secret(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> Result<(Transaction, SecretKey), SignerError>;
    fn get_public_key(&self) -> Result<PublicKey, SignerError>;
}

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

    /// Helper function to create a [SecretKeySigner](`SecretKeySigner`) using seed phrase with default HD path.
    pub fn seed_phrase(
        seed_phrase: &str,
        password: Option<&str>,
    ) -> Result<SecretKeySigner, SecretError> {
        Self::seed_phrase_with_hd_path(
            seed_phrase,
            BIP32Path::from_str("m/44'/397'/0'").expect("Valid HD path"),
            password,
        )
    }

    /// Helper function to create a [SecretKeySigner](`SecretKeySigner`) using a secret key.
    pub fn secret_key(secret_key: SecretKey) -> SecretKeySigner {
        SecretKeySigner::new(secret_key)
    }

    /// Helper function to create a [SecretKeySigner](`SecretKeySigner`) using seed phrase with a custom HD path.
    pub fn seed_phrase_with_hd_path(
        seed_phrase: &str,
        hd_path: BIP32Path,
        password: Option<&str>,
    ) -> Result<SecretKeySigner, SecretError> {
        let secret_key = get_secret_key_from_seed(hd_path, seed_phrase, password)?;
        Ok(SecretKeySigner::new(secret_key))
    }

    /// Helper function to create a [AccessKeyFileSigner](`AccessKeyFileSigner`) using a path to the access key file.
    pub fn access_keyfile(path: PathBuf) -> Result<AccessKeyFileSigner, AccessKeyFileError> {
        AccessKeyFileSigner::new(path)
    }

    /// Helper function to create a [LedgerSigner](`ledger::LedgerSigner`) using default HD path.
    #[cfg(feature = "ledger")]
    pub fn ledger() -> ledger::LedgerSigner {
        ledger::LedgerSigner::new(BIP32Path::from_str("44'/397'/0'/0'/1'").expect("Valid HD path"))
    }

    /// Helper function to create a [LedgerSigner](`ledger::LedgerSigner`) using a custom HD path.
    #[cfg(feature = "ledger")]
    pub const fn ledger_with_hd_path(hd_path: BIP32Path) -> ledger::LedgerSigner {
        ledger::LedgerSigner::new(hd_path)
    }

    /// Helper function to create a [KeystoreSigner](`keystore::KeystoreSigner`) with predefined public key.
    #[cfg(feature = "keystore")]
    pub fn keystore(pub_key: PublicKey) -> keystore::KeystoreSigner {
        keystore::KeystoreSigner::new_with_pubkey(pub_key)
    }

    /// Helper function to create a [KeystoreSigner](`keystore::KeystoreSigner`). The provided function will query provided account for public keys and search
    /// in the system keychain for the corresponding secret keys.
    #[cfg(feature = "keystore")]
    pub async fn keystore_search_for_keys(
        account_id: AccountId,
        network: &NetworkConfig,
    ) -> Result<keystore::KeystoreSigner, crate::errors::KeyStoreError> {
        keystore::KeystoreSigner::search_for_keys(account_id, network).await
    }

    /// Helper function to create a [SecretKeySigner](`secret_key::SecretKeySigner`) from a [near_workspaces::Account](`near_workspaces::Account`) for testing purposes.
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
pub fn get_signed_delegate_action(
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

/// Generates a new seed phrase with optional customization
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

/// Generates a new seed phrase with default settings (12 words, default HD path)
pub fn generate_seed_phrase() -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(None, None, None)
}

/// Generates a new seed phrase with a custom HD path
pub fn generate_seed_phrase_with_hd_path(
    hd_path: BIP32Path,
) -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(None, Some(hd_path), None)
}

/// Generates a new seed phrase with a custom passphrase
pub fn generate_seed_phrase_with_passphrase(
    passphrase: &str,
) -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(None, None, Some(passphrase))
}

/// Generates a new seed phrase with a custom word count
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

pub fn generate_secret_key_from_seed_phrase(seed_phrase: String) -> Result<SecretKey, SecretError> {
    get_secret_key_from_seed(
        DEFAULT_HD_PATH.parse().expect("Valid HD path"),
        &seed_phrase,
        None,
    )
}
