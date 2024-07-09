use std::{path::PathBuf, str::FromStr};

use near_crypto::{PublicKey, SecretKey};
use near_primitives::{
    action::delegate::SignedDelegateAction,
    hash::CryptoHash,
    transaction::{SignedTransaction, Transaction},
    types::{BlockHeight, Nonce},
};
use slipped10::BIP32Path;

use crate::transactions::PrepopulateTransaction;

use self::{
    access_keyfile_signer::AccessKeyFileSigner, ledger::LedgerSigner, secret_key::SecretKeySigner,
    seed_signer::SeedSigner,
};

pub mod access_keyfile_signer;
#[cfg(feature = "ledger")]
pub mod ledger;
pub mod secret_key;
pub mod seed_signer;

pub trait SignerTrait {
    fn sign_meta(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
        max_block_height: BlockHeight,
    ) -> anyhow::Result<SignedDelegateAction> {
        let (unsigned_transaction, signer_secret_key) =
            self.unsigned_tx(tr, public_key, nonce, block_hash)?;

        get_signed_delegate_action(unsigned_transaction, signer_secret_key, max_block_height)
    }

    fn sign(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> anyhow::Result<SignedTransaction> {
        let (unsigned_transaction, signer_secret_key) =
            self.unsigned_tx(tr, public_key, nonce, block_hash)?;
        let signature = signer_secret_key.sign(unsigned_transaction.get_hash_and_size().0.as_ref());

        Ok(SignedTransaction::new(signature, unsigned_transaction))
    }

    fn unsigned_tx(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> anyhow::Result<(Transaction, SecretKey)>;
    fn get_public_key(&self) -> anyhow::Result<PublicKey>;
}

#[derive(Debug, Clone)]
pub enum Signer {
    SeedPhrase(SeedSigner),
    SecretKey(SecretKeySigner),
    AccessKeyFile(AccessKeyFileSigner),
    #[cfg(feature = "ledger")]
    Ledger(LedgerSigner),
}

impl From<Signer> for Box<dyn SignerTrait> {
    fn from(signer: Signer) -> Self {
        match signer {
            Signer::SeedPhrase(seed_signer) => Box::new(seed_signer),
            Signer::SecretKey(secret_key) => Box::new(secret_key),
            Signer::AccessKeyFile(access_keyfile_signer) => Box::new(access_keyfile_signer),
            Signer::Ledger(ledger_signer) => Box::new(ledger_signer),
        }
    }
}

impl Signer {
    pub fn seed_phrase(seed_phrase: String) -> Self {
        Self::seed_phrase_with_hd_path(
            seed_phrase,
            BIP32Path::from_str("m/44'/397'/0'").expect("Valid HD path"),
        )
    }

    pub fn secret_key(secret_key: SecretKey) -> Self {
        Self::SecretKey(SecretKeySigner::new(secret_key))
    }

    pub fn seed_phrase_with_hd_path(seed_phrase: String, hd_path: BIP32Path) -> Self {
        Self::SeedPhrase(SeedSigner::new(seed_phrase, hd_path))
    }

    pub fn access_keyfile(path: PathBuf) -> Self {
        Self::AccessKeyFile(AccessKeyFileSigner::new(path))
    }

    #[cfg(feature = "ledger")]
    pub fn ledger() -> Self {
        Self::Ledger(LedgerSigner::new(
            BIP32Path::from_str("44'/397'/0'/0'/1'").expect("Valid HD path"),
        ))
    }

    #[cfg(feature = "ledger")]
    pub fn ledger_with_hd_path(hd_path: BIP32Path) -> Self {
        Self::Ledger(LedgerSigner::new(hd_path))
    }
}

pub fn get_signed_delegate_action(
    unsigned_transaction: Transaction,
    private_key: SecretKey,
    max_block_height: u64,
) -> anyhow::Result<SignedDelegateAction> {
    use near_primitives::signable_message::{SignableMessage, SignableMessageType};

    let actions = unsigned_transaction
        .actions
        .into_iter()
        .map(near_primitives::action::delegate::NonDelegateAction::try_from)
        .collect::<Result<_, _>>()
        .map_err(|_| anyhow::anyhow!("Delegate action can't contain delegate action"))?;
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
