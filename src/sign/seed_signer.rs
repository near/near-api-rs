use std::str::FromStr;

use near_crypto::{PublicKey, SecretKey};
use near_primitives::{
    action::delegate::SignedDelegateAction,
    hash::CryptoHash,
    transaction::{SignedTransaction, Transaction},
    types::{BlockHeight, Nonce},
};
use slipped10::BIP32Path;

use crate::transactions::PrepopulateTransaction;

use super::SignerTrait;

#[derive(Debug, Clone)]
pub struct SeedSigner {
    seed_phrase: String,
    hd_path: BIP32Path,
}

impl SignerTrait for SeedSigner {
    fn sign_meta(
        &self,
        tr: PrepopulateTransaction,
        nonce: Nonce,
        block_hash: CryptoHash,
        max_block_height: BlockHeight,
    ) -> anyhow::Result<SignedDelegateAction> {
        let (unsigned_transaction, signer_secret_key) = self.unsigned_tx(tr, nonce, block_hash)?;

        get_signed_delegate_action(unsigned_transaction, signer_secret_key, max_block_height)
    }

    fn sign(
        &self,
        tr: PrepopulateTransaction,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> anyhow::Result<SignedTransaction> {
        let (unsigned_transaction, signer_secret_key) = self.unsigned_tx(tr, nonce, block_hash)?;
        let signature = signer_secret_key.sign(unsigned_transaction.get_hash_and_size().0.as_ref());

        Ok(SignedTransaction::new(signature, unsigned_transaction))
    }

    fn get_public_key(&self) -> anyhow::Result<PublicKey> {
        let key_pair_properties = get_key_pair_properties_from_seed_phrase(
            self.hd_path.clone(),
            self.seed_phrase.clone(),
        )?;

        Ok(PublicKey::from_str(&key_pair_properties.public_key_str)?)
    }
}

impl SeedSigner {
    pub fn new(seed_phrase: String, hd_path: BIP32Path) -> Self {
        Self {
            seed_phrase,
            hd_path,
        }
    }

    fn unsigned_tx(
        &self,
        tr: PrepopulateTransaction,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> anyhow::Result<(Transaction, SecretKey)> {
        let key_pair_properties = get_key_pair_properties_from_seed_phrase(
            self.hd_path.clone(),
            self.seed_phrase.clone(),
        )?;

        let signer_secret_key: SecretKey =
            SecretKey::from_str(&key_pair_properties.secret_keypair_str)?;
        let signer_public_key = PublicKey::from_str(&key_pair_properties.public_key_str)?;

        Ok((
            near_primitives::transaction::Transaction {
                public_key: signer_public_key.clone(),
                block_hash,
                nonce,
                signer_id: tr.signer_id.clone(),
                receiver_id: tr.receiver_id.clone(),
                actions: tr.actions.clone(),
            },
            signer_secret_key,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct KeyPairProperties {
    pub seed_phrase_hd_path: BIP32Path,
    pub master_seed_phrase: String,
    pub implicit_account_id: near_primitives::types::AccountId,
    pub public_key_str: String,
    pub secret_keypair_str: String,
}

pub fn get_key_pair_properties_from_seed_phrase(
    seed_phrase_hd_path: BIP32Path,
    master_seed_phrase: String,
) -> anyhow::Result<KeyPairProperties> {
    let master_seed = bip39::Mnemonic::parse(&master_seed_phrase)?.to_seed("");
    let derived_private_key = slipped10::derive_key_from_path(
        &master_seed,
        slipped10::Curve::Ed25519,
        &seed_phrase_hd_path,
    )
    .map_err(|err| anyhow::anyhow!("Failed to derive a key from the master key: {}", err))?;

    let signing_key = ed25519_dalek::SigningKey::from_bytes(&derived_private_key.key);

    let public_key = signing_key.verifying_key();
    let implicit_account_id = near_primitives::types::AccountId::try_from(hex::encode(public_key))?;
    let public_key_str = format!("ed25519:{}", bs58::encode(&public_key).into_string());
    let secret_keypair_str = format!(
        "ed25519:{}",
        bs58::encode(signing_key.to_keypair_bytes()).into_string()
    );
    let key_pair_properties: KeyPairProperties = KeyPairProperties {
        seed_phrase_hd_path,
        master_seed_phrase,
        implicit_account_id,
        public_key_str,
        secret_keypair_str,
    };
    Ok(key_pair_properties)
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
