use std::str::FromStr;

use near_crypto::PublicKey;
use near_primitives::{
    action::delegate::SignedDelegateAction,
    hash::CryptoHash,
    transaction::SignedTransaction,
    types::{BlockHeight, Nonce},
};
use slipped10::BIP32Path;

use crate::transactions::PrepopulateTransaction;

use self::seed_signer::SeedSigner;

pub mod seed_signer;

pub trait SignerTrait {
    fn sign_meta(
        &self,
        tr: PrepopulateTransaction,
        nonce: Nonce,
        block_hash: CryptoHash,
        max_block_height: BlockHeight,
    ) -> anyhow::Result<SignedDelegateAction>;

    fn sign(
        &self,
        tr: PrepopulateTransaction,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> anyhow::Result<SignedTransaction>;

    fn get_public_key(&self) -> anyhow::Result<PublicKey>;
}

#[derive(Debug, Clone)]
pub enum Signer {
    SeedPhrase(SeedSigner),
}

impl From<Signer> for Box<dyn SignerTrait> {
    fn from(signer: Signer) -> Self {
        match signer {
            Signer::SeedPhrase(seed_signer) => Box::new(seed_signer),
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

    pub fn seed_phrase_with_hd_path(seed_phrase: String, hd_path: BIP32Path) -> Self {
        Self::SeedPhrase(SeedSigner::new(seed_phrase, hd_path))
    }
}
