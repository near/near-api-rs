use std::str::FromStr;

use near_crypto::{PublicKey, SecretKey};
use near_primitives::{hash::CryptoHash, transaction::Transaction, types::Nonce};
use slipped10::BIP32Path;

use crate::transactions::PrepopulateTransaction;

use super::SignerTrait;

#[derive(Debug, Clone)]
pub struct SeedSigner {
    seed_phrase: String,
    hd_path: BIP32Path,
}

impl SignerTrait for SeedSigner {
    fn unsigned_tx(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> anyhow::Result<(Transaction, SecretKey)> {
        let key_pair_properties = get_key_pair_properties_from_seed_phrase(
            self.hd_path.clone(),
            self.seed_phrase.clone(),
        )?;

        let signer_secret_key: SecretKey =
            SecretKey::from_str(&key_pair_properties.secret_keypair_str)?;

        Ok((
            near_primitives::transaction::Transaction {
                public_key,
                block_hash,
                nonce,
                signer_id: tr.signer_id.clone(),
                receiver_id: tr.receiver_id.clone(),
                actions: tr.actions.clone(),
            },
            signer_secret_key,
        ))
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
