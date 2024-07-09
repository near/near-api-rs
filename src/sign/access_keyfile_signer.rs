use std::path::{Path, PathBuf};

use anyhow::Context;
use near_crypto::{PublicKey, SecretKey};
use near_primitives::{hash::CryptoHash, transaction::Transaction, types::Nonce};
use serde::Deserialize;

use crate::transactions::PrepopulateTransaction;

use super::SignerTrait;

#[derive(Debug, Clone)]
pub struct AccessKeyFileSigner {
    path: PathBuf,
}

impl AccessKeyFileSigner {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl SignerTrait for AccessKeyFileSigner {
    fn unsigned_tx(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> anyhow::Result<(Transaction, SecretKey)> {
        let signer = load_access_key_file(&self.path)?;

        Ok((
            near_primitives::transaction::Transaction {
                public_key,
                block_hash,
                nonce,
                signer_id: tr.signer_id.clone(),
                receiver_id: tr.receiver_id.clone(),
                actions: tr.actions.clone(),
            },
            signer.private_key,
        ))
    }

    fn get_public_key(&self) -> anyhow::Result<PublicKey> {
        let key_pair_properties = load_access_key_file(&self.path)?;

        Ok(key_pair_properties.public_key)
    }
}

#[derive(Debug, Deserialize)]
pub struct AccountKeyPair {
    pub public_key: near_crypto::PublicKey,
    pub private_key: near_crypto::SecretKey,
}

fn load_access_key_file(path: &Path) -> anyhow::Result<AccountKeyPair> {
    let data = std::fs::read_to_string(path).context("Access key file not found!")?;
    serde_json::from_str(&data)
        .with_context(|| format!("Error reading data from file: {:?}", &path))
}
