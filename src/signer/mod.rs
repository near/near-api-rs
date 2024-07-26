use std::{
    path::{Path, PathBuf},
    str::FromStr,
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

#[derive(Debug, Clone)]
pub enum Signer {
    SecretKey(SecretKeySigner),
    AccessKeyFile(AccessKeyFileSigner),
    #[cfg(feature = "ledger")]
    Ledger(ledger::LedgerSigner),
    Keystore(KeystoreSigner),
}

impl From<Signer> for Box<dyn SignerTrait> {
    fn from(signer: Signer) -> Self {
        match signer {
            Signer::SecretKey(secret_key) => Box::new(secret_key),
            Signer::AccessKeyFile(access_keyfile_signer) => Box::new(access_keyfile_signer),
            #[cfg(feature = "ledger")]
            Signer::Ledger(ledger_signer) => Box::new(ledger_signer),
            Signer::Keystore(keystore_signer) => Box::new(keystore_signer),
        }
    }
}

impl Signer {
    pub fn as_signer(&self) -> &dyn SignerTrait {
        match self {
            Signer::SecretKey(secret_key) => secret_key,
            Signer::AccessKeyFile(access_keyfile_signer) => access_keyfile_signer,
            #[cfg(feature = "ledger")]
            Signer::Ledger(ledger_signer) => ledger_signer,
            Signer::Keystore(keystore_signer) => keystore_signer,
        }
    }

    pub fn seed_phrase(seed_phrase: String, password: Option<String>) -> Result<Self, SecretError> {
        Self::seed_phrase_with_hd_path(
            seed_phrase,
            BIP32Path::from_str("m/44'/397'/0'").expect("Valid HD path"),
            password,
        )
    }

    pub fn secret_key(secret_key: SecretKey) -> Self {
        Self::SecretKey(SecretKeySigner::new(secret_key))
    }

    pub fn seed_phrase_with_hd_path(
        seed_phrase: String,
        hd_path: BIP32Path,
        password: Option<String>,
    ) -> Result<Self, SecretError> {
        let secret_key = get_secret_key_from_seed(hd_path, seed_phrase, password)?;
        Ok(Self::SecretKey(SecretKeySigner::new(secret_key)))
    }

    pub fn access_keyfile(path: PathBuf) -> Result<Self, AccessKeyFileError> {
        Ok(Self::AccessKeyFile(AccessKeyFileSigner::new(path)?))
    }

    #[cfg(feature = "ledger")]
    pub fn ledger() -> Self {
        Self::Ledger(ledger::LedgerSigner::new(
            BIP32Path::from_str("44'/397'/0'/0'/1'").expect("Valid HD path"),
        ))
    }

    #[cfg(feature = "ledger")]
    pub fn ledger_with_hd_path(hd_path: BIP32Path) -> Self {
        Self::Ledger(ledger::LedgerSigner::new(hd_path))
    }

    pub fn keystore(pub_key: PublicKey) -> Self {
        Self::Keystore(KeystoreSigner::new_with_pubkey(pub_key))
    }

    pub async fn keystore_search_for_keys(
        account_id: AccountId,
        network: &NetworkConfig,
    ) -> Result<Self, KeyStoreError> {
        Ok(Self::Keystore(
            KeystoreSigner::search_for_keys(account_id, network).await?,
        ))
    }

    #[cfg(feature = "workspaces")]
    pub fn from_workspace(account: &near_workspaces::Account) -> Self {
        // TODO: remove this unwrap
        Self::secret_key(account.secret_key().to_string().parse().unwrap())
    }
}

pub fn get_signed_delegate_action(
    unsigned_transaction: Transaction,
    private_key: SecretKey,
    max_block_height: u64,
) -> core::result::Result<SignedDelegateAction, MetaSignError> {
    use near_primitives::signable_message::{SignableMessage, SignableMessageType};

    let actions = unsigned_transaction
        .actions
        .into_iter()
        .map(near_primitives::action::delegate::NonDelegateAction::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| MetaSignError::DelegateActionIsNotSupported)?;
    let delegate_action = near_primitives::action::delegate::DelegateAction {
        sender_id: unsigned_transaction.signer_id.clone(),
        receiver_id: unsigned_transaction.receiver_id,
        actions,
        nonce: unsigned_transaction.nonce,
        max_block_height,
        public_key: unsigned_transaction.public_key,
    };

    // create a new signature here signing the delegate action + discriminant
    let signable = SignableMessage::new(&delegate_action, SignableMessageType::DelegateAction);
    let signer =
        near_crypto::InMemorySigner::from_secret_key(unsigned_transaction.signer_id, private_key);
    let signature = signable.sign(&signer);

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
