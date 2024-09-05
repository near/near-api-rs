use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use near_crypto::{ED25519SecretKey, PublicKey, SecretKey};
use near_primitives::{
    action::delegate::SignedDelegateAction,
    hash::CryptoHash,
    transaction::{SignedTransaction, Transaction},
    types::{AccountId, BlockHeight, Nonce},
};
use serde::Deserialize;
use slipped10::BIP32Path;

use crate::{
    config::NetworkConfig,
    errors::{AccessKeyFileError, KeyStoreError, MetaSignError, SecretError, SignerError},
    types::transactions::PrepopulateTransaction,
};

use self::{
    access_keyfile_signer::AccessKeyFileSigner, keystore::KeystoreSigner,
    secret_key::SecretKeySigner,
};

pub mod access_keyfile_signer;
pub mod keystore;
#[cfg(feature = "ledger")]
pub mod ledger;
pub mod secret_key;

#[derive(Debug, Deserialize, Clone)]
pub struct AccountKeyPair {
    pub public_key: near_crypto::PublicKey,
    pub private_key: near_crypto::SecretKey,
}

impl AccountKeyPair {
    fn load_access_key_file(path: &Path) -> Result<AccountKeyPair, AccessKeyFileError> {
        let data = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }
}

pub trait SignerTrait {
    fn sign_meta(
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

    fn sign(
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
    signer_type: SignerType,
    nonce_cache: tokio::sync::RwLock<HashMap<AccountId, AtomicU64>>,
}

pub enum SignerType {
    SecretKey(SecretKeySigner),
    AccessKeyFile(AccessKeyFileSigner),
    #[cfg(feature = "ledger")]
    Ledger(ledger::LedgerSigner),
    Keystore(KeystoreSigner),
    Custom(Box<dyn SignerTrait>),
}

impl Signer {
    fn from_signer_type(signer_type: SignerType) -> Arc<Self> {
        Arc::new(Self {
            signer_type,
            nonce_cache: tokio::sync::RwLock::new(HashMap::new()),
        })
    }

    /// Fetches the transaction nonce and block hash associated to the access key. Internally
    /// caches the nonce as to not need to query for it every time, and ending up having to run
    /// into contention with others.
    pub async fn fetch_tx_nonce(
        &self,
        account_id: AccountId,
        network: &NetworkConfig,
    ) -> Result<(Nonce, CryptoHash, BlockHeight), SignerError> {
        let nonces = self.nonce_cache.read().await;
        println!("nonces: {:?}", nonces);

        if let Some(nonce) = nonces.get(&account_id) {
            let nonce = nonce.fetch_add(1, Ordering::SeqCst);
            drop(nonces);
            println!("nonce: {}", nonce);

            // Fetch latest block_hash since the previous one is now invalid for new transactions:
            let nonce_data = crate::account::Account(account_id.clone())
                .access_key(self.as_signer().get_public_key()?)
                .fetch_from(network)
                .await?;

            Ok((nonce + 1, nonce_data.block_hash, nonce_data.block_height))
        } else {
            drop(nonces);
            // It's initialization, so it's better to take write lock, so other will wait
            // for the nonce to be initialized.
            let mut write_nonce = self.nonce_cache.write().await;

            // Fetch latest block_hash since the previous one is now invalid for new transactions:
            let nonce_data = crate::account::Account(account_id.clone())
                .access_key(self.as_signer().get_public_key()?)
                .fetch_from(network)
                .await?;

            // case where multiple writers end up at the same lock acquisition point and tries
            // to overwrite the cached value that a previous writer already wrote.
            let nonce = write_nonce
                .entry(account_id.clone())
                .or_insert_with(|| AtomicU64::new(nonce_data.data.nonce + 1))
                .fetch_max(nonce_data.data.nonce + 1, Ordering::SeqCst)
                .max(nonce_data.data.nonce + 1);

            Ok((nonce, nonce_data.block_hash, nonce_data.block_height))
        }
    }

    pub fn as_signer(&self) -> &dyn SignerTrait {
        match &self.signer_type {
            SignerType::SecretKey(secret_key) => secret_key,
            SignerType::AccessKeyFile(access_keyfile_signer) => access_keyfile_signer,
            #[cfg(feature = "ledger")]
            SignerType::Ledger(ledger_signer) => ledger_signer,
            SignerType::Keystore(keystore_signer) => keystore_signer,
            SignerType::Custom(custom_signer) => custom_signer.as_ref(),
        }
    }

    pub fn seed_phrase(
        seed_phrase: String,
        password: Option<String>,
    ) -> Result<Arc<Self>, SecretError> {
        Self::seed_phrase_with_hd_path(
            seed_phrase,
            BIP32Path::from_str("m/44'/397'/0'").expect("Valid HD path"),
            password,
        )
    }

    pub fn secret_key(secret_key: SecretKey) -> Arc<Self> {
        Self::from_signer_type(SignerType::SecretKey(SecretKeySigner::new(secret_key)))
    }

    pub fn seed_phrase_with_hd_path(
        seed_phrase: String,
        hd_path: BIP32Path,
        password: Option<String>,
    ) -> Result<Arc<Self>, SecretError> {
        let secret_key = get_secret_key_from_seed(hd_path, seed_phrase, password)?;
        Ok(Self::from_signer_type(SignerType::SecretKey(
            SecretKeySigner::new(secret_key),
        )))
    }

    pub fn access_keyfile(path: PathBuf) -> Result<Arc<Self>, AccessKeyFileError> {
        Ok(Self::from_signer_type(SignerType::AccessKeyFile(
            AccessKeyFileSigner::new(path)?,
        )))
    }

    #[cfg(feature = "ledger")]
    pub fn ledger() -> Arc<Self> {
        Self::from_signer_type(SignerType::Ledger(ledger::LedgerSigner::new(
            BIP32Path::from_str("44'/397'/0'/0'/1'").expect("Valid HD path"),
        )))
    }

    #[cfg(feature = "ledger")]
    pub fn ledger_with_hd_path(hd_path: BIP32Path) -> Arc<Self> {
        Self::from_signer_type(SignerType::Ledger(ledger::LedgerSigner::new(hd_path)))
    }

    pub fn keystore(pub_key: PublicKey) -> Arc<Self> {
        Self::from_signer_type(SignerType::Keystore(KeystoreSigner::new_with_pubkey(
            pub_key,
        )))
    }

    pub async fn keystore_search_for_keys(
        account_id: AccountId,
        network: &NetworkConfig,
    ) -> Result<Arc<Self>, KeyStoreError> {
        Ok(Self::from_signer_type(SignerType::Keystore(
            KeystoreSigner::search_for_keys(account_id, network).await?,
        )))
    }

    #[cfg(feature = "workspaces")]
    pub fn from_workspace(account: &near_workspaces::Account) -> Arc<Self> {
        Self::from_signer_type(SignerType::SecretKey(SecretKeySigner::new(
            account.secret_key().to_string().parse().unwrap(),
        )))
    }

    pub fn custom(signer: Box<dyn SignerTrait>) -> Arc<Self> {
        Self::from_signer_type(SignerType::Custom(signer))
    }
}

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

pub fn get_secret_key_from_seed(
    seed_phrase_hd_path: BIP32Path,
    master_seed_phrase: String,
    password: Option<String>,
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
